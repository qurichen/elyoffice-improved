use super::*;

#[test]
fn the_default_encoding_follows_the_apps_language() {
    use ely_gpui_component::i18n::I18n;
    use ely_lok::sheet::csv::Encoding;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("gb.csv");
    // 你好 in GB18030: not UTF-8.
    std::fs::write(&path, [0xC4, 0xE3, 0xBA, 0xC3, b'\n']).expect("writes");
    for (tag, expected) in [
        ("zh-CN", Encoding::Gb18030),
        ("en-US", Encoding::Windows1252),
    ] {
        cx.update(|app| I18n::set_locale(tag, app));
        update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
        asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(_)));
        let found = cx.update(|app| match book.read(app).dialog.as_ref() {
            Some(Dialog::Import(pending)) => pending.how.encoding,
            _ => unreachable!("an import dialog"),
        });
        assert_eq!(found, expected, "{tag}");
        update(&mut cx, &book, |book, cx| {
            book.dialog = None;
            cx.notify();
        });
    }
}
