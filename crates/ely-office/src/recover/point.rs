use std::{io, path::PathBuf, time::SystemTime};

use ely_files::{Recovery, RecoveryKey};

#[cfg(test)]
mod tests;

pub(super) fn write(
    folder: PathBuf,
    bytes: &[u8],
    sealed: bool,
    keep: usize,
    cap: u64,
    load: impl FnOnce() -> RecoveryKey,
) -> io::Result<bool> {
    let (recovery, session_only) = if sealed {
        let key = load();
        (Recovery::sensitive(folder, key.key), !key.lasting)
    } else {
        (Recovery::new(folder), false)
    };
    recovery.with_limits(keep, cap).write_once(bytes, None, SystemTime::now())
        .map(|_| session_only)
}
