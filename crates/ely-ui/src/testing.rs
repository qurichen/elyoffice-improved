use gpui::TestAppContext;

/// Closes a test resource despite retained background tasks.
pub fn reap_on_quit<T: Send + 'static>(
    resource: &std::sync::Arc<std::sync::Mutex<T>>,
    app: &gpui::App,
    close: fn(&mut T) -> std::io::Result<()>,
) {
    let resource = std::sync::Arc::downgrade(resource);
    app.on_app_quit(move |_| {
        if let Some(resource) = resource.upgrade() {
            let mut resource = match resource.lock() {
                Ok(resource) => resource,
                Err(poison) => {
                    log::warn!("test: resource lock poisoned during cleanup");
                    poison.into_inner()
                }
            };
            if let Err(error) = close(&mut resource) {
                if std::thread::panicking() {
                    log::error!("test: cleanup failed during unwinding: {error}");
                } else {
                    panic!("the test resource did not close: {error}");
                }
            }
        }
        async {}
    }).detach();
}

/// Runs application cleanup before the test context leaves.
pub struct QuitOnDrop(TestAppContext);

/// Shuts down test workers, including during unwinding.
pub fn quit_on_drop(cx: &TestAppContext) -> QuitOnDrop {
    QuitOnDrop(cx.clone())
}

impl Drop for QuitOnDrop {
    fn drop(&mut self) {
        self.0.quit();
        self.0.run_until_parked();
    }
}

#[test]
fn the_guard_runs_registered_cleanup_during_unwinding() {
    use std::{cell::Cell, rc::Rc};
    let mut cx = TestAppContext::single();
    let cleaned = Rc::new(Cell::new(false));
    let seen = Rc::clone(&cleaned);
    cx.on_quit(move || seen.set(true));
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = quit_on_drop(&cx);
        panic!("synthetic test failure");
    }));
    assert!(failed.is_err());
    assert!(cleaned.get());
}
