//! Permissions, owner access and protected copies (PDF-11).

use std::{io, path::Path};

use ely_ipc::Unpack;

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// A protected copy's passwords and what it allows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lock {
    /// Empty opens without asking.
    pub user: String,
    pub owner: String,
    /// `proto::may` bits.
    pub allow: u32,
}

impl Worker {
    /// `proto::may` bits for `doc` now.
    pub fn permissions(&mut self, doc: Doc) -> io::Result<u32> {
        let reply = self.request(tag::PERMISSIONS, &self.whole(&doc)?.bytes())?;
        Ok(Unpack::new(&reply).u64()? as u32)
    }

    /// Tries an owner password; false keeps the old access.
    pub fn owner(&mut self, doc: Doc, password: &str) -> io::Result<bool> {
        let reply = self.request(tag::OWNER, &self.whole(&doc)?.str(password).bytes())?;
        Ok(Unpack::new(&reply).u64()? != 0)
    }

    /// Writes `out` with `lock`, or with no protection.
    pub fn protect(&mut self, doc: Doc, out: &Path, lock: Option<&Lock>) -> io::Result<()> {
        let out = out.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        let (user, owner, allow) = lock.map_or(("", "", 0), |lock| (lock.user.as_str(), lock.owner.as_str(), lock.allow));
        let ask = self.whole(&doc)?.str(out).u64(u64::from(lock.is_some())).str(user).str(owner).u64(u64::from(allow));
        self.request(tag::PROTECT, &ask.bytes()).map(|_| ())
    }
}
