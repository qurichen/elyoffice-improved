use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};

use crate::Recovery;

/// The background recovery task: one per document (§6.2).
pub struct RecoveryTask {
    status: Arc<Mutex<Option<SystemTime>>>,
    stop: Arc<AtomicBool>,
    bell: Arc<Condvar>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl RecoveryTask {
    /// Runs `snapshot` every interval; failures keep looping.
    pub fn start(
        recovery: Recovery,
        interval: Duration,
        snapshot: impl Fn() -> (Vec<u8>, Option<String>) + Send + 'static,
    ) -> Self {
        let status = Arc::new(Mutex::new(recovery.last_ok()));
        let shared = Arc::new(Mutex::new(recovery));
        let stop = Arc::new(AtomicBool::new(false));
        let bell = Arc::new(Condvar::new());
        let (worker, worker_status, flag, worker_bell) = (
            Arc::clone(&shared),
            Arc::clone(&status),
            Arc::clone(&stop),
            Arc::clone(&bell),
        );
        let join = std::thread::spawn(move || {
            {
                let recovery = worker.lock().expect("the recovery lock");
                if let Err(error) = recovery.maintain(SystemTime::now()) {
                    log::warn!("recovery: the opening maintain failed: {error}");
                }
            }
            let gate = Mutex::new(());
            while !flag.load(Ordering::Relaxed) {
                let hold = gate.lock().expect("the gate lock");
                let (hold, waited) = worker_bell
                    .wait_timeout(hold, interval)
                    .expect("the gate wait");
                drop(hold);
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                if !waited.timed_out() {
                    continue;
                }
                let (content, cursor) = snapshot();
                if flag.load(Ordering::Relaxed) {
                    log::info!("recovery: a snapshot dropped after cancel");
                    break;
                }
                let mut recovery = worker.lock().expect("the recovery lock");
                let written = recovery.write_once(&content, cursor.as_deref(), SystemTime::now());
                let last_ok = recovery.last_ok();
                drop(recovery);
                *worker_status.lock().expect("the status lock") = last_ok;
                if let Err(error) = written {
                    log::warn!("recovery: the scheduled write failed: {error}");
                }
            }
        });
        Self {
            status,
            stop,
            bell,
            join: Some(join),
        }
    }

    /// The last successful scheduled write, never behind disk I/O.
    pub fn last_ok(&self) -> Option<SystemTime> {
        *self.status.lock().expect("the status lock")
    }

    /// Stops the loop and joins its thread.
    pub fn stop(mut self) {
        self.cancel();
    }

    /// Sets the flag, wakes the sleeper and joins.
    fn cancel(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.bell.notify_all();
        if let Some(join) = self.join.take() {
            join.join().expect("the recovery thread joins");
        }
    }
}

impl Drop for RecoveryTask {
    fn drop(&mut self) {
        self.cancel();
    }
}
