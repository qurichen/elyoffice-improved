use std::{io, path::Path};

pub(super) fn validate(name: &Path) -> io::Result<()> {
    let valid = name.file_name().is_some_and(|file| file == name.as_os_str());
    #[cfg(windows)]
    let valid = valid && windows_name(name.as_os_str());
    if !valid {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "an attachment needs one ordinary file name"));
    }
    Ok(())
}

#[cfg(windows)]
fn windows_name(name: &std::ffi::OsStr) -> bool {
    use std::os::windows::ffi::OsStrExt as _;
    let units: Vec<_> = name.encode_wide().collect();
    if units.iter().any(|&u| u < 32 || [b'<', b'>', b':', b'"', b'|', b'?', b'*'].contains(&(u as u8)) && u < 128)
        || units.last().is_some_and(|&u| u == b'.' as u16 || u == b' ' as u16)
    { return false; }
    let head = name.to_string_lossy().split('.').next().unwrap_or_default().trim_end_matches(' ').to_ascii_uppercase();
    if matches!(head.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$") { return false; }
    let port = head.strip_prefix("COM").or_else(|| head.strip_prefix("LPT"));
    !port.is_some_and(|n| matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"))
}
