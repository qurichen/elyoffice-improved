use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{App, AppContext as _, Context, Entity, SharedString};

/// Where a long task stands: range, state, error.
pub enum TaskState {
    Running { done: u32, total: u32 },
    Succeeded,
    Failed(SharedString),
    Cancelled,
}

/// A beat of work; the flag stops it early.
type Work = dyn Fn(u32, &AtomicBool) -> Result<(), SharedString> + Send + Sync;

/// A named background task with progress, retry, and a cancel.
pub struct LongTask {
    name: SharedString,
    state: TaskState,
    total: u32,
    work: Arc<Work>,
    stop: Arc<AtomicBool>,
    halt: Option<gpui::Task<()>>,
}

impl LongTask {
    pub fn name(&self) -> &SharedString {
        &self.name
    }

    pub fn state(&self) -> &TaskState {
        &self.state
    }

    /// Runs `work` off-thread; cancel lands at the next checkpoint.
    pub fn start(
        name: impl Into<SharedString>,
        total: u32,
        work: impl Fn(u32, &AtomicBool) -> Result<(), SharedString> + Send + Sync + 'static,
        cx: &mut App,
    ) -> Entity<Self> {
        let entity = cx.new(|_| LongTask {
            name: name.into(),
            state: TaskState::Running { done: 0, total },
            total,
            work: Arc::new(work),
            stop: Arc::new(AtomicBool::new(false)),
            halt: None,
        });
        entity.update(cx, |this, cx| this.spawn_loop(cx));
        entity
    }

    /// Asks the loop to stop; it reports Cancelled once stopped.
    pub fn cancel(&mut self, _: &mut Context<Self>) {
        if matches!(self.state, TaskState::Running { .. }) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    /// Runs the same work again after a failure.
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.state, TaskState::Failed(_)) {
            return;
        }
        log::info!("task: {} retried", self.name);
        self.stop.store(false, Ordering::Relaxed);
        self.state = TaskState::Running {
            done: 0,
            total: self.total,
        };
        cx.notify();
        self.spawn_loop(cx);
    }

    fn spawn_loop(&mut self, cx: &mut Context<Self>) {
        let name = self.name.clone();
        let total = self.total;
        let work = self.work.clone();
        let stop = self.stop.clone();
        self.halt = Some(cx.spawn(async move |this, cx| {
            for done in 1..=total {
                if stop.load(Ordering::Relaxed) {
                    return Self::finish(&this, cx, TaskState::Cancelled, &name);
                }
                let result = {
                    let work = work.clone();
                    let stop = stop.clone();
                    cx.background_executor()
                        .spawn(async move { work(done, &stop) })
                        .await
                };
                match result {
                    Err(error) => {
                        return Self::finish(&this, cx, TaskState::Failed(error), &name);
                    }
                    Ok(()) if stop.load(Ordering::Relaxed) => {
                        return Self::finish(&this, cx, TaskState::Cancelled, &name);
                    }
                    Ok(()) => {
                        let step = this.update(cx, |this, cx| {
                            this.state = TaskState::Running { done, total };
                            log::info!("task: {} {done}/{total}", this.name);
                            cx.notify();
                        });
                        if let Err(error) = step {
                            log::error!("task: {name} lost its owner: {error:#}");
                            return;
                        }
                    }
                }
            }
            Self::finish(&this, cx, TaskState::Succeeded, &name);
        }));
    }

    fn finish(
        this: &gpui::WeakEntity<Self>,
        cx: &mut gpui::AsyncApp,
        state: TaskState,
        name: &SharedString,
    ) {
        match &state {
            TaskState::Succeeded => log::info!("task: {name} done"),
            TaskState::Cancelled => log::info!("task: {name} cancelled"),
            TaskState::Failed(error) => log::info!("task: {name} failed: {error}"),
            TaskState::Running { .. } => unreachable!("a finish has no progress"),
        }
        let done = this.update(cx, |this, cx| {
            this.state = state;
            cx.notify();
        });
        if let Err(error) = done {
            log::error!("task: {name} lost its owner: {error:#}");
        }
    }
}
