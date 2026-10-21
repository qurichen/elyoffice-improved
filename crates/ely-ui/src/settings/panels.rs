//! Side panel widths, kept per app (§18.1).

use std::{io, time::Duration};

use ely_platform::{AppId, settings::Store};
use gpui::{App, AsyncApp, Global, Pixels, px};
use serde_json::{Value, json};

const KEY: &str = "panels";
const SETTLE: Duration = Duration::from_millis(500);

/// Widths the user dragged to; none keeps the default.
pub(crate) struct Panels {
    store: Option<Store>,
    left: Option<Pixels>,
    right: Option<Pixels>,
    moves: u64,
    saved: u64,
}

impl Global for Panels {}

/// Reads `app`'s widths; a bad entry is reported, not guessed.
pub(super) fn install(app: AppId, cx: &mut App) {
    let store = Store::app(app);
    let read = store.as_ref().map_err(|error| io::Error::new(error.kind(), error.to_string())).and_then(|store| {
        let fields = store.read()?;
        parse(fields.get(KEY))
    });
    let (left, right) = read.unwrap_or_else(|error| {
        log::error!("settings: panel widths unreadable: {error}");
        cx.global_mut::<super::Current>().app_error.insert(super::PANELS, error.to_string().into());
        (None, None)
    });
    cx.set_global(Panels { store: store.ok(), left, right, moves: 0, saved: 0 });
    // Quitting right after a drag still keeps the width.
    cx.on_app_quit(|cx| {
        flush(cx);
        async {}
    })
    .detach();
}

/// Absent or null is default; else a positive width.
fn parse(saved: Option<&Value>) -> io::Result<(Option<Pixels>, Option<Pixels>)> {
    let invalid = |why: String| io::Error::new(io::ErrorKind::InvalidData, format!("settings: panels {why}"));
    let fields = match saved {
        None | Some(Value::Null) => return Ok((None, None)),
        Some(Value::Object(fields)) => fields,
        Some(other) => return Err(invalid(format!("cannot be {other}"))),
    };
    let width = |side: &str| match fields.get(side) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) if n.as_f64().is_some_and(|n| n.is_finite() && n > 0.0) => {
            Ok(n.as_f64().map(|n| px(n as f32)))
        }
        Some(other) => Err(invalid(format!("{side} cannot be {other}"))),
    };
    Ok((width("left")?, width("right")?))
}

/// Defaults in force, nothing kept: for tests.
#[cfg(any(test, feature = "test-support"))]
pub(super) fn install_defaults(cx: &mut App) {
    cx.set_global(Panels { store: None, left: None, right: None, moves: 0, saved: 0 });
}

/// The user's width for the left or right panel.
pub(crate) fn width(left: bool, cx: &App) -> Option<Pixels> {
    let panels = cx.global::<Panels>();
    if left { panels.left } else { panels.right }
}

/// Takes a dragged width; saved once dragging rests.
pub(crate) fn set_width(left: bool, width: Pixels, cx: &mut App) {
    let panels = cx.global_mut::<Panels>();
    *(if left { &mut panels.left } else { &mut panels.right }) = Some(width);
    panels.moves += 1;
    let moves = panels.moves;
    cx.spawn(async move |cx: &mut AsyncApp| {
        cx.background_executor().timer(SETTLE).await;
        let Some((store, saved)) = cx.update(|cx| pending(cx).filter(|_| cx.global::<Panels>().moves == moves)) else {
            return;
        };
        let written = cx.background_executor().spawn(async move { store.set(KEY, saved) }).await;
        cx.update(|cx| settled(moves, written, cx));
    })
    .detach();
}

/// What to write while the last drag is unkept.
fn pending(cx: &App) -> Option<(Store, Value)> {
    let panels = cx.global::<Panels>();
    let store = panels.store.clone().filter(|_| panels.moves != panels.saved)?;
    let n = |w: Option<Pixels>| w.map(|w| f64::from(f32::from(w)));
    Some((store, json!({ "left": n(panels.left), "right": n(panels.right) })))
}

/// Records a write; failures show and retry later.
fn settled(moves: u64, written: io::Result<()>, cx: &mut App) {
    match written {
        Ok(()) => {
            cx.global_mut::<Panels>().saved = moves;
            super::unsaved(super::PANELS, None, cx);
        }
        Err(error) => {
            log::error!("settings: panel widths not saved: {error}");
            super::unsaved(super::PANELS, Some(error.to_string().into()), cx);
        }
    }
}

/// Writes a pending width now; quitting cannot wait.
fn flush(cx: &mut App) {
    if let Some((store, saved)) = pending(cx) {
        let moves = cx.global::<Panels>().moves;
        let written = store.set(KEY, saved);
        settled(moves, written, cx);
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;
    use serde_json::json;

    #[test]
    fn widths_read_strictly() {
        assert_eq!(super::parse(None).unwrap(), (None, None));
        assert_eq!(super::parse(Some(&json!({"left": 300, "right": null}))).unwrap(), (Some(px(300.0)), None));
        for bad in [json!({"left": "broken", "right": 320}), json!({"left": -4}), json!([1])] {
            assert!(super::parse(Some(&bad)).is_err(), "{bad}");
        }
    }
}
