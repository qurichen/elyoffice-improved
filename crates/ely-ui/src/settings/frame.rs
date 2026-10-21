//! Each app's main window opens where it was left (§18.1).

use std::time::Duration;

use ely_platform::{AppId, settings::Store};
use gpui::{App, AppContext as _, Bounds, Context, Entity, Global, Pixels, Window, WindowBounds, point, px, size};
use serde_json::{Value, json};

const KEY: &str = "window";
const SETTLE: Duration = Duration::from_millis(500);

/// Where the window was last, if that place still shows.
pub fn restore(app: AppId, cx: &mut App) -> Option<WindowBounds> {
    let unreadable = |why: String, cx: &mut App| {
        log::error!("settings: the window place is unreadable: {why}");
        cx.global_mut::<super::Current>().app_error.insert(super::WINDOW, why.into());
        None
    };
    let saved = match Store::app(app).and_then(|store| store.read()) {
        Ok(fields) => fields.get(KEY).cloned()?,
        Err(error) => return unreadable(error.to_string(), cx),
    };
    let Some(bounds) = parse(&saved) else {
        return unreadable(format!("the window place cannot be {saved}"), cx);
    };
    let center = bounds.get_bounds().center();
    // A display since unplugged would hide the window.
    cx.displays().iter().any(|display| display.bounds().contains(&center)).then_some(bounds)
}

fn parse(saved: &Value) -> Option<WindowBounds> {
    let number = |key: &str| saved.get(key)?.as_f64().map(|n| px(n as f32));
    let bounds = Bounds::new(point(number("x")?, number("y")?), size(number("width")?, number("height")?));
    match saved.get("mode")?.as_str()? {
        "windowed" => Some(WindowBounds::Windowed(bounds)),
        "maximized" => Some(WindowBounds::Maximized(bounds)),
        "fullscreen" => Some(WindowBounds::Fullscreen(bounds)),
        _ => None,
    }
}

fn written(bounds: WindowBounds) -> Value {
    let (mode, frame): (&str, Bounds<Pixels>) = match bounds {
        WindowBounds::Windowed(frame) => ("windowed", frame),
        WindowBounds::Maximized(frame) => ("maximized", frame),
        WindowBounds::Fullscreen(frame) => ("fullscreen", frame),
    };
    let n = |pixels: Pixels| f64::from(f32::from(pixels));
    json!({
        "mode": mode,
        "x": n(frame.origin.x),
        "y": n(frame.origin.y),
        "width": n(frame.size.width),
        "height": n(frame.size.height),
    })
}

struct Frame {
    app: AppId,
    moves: u64,
    saved: u64,
    last: Option<WindowBounds>,
}

impl Frame {
    /// Records a write; failures show and retry later.
    fn settled(&mut self, moves: u64, written: std::io::Result<()>, cx: &mut App) {
        match written {
            Ok(()) => {
                self.saved = moves;
                super::unsaved(super::WINDOW, None, cx);
            }
            Err(error) => {
                log::error!("settings: the window place is not saved: {error}");
                super::unsaved(super::WINDOW, Some(error.to_string().into()), cx);
            }
        }
    }

    /// Writes a pending place now; quitting cannot wait.
    fn flush(&mut self) {
        let Some(bounds) = self.last.filter(|_| self.moves != self.saved) else {
            return;
        };
        match Store::app(self.app).and_then(|store| store.set(KEY, written(bounds))) {
            Ok(()) => self.saved = self.moves,
            Err(error) => log::error!("settings: the window place is not saved at quit: {error}"),
        }
    }
}

/// The followed windows stay alive with the app.
struct Frames(Vec<Entity<Frame>>);

impl Global for Frames {}

/// Saves `window`'s place shortly after it stops changing.
pub fn follow(app: AppId, window: &mut Window, cx: &mut App) {
    let frame = cx.new(|cx: &mut Context<Frame>| {
        // Quitting right after a move still keeps the place.
        cx.on_app_quit(|frame, _| {
            frame.flush();
            async {}
        })
        .detach();
        cx.observe_window_bounds(window, |frame, window, cx| {
            frame.moves += 1;
            frame.last = Some(window.window_bounds());
            let (moves, app, bounds) = (frame.moves, frame.app, window.window_bounds());
            cx.spawn(async move |frame, cx| {
                cx.background_executor().timer(SETTLE).await;
                if frame.read_with(cx, |frame, _| frame.moves != moves).unwrap_or(true) {
                    return;
                }
                let saved = cx
                    .background_executor()
                    .spawn(async move { Store::app(app)?.set(KEY, written(bounds)) })
                    .await;
                frame.update(cx, |frame, cx| frame.settled(moves, saved, cx)).ok();
            })
            .detach();
        })
        .detach();
        Frame { app, moves: 0, saved: 0, last: None }
    });
    if !cx.has_global::<Frames>() {
        cx.set_global(Frames(Vec::new()));
    }
    cx.global_mut::<Frames>().0.push(frame);
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, WindowBounds, point, px, size};

    #[test]
    fn a_place_reads_back_as_written() {
        let frame = Bounds::new(point(px(40.0), px(60.0)), size(px(900.0), px(600.0)));
        for bounds in [WindowBounds::Windowed(frame), WindowBounds::Maximized(frame), WindowBounds::Fullscreen(frame)] {
            assert_eq!(super::parse(&super::written(bounds)), Some(bounds));
        }
        assert_eq!(super::parse(&serde_json::json!({"mode": "tiled", "x": 0})), None);
    }
}
