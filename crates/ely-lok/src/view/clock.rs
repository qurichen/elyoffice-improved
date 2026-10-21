//! Times an edit to the tile showing it (PERF-02).
use std::time::Instant;

/// One edit's clock: sent when, its revision, its generation.
type Edit = (Instant, Option<u64>, Option<u64>);

#[derive(Default)]
pub(super) struct EditClock {
    edit: Option<Edit>,
    /// The tile that shows the edit, until it is drawn.
    awaited: Option<usize>,
    /// The last generation bump: its revision and generation.
    bumped: (u64, u64),
}

impl EditClock {
    /// An edit queued; an unseen earlier start stays.
    pub(super) fn sent(&mut self) {
        self.edit.get_or_insert((Instant::now(), None, None));
    }

    /// The edit landed at `revision`; its batch may precede it.
    pub(super) fn landed(&mut self, revision: Option<u64>) {
        let Some((_, rev @ None, generation)) = &mut self.edit else {
            return;
        };
        let Some(revision) = revision else {
            self.edit = None;
            return;
        };
        *rev = Some(revision);
        if self.bumped.0 >= revision {
            *generation = Some(self.bumped.1);
        }
    }

    /// A batch at `revision` dirtied tiles into `generation`.
    pub(super) fn bumped(&mut self, revision: u64, generation: u64) {
        self.bumped = (revision, generation);
        if let Some((_, Some(rev), shown @ None)) = &mut self.edit
            && revision >= *rev
        {
            *shown = Some(generation);
        }
    }

    /// A tile of `generation` came as `tile`; it awaits drawing.
    pub(super) fn painted(&mut self, generation: u64, tile: usize) {
        if let Some((_, _, Some(shown))) = self.edit
            && generation >= shown
        {
            self.awaited = Some(tile);
        }
    }

    /// `tile` reached the window; the edit it shows is timed.
    pub(super) fn drawn(&mut self, tile: usize) {
        if self.awaited == Some(tile)
            && let Some((at, ..)) = self.edit.take()
        {
            log::debug!("lok: edit to tile {} ms", at.elapsed().as_millis());
            self.awaited = None;
        }
    }

    /// Whether a drawn `tile` would end the timing.
    pub(super) fn awaits(&self, tile: usize) -> bool {
        self.awaited == Some(tile)
    }

    #[cfg(test)]
    fn running(&self) -> bool {
        self.edit.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::EditClock;

    #[test]
    fn a_batch_before_the_landing_still_ends_the_edit() {
        let mut clock = EditClock::default();
        clock.sent();
        clock.bumped(7, 3);
        clock.landed(Some(7));
        clock.painted(2, 20);
        clock.drawn(20);
        assert!(clock.running(), "an older picture leaves it running");
        clock.painted(3, 30);
        assert!(clock.running(), "arriving is not yet drawn");
        clock.drawn(30);
        assert!(!clock.running(), "the edit's own picture ends it");
    }

    #[test]
    fn only_a_batch_at_the_edit_revision_counts() {
        let mut clock = EditClock::default();
        clock.sent();
        clock.bumped(6, 3);
        clock.landed(Some(7));
        clock.painted(3, 30);
        clock.drawn(30);
        assert!(clock.running(), "a batch before the edit shows the old text");
        clock.bumped(7, 4);
        clock.painted(4, 40);
        clock.drawn(40);
        assert!(!clock.running());
    }

    #[test]
    fn an_unread_revision_drops_the_edit() {
        let mut clock = EditClock::default();
        clock.sent();
        clock.landed(None);
        assert!(!clock.running());
    }

    #[test]
    fn a_tile_that_never_draws_keeps_the_edit_open() {
        let mut clock = EditClock::default();
        clock.sent();
        clock.bumped(5, 2);
        clock.landed(Some(5));
        clock.painted(2, 50);
        assert!(clock.awaits(50));
        clock.drawn(51);
        assert!(clock.running(), "another tile's drawing does not count");
    }
}
