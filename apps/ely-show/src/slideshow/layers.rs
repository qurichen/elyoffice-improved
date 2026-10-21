//! The shown and next slides' layers, from the copy.

use std::sync::Arc;

use ely_office::prelude::*;
use gpui::Context;

use super::{Show, Stack};
use crate::preview::pictures;

/// Reads at most this many slides at once.
const AT_ONCE: usize = 2;

impl Show {
    pub fn stack(&self, part: usize) -> Option<&Stack> {
        self.layers.get(&part)
    }

    fn wanted(&self) -> Vec<usize> {
        self.part().into_iter().chain(self.upcoming()).collect()
    }

    /// Reads wanted layers, drops the rest.
    pub(super) fn fetch(&mut self, cx: &mut Context<Self>) {
        let wanted = self.wanted();
        let gone = &mut self.gone;
        self.layers.retain(|part, stack| {
            let kept = wanted.contains(part);
            if !kept {
                gone.push(stack.clone());
            }
            kept
        });
        for part in wanted {
            if self.layers.contains_key(&part)
                || self.asking.contains(&part)
                || self.asking.len() >= AT_ONCE
            {
                continue;
            }
            self.asking.insert(part);
            log::info!("show: reads the layers of slide {part}");
            let pixels = self.pixels;
            ely_lok::read_off(
                &self.worker,
                &self.doc,
                move |worker, doc| doc.slide_layers(worker, part, pixels),
                move |show: &mut Self, read, cx| {
                    show.asking.remove(&part);
                    match read {
                        Ok(layers) if show.wanted().contains(&part) => {
                            if let Some(old) = show.layers.insert(part, Arc::new(pictures(layers))) {
                                show.gone.push(old);
                            }
                            if show.part() == Some(part) {
                                show.arrived(cx);
                            }
                            cx.notify();
                        }
                        Ok(_) => log::info!("show: slide {part} is no longer wanted"),
                        Err(error) => return show.fail(&format!("the layers of slide {part}: {error}"), cx),
                    }
                    show.fetch(cx);
                },
                cx,
            );
        }
    }

    /// Ends the show and says why in the deck.
    pub(super) fn fail(&mut self, why: &str, cx: &mut Context<Self>) {
        log::error!("show: {why}");
        let (deck, why) = (self.deck.clone(), why.to_string());
        self.close(cx);
        cx.defer(move |cx| {
            if let Some(deck) = deck.upgrade() {
                deck.update(cx, |deck, cx| deck.engine_failed(&why, cx));
            }
        });
    }
}

impl Show {
    /// Frees, in window `which`, layers nothing draws any more.
    pub(super) fn free(&mut self, which: usize, window: &mut gpui::Window) {
        // A stack the leaving slide still holds waits.
        let (free, held): (Vec<_>, Vec<_>) = self.gone.drain(..).partition(|stack| Arc::strong_count(stack) == 1);
        self.gone = held;
        for stack in free {
            for (_, image) in stack.iter() {
                self.retired[0].push(image.clone());
                if self.presenter {
                    self.retired[1].push(image.clone());
                }
            }
        }
        self.retired[which].release_all(window);
    }
}
