use std::io;

use super::LokView;
use crate::Doc;

#[derive(Clone)]
pub struct RequestRead {
    at: Doc,
    pointer: Option<u64>,
    part: usize,
    sheets: u64,
    headers: u64,
}

#[derive(Default)]
pub(super) struct Pointer {
    down: Option<(u64, Option<Doc>)>,
    up: Option<(u64, Option<Doc>)>,
}

impl Pointer {
    pub(super) fn clear(&mut self) {
        self.down = None;
        self.up = None;
    }

    pub(super) fn press(&mut self, seq: u64) {
        self.down = Some((seq, None));
        self.up = None;
    }

    pub(super) fn release(&mut self, seq: u64) {
        if self.down.is_some() {
            self.up = Some((seq, None));
        }
    }

    pub(super) fn landed(&mut self, seq: u64, now: Option<u64>, doc: &Doc) {
        for entry in [&mut self.down, &mut self.up].into_iter().flatten() {
            if entry.0 == seq {
                entry.1 = now.map(|revision| {
                    let mut exact = doc.clone();
                    exact.rev = revision;
                    exact
                });
            }
        }
    }
}

impl LokView {
    /// Captures the first client context of a menu request.
    pub fn request_read(&self) -> RequestRead {
        let at = self.read_at();
        let pointer = self.pointer_input.down.as_ref().and_then(|(seq, down)| {
            let up = self.pointer_input.up.as_ref().and_then(|(_, up)| up.as_ref());
            (self.pointer_held() || down.is_none() || up.is_none()
                || down.as_ref() == Some(&at) || up == Some(&at)).then_some(*seq)
        });
        RequestRead { at, pointer, part: self.part, sheets: self.cell.sheets, headers: self.cell.headers }
    }

    /// Resolves the original read after its own paired release.
    pub fn checked_request_read(&self, read: &RequestRead) -> io::Result<Doc> {
        let stale = || io::Error::other(format!("{}the menu request context changed", crate::STALE));
        if self.pointer_held() || !self.settled() || self.part != read.part
            || self.cell.sheets != read.sheets || self.cell.headers != read.headers
        {
            return Err(stale());
        }
        let current = self.read_at();
        let Some(seq) = read.pointer else {
            return (current == read.at).then_some(current).ok_or_else(stale);
        };
        let (Some((down_seq, Some(down))), Some((up_seq, Some(up)))) =
            (&self.pointer_input.down, &self.pointer_input.up) else { return Err(stale()) };
        if *down_seq != seq || *up_seq <= seq || !down.same_document(up)
            || down.rev.checked_add(1) != Some(up.rev) || current != *up
            || (read.at != *down && read.at != *up)
        {
            return Err(stale());
        }
        Ok(up.clone())
    }
}
