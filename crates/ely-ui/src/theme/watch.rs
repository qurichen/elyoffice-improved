use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use futures::StreamExt;
use gpui::{App, AsyncApp};

use super::{OmarchyState, apply, omarchy};

const SETTLE: Duration = Duration::from_millis(250);

/// Loads the Omarchy theme now and after each change.
pub(super) fn start(dir: PathBuf, cx: &mut App) {
    reload(&dir, cx);
    let (watcher, mut changes) = match omarchy::changes(&dir) {
        Ok(watch) => watch,
        Err(error) => {
            log::error!("theme: omarchy sync does not follow changes: {error:#}");
            return;
        }
    };
    cx.spawn(async move |cx: &mut AsyncApp| {
        let _watcher = watcher;
        while changes.next().await.is_some() {
            cx.background_executor().timer(SETTLE).await;
            while changes.try_recv().is_ok() {}
            cx.update(|cx| reload(&dir, cx));
        }
    })
    .detach();
}

fn reload(dir: &Path, cx: &mut App) {
    match omarchy::load(dir) {
        Ok(Some(theme)) => {
            let name = theme.name.as_deref().unwrap_or("unnamed");
            if !theme.standing_in.is_empty() {
                log::warn!(
                    "theme: omarchy {name} lacks {}, Ely's colors stand in",
                    theme.standing_in.join(", ")
                );
            }
            cx.global_mut::<OmarchyState>().0 = Some(theme);
            apply(cx, false);
        }
        Ok(None) => {
            if cx.global_mut::<OmarchyState>().0.take().is_some() {
                log::info!("theme: omarchy theme has no colors.toml, Ely's own colors again");
                apply(cx, false);
            }
        }
        Err(error) => {
            log::error!("theme: omarchy theme ignored, the current colors stay: {error:#}");
        }
    }
}
