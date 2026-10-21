use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

/// The debounce after typing stops: Mark's 1 second (§6.2).
pub const DEBOUNCE: Duration = Duration::from_secs(1);

/// How an app saves (§6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Explicit save only; recovery copies run behind (§6.2).
    Explicit,
    /// Opt-in autosave for normal unsigned files.
    AutoSave,
    /// ElyMark: autosave once typing stops for a second.
    MarkAutoSave,
}

/// What a tick decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Nothing,
    /// A save of that revision should run now.
    SaveDue(u64),
    /// Autosave stopped by an external change or compat risk (§6.2).
    AutoSaveStopped,
}

/// The save policy engine: debounce, stop rules and revision tracking.
pub struct PolicyEngine {
    policy: Policy,
    has_path: bool,
    stopped: bool,
    revision: u64,
    persisted: u64,
    last_edit: Option<(u64, Instant)>,
}

impl PolicyEngine {
    pub fn new(policy: Policy, has_path: bool) -> Self {
        Self {
            policy,
            has_path,
            stopped: false,
            revision: 0,
            persisted: 0,
            last_edit: None,
        }
    }

    pub fn policy(&self) -> Policy {
        self.policy
    }

    /// Whether autosave still runs.
    pub fn autosaving(&self) -> bool {
        !self.stopped && self.policy != Policy::Explicit
    }

    /// The first save assigned a path; Mark autosave can start.
    pub fn set_has_path(&mut self) {
        self.has_path = true;
    }

    /// An edit happened; the debounce restarts (§6.2).
    pub fn edit(&mut self, now: Instant) {
        self.revision += 1;
        self.last_edit = Some((self.revision, now));
    }

    /// Time passed; maybe a save is due.
    pub fn tick(&mut self, now: Instant) -> Action {
        if !self.autosaving() || !self.has_path {
            return Action::Nothing;
        }
        let Some((revision, at)) = self.last_edit else {
            return Action::Nothing;
        };
        if revision > self.persisted && now.saturating_duration_since(at) >= DEBOUNCE {
            return Action::SaveDue(revision);
        }
        Action::Nothing
    }

    /// An external change stops auto-overwrite for good (§6.2).
    pub fn external_modified(&mut self) -> Action {
        self.stop()
    }

    /// A compat risk stops auto-overwrite for good (§6.2).
    pub fn compat_risk(&mut self) -> Action {
        self.stop()
    }

    /// The shared stop: once, loud, permanent.
    fn stop(&mut self) -> Action {
        if !self.autosaving() {
            return Action::Nothing;
        }
        self.stopped = true;
        self.last_edit = None;
        log::warn!("policy: autosave stopped; the user decides from here");
        Action::AutoSaveStopped
    }

    /// A save finished; newer edits keep their due.
    pub fn save_finished(&mut self, revision: u64, ok: bool) {
        if !ok {
            return;
        }
        self.persisted = self.persisted.max(revision);
        if self.last_edit.map(|(r, _)| r) == Some(self.persisted) {
            self.last_edit = None;
        }
    }
}
