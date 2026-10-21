use std::{
    io,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};

pub(super) const LEGACY_SERVICE: &str = "ElyOffice Recovery";
pub(super) const LEGACY_USER: &str = "recovery-key";
pub(super) const PROFILE_SERVICE: &str = "ElyOffice Recovery Profile";

pub(super) struct Scope {
    pub dir: PathBuf,
    pub user: String,
    pub profile: bool,
}

impl Scope {
    pub fn current() -> io::Result<Self> {
        if std::env::var_os("ELY_DATA_DIR").is_none() {
            return Self::legacy();
        }
        let root = ely_platform::data_root().map_err(io::Error::other)?;
        Self::explicit(&root)
    }

    pub fn legacy() -> io::Result<Self> {
        Ok(Self {
            dir: super::shared_dir(&super::account_home()?),
            user: LEGACY_USER.into(),
            profile: false,
        })
    }

    pub fn explicit(root: &Path) -> io::Result<Self> {
        validate(root)?;
        std::fs::create_dir_all(root)?;
        let dir = std::fs::canonicalize(root)?;
        validate(&dir)?;
        let mut digest = Sha256::new();
        digest.update(b"ElyOffice recovery profile v1\0");
        hash_path(&mut digest, &dir)?;
        let user = format!("profile-{:x}", digest.finalize());
        Ok(Self { dir, user, profile: true })
    }
}

fn validate(root: &Path) -> io::Result<()> {
    if !root.is_absolute()
        || root.parent().is_none()
        || root.components().any(|part| part == Component::ParentDir)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ELY_DATA_DIR must be an absolute non-root directory without '..'",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn hash_path(digest: &mut Sha256, path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    digest.update(b"windows-utf16le\0");
    for unit in path.as_os_str().encode_wide() {
        digest.update(unit.to_le_bytes());
    }
    Ok(())
}

#[cfg(unix)]
fn hash_path(digest: &mut Sha256, path: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    digest.update(b"unix-bytes\0");
    digest.update(path.as_os_str().as_bytes());
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn hash_path(digest: &mut Sha256, path: &Path) -> io::Result<()> {
    let text = path.to_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "profile path encoding")
    })?;
    digest.update(b"utf8\0");
    digest.update(text.as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests;
