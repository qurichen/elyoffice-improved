use super::*;
use gpui::{Modifiers, ParentElement as _, Styled as _, VisualTestContext};

struct ImportHost(Entity<Book>);

impl gpui::Render for ImportHost {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        let dialog = self.0.update(cx, |book, cx| book.dialog_element(cx));
        gpui::div().relative().size_full().children(dialog)
    }
}

fn frames(shown: &mut VisualTestContext) {
    for _ in 0..3 {
        shown.update(|window, _| window.refresh());
        shown.run_until_parked();
    }
}

fn start(cx: &mut TestAppContext, dir: &Path, bytes: &[u8]) -> Option<(WindowHandle<ImportHost>, Entity<Book>, VisualTestContext)> {
    let (_, book) = book(cx, dir)?;
    cx.update(ely_gpui_component::init_for_tests);
    let input = dir.join("source.csv");
    std::fs::write(&input, bytes).expect("an owned input");
    update(cx, &book, |book, cx| book.ask_import(input, cx));
    asked(cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    let window = cx.add_window(|_, _| ImportHost(book.clone()));
    cx.update(|app| book.update(app, |book, _| book.core.window = Some(window.into())));
    let mut shown = VisualTestContext::from_window(window.into(), cx);
    frames(&mut shown);
    Some((window, book, shown))
}

fn toggle(shown: &mut VisualTestContext) {
    frames(shown);
    let label = shown.debug_bounds("switch-label").expect("the keep-all-fields-as-text switch");
    shown.simulate_click(label.center(), Modifiers::none());
    frames(shown);
}

fn evidence(name: &str, dir: &Path, saved: &Path) {
    if let Ok(root) = std::env::var("ELY_TEXT_IMPORT_EVIDENCE") {
        let target = Path::new(&root).join(name);
        std::fs::create_dir(&target).expect("a fresh evidence folder");
        std::fs::copy(dir.join("source.csv"), target.join("input.csv")).expect("input evidence");
        std::fs::copy(saved, target.join("saved.xlsx")).expect("saved evidence");
    }
}

fn saved_and_reopened(cx: &mut TestAppContext, window: WindowHandle<ImportHost>, book: &Entity<Book>, dir: &Path, name: &str, expected: &[(&str, &str)]) {
    for (cell, text) in expected { assert_eq!(cells(cx, book, cell), text.trim(), "{cell}"); }
    let saved = dir.join("kept.xlsx");
    let id = cx.update(|app| book.read(app).active_tab().expect("the imported tab").id);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.write_as(id, saved.clone(), None, window, cx))
    }).expect("the import window");
    settle(cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == "kept.xlsx"));
    evidence(name, dir, &saved);
    update(cx, book, |book, cx| {
        book.close_tab(id, cx);
        book.open_file(saved.clone(), cx);
    });
    settle(cx, |app| book.read(app).active_tab().is_some());
    for (cell, text) in expected { assert_eq!(cells(cx, book, cell), text.trim(), "reopened {cell}"); }
}

const CSV: &str = "00123;12345678901234567890;1.234567890123456789E+19;2026-10-28;0.12345678901234567890;=1+2;-0;中文😀;\"  padded  \"\r\n";
const VALUES: [(&str, &str); 9] = [("A1", "00123"), ("B1", "12345678901234567890"),
    ("C1", "1.234567890123456789E+19"), ("D1", "2026-10-28"),
    ("E1", "0.12345678901234567890"), ("F1", "=1+2"), ("G1", "-0"),
    ("H1", "中文😀"), ("I1", "  padded  ")];

fn text_roundtrip(name: &str, bytes: Vec<u8>) {
    let dir = tempfile::tempdir().expect("an owned folder");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((window, book, mut shown)) = start(&mut cx, dir.path(), &bytes) else {
        eprintln!("SKIP: no LOK build"); return;
    };
    toggle(&mut shown);
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some());
    saved_and_reopened(&mut cx, window, &book, dir.path(), name, &VALUES);
    assert_eq!(std::fs::read(dir.path().join("source.csv")).expect("input remains"), bytes);
}

#[test]
fn utf8_text_only_preserves_lexemes_through_save_and_reopen() {
    text_roundtrip("utf8", CSV.as_bytes().to_vec());
}

#[test]
fn utf16_text_only_preserves_lexemes_through_save_and_reopen() {
    let mut bytes = vec![0xff, 0xfe];
    for unit in CSV.encode_utf16() { bytes.extend(unit.to_le_bytes()); }
    text_roundtrip("utf16le", bytes);
}

#[test]
fn turning_text_only_off_restores_previous_column_choices() {
    let dir = tempfile::tempdir().expect("an owned folder");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_, book, mut shown)) = start(&mut cx, dir.path(), b"7;8\n") else {
        eprintln!("SKIP: no LOK build"); return;
    };
    update(&mut cx, &book, |book, _| match book.dialog.as_mut() {
        Some(Dialog::Import(pending)) => pending.how.columns = vec![Column::Skip, Column::DateYmd],
        _ => unreachable!("the import choices"),
    });
    toggle(&mut shown);
    toggle(&mut shown);
    cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(Dialog::Import(pending)) => assert_eq!(pending.how.columns, [Column::Skip, Column::DateYmd]),
        _ => unreachable!("the import choices"),
    });
}

#[test]
fn text_only_survives_rescan_and_covers_late_wider_rows() {
    let dir = tempfile::tempdir().expect("an owned folder");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let text = "7;1.50\n".repeat(10) + "007;2.50;1E+020\n";
    let Some((_, book, mut shown)) = start(&mut cx, dir.path(), text.as_bytes()) else {
        eprintln!("SKIP: no LOK build"); return;
    };
    for separator in [',', ';'] {
        if separator == ';' { toggle(&mut shown); }
        update(&mut cx, &book, |book, cx| {
            if let Some(Dialog::Import(pending)) = &mut book.dialog { pending.how.separator = separator; }
            book.scan_import(cx);
        });
        asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    }
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some());
    for (cell, text) in [("B1", "1.50"), ("A11", "007"), ("C11", "1E+020")] {
        assert_eq!(cells(&mut cx, &book, cell), text);
    }
}

#[test]
fn ordinary_import_still_reads_numbers_without_opt_in() {
    let dir = tempfile::tempdir().expect("an owned folder");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let bytes = b"7;1.50;=1+2\n";
    let Some((window, book, _shown)) = start(&mut cx, dir.path(), bytes) else {
        eprintln!("SKIP: no LOK build"); return;
    };
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some());
    saved_and_reopened(&mut cx, window, &book, dir.path(), "normal", &[("A1", "7"), ("B1", "1.5"), ("C1", "=1+2")]);
    assert_eq!(std::fs::read(dir.path().join("source.csv")).expect("input remains"), bytes);
}
