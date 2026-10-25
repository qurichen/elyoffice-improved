use super::*;

#[test]
fn repeated_edit_save_and_close_leaves_no_scratch_documents() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    for cycle in 0..24 {
        let id = blank(&mut cx, &docs);
        let marker = format!("Cycle{cycle} 中文 😀");
        type_into(&mut cx, &docs, &marker);
        if cycle % 3 == 0 {
            with_window(&mut cx, &docs, |docs, _, cx| docs.save_as(id, None, cx));
            assert!(cx.did_prompt_for_new_path());
            cx.simulate_new_path_selection(|_| None);
            for _ in 0..30 {
                cx.executor().tick();
            }
            cx.update(|app| {
                let tab = docs.read(app).tab(id).expect("the draft stays");
                assert!(tab.dirty(app));
                assert!(tab.session.path().is_none());
            });
        }
        let extension = if cycle % 2 == 0 { "docx" } else { "odt" };
        let target = dir.path().join(format!("saved-{cycle}.{extension}"));
        with_window(&mut cx, &docs, |docs, window, cx| {
            docs.write_as(id, target.clone(), None, window, cx)
        });
        written(&cx, &target, None);
        settle(&cx, |app| {
            !docs.read(app).tab(id).expect("the tab").dirty(app)
        });
        assert!(text_of(&mut cx, &docs, &target).contains(&marker));
        let (work, view) = cx.update(|app| {
            let tab = docs.read(app).tab(id).expect("the saved tab");
            (tab.work.clone(), tab.view.downgrade())
        });
        with_window(&mut cx, &docs, |docs, window, cx| {
            docs.request_close(id, After::Tab, window, cx)
        });
        settle(&cx, |app| {
            docs.read(app).core.tabs.is_empty() && !work.exists() && view.upgrade().is_none()
        });
        assert_eq!(
            std::fs::read_dir(dir.path().join("work"))
                .expect("scratch root")
                .count(),
            0,
            "cycle {cycle} released its document"
        );
        eprintln!("Lifecycle cycle {cycle}: retained content and released scratch");
    }
}
