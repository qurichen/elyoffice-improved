use super::parent_dir;
use std::{fs, io, path::Path};

pub(super) fn prepare_staging(target: &Path) -> io::Result<tempfile::NamedTempFile> {
    let metadata = match fs::metadata(target) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    #[cfg(windows)]
    if metadata
        .as_ref()
        .is_some_and(|meta| meta.permissions().readonly())
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the target is read-only",
        ));
    }
    #[cfg(windows)]
    let staging = if metadata.is_some() {
        tempfile::Builder::new().make_in(parent_dir(target), |path| {
            super::windows::create_private(target, path, true)
        })?
    } else {
        tempfile::NamedTempFile::new_in(parent_dir(target))?
    };
    #[cfg(not(windows))]
    let staging = tempfile::NamedTempFile::new_in(parent_dir(target))?;
    if let Some(metadata) = metadata {
        #[cfg(not(windows))]
        super::copy_acl(target, staging.path())?;
        staging.as_file().set_permissions(metadata.permissions())?;
    }
    Ok(staging)
}

pub(super) fn create_backup(source: &Path, target: &Path) -> io::Result<fs::File> {
    let metadata = fs::metadata(source)?;
    #[cfg(windows)]
    let file = super::windows::create_private(source, target, false)?;
    #[cfg(not(windows))]
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    #[cfg(not(windows))]
    super::copy_acl(source, target)?;
    file.set_permissions(metadata.permissions())?;
    Ok(file)
}
