use std::{io, os::windows::ffi::OsStrExt as _, path::Path};

use windows_sys::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
    UI::{Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW},
        WindowsAndMessaging::SW_SHOWNORMAL},
};

fn file_arg(path: &Path) -> io::Result<Vec<u16>> {
    if path.as_os_str().encode_wide().any(|unit| unit == 0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a path holds NUL"));
    }
    let path = std::path::absolute(path)?;
    std::fs::metadata(&path)?;
    Ok(path.as_os_str().encode_wide().chain([0]).collect())
}

pub fn open_default(path: &Path) -> io::Result<()> {
    let file = file_arg(path)?;
    let initialized = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    if initialized < 0 && initialized != RPC_E_CHANGED_MODE {
        return Err(io::Error::other(format!("the system launcher could not initialize COM: {initialized:#x}")));
    }
    let mut request: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    request.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    request.fMask = SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC;
    request.lpVerb = windows_sys::w!("open");
    request.lpFile = file.as_ptr();
    request.nShow = SW_SHOWNORMAL;
    let opened = unsafe { ShellExecuteExW(&mut request) };
    let result = if opened == 0 { Err(io::Error::last_os_error()) } else { Ok(()) };
    if initialized >= 0 { unsafe { CoUninitialize() }; }
    result?;
    log::info!("apps: the system opened an output");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_paths_preserve_spaces_unicode_and_metacharacters() {
        let dir = tempfile::tempdir().expect("fixture folder");
        let path = dir.path().join("报告 & (1) %USERNAME%.txt");
        std::fs::write(&path, b"fixture").expect("fixture file");
        let wide = file_arg(&path).expect("literal argument");
        assert_eq!(wide.last(), Some(&0));
        assert_eq!(&wide[..wide.len() - 1], path.as_os_str().encode_wide().collect::<Vec<_>>());
    }

    #[test]
    fn missing_and_nul_paths_fail_without_starting_an_app() {
        let dir = tempfile::tempdir().expect("fixture folder");
        assert_eq!(open_default(&dir.path().join("missing.txt")).unwrap_err().kind(), io::ErrorKind::NotFound);
        assert_eq!(open_default(Path::new("before\0after")).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }
}
