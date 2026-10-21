use std::time::{Duration, Instant};

use std::{cell::Cell, ops::Not as _, rc::Rc};

use ely_gpui_component::{layout::resize_edge, motion, theme::ActiveTheme};
use gpui::{
    AnyView, App, AppContext as _, Axis, DragMoveEvent, ElementId, EmptyView, InteractiveElement as _, IntoElement,
    ParentElement, Pixels, RenderOnce, StatefulInteractiveElement as _, Styled, Window, div, px,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Left,
    Right,
}

/// Openness of a panel; mid-flight flips never jump.
#[derive(Clone, Copy, Debug)]
struct Slide {
    from: f32,
    to: f32,
    since: Instant,
}

impl Slide {
    fn new(open: bool, now: Instant) -> Self {
        let share = if open { 1.0 } else { 0.0 };
        Self {
            from: share,
            to: share,
            since: now,
        }
    }

    fn share(&self, now: Instant, length: Duration) -> f32 {
        let elapsed = now.saturating_duration_since(self.since).as_secs_f32();
        let t = (elapsed / length.as_secs_f32().max(f32::EPSILON)).min(1.0);
        self.from + (self.to - self.from) * motion::ease_in_out_cubic(t)
    }

    fn aim(&mut self, open: bool, now: Instant, length: Duration) {
        let to = if open { 1.0 } else { 0.0 };
        if to != self.to {
            *self = Self {
                from: self.share(now, length),
                to,
                since: now,
            };
        }
    }

    fn running(&self, now: Instant, length: Duration) -> bool {
        self.from != self.to && now.saturating_duration_since(self.since) < length
    }
}

/// The slide, plus the view kept while closing.
struct Slot {
    slide: Slide,
    view: Option<AnyView>,
}

/// A sliding side column; no view, no room.
#[derive(IntoElement)]
pub(super) struct SidePanel {
    id: ElementId,
    side: Side,
    view: Option<AnyView>,
    width: Pixels,
}

impl SidePanel {
    /// `width` is what the shell left; no view, no room.
    pub(super) fn new(id: impl Into<ElementId>, side: Side, view: Option<AnyView>, width: Pixels) -> Self {
        Self {
            id: id.into(),
            side,
            view,
            width,
        }
    }
}

impl RenderOnce for SidePanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = self.view.is_some();
        let length = motion::duration(motion::BASE, cx);
        let now = Instant::now();
        let slot = window.use_keyed_state(self.id.clone(), cx, |_, _| Slot {
            slide: Slide::new(open, now),
            view: None,
        });
        let (share, running, view) = slot.update(cx, |slot, _| {
            slot.slide.aim(open, now, length);
            if open {
                slot.view = self.view;
            }
            let share = slot.slide.share(now, length);
            if !open && !slot.slide.running(now, length) {
                slot.view = None;
            }
            (share, slot.slide.running(now, length), slot.view.clone())
        });
        let Some(view) = view else {
            return div().into_any_element();
        };
        if running {
            window.request_animation_frame();
        }
        let full = self.width;
        let shown = full * share;
        let theme = cx.theme();
        let inner = div()
            .w(full)
            .h_full()
            .flex_none()
            .border_color(theme.colors.border)
            .child(view);
        let inner = match self.side {
            Side::Left => inner.border_r_1().ml(shown - full),
            Side::Right => inner.border_l_1(),
        };
        // The inner edge drags the width.
        let grip = resize_edge((self.id.clone(), "edge"), Axis::Horizontal, cx)
            .absolute()
            .top_0()
            .bottom_0()
            .on_drag(Grab { side: self.side, start: full, anchor: Rc::new(Cell::new(None)) }, |_, _, _, cx| {
                cx.new(|_| EmptyView)
            });
        let grip = match self.side {
            Side::Left => grip.right_0(),
            Side::Right => grip.left_0(),
        };
        let side = self.side;
        div()
            .id(self.id)
            .relative()
            .flex_none()
            .h_full()
            .w(shown)
            .overflow_hidden()
            .bg(theme.colors.bg)
            .on_drag_move(move |event: &DragMoveEvent<Grab>, window, cx| {
                let grab = event.drag(cx);
                if grab.side != side {
                    return;
                }
                let at = event.event.position.x;
                let anchor = grab.anchor.get().unwrap_or_else(|| {
                    grab.anchor.set(Some(at));
                    at
                });
                let moved = if side == Side::Left { at - anchor } else { anchor - at };
                let wanted = clamp(side, grab.start + moved, window);
                crate::settings::panels::set_width(side == Side::Left, wanted, cx);
                window.refresh();
            })
            .child(inner)
            .children(running.not().then_some(grip))
            .into_any_element()
    }
}

/// A drag on a panel's edge, from the pressed width.
struct Grab {
    side: Side,
    start: Pixels,
    anchor: Rc<Cell<Option<Pixels>>>,
}

/// Defaults leave the page most of the window.
fn default_width(side: Side) -> Pixels {
    match side {
        Side::Left => px(220.0),
        // A name column and an editor beside it.
        Side::Right => px(320.0),
    }
}

/// The user's width, else the default, within bounds.
pub(super) fn fitted(side: Side, window: &Window, cx: &App) -> Pixels {
    let chosen = crate::settings::panels::width(side == Side::Left, cx);
    clamp(side, chosen.unwrap_or(default_width(side)), window)
}

/// The narrowest a panel works: a list, or properties.
fn least(side: Side) -> Pixels {
    match side {
        Side::Left => px(160.0),
        Side::Right => px(280.0),
    }
}

/// At least its least, under half the window.
fn clamp(side: Side, wanted: Pixels, window: &Window) -> Pixels {
    let most = (window.viewport_size().width * 0.45).max(least(side));
    wanted.clamp(least(side), most)
}

/// The content keeps this much; panels give way first.
const CONTENT_LEAST: f32 = 360.0;

/// Shown widths: shrink both, then hide right, then left.
pub(super) fn room(window: Pixels, left: Option<Pixels>, right: Option<Pixels>) -> (Option<Pixels>, Option<Pixels>) {
    let spare = window - px(CONTENT_LEAST);
    if let (Some(l), Some(r)) = (left, right) {
        let r = r.min((spare - l).max(least(Side::Right)));
        let l = l.min((spare - r).max(least(Side::Left)));
        if l + r <= spare {
            return (Some(l), Some(r));
        }
        return (alone(Side::Left, l.max(left.unwrap_or(l)), spare), None);
    }
    (left.and_then(|l| alone(Side::Left, l, spare)), right.and_then(|r| alone(Side::Right, r, spare)))
}

/// One panel in `spare`, if its least fits.
fn alone(side: Side, wanted: Pixels, spare: Pixels) -> Option<Pixels> {
    let shown = wanted.min(spare);
    (shown >= least(side)).then_some(shown)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use gpui::px;

    use super::{Slide, room};

    const LENGTH: Duration = Duration::from_millis(200);

    fn after(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn a_panel_starts_where_it_is_aimed_and_stays() {
        let t0 = Instant::now();

        assert_eq!(Slide::new(true, t0).share(after(t0, 5000), LENGTH), 1.0);
        assert_eq!(Slide::new(false, t0).share(after(t0, 5000), LENGTH), 0.0);
        assert!(!Slide::new(true, t0).running(t0, LENGTH));
    }

    #[test]
    fn opening_rises_without_stepping_back_and_ends_at_one() {
        let t0 = Instant::now();
        let mut slide = Slide::new(false, t0);
        slide.aim(true, t0, LENGTH);

        let shares: Vec<f32> = (0..=20)
            .map(|i| slide.share(after(t0, i * 10), LENGTH))
            .collect();
        assert_eq!(shares[0], 0.0);
        assert_eq!(shares[20], 1.0);
        assert!(
            shares.windows(2).all(|pair| pair[1] >= pair[0]),
            "{shares:?}"
        );
        assert!(slide.running(after(t0, 100), LENGTH));
        assert!(!slide.running(after(t0, 200), LENGTH));
        assert_eq!(slide.share(after(t0, 900), LENGTH), 1.0);
    }

    #[test]
    fn flipping_the_aim_mid_flight_continues_from_where_it_was() {
        let t0 = Instant::now();
        let mut slide = Slide::new(false, t0);
        slide.aim(true, t0, LENGTH);
        let mid = after(t0, 80);
        let before = slide.share(mid, LENGTH);
        assert!(before > 0.0 && before < 1.0);

        slide.aim(false, mid, LENGTH);

        assert_eq!(slide.share(mid, LENGTH), before);
        assert!(slide.share(after(mid, 40), LENGTH) < before);
        assert_eq!(slide.share(after(mid, 400), LENGTH), 0.0);
    }

    #[test]
    fn aiming_again_at_the_same_target_does_not_restart() {
        let t0 = Instant::now();
        let mut slide = Slide::new(false, t0);
        slide.aim(true, t0, LENGTH);
        let at = after(t0, 120);
        let share = slide.share(at, LENGTH);

        slide.aim(true, at, LENGTH);

        assert_eq!(slide.share(at, LENGTH), share);
        assert_eq!(slide.share(after(t0, 200), LENGTH), 1.0);
    }

    #[test]
    fn reduced_motion_settles_in_a_millisecond() {
        let t0 = Instant::now();
        let mut slide = Slide::new(false, t0);
        slide.aim(true, t0, Duration::from_millis(1));

        assert_eq!(slide.share(after(t0, 1), Duration::from_millis(1)), 1.0);
        assert!(!slide.running(after(t0, 1), Duration::from_millis(1)));
    }

    #[test]
    fn panels_give_way_so_the_content_keeps_its_room() {
        let (left, right) = (Some(px(220.0)), Some(px(320.0)));
        assert_eq!(room(px(1100.0), left, right), (left, right), "wide: as chosen");
        // 800 leaves 440: both shrink to their least.
        assert_eq!(room(px(800.0), left, right), (Some(px(160.0)), Some(px(280.0))));
        // 640 leaves 280: the right goes, the left stays.
        assert_eq!(room(px(640.0), left, right), (left, None));
        // A wider left alone shrinks to the spare room.
        assert_eq!(room(px(640.0), Some(px(400.0)), None), (Some(px(280.0)), None));
        // Too narrow for even the least: hidden.
        assert_eq!(room(px(500.0), left, None), (None, None));
        assert_eq!(room(px(640.0), None, right), (None, Some(px(280.0))));
    }
}
