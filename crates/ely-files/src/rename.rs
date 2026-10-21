//! Renames that never replace a file (MARK-01).

use std::{io, path::Path};

/// Renames `from` to `to`; an existing `to` fails, atomically.
#[cfg(target_os = "macos")]
pub fn rename_new(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (c_path(from)?, c_path(to)?);
    // SAFETY: both are NUL-terminated paths that outlive the call.
    let done = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    if done == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
pub fn rename_new(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (c_path(from)?, c_path(to)?);
    // SAFETY: both are NUL-terminated paths that outlive the call.
    let done = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if done == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
pub fn rename_new(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain([0])
            .collect::<Vec<u16>>()
    };
    let (from, to) = (wide(from), wide(to));
    // SAFETY: both are NUL-terminated wide paths; no flag replaces.
    let done = unsafe {
        windows_sys::Win32::Storage::FileSystem::MoveFileExW(from.as_ptr(), to.as_ptr(), 0)
    };
    if done != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub fn rename_new(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "renaming is not built for this platform",
    ))
}

#[cfg(unix)]
fn c_path(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt as _;
    std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a path holds NUL"))
}

#[cfg(test)]
mod tests {
    use super::rename_new;

    #[test]
    fn a_rename_moves_and_never_replaces() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let (a, b, c) = (
            dir.path().join("a.md"),
            dir.path().join("b.md"),
            dir.path().join("c.md"),
        );
        std::fs::write(&a, "a").expect("writes");
        std::fs::write(&b, "b").expect("writes");
        let error = rename_new(&a, &b).expect_err("b exists");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&b).expect("reads"), "b");
        rename_new(&a, &c).expect("renames");
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(&c).expect("reads"), "a");
    }
}
