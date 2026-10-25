use super::*;
use gpui::TestAppContext;
use std::{cell::RefCell, rc::Rc, time::{Duration, Instant}};

fn settle(cx: &TestAppContext, view: &Entity<LokView>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if !cx.executor().tick() {
            cx.executor().advance_clock(Duration::from_millis(20));
        }
        if cx.update(|app| view.read(app).settled()) {
            return;
        }
    }
    panic!("the queued request did not finish");
}

fn errors(cx: &TestAppContext, view: &Entity<LokView>) -> Rc<RefCell<Vec<String>>> {
    let errors = Rc::new(RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = Rc::clone(&errors);
        app.subscribe(view, move |_, error: &LokError, _| errors.borrow_mut().push(error.0.clone())).detach();
    });
    errors
}

fn saved(shared: &Mutex<Worker>, doc: &Doc) -> String {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("check.fods");
    doc.save_as(&mut shared.lock().expect("the worker lock"), &crate::file_url(&path), "fods").expect("saves");
    std::fs::read_to_string(path).expect("the saved file")
}

#[test]
fn stale_selection_commands_are_rejected_before_writing() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let (view, shared) = cx.update(|app| live_view_of(app, &install, "in.fods", crate::client::tests::input::FODS));
    let errors = errors(&cx, &view);
    let doc = cx.update(|app| view.read(app).doc.clone());
    let at = {
        let mut worker = shared.lock().expect("the worker lock");
        let at = worker.now(&doc).expect("the original state");
        worker.run(&doc, "cell.goto", &["B1"]).expect("another selection");
        at
    };
    cx.update(|app| view.update(app, |view, _| view.run_at(&at, "cell.enter", &["SHOULD_NOT_APPEAR"]))).expect("queues");
    settle(&cx, &view);
    assert!(errors.borrow().iter().any(|text| text.starts_with(crate::STALE)), "the caller sees the stale-state error: {:?}", errors.borrow());
    assert!(!saved(&shared, &doc).contains("SHOULD_NOT_APPEAR"), "no unintended cell write");
}

#[test]
fn another_documents_handle_cannot_be_queued_on_this_view() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let (view, shared) = cx.update(|app| live_view_of(app, &install, "in.fods", crate::client::tests::input::FODS));
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("other.fods");
    std::fs::write(&path, crate::client::tests::input::FODS).expect("writes");
    let other = shared.lock().expect("the worker lock").load(&crate::file_url(&path)).expect("another document");
    let sent = cx.update(|app| view.read(app).sent());
    let result = cx.update(|app| view.update(app, |view, _| view.run_at(&other, "cell.enter", &["WRONG_DOCUMENT"])));
    assert!(result.is_err(), "the view must reject a different document before queueing");
    assert_eq!(cx.update(|app| view.read(app).sent()), sent);
    assert!(!saved(&shared, &other).contains("WRONG_DOCUMENT"));
}

#[test]
fn changing_sheets_invalidates_the_previous_state() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let (view, shared) = cx.update(|app| live_view_of(app, &install, "in.fods", crate::client::tests::input::FODS));
    let doc = cx.update(|app| view.read(app).doc.clone());
    let mut worker = shared.lock().expect("the worker lock");
    let at = worker.now(&doc).expect("the state");
    worker.run(&at, "sheet.insert", &["Second", "2"]).expect("adds a sheet");
    let at = worker.now(&doc).expect("the state before switching");
    let current = doc.sheets(&mut worker).expect("the sheets").current;
    doc.show_sheet(&mut worker, 1 - current).expect("switches sheets");
    assert!(worker.now(&doc).expect("the new state").newer_than(&at), "sheet context must invalidate earlier requests");
}

#[test]
fn a_current_batch_checks_once_and_keeps_command_order() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let (view, shared) = cx.update(|app| live_view_of(app, &install, "in.fods", crate::client::tests::input::FODS));
    let errors = errors(&cx, &view);
    let doc = cx.update(|app| view.read(app).doc.clone());
    let at = shared.lock().expect("the worker lock").now(&doc).expect("the current state");
    cx.update(|app| view.update(app, |view, _| view.run_all_at(&at, &[("cell.goto", &["B1"]), ("cell.enter", &["ORDERED_BATCH"])]))).expect("queues");
    settle(&cx, &view);
    assert!(errors.borrow().is_empty(), "one check before the batch: {:?}", errors.borrow());
    assert_eq!(doc.selection_text(&mut shared.lock().expect("the worker lock")).expect("the new cell"), "ORDERED_BATCH");
}
