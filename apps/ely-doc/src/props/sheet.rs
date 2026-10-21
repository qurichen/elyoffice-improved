//! The page as reported, and page edits not yet echoed.

use ely_office::panel::Panel as _;
use gpui::{App, Context};

use super::Props;

const SIZE: &str = ".uno:AttributePageSize";
const PAGE: &str = ".uno:AttributePage";

/// The page: size in 1/100 mm, orientation, layout, number type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Sheet {
    pub(super) size: (i32, i32),
    pub(super) landscape: bool,
    pub(super) layout: i32,
    pub(super) numtype: i32,
}

impl Sheet {
    /// The same page, within the kernel's twips rounding.
    fn matches(&self, other: &Sheet) -> bool {
        let near = |a: i32, b: i32| (a - b).abs() <= 2;
        (self.landscape, self.layout, self.numtype)
            == (other.landscape, other.layout, other.numtype)
            && near(self.size.0, other.size.0)
            && near(self.size.1, other.size.1)
    }
}

/// Page edits sent ahead of the kernel's echo.
pub(super) struct Sent {
    before: (Option<String>, Option<String>),
    sheets: Vec<Sheet>,
}

/// A "width x height" state in 1/100 mm.
pub(super) fn size_of(text: &str) -> Option<(i32, i32)> {
    let (width, height) = text.split_once(" x ")?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

/// The page item: landscape, layout, number type.
pub(super) fn page_of(text: &str) -> Option<(bool, i32, i32)> {
    let mut parts = text.split(',');
    let landscape = parts.next()?.parse().ok()?;
    Some((
        landscape,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

impl Props {
    fn reported(&self, cx: &App) -> (Option<String>, Option<String>) {
        let read = |uno| self.state(uno, cx).map(|state| state.to_string());
        (read(SIZE), read(PAGE))
    }

    fn reported_sheet(&self, cx: &App) -> Option<Sheet> {
        let (size, page) = self.reported(cx);
        let (landscape, layout, numtype) = page_of(&page?)?;
        Some(Sheet {
            size: size_of(&size?)?,
            landscape,
            layout,
            numtype,
        })
    }

    /// Edits build on the last sent page, else the kernel's.
    pub(super) fn sheet(&self, cx: &App) -> Option<Sheet> {
        match &self.sheet_sent {
            Some(sent) => sent.sheets.last().copied(),
            None => self.reported_sheet(cx),
        }
    }

    /// Sends `next`; one command, so one undo step.
    pub(super) fn send_sheet(&mut self, next: Sheet, cx: &mut Context<Self>) {
        let Some(now) = self.sheet(cx) else {
            return;
        };
        if next == now {
            return;
        }
        let before = self.reported(cx);
        self.sheet_sent
            .get_or_insert(Sent {
                before,
                sheets: Vec::new(),
            })
            .sheets
            .push(next);
        let (w, h) = (next.size.0.to_string(), next.size.1.to_string());
        let item = (next.landscape, next.layout, next.numtype);
        if item == (now.landscape, now.layout, now.numtype) {
            return self.run("page.size", &[&w, &h], cx);
        }
        let page = [
            next.landscape.to_string(),
            next.layout.to_string(),
            next.numtype.to_string(),
        ];
        self.run(
            "page.orientation",
            &[&w, &h, &page[0], &page[1], &page[2]],
            cx,
        );
    }

    /// Drops sent pages the kernel has answered.
    pub(super) fn track_sheet(&mut self, cx: &App) {
        let reported = self.reported(cx);
        let seen = self.reported_sheet(cx);
        let Some(sent) = &mut self.sheet_sent else {
            return;
        };
        if reported == sent.before {
            return;
        }
        let at = seen.and_then(|seen| sent.sheets.iter().position(|sheet| sheet.matches(&seen)));
        match at {
            Some(at) if at + 1 < sent.sheets.len() => {
                sent.sheets.drain(..=at);
                sent.before = reported;
            }
            _ => self.sheet_sent = None,
        }
    }
}
