use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub(in crate::recover) struct Writer(Arc<Mutex<bool>>);

pub(in crate::recover) struct Pending {
    writer: Writer,
    path: PathBuf,
    bytes: Vec<u8>,
}

impl Writer {
    pub(in crate::recover) fn prepare(&self, path: PathBuf, bytes: Vec<u8>) -> Pending {
        Pending { writer: self.clone(), path, bytes }
    }

    pub(in crate::recover) fn finish(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut closed = self.0.lock().map_err(|_| io::Error::other("recovery index lock poisoned"))?;
        // 封口与提交同锁。
        *closed = true;
        publish(path, bytes)?;
        log::info!("recovery: final index committed");
        Ok(())
    }
}

impl Pending {
    pub(in crate::recover) fn publish(self) -> io::Result<bool> {
        let closed = self.writer.0.lock().map_err(|_| io::Error::other("recovery index lock poisoned"))?;
        if *closed {
            log::info!("recovery: an older index write was superseded at quit");
            return Ok(false);
        }
        publish(&self.path, &self.bytes).map(|()| true)
    }
}

fn publish(path: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::create_dir_all(path.parent().expect("the index has a folder"))?;
    ely_files::save_atomic(path, bytes, 0).map(|_| ())
}
