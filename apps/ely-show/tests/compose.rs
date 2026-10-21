//! Transition frames drawn by Metal, read back as pixels.

#[cfg(target_os = "macos")]
#[path = "../src/preview/transition.rs"]
#[allow(dead_code, unused_imports)]
mod transition;

#[cfg(target_os = "macos")]
#[path = "../src/preview/timeline.rs"]
#[allow(dead_code, unused_imports)]
mod timeline;

#[cfg(target_os = "macos")]
#[path = "../src/preview/compose.rs"]
mod compose;

#[cfg(target_os = "macos")]
fn main() {
    use std::sync::Arc;

    use ely_lok::show::Part;
    use gpui::{
        AppContext as _, Context, IntoElement, ParentElement as _, Render, RenderImage,
        Styled as _, VisualTestAppContext, Window, div, px, size,
    };
    use image::{Frame, ImageBuffer, Rgba, RgbaImage};

    type Stack = Vec<(Part, Arc<RenderImage>)>;
    // Pixels go in as BGRA; `fill` picks the painted columns.
    let picture = |bgra: [u8; 4], fill: fn(u32) -> bool| {
        let image = ImageBuffer::<Rgba<u8>, _>::from_fn(16, 8, |x, _| match fill(x) {
            true => Rgba(bgra),
            false => Rgba([0, 0, 0, 0]),
        });
        Arc::new(RenderImage::new([Frame::new(image)]))
    };
    let plain = |bgra: [u8; 4]| vec![(Part::Still, picture(bgra, |_| true))];
    struct Shot {
        old: Stack,
        new: Stack,
        preset: &'static str,
        /// The old slide's effects, as when it left.
        runs: Option<Vec<timeline::Run>>,
    }
    impl Render for Shot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let frame = transition::frame(self.preset, 0.5);
            let (w, h) = (px(200.0), px(100.0));
            let old = match &self.runs {
                Some(runs) => compose::left(
                    (&self.old, runs),
                    ((1600.0, 800.0), 1.0),
                    (w, h),
                    gpui::green(),
                ),
                None => {
                    compose::slide(&self.old, |_| true, (w, h), gpui::green()).into_any_element()
                }
            };
            let shown = compose::transition((Some(old), &self.new), &frame, (w, h), gpui::green());
            div().w(w).h(h).relative().child(shown)
        }
    }
    let mut cx = VisualTestAppContext::new(gpui_platform::current_platform(false));
    let mut shoot = |shot: Shot| -> RgbaImage {
        let window = cx
            .open_offscreen_window(size(px(200.0), px(100.0)), |_, cx| cx.new(|_| shot))
            .expect("opens a window");
        // Pictures load after the first frame.
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
                .expect("draws");
        }
        cx.capture_screenshot(window.into()).expect("draws")
    };
    let at = |shot: &RgbaImage, x: f32| {
        *shot.get_pixel((shot.width() as f32 * x) as u32, shot.height() / 2)
    };
    for preset in ["uncover-left", "cover-left", "push-left"] {
        let (old, new) = (plain([0, 0, 255, 255]), plain([255, 0, 0, 255]));
        let shot = shoot(Shot {
            old,
            new,
            preset,
            runs: None,
        });
        let (left, right) = (at(&shot, 0.25), at(&shot, 0.75));
        // Halfway, the old slide is left, the new one right.
        assert!(left[0] > 200 && left[2] < 50, "{preset}: left is {left:?}");
        assert!(
            right[2] > 200 && right[0] < 50,
            "{preset}: right is {right:?}"
        );
    }
    // A red object, its exit done, covered from the right.
    let object = || {
        let moving = Part::Moving {
            hash: "gone".into(),
            visible: true,
            bounds: (0, 0, 400, 800),
        };
        vec![
            (Part::Still, picture([0, 0, 0, 0], |_| true)),
            (moving, picture([0, 0, 255, 255], |x| x < 4)),
        ]
    };
    let exit = timeline::Run {
        hash: "gone".into(),
        preset: "ooo-exit-disappear".into(),
        begin: 0.0,
        end: 0.5,
    };
    for (runs, red) in [(vec![exit], false), (Vec::new(), true)] {
        let shot = shoot(Shot {
            old: object(),
            new: plain([255, 0, 0, 255]),
            preset: "cover-left",
            runs: Some(runs),
        });
        let pixel = at(&shot, 0.1);
        assert_eq!(
            pixel[0] > 200 && pixel[1] < 50,
            red,
            "the object is {pixel:?}"
        );
    }
    println!("compose: halfway frames show both slides in order; exits stay gone");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("SKIP: the pixel check runs on macOS");
}
