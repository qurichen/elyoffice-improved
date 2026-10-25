use super::*;

#[test]
fn cancelling_save_as_retains_unicode_edits_and_allows_retry() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("the built engine is required");
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Round11 中文 😀");
    with_window(&mut cx, &docs, |docs, _, cx| docs.save_as(id, None, cx));
    assert!(cx.did_prompt_for_new_path());
    cx.simulate_new_path_selection(|_| None);
    for _ in 0..30 {
        cx.executor().tick();
    }
    cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("the tab stays");
        assert!(tab.dirty(app));
        assert!(tab.session.path().is_none());
        assert!(!tab.exporting);
    });
    let saved = dir.path().join("中文保存.docx");
    with_window(&mut cx, &docs, |docs, _, cx| docs.save_as(id, None, cx));
    assert!(cx.did_prompt_for_new_path());
    cx.simulate_new_path_selection(|_| Some(saved.clone()));
    written(&cx, &saved, None);
    settle(&cx, |app| {
        !docs.read(app).active_tab().expect("the tab").dirty(app)
    });
    let text = text_of(&mut cx, &docs, &saved);
    for token in ["Round11", "中文", "😀"] {
        assert!(text.contains(token), "saved content retains {token}");
    }
}

#[test]
fn cancelling_save_as_during_quit_keeps_the_unsaved_tab() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("the built engine is required");
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Do not lose this");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.save_as(id, Some(After::Quit), cx)
    });
    assert!(cx.did_prompt_for_new_path());
    cx.simulate_new_path_selection(|_| None);
    for _ in 0..30 {
        cx.executor().tick();
    }
    cx.update(|app| {
        let core = &docs.read(app).core;
        assert_eq!(core.tabs.len(), 1);
        assert_eq!(core.active, Some(id));
        assert!(core.tabs[0].dirty(app));
        assert!(core.tabs[0].session.path().is_none());
    });
}

#[test]
fn unsupported_save_as_suffix_creates_nothing_and_keeps_edits() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("the built engine is required");
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Keep my draft");
    let target = dir.path().join("not-a-document.pdf");
    with_window(&mut cx, &docs, |docs, _, cx| docs.save_as(id, None, cx));
    cx.simulate_new_path_selection(|_| Some(target.clone()));
    for _ in 0..30 {
        cx.executor().tick();
    }
    assert!(!target.exists());
    cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("the draft stays");
        assert!(tab.dirty(app));
        assert!(tab.session.path().is_none());
        assert!(!tab.exporting);
    });
}

#[test]
fn closing_a_saved_tab_removes_scratch_after_the_kernel_releases_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("the built engine is required");
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "RetainedAfterClose 中文");
    let path = dir.path().join("saved.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    settle(&cx, |app| {
        !docs.read(app).tab(id).expect("the tab").dirty(app)
    });
    let work = cx.update(|app| docs.read(app).tab(id).expect("the tab").work.clone());
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.request_close(id, After::Tab, window, cx)
    });
    settle(&cx, |app| {
        docs.read(app).tab(id).is_none() && !work.exists()
    });
    assert!(text_of(&mut cx, &docs, &path).contains("RetainedAfterClose"));
}

#[test]
fn save_as_refuses_the_owned_scratch_tree_and_retains_edits() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("the built engine is required");
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "KeepThisDraft");
    let target = dir.path().join("work/would-be-deleted.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, target.clone(), None, window, cx)
    });
    settle(&cx, |app| {
        !docs.read(app).tab(id).expect("the draft").exporting
    });
    assert!(
        !target.exists(),
        "a persistent document must not be saved in disposable scratch"
    );
    cx.update(|app| {
        let tab = docs.read(app).tab(id).expect("the draft remains");
        assert!(tab.dirty(app));
        assert!(tab.session.path().is_none());
    });
}

#[test]
fn opening_owned_scratch_as_a_user_document_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("the built engine is required");
    let _ = blank(&mut cx, &docs);
    let internal = dir.path().join("work/copied.fodt");
    std::fs::copy(dir.path().join("work/doc-0/blank.fodt"), &internal)
        .expect("a synthetic internal copy");
    cx.update(|app| docs.update(app, |docs, cx| docs.open_file(internal.clone(), cx)));
    settle(&cx, |app| docs.read(app).core.loading.is_empty());
    cx.update(|app| assert_eq!(docs.read(app).core.tabs.len(), 1));
    assert!(
        internal.exists(),
        "refusing the user location must not delete the source"
    );
}
