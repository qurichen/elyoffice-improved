//! Guides over the canvas; LOK paints none (SHOW-06).

use ely_gpui_component::theme::ActiveTheme as _;
use ely_lok::{
    LokView,
    show::{Guide, GuideKind},
};
use ely_office::prelude::*;
use gpui::{
    AnyElement, Context, CursorStyle, Entity, InteractiveElement as _, IntoElement, MouseButton,
    MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point, SharedString, Styled as _,
    div, px,
};

use crate::deck::Deck;

/// The strip a pointer grabs a guide by.
const GRIP: f32 = 6.0;

/// A guide being dragged, and where it is now.
#[derive(Clone, Copy)]
pub struct Drag {
    pub index: usize,
    pub kind: GuideKind,
    pub at: Point<Pixels>,
}

fn twips(mm100: i64) -> i32 {
    (mm100 * 72 / 127) as i32
}

fn mm100(twips: i32) -> i64 {
    i64::from(twips) * 127 / 72
}

impl Deck {
    /// The guide lines and their grips, over `view`.
    pub fn guide_layer(
        &self,
        view: &Entity<LokView>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let tab = self.active_tab()?;
        let guides = tab.extra.objects.as_ref()?.guides.clone();
        if guides.is_empty() {
            return None;
        }
        let (w, h) = tab.extra.shown_size();
        let lok = view.read(cx);
        let top_left = lok.place((0, 0));
        let bottom_right = lok.place((twips(w), twips(h)));
        let color = cx.theme().colors.accent;
        let drag = self.guide_drag;
        let places: Vec<Point<Pixels>> = guides
            .iter()
            .enumerate()
            .map(
                |(index, guide)| match drag.filter(|drag| drag.index == index) {
                    Some(drag) => drag.at,
                    None => lok.place((twips(guide.at.0), twips(guide.at.1))),
                },
            )
            .collect();
        let bounds = (top_left, bottom_right);
        let lines: Vec<AnyElement> = guides
            .iter()
            .enumerate()
            .map(|(index, guide)| self.guide_line((index, guide), places[index], bounds, color, cx))
            .collect();
        let mut layer = div().absolute().inset_0().children(lines);
        if drag.is_some() {
            // Catches the pointer anywhere until release.
            layer = layer.child(
                div()
                    .id("show-guide-drag")
                    .absolute()
                    .inset_0()
                    .on_mouse_move(cx.listener(|deck, event: &MouseMoveEvent, _, cx| {
                        deck.drag_guide(event.position, cx)
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|deck, _: &MouseUpEvent, _, cx| deck.drop_guide(cx)),
                    ),
            );
        }
        Some(layer.into_any_element())
    }

    fn guide_line(
        &self,
        (index, guide): (usize, &Guide),
        at: Point<Pixels>,
        (from, to): (Point<Pixels>, Point<Pixels>),
        color: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = SharedString::from(format!("show-guide-{index}"));
        let kind = guide.kind;
        let grip = div()
            .id(id)
            .absolute()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |deck, event: &gpui::MouseDownEvent, _, cx| {
                    let origin = deck.view_origin(cx);
                    deck.guide_drag = Some(Drag {
                        index,
                        kind,
                        at: event.position - origin,
                    });
                    cx.stop_propagation();
                    cx.notify();
                }),
            );
        let half = px(GRIP / 2.0);
        match kind {
            GuideKind::Horizontal => grip
                .left(from.x)
                .w(to.x - from.x)
                .top(at.y - half)
                .h(px(GRIP))
                .cursor(CursorStyle::ResizeUpDown)
                .child(div().w_full().h(px(1.0)).bg(color)),
            GuideKind::Vertical => grip
                .top(from.y)
                .h(to.y - from.y)
                .left(at.x - half)
                .w(px(GRIP))
                .cursor(CursorStyle::ResizeLeftRight)
                .child(div().h_full().w(px(1.0)).bg(color)),
            GuideKind::Point => grip
                .left(at.x - half)
                .top(at.y - half)
                .size(px(GRIP))
                .cursor(CursorStyle::Crosshair)
                .child(div().size(px(GRIP)).border_1().border_color(color)),
        }
        .into_any_element()
    }

    fn view_origin(&self, cx: &Context<Self>) -> Point<Pixels> {
        self.active_tab()
            .map_or_else(Point::default, |tab| tab.view.read(cx).origin())
    }

    fn drag_guide(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let origin = self.view_origin(cx);
        if let Some(drag) = self.guide_drag.as_mut() {
            drag.at = position - origin;
            cx.notify();
        }
    }

    /// Moves the guide where dropped; off the slide deletes it.
    fn drop_guide(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.guide_drag.take() else {
            return;
        };
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (w, h) = tab.extra.shown_size();
        let (x, y) = tab.view.read(cx).twips_at(drag.at);
        let (x, y) = (mm100(x), mm100(y));
        let off = match drag.kind {
            GuideKind::Horizontal => !(0..=h).contains(&y),
            GuideKind::Vertical => !(0..=w).contains(&x),
            GuideKind::Point => !(0..=w).contains(&x) || !(0..=h).contains(&y),
        };
        let index = drag.index.to_string();
        if off {
            log::info!("show: guide {index} dragged off the slide");
            self.object_command("guide.delete", &[&index, "true"], cx);
        } else {
            self.object_command("guide.move", &[&index, &x.to_string(), &y.to_string()], cx);
        }
        cx.notify();
    }
}
