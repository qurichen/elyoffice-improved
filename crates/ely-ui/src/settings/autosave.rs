//! Saving ordinary files on their own, kept per app (§6.2).

use std::io;

use ely_platform::{AppId, settings::Store};
use gpui::{App, AsyncApp, Global};
use serde_json::Value;

const KEY: &str = "autosave";

/// Whether autosave is on, and the latest switch.
struct Autosave {
    store: Option<Store>,
    on: bool,
    turns: u64,
}

impl Global for Autosave {}

/// Reads `app`'s choice; a bad entry is reported and off.
pub(super) fn install(app: AppId, cx: &mut App) {
    let store = Store::app(app);
    let read = store
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))
        .and_then(|store| parse(store.read()?.get(KEY)));
    let on = read.unwrap_or_else(|error| {
        log::error!("settings: autosave unreadable: {error}");
        cx.global_mut::<super::Current>().app_error.insert(super::AUTOSAVE, error.to_string().into());
        false
    });
    cx.set_global(Autosave { store: store.ok(), on, turns: 0 });
}

/// Absent is off; else a boolean.
fn parse(saved: Option<&Value>) -> io::Result<bool> {
    match saved {
        None => Ok(false),
        Some(Value::Bool(on)) => Ok(*on),
        Some(other) => Err(io::Error::new(io::ErrorKind::InvalidData, format!("settings: autosave cannot be {other}"))),
    }
}

/// Off without reading the file: for tests.
#[cfg(any(test, feature = "test-support"))]
pub(super) fn install_defaults(cx: &mut App) {
    cx.set_global(Autosave { store: None, on: false, turns: 0 });
}

/// Whether ordinary files save on their own.
pub fn on(cx: &App) -> bool {
    cx.global::<Autosave>().on
}

/// Takes the switch now; the latest one is written.
pub(super) fn set(on: bool, cx: &mut App) {
    let autosave = cx.global_mut::<Autosave>();
    autosave.on = on;
    autosave.turns += 1;
    let turn = autosave.turns;
    let Some(store) = autosave.store.clone() else {
        return super::unsaved(super::AUTOSAVE, Some("no settings file for this app".into()), cx);
    };
    cx.spawn(async move |cx: &mut AsyncApp| {
        let latest = cx.update(|cx| cx.global::<Autosave>().turns == turn);
        if !latest {
            return;
        }
        let written = cx.background_executor().spawn(async move { store.set(KEY, Value::Bool(on)) }).await;
        cx.update(|cx| {
            super::unsaved(super::AUTOSAVE, written.err().map(|error| {
                log::error!("settings: autosave not saved: {error}");
                error.to_string().into()
            }), cx)
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn the_switch_reads_strictly() {
        assert!(!super::parse(None).unwrap());
        assert!(super::parse(Some(&json!(true))).unwrap());
        for bad in [json!("yes"), json!(1), json!(null)] {
            assert!(super::parse(Some(&bad)).is_err(), "{bad}");
        }
    }
}
