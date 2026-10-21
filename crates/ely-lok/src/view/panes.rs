//! Frozen rows and columns stay; the rest scrolls (TABLE-02).

/// One axis: frozen extent and scroll past it, in twips.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Axis {
    pub frozen: i32,
    pub scroll: i32,
}

impl Axis {
    /// Where document `doc` shows, in twips from the view's start.
    pub fn show(self, doc: i32) -> i32 {
        if doc < self.frozen {
            doc
        } else {
            (doc - self.scroll).max(self.frozen)
        }
    }

    /// The document point under view twips `at`.
    pub fn doc(self, at: i32) -> i32 {
        if at < self.frozen {
            at
        } else {
            at + self.scroll
        }
    }

    /// Each pane: document start and end, and its view start.
    pub fn panes(self, length: i32) -> Vec<(i32, i32, i32)> {
        let mut panes = Vec::with_capacity(2);
        if self.frozen > 0 {
            panes.push((0, self.frozen, 0));
        }
        let start = self.frozen + self.scroll;
        panes.push((start, start + (length - self.frozen).max(0), self.frozen));
        panes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frozen_part_stays_and_the_rest_scrolls() {
        let axis = Axis {
            frozen: 300,
            scroll: 1000,
        };
        assert_eq!(axis.show(100), 100, "frozen rows stay");
        assert_eq!(axis.show(1500), 500, "the rest moves by the scroll");
        assert_eq!(axis.show(800), 300, "the hidden band folds to the line");
        assert_eq!(axis.doc(100), 100);
        assert_eq!(axis.doc(500), 1500);
        assert_eq!(axis.panes(2000), [(0, 300, 0), (1300, 3000, 300)]);
    }

    #[test]
    fn without_a_freeze_one_pane_scrolls() {
        let axis = Axis {
            frozen: 0,
            scroll: 400,
        };
        assert_eq!(axis.show(1000), 600);
        assert_eq!(axis.doc(600), 1000);
        assert_eq!(axis.panes(2000), [(400, 2400, 0)]);
    }
}
