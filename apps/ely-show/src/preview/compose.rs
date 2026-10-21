//! Slides drawn from their layers: effects and transitions.

use std::sync::Arc;

use ely_lok::show::Part;
use gpui::{AnyElement, IntoElement, ParentElement as _, RenderImage, Styled as _, div, img, px};

use super::{timeline, transition};

type Px = gpui::Pixels;
/// A slide's layers, bottom first.
pub type Stack = [(Part, Arc<RenderImage>)];

/// A whole slide on its page; `moving` picks objects.
pub fn slide(
    layers: &Stack,
    moving: fn(bool) -> bool,
    (w, h): (Px, Px),
    paper: gpui::Hsla,
) -> gpui::Div {
    let shown = layers.iter().filter(|(part, _)| match part {
        Part::Moving { visible, .. } => moving(*visible),
        _ => true,
    });
    let pictures = shown.map(|(_, image)| img(image.clone()).absolute().size_full());
    div().relative().w(w).h(h).bg(paper).children(pictures)
}

/// A transition frame: old slide as drawn, new unplayed.
pub fn transition(
    (from, to): (Option<AnyElement>, &Stack),
    frame: &transition::Frame,
    (w, h): (Px, Px),
    paper: gpui::Hsla,
) -> AnyElement {
    let at = |shift: (f32, f32)| {
        div()
            .absolute()
            .left(w * shift.0)
            .top(h * shift.1)
            .w(w)
            .h(h)
    };
    let old = from.map(|from| at(frame.from_shift).child(from));
    let new = frame.to_areas.iter().map(|area| {
        let (x0, y0, x1, y1) = *area;
        let whole = slide(to, |visible| visible, (w, h), paper);
        div()
            .absolute()
            .left(w * (x0 + frame.to_shift.0))
            .top(h * (y0 + frame.to_shift.1))
            .w(w * (x1 - x0))
            .h(h * (y1 - y0))
            .overflow_hidden()
            .opacity(frame.to_opacity)
            .child(div().absolute().left(-(w * x0)).top(-(h * y0)).child(whole))
    });
    // Uncover slides the old slide off the new one beneath.
    let old = old.map(IntoElement::into_any_element);
    let new = new.map(IntoElement::into_any_element);
    let pieces: Vec<AnyElement> = match frame.old_on_top {
        true => new.chain(old).collect(),
        false => old.into_iter().chain(new).collect(),
    };
    div()
        .size_full()
        .children(pieces)
        .child(
            div()
                .absolute()
                .size_full()
                .bg(gpui::black())
                .opacity(frame.black),
        )
        .into_any_element()
}

/// The layers at second `t`, moved as effects say.
pub fn effects(
    layers: &[(Part, Arc<RenderImage>)],
    runs: &[timeline::Run],
    slide: (f32, f32),
    t: f64,
    (w, h): (Px, Px),
) -> AnyElement {
    let pieces = layers.iter().filter_map(|(part, image)| {
        let full = div().absolute().left(px(0.0)).top(px(0.0)).w(w).h(h);
        match part {
            Part::Still => Some(
                full.child(img(image.clone()).size_full())
                    .into_any_element(),
            ),
            Part::Moving {
                hash,
                visible,
                bounds,
            } => {
                let (x, y) = (bounds.0 as f32 / slide.0, bounds.1 as f32 / slide.1);
                let (bw, bh) = (bounds.2 as f32 / slide.0, bounds.3 as f32 / slide.1);
                let look = timeline::object_look(runs, hash, *visible, t, y);
                look.visible
                    .then(|| moved(image, look, (x, y, bw, bh), (w, h)))
            }
        }
    });
    div().size_full().children(pieces).into_any_element()
}

/// One object's layer, shifted, scaled about its centre and wiped.
fn moved(
    image: &Arc<RenderImage>,
    look: timeline::Look,
    (x, y, bw, bh): (f32, f32, f32, f32),
    (w, h): (Px, Px),
) -> AnyElement {
    let (cx, cy) = (x + bw / 2.0, y + bh / 2.0);
    let s = look.scale.max(0.001);
    // Scaling the whole layer about the object's centre.
    let left = w * ((cx - cx * s) + look.shift.0);
    let top = h * ((cy - cy * s) + look.shift.1);
    let clip_top = y + bh * (1.0 - look.reveal);
    let picture = img(image.clone()).absolute().left(left).w(w * s).h(h * s);
    div()
        .absolute()
        .left(px(0.0))
        .top(h * clip_top)
        .w(w)
        .h(h * (1.0 - clip_top))
        .overflow_hidden()
        .opacity(look.opacity)
        .child(picture.top(top - h * clip_top))
        .into_any_element()
}

/// A left slide, its effects as at second `at`.
pub fn left(
    (stack, runs): (&Stack, &[timeline::Run]),
    (slide, at): ((f32, f32), f64),
    size: (Px, Px),
    paper: gpui::Hsla,
) -> AnyElement {
    div()
        .size_full()
        .bg(paper)
        .child(effects(stack, runs, slide, at, size))
        .into_any_element()
}
