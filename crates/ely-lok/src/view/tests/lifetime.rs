use super::*;

fn closing_a_retained_view(retire: bool) -> Vec<String> {
    let install = crate::client::tests::install().expect("the pinned LOK build");
    let cx = gpui::TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let (view, worker) = cx.update(|app| live_view(app, &install));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &LokError, _| {
            errors.borrow_mut().push(error.0.clone());
        }).detach();
    });
    for _ in 0..80 { cx.executor().tick(); }
    let doc = cx.update(|app| view.read(app).read_at());
    if retire {
        cx.update(|app| view.update(app, |view, _| { view.retire_all(); }));
    }
    worker.lock().expect("the worker lock").close_doc(doc).expect("close the document");
    for _ in 0..80 { cx.executor().tick(); }
    let seen = errors.borrow().clone();
    drop(view);
    seen
}

#[test]
fn a_retired_view_retained_by_a_callback_stops_watching_before_document_close() {
    assert!(closing_a_retained_view(true).is_empty(), "retired views do not poll closed documents");
}

#[test]
fn a_live_view_still_reports_unexpected_document_close_without_input() {
    let errors = closing_a_retained_view(false);
    assert!(errors.iter().any(|error| error.contains("no such document")),
        "active view errors remain visible: {errors:?}");
}

#[test]
fn quitting_reaps_a_worker_kept_by_a_view() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let quit = ely_ui::testing::quit_on_drop(&cx);
    let (_view, worker) = cx.update(|app| live_view(app, &install));
    drop(quit);
    assert!(worker.lock().expect("the worker lock").ended().is_some(), "quit reaps the still-owned worker");
}

#[test]
fn unwinding_with_a_poisoned_worker_still_reaps_it() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let (_view, worker) = cx.update(|app| live_view(app, &install));
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _quit = ely_ui::testing::quit_on_drop(&cx);
        let _held = worker.lock().expect("the worker lock");
        panic!("synthetic failure while holding the worker lock");
    }));
    assert!(failed.is_err(), "the original failure remains observable");
    let mut worker = worker.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(worker.ended().is_some(), "unwind reaps the still-owned worker");
}
