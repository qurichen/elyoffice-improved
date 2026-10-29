use std::{cell::RefCell, rc::Rc};

use ely_gpui_component::tables::{FilterRule, Test};
use ely_lok::Request;
use gpui::{MouseButton, ParentElement as _, Styled as _, VisualTestContext, point, px};

use super::*;
use super::data::{asked, cells, quiet, run};

const DATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell office:value-type="string"><text:p>K</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="3"><text:p>3</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="1"><text:p>1</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="2"><text:p>2</text:p></table:table-cell></table:table-row>
 <table:table-row table:number-rows-repeated="3"><table:table-cell/></table:table-row>
 <table:table-row><table:table-cell office:value-type="string"><text:p>L</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="9"><text:p>9</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="7"><text:p>7</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="8"><text:p>8</text:p></table:table-cell></table:table-row>
 </table:table><table:table table:name="T">
 <table:table-row><table:table-cell office:value-type="string"><text:p>Other</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="6"><text:p>6</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="4"><text:p>4</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="5"><text:p>5</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

fn open(cx: &mut TestAppContext, book: &Entity<Book>) {
    update(cx, book, |book, cx| book.new_from_text("context.fods", DATA, cx));
    settle(cx, |app| sheet_names(app, book) == ["S", "T"]);
    fresh(cx, book);
}

fn errors(cx: &TestAppContext, book: &Entity<Book>) -> Rc<RefCell<Vec<String>>> {
    let errors = Rc::new(RefCell::new(Vec::new()));
    cx.update(|app| {
        let view = book.read(app).active_tab().expect("a tab").view.clone();
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &ely_lok::LokError, _| errors.borrow_mut().push(error.0.clone())).detach();
    });
    errors
}

fn saved(cx: &TestAppContext, book: &Entity<Book>, path: &Path) -> String {
    quiet(cx, book);
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        tab.doc.save_as(&mut book.core.worker.lock().expect("the worker lock"),
            &ely_lok::file_url(path), "fods").expect("saves");
    });
    std::fs::read_to_string(path).expect("the saved workbook")
}

fn sheet(cx: &mut TestAppContext, book: &Entity<Book>, index: usize) {
    update(cx, book, |book, cx| book.show_sheet(index, cx));
    fresh(cx, book);
}

fn column_menu(cx: &mut TestAppContext, book: &Entity<Book>) {
    run(cx, book, "cell.goto", &["A1"]);
    run(cx, book, "data.autofilter", &[]);
    sheet(cx, book, 1);
    run(cx, book, "cell.goto", &["A1"]);
    run(cx, book, "data.autofilter", &[]);
    sheet(cx, book, 0);
    let tab = cx.update(|app| book.read(app).core.active.expect("a tab"));
    update(cx, book, |book, cx| book.ask_column(tab, &Request::Filter { column: 0, row: 0 }, cx));
    asked(cx, book, |dialog| matches!(dialog, Dialog::Column { .. }));
}

#[test]
fn sort_confirmation_cannot_follow_another_range() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    let errors = errors(&cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1:A4"]);
    update(&mut cx, &book, |book, cx| book.ask_data(crate::data::Asked::Sort, cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Sort { .. }));
    run(&mut cx, &book, "cell.goto", &["A8:A11"]);
    update(&mut cx, &book, |book, cx| book.data_asked(cx));
    assert_eq!(cells(&mut cx, &book, "A9:A11"), "9\n7\n8", "the new range must not be sorted");
    assert_eq!(cells(&mut cx, &book, "A2:A4"), "3\n1\n2");
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)));
}

#[test]
fn filter_confirmation_cannot_follow_another_sheet() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    let errors = errors(&cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1:A4"]);
    update(&mut cx, &book, |book, cx| book.ask_data(crate::data::Asked::Filter, cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Filter { .. }));
    update(&mut cx, &book, |book, _| {
        let Some(Dialog::Filter { rules, .. }) = &mut book.dialog else { unreachable!("the filter") };
        *rules = vec![FilterRule { column: "0".into(), test: Test::Equals, value: "1".into() }];
    });
    sheet(&mut cx, &book, 1);
    update(&mut cx, &book, |book, cx| book.data_asked(cx));
    let xml = saved(&cx, &book, &dir.path().join("filter.fods"));
    assert!(!xml.contains("table:visibility=\"filter\""), "the new sheet must not be filtered");
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)));
}

#[test]
fn columns_confirmation_cannot_follow_another_range() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    let errors = errors(&cx, &book);
    for (cell, text) in [("A2", "one;two"), ("C2", "left;right")] {
        run(&mut cx, &book, "cell.goto", &[cell]);
        run(&mut cx, &book, "cell.enter", &[text]);
    }
    run(&mut cx, &book, "cell.goto", &["A2"]);
    update(&mut cx, &book, |book, cx| book.ask_columns(cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Columns { .. }));
    update(&mut cx, &book, |book, _| {
        let Some(Dialog::Columns { separators, .. }) = &mut book.dialog else { unreachable!("the split") };
        *separators = vec!["59".into()];
    });
    run(&mut cx, &book, "cell.goto", &["C2"]);
    update(&mut cx, &book, |book, cx| book.columns_asked(cx));
    assert_eq!(cells(&mut cx, &book, "C2"), "left;right", "the new range must not be split");
    let filled = cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let json = {
            let mut worker = book.core.worker.lock().expect("the worker lock");
            worker.run(&tab.doc, "cell.goto", &["D2"]).expect("the next cell");
            worker.values(&tab.doc, ely_lok::sheet::SELECTED)
        }.expect("the selection");
        ely_lok::sheet::selected_range(&json).expect("the range").expect("one cell").filled
    });
    assert_eq!(filled, 0, "the adjacent cell retains no stored content");
    assert_eq!(cells(&mut cx, &book, "A2"), "one;two");
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)));
}

#[test]
fn column_sort_cannot_follow_another_sheet() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    let errors = errors(&cx, &book);
    column_menu(&mut cx, &book);
    sheet(&mut cx, &book, 1);
    update(&mut cx, &book, |book, cx| book.column_asked(Some(true), cx));
    assert_eq!(cells(&mut cx, &book, "A2:A4"), "6\n4\n5", "the menu must not sort another sheet");
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)));
}

#[test]
fn column_filter_cannot_follow_another_sheet() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    let errors = errors(&cx, &book);
    column_menu(&mut cx, &book);
    update(&mut cx, &book, |book, _| {
        let Some(Dialog::Column { picked, .. }) = &mut book.dialog else { unreachable!("the column") };
        *picked = vec!["1".into()];
    });
    sheet(&mut cx, &book, 1);
    update(&mut cx, &book, |book, cx| book.column_asked(None, cx));
    let xml = saved(&cx, &book, &dir.path().join("column-filter.fods"));
    assert!(!xml.contains("table:visibility=\"filter\""), "the menu must not filter another sheet");
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)));
}

struct Grid(Entity<Book>);

impl gpui::Render for Grid {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        let (view, dialog) = self.0.update(cx, |book, cx| {
            (book.active_tab().map(|tab| tab.view.clone()), book.dialog_element(cx))
        });
        gpui::div().size_full().relative()
            .child(gpui::div().w(px(450.0)).h(px(300.0)).children(view)).children(dialog)
    }
}

#[test]
fn a_filter_arrow_press_and_its_release_keep_the_menu_usable() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1:A4"]);
    run(&mut cx, &book, "data.autofilter", &[]);
    fresh(&cx, &book);
    let errors = errors(&cx, &book);
    let window = cx.add_window(|_, _| Grid(book.clone()));
    update(&mut cx, &book, |book, _| book.core.window = Some(window.into()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    shown.run_until_parked();
    quiet(&cx, &book);
    let (view, on, requests) = cx.update(|app| {
        let tab = book.read(app).active_tab().expect("a tab");
        let view = tab.view.read(app);
        let height = tab.extra.geometry.rows.size(0);
        let width = tab.extra.geometry.columns.size(0);
        assert!(view.viewport().0 > 0.0 && height > 0 && width > 0);
        let on = view.origin() + view.place((width - height / 2, height / 2));
        (tab.view.clone(), on, view.cell().requests)
    });
    shown.simulate_mouse_down(on, MouseButton::Left, Default::default());
    settle(&cx, |app| view.read(app).settled() && view.read(app).cell().requests > requests);
    let sent = cx.update(|app| view.read(app).sent());
    shown.run_until_parked();
    shown.simulate_mouse_up(point(px(500.0), px(250.0)), MouseButton::Left, Default::default());
    quiet(&cx, &book);
    assert!(cx.update(|app| view.read(app).sent()) > sent, "the paired release reached the view");
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Column { .. }));
    update(&mut cx, &book, |book, cx| book.column_asked(Some(true), cx));
    assert_eq!(cells(&mut cx, &book, "A2:A4"), "1\n2\n3", "the completed click may sort its original block");
    assert!(errors.borrow().is_empty(), "the trigger release is not a stale edit: {:?}", errors.borrow());
}

#[test]
fn a_held_filter_request_cannot_move_to_another_sheet() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    for index in [0, 1] {
        sheet(&mut cx, &book, index);
        run(&mut cx, &book, "cell.goto", &["A1:A4"]);
        run(&mut cx, &book, "data.autofilter", &[]);
    }
    sheet(&mut cx, &book, 0);
    let errors = errors(&cx, &book);
    let window = cx.add_window(|_, _| Grid(book.clone()));
    update(&mut cx, &book, |book, _| book.core.window = Some(window.into()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    shown.run_until_parked();
    quiet(&cx, &book);
    let (view, on, requests) = cx.update(|app| {
        let tab = book.read(app).active_tab().expect("a tab");
        let view = tab.view.read(app);
        let height = tab.extra.geometry.rows.size(0);
        let width = tab.extra.geometry.columns.size(0);
        assert!(view.viewport().0 > 0.0 && height > 0 && width > 0);
        let on = view.origin() + view.place((width - height / 2, height / 2));
        (tab.view.clone(), on, view.cell().requests)
    });
    shown.simulate_mouse_down(on, MouseButton::Left, Default::default());
    settle(&cx, |app| view.read(app).settled() && view.read(app).cell().requests > requests);
    assert!(cx.update(|app| book.read(app).dialog.is_none()), "the held request has not opened a menu");
    sheet(&mut cx, &book, 1);
    let sent = cx.update(|app| view.read(app).sent());
    shown.run_until_parked();
    shown.simulate_mouse_up(point(px(500.0), px(250.0)), MouseButton::Left, Default::default());
    quiet(&cx, &book);
    assert!(cx.update(|app| view.read(app).sent()) > sent, "the paired release reached the view");
    settle(&cx, |app| book.read(app).dialog.as_ref().is_some_and(|d| matches!(d, Dialog::Column { .. })) || !errors.borrow().is_empty());
    update(&mut cx, &book, |book, cx| book.column_asked(Some(true), cx));
    assert_eq!(cells(&mut cx, &book, "A2:A4"), "6\n4\n5", "the held request must not be rebound to the new sheet");
    assert!(errors.borrow().iter().any(|text| text.starts_with(ely_lok::STALE)));
}

#[test]
fn a_quick_filter_click_keeps_its_release_context() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1:A4"]);
    run(&mut cx, &book, "data.autofilter", &[]);
    fresh(&cx, &book);
    let errors = errors(&cx, &book);
    let window = cx.add_window(|_, _| Grid(book.clone()));
    update(&mut cx, &book, |book, _| book.core.window = Some(window.into()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    shown.run_until_parked();
    quiet(&cx, &book);
    let (view, on) = cx.update(|app| {
        let tab = book.read(app).active_tab().expect("a tab");
        let view = tab.view.read(app);
        let height = tab.extra.geometry.rows.size(0);
        let width = tab.extra.geometry.columns.size(0);
        assert!(view.viewport().0 > 0.0 && height > 0 && width > 0);
        (tab.view.clone(), view.origin() + view.place((width - height / 2, height / 2)))
    });
    shown.update(|window, app| {
        window.dispatch_event(gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
            position: on, modifiers: Default::default(), button: MouseButton::Left,
            click_count: 1, first_mouse: false,
        }), app);
        window.dispatch_event(gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
            position: point(px(500.0), px(250.0)), modifiers: Default::default(),
            button: MouseButton::Left, click_count: 1,
        }), app);
        assert!(!view.read(app).settled(), "both physical events precede their landed callbacks");
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Column { .. }));
    update(&mut cx, &book, |book, cx| book.column_asked(Some(true), cx));
    assert_eq!(cells(&mut cx, &book, "A2:A4"), "1\n2\n3");
    assert!(errors.borrow().is_empty(), "a quick click remains usable: {:?}", errors.borrow());
}
