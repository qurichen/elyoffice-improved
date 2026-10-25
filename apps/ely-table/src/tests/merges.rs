use super::data::{asked, cells, run};
use super::*;
use gpui::{KeyUpEvent, Keystroke, ParentElement as _, Styled as _, VisualTestContext};

const DATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:of="urn:oasis:names:tc:opendocument:xmlns:of:1.2"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell table:number-columns-repeated="2"/></table:table-row>
 <table:table-row><table:table-cell office:value-type="string"><text:p>First</text:p><text:p>Second</text:p></table:table-cell><table:table-cell/></table:table-row>
 <table:table-row><table:table-cell office:value-type="string"><text:p>Visible</text:p></table:table-cell><table:table-cell table:formula="of:=&quot;&quot;" office:value-type="string" office:string-value=""><text:p/></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="string"><text:p>Left</text:p></table:table-cell><table:table-cell office:value-type="string"><text:p>Right</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

struct DialogHost(Entity<Book>);

impl gpui::Render for DialogHost {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        let dialog = self.0.update(cx, |book, cx| book.dialog_element(cx));
        gpui::div().size_full().children(dialog)
    }
}

fn open(cx: &mut TestAppContext, book: &Entity<Book>) {
    update(cx, book, |book, cx| book.new_from_text("merge.fods", DATA, cx));
    settle(cx, |app| sheet_names(app, book) == ["S"]);
    fresh(cx, book);
}

fn merge(cx: &mut TestAppContext, book: &Entity<Book>, range: &str) {
    run(cx, book, "cell.goto", &[range]);
    fresh(cx, book);
    let sent = cx.update(|app| book.read(app).active_tab().expect("a tab").view.read(app).sent());
    update(cx, book, |book, cx| book.merge(book.core.active, cx));
    settle(cx, |app| {
        let book = book.read(app);
        book.dialog.is_some() || book.active_tab().is_some_and(|tab| {
            let view = tab.view.read(app);
            view.sent() > sent && view.settled()
        })
    });
}

fn saved(cx: &mut TestAppContext, book: &Entity<Book>, path: &Path) -> String {
    fresh(cx, book);
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        tab.doc.save_as(&mut worker, &ely_lok::file_url(path), "fods").expect("saves");
    });
    std::fs::read_to_string(path).expect("the saved sheet")
}

fn spans(xml: &str) -> usize {
    xml.matches("table:number-columns-spanned=\"2\"").count()
}

fn choose(cx: &mut VisualTestContext, tabs: usize) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    for key in std::iter::repeat_n("tab", tabs).chain(["enter"]) {
        cx.simulate_keystrokes(key);
        cx.simulate_event(KeyUpEvent { keystroke: Keystroke::parse(key).expect("a key") });
        cx.run_until_parked();
    }
}

#[test]
fn empty_cells_merge_and_split_without_a_text_selection() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    merge(&mut cx, &book, "A1:B1");
    cx.update(|app| assert!(book.read(app).dialog.is_none(), "empty cells need no confirmation"));
    let path = dir.path().join("merged.fods");
    assert_eq!(spans(&saved(&mut cx, &book, &path)), 1, "the merge reached the file");
    run(&mut cx, &book, "cells.split", &[]);
    assert_eq!(spans(&saved(&mut cx, &book, &path)), 0, "split restores separate cells");
    assert_eq!(cells(&mut cx, &book, "A4:B4"), "Left\tRight", "other data survives");
}

#[test]
fn a_multiline_cell_is_one_filled_cell_when_merging() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    merge(&mut cx, &book, "A2:B2");
    cx.update(|app| assert!(book.read(app).dialog.is_none(), "two lines are not two cells"));
    let xml = saved(&mut cx, &book, &dir.path().join("multiline.fods"));
    assert_eq!(spans(&xml), 1);
    assert!(xml.contains("<text:p>First</text:p>"));
    assert!(xml.contains("<text:p>Second</text:p>"));
}

#[test]
fn empty_result_formulas_require_confirmation_and_cancel_preserves_them() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    let window = cx.add_window(|_, _| DialogHost(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    merge(&mut cx, &book, "A3:B3");
    cx.update(|app| assert!(matches!(book.read(app).dialog, Some(Dialog::Merge { .. })), "a formula still holds data"));
    choose(&mut shown, 1);
    settle(&cx, |app| book.read(app).dialog.is_none());
    let xml = saved(&mut cx, &book, &dir.path().join("cancelled.fods"));
    assert_eq!(spans(&xml), 0, "cancel changes no geometry");
    assert!(xml.contains("table:formula=\"of:=&quot;&quot;\""), "cancel retains the formula");
    assert_eq!(cells(&mut cx, &book, "A3"), "Visible");
}

#[test]
fn merge_choices_keep_or_move_the_other_cells_content() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    let window = cx.add_window(|_, _| DialogHost(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    for (tabs, moved) in [(2, false), (3, true)] {
        merge(&mut cx, &book, "A4:B4");
        asked(&cx, &book, |dialog| matches!(dialog, Dialog::Merge { .. }));
        choose(&mut shown, tabs);
        settle(&cx, |app| book.read(app).dialog.is_none());
        let xml = saved(&mut cx, &book, &dir.path().join(format!("choice-{tabs}.fods")));
        assert_eq!(spans(&xml), 1, "the button merges");
        run(&mut cx, &book, "cells.split", &[]);
        if moved {
            let text = cells(&mut cx, &book, "A4");
            assert!(text.contains("Left") && text.contains("Right"), "both move into A4: {text}");
        } else {
            assert_eq!(cells(&mut cx, &book, "A4:B4"), "Left\tRight", "keep hides without discarding");
        }
    }
}

#[test]
fn a_merge_confirmation_cannot_follow_a_new_selection() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    let errors = stale_errors(&cx, &book);
    let window = cx.add_window(|_, _| DialogHost(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    merge(&mut cx, &book, "A4:B4");
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Merge { .. }));
    run(&mut cx, &book, "cell.goto", &["A3:B3"]);
    choose(&mut shown, 2);
    let xml = saved(&mut cx, &book, &dir.path().join("stale-selection.fods"));
    assert_eq!(spans(&xml), 0, "neither range merges");
    assert!(xml.contains("table:formula=\"of:=&quot;&quot;\""));
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)), "the stale decision is reported");
    assert_eq!(cells(&mut cx, &book, "A4:B4"), "Left\tRight");
}

#[test]
fn a_merge_confirmation_cannot_follow_a_sheet_switch() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    run(&mut cx, &book, "sheet.insert", &["Second", "2"]);
    update(&mut cx, &book, |book, cx| book.show_sheet(0, cx));
    fresh(&cx, &book);
    let errors = stale_errors(&cx, &book);
    let window = cx.add_window(|_, _| DialogHost(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    merge(&mut cx, &book, "A4:B4");
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Merge { .. }));
    update(&mut cx, &book, |book, cx| book.show_sheet(1, cx));
    fresh(&cx, &book);
    choose(&mut shown, 2);
    let xml = saved(&mut cx, &book, &dir.path().join("stale-sheet.fods"));
    assert_eq!(spans(&xml), 0);
    assert!(xml.contains("<text:p>Left</text:p>") && xml.contains("<text:p>Right</text:p>"));
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)), "the old sheet's decision is rejected");
}

fn stale_errors(cx: &TestAppContext, book: &Entity<Book>) -> std::rc::Rc<std::cell::RefCell<Vec<String>>> {
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|app| {
        let view = book.read(app).active_tab().expect("a tab").view.clone();
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &ely_lok::LokError, _| errors.borrow_mut().push(error.0.clone())).detach();
    });
    errors
}
