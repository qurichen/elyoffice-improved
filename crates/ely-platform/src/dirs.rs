use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use directories::BaseDirs;

use crate::AppId;

/// App-owned state, optionally under the explicit trial profile.
pub fn data_dir(app: AppId) -> Result<PathBuf> {
    Ok(root()?.join(app.slug()))
}

/// The suite's folder: each app's, and the shared settings.
pub fn root() -> Result<PathBuf> {
    root_from(std::env::var_os("ELY_DATA_DIR"))
}

fn root_from(override_dir: Option<OsString>) -> Result<PathBuf> {
    if let Some(value) = override_dir {
        let path = PathBuf::from(value);
        ensure!(
            path.is_absolute()
                && path.parent().is_some()
                && !path.components().any(|part| part == Component::ParentDir),
            "ELY_DATA_DIR must be an absolute non-root directory without '..'"
        );
        return Ok(path);
    }
    let base = BaseDirs::new().context("no home directory")?;
    Ok(base.data_local_dir().join("ElyOffice"))
}

/// Omarchy's active theme directory; `ELY_OMARCHY_DIR` overrides it.
pub fn omarchy_current_dir() -> Option<PathBuf> {
    let home = BaseDirs::new()?.home_dir().to_path_buf();
    omarchy_dir(&home, crate::env::value("ELY_OMARCHY_DIR"))
}

/// Omarchy keeps its state under `~/.local/state`, whatever `XDG_STATE_HOME` says.
fn omarchy_dir(home: &Path, over: Option<String>) -> Option<PathBuf> {
    if let Some(dir) = over {
        return Some(PathBuf::from(dir));
    }
    let current = home.join(".local/state/omarchy/current");
    current.is_dir().then_some(current)
}

#[cfg(test)]
mod tests {
    use super::{data_dir, omarchy_dir, root_from};
    use crate::AppId;

    #[test]
    fn each_app_has_its_own_directory_under_the_suite_root() {
        let doc = data_dir(AppId::Doc).unwrap();
        let mail = data_dir(AppId::Mail).unwrap();

        let root = super::root().unwrap();
        assert_eq!(doc, root.join("ely-doc"));
        assert_eq!(mail, root.join("ely-mail"));
        assert_ne!(doc, mail);
    }

    #[test]
    fn an_unset_override_keeps_the_platform_default() {
        let base = directories::BaseDirs::new().unwrap();
        assert_eq!(
            root_from(None).unwrap(),
            base.data_local_dir().join("ElyOffice")
        );
    }

    #[test]
    fn an_absolute_unicode_profile_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("隔离 profile & 100%");
        assert_eq!(
            root_from(Some(path.clone().into_os_string())).unwrap(),
            path
        );
        assert!(!path.exists(), "resolving a profile writes nothing");
    }

    #[test]
    fn invalid_overrides_never_select_the_default() {
        let root = if cfg!(windows) { "C:/" } else { "/" };
        let parent = std::path::Path::new(root).join("trial/../other");
        for path in ["", "relative", ".", root, parent.to_str().unwrap()] {
            assert!(root_from(Some(path.into())).is_err(), "{path:?}");
        }
        #[cfg(windows)]
        for path in ["C:relative", "\\relative"] {
            assert!(root_from(Some(path.into())).is_err(), "{path:?}");
        }
    }

    #[test]
    fn omarchy_is_found_under_home_and_the_override_wins() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(omarchy_dir(home.path(), None), None);

        let current = home.path().join(".local/state/omarchy/current");
        std::fs::create_dir_all(&current).unwrap();
        assert_eq!(omarchy_dir(home.path(), None), Some(current));
        assert_eq!(
            omarchy_dir(home.path(), Some("/elsewhere".into())),
            Some("/elsewhere".into())
        );
    }
}
