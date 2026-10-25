use super::*;

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
