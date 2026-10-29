use std::{fs, io, time::SystemTime};

use ely_platform::lock::FileLock;

use super::{Recovery, stamp_of};

impl Recovery {
    pub(super) fn lock(&self) -> io::Result<FileLock> {
        fs::create_dir_all(&self.dir)?;
        FileLock::try_take(&self.dir.join(".points.lock"))?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::WouldBlock, "another task is changing recovery points")
        })
    }

    pub(super) fn next_stamp(&self, now: SystemTime) -> io::Result<u128> {
        let requested = stamp_of(now)?;
        let after = |stamp: u128| {
            stamp.checked_add(1).filter(|n| *n <= u64::MAX as u128)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "recovery timestamp overflow"))
        };
        let mut stamp = match self.stamps()?.first() {
            Some(newest) if *newest >= requested => after(*newest)?,
            _ => requested,
        };
        loop {
            if stamp > u64::MAX as u128 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "recovery timestamp overflow"));
            }
            let mut occupied = false;
            for ext in ["bin", "enc", "cursor", "history"] {
                match fs::symlink_metadata(self.dir.join("points").join(format!("{stamp}.{ext}"))) {
                    Ok(_) => occupied = true,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            if !occupied {
                if stamp != requested {
                    log::warn!("recovery: the clock or an occupied timestamp required advancing point order");
                }
                return Ok(stamp);
            }
            stamp = after(stamp)?;
        }
    }
}
