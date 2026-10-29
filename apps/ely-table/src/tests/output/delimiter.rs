use super::*;

fn check_import(name: &str, bytes: Vec<u8>) {
    let dir = tempfile::tempdir().expect("an owned fixture folder");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let input = dir.path().join("quoted.csv");
    std::fs::write(&input, &bytes).expect("the original input");
    update(&mut cx, &book, |book, cx| book.ask_import(input.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(Dialog::Import(pending)) => {
            assert_eq!(pending.how.separator, ';');
            assert_eq!(pending.rows[0], ["姓名,a,b", "00123", "12345678901234567890"]);
            assert_eq!(pending.how.columns, [Column::Standard, Column::Text, Column::Text]);
        }
        _ => unreachable!("the import choices"),
    });
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == "quoted.csv"));
    let expected = [("A1", "姓名,a,b"), ("B1", "00123"), ("C1", "12345678901234567890"),
                    ("A2", "行😀,c,d"), ("B2", "00456"), ("C2", "98765432109876543210")];
    for (cell, text) in expected { assert_eq!(cells(&mut cx, &book, cell), text); }
    let saved = dir.path().join("quoted.xlsx");
    let id = cx.update(|app| book.read(app).active_tab().unwrap().id);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.write_as(id, saved.clone(), None, window, cx))
    }).expect("the test window");
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == "quoted.xlsx"));
    assert_eq!(std::fs::read(&input).expect("input stays"), bytes);
    if let Ok(root) = std::env::var("ELY_IMPORT_EVIDENCE") {
        let evidence = Path::new(&root).join(name);
        std::fs::create_dir(&evidence).expect("a fresh owned evidence folder");
        std::fs::copy(&input, evidence.join("input.csv")).expect("input evidence");
        std::fs::copy(&saved, evidence.join("saved.xlsx")).expect("saved evidence");
    }
    update(&mut cx, &book, |book, cx| {
        book.close_tab(id, cx);
        book.open_file(saved.clone(), cx);
    });
    settle(&cx, |app| book.read(app).active_tab().is_some());
    for (cell, text) in expected { assert_eq!(cells(&mut cx, &book, cell), text); }
}

const CSV: &str = "\"姓名,a,b\";00123;12345678901234567890\r\n\"行😀,c,d\";00456;98765432109876543210\r\n";

#[test]
fn utf8_delimiter_preview_import_save_and_reopen_preserve_columns() {
    check_import("utf8", CSV.as_bytes().to_vec());
}

#[test]
fn utf16_delimiter_preview_import_save_and_reopen_preserve_columns() {
    let mut bytes = vec![0xff, 0xfe];
    for ch in CSV.encode_utf16() { bytes.extend(ch.to_le_bytes()); }
    check_import("utf16le", bytes);
}
