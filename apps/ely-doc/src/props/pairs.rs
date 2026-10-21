//! Two-value kernel states edited one side at a time.

use ely_office::panel::Panel as _;
use gpui::{App, Context};

use super::Props;

/// Pairs sent ahead of the kernel's echo.
pub(super) struct Sent {
    /// The state when the first pair left.
    before: Option<String>,
    pairs: Vec<(i32, i32)>,
}

/// Whole twips in 1/100 mm, rounded as the kernel does.
pub(super) fn mm100_of_twips(twips: i32) -> i32 {
    (twips * 127 + 36) / 72
}

/// A "first,second" state.
fn parse(text: &str) -> Option<(i32, i32)> {
    let (first, second) = text.split_once(',')?;
    Some((first.parse().ok()?, second.parse().ok()?))
}

impl Props {
    /// Edits build on the last sent pair, else the kernel's.
    pub(super) fn pair(&self, uno: &'static str, cx: &App) -> Option<(i32, i32)> {
        match self.pending.get(uno) {
            Some(sent) => sent.pairs.last().copied(),
            None => self.state(uno, cx).as_deref().and_then(parse),
        }
    }

    /// Sets one side in twips; the other stays.
    pub(super) fn set_pair(
        &mut self,
        (uno, command): (&'static str, &'static str),
        second: bool,
        twips: i32,
        cx: &mut Context<Self>,
    ) {
        let Some((first, other)) = self.pair(uno, cx) else {
            return;
        };
        let value = mm100_of_twips(twips);
        let next = if second {
            (first, value)
        } else {
            (value, other)
        };
        if next == (first, other) {
            return;
        }
        let before = self.state(uno, cx).map(|state| state.to_string());
        self.pending
            .entry(uno)
            .or_insert(Sent {
                before,
                pairs: Vec::new(),
            })
            .pairs
            .push(next);
        let values = [next.0.to_string(), next.1.to_string()];
        self.run(command, &[&values[0], &values[1]], cx);
    }

    /// Drops sent pairs the kernel has answered.
    pub(super) fn track_pairs(&mut self, cx: &App) {
        let reported: Vec<(&'static str, Option<String>)> = self
            .pending
            .keys()
            .map(|&uno| (uno, self.state(uno, cx).map(|state| state.to_string())))
            .collect();
        for (uno, reported) in reported {
            let sent = self.pending.get_mut(uno).expect("listed above");
            if reported == sent.before {
                continue;
            }
            let at = reported
                .as_deref()
                .and_then(parse)
                .and_then(|seen| sent.pairs.iter().position(|pair| *pair == seen));
            match at {
                Some(at) if at + 1 < sent.pairs.len() => {
                    sent.pairs.drain(..=at);
                    sent.before = reported;
                }
                _ => {
                    self.pending.remove(uno);
                }
            }
        }
    }
}
