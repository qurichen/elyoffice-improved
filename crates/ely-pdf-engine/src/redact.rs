//! Removal and cleaning into new files (PDF-09, PDF-10).

use std::{io, path::Path};

use serde_json::{Value, json};

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// A region to remove, in page points.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub page: usize,
    pub rect: [f32; 4],
}

/// The written file's check: zeros, and one version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redacted {
    pub regions: usize,
    /// Characters still inside a region.
    pub text: usize,
    /// Pixels inside a region that are not black.
    pub light: usize,
    /// Drawings still inside a region.
    pub objects: usize,
    pub versions: usize,
}

/// What cleaning reaches, or what is left after.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reach {
    /// Info entries, plus one for an XMP stream.
    pub meta: usize,
    pub annots: usize,
    pub files: usize,
    /// Pages with hidden text or layers.
    pub hidden: usize,
    pub scripts: usize,
    pub hidden_annots: usize,
}

/// What to clean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cleaning {
    pub meta: bool,
    pub annots: bool,
    pub files: bool,
    /// Invisible text, scripts and hidden annotations.
    pub hidden: bool,
}

impl Cleaning {
    fn mask(self) -> u64 {
        u64::from(self.meta) | u64::from(self.annots) << 1 | u64::from(self.files) << 2 | u64::from(self.hidden) << 3
    }
}

fn count(value: &Value, key: &str) -> io::Result<usize> {
    let n = value[key].as_u64().ok_or_else(|| io::Error::other(format!("a bad {key} in {value}")))?;
    usize::try_from(n).map_err(io::Error::other)
}

fn reach(bytes: &[u8]) -> io::Result<Reach> {
    let value: Value = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    Ok(Reach {
        meta: count(&value, "meta")?,
        annots: count(&value, "annots")?,
        files: count(&value, "files")?,
        hidden: count(&value, "hidden")?,
        scripts: count(&value, "scripts")?,
        hidden_annots: count(&value, "hidden_annots")?,
    })
}

fn utf8(path: &Path) -> io::Result<&str> {
    path.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))
}

impl Worker {
    /// Writes `out` without `regions`; an unclean result is an error.
    pub fn redact(&mut self, doc: Doc, regions: &[Region], out: &Path) -> io::Result<Redacted> {
        let spec: Vec<Value> = regions.iter().map(|r| json!({ "page": r.page, "rect": r.rect })).collect();
        let ask = self.on(&doc)?.str(&Value::Array(spec).to_string()).str(utf8(out)?);
        let value: Value = serde_json::from_slice(&self.request(tag::REDACT, &ask.bytes())?).map_err(io::Error::other)?;
        Ok(Redacted {
            regions: count(&value, "regions")?,
            text: count(&value, "text")?,
            light: count(&value, "light")?,
            objects: count(&value, "objects")?,
            versions: count(&value, "versions")?,
        })
    }

    /// Counts what cleaning reaches; `scratch` holds a passing copy.
    pub fn clean_scan(&mut self, doc: Doc, scratch: &Path) -> io::Result<Reach> {
        reach(&self.request(tag::CLEAN_SCAN, &self.whole(&doc)?.str(utf8(scratch)?).bytes())?)
    }

    /// Writes `out` cleaned; what is left in it.
    pub fn clean(&mut self, doc: Doc, what: Cleaning, out: &Path) -> io::Result<Reach> {
        let ask = self.whole(&doc)?.u64(what.mask()).str(utf8(out)?);
        reach(&self.request(tag::CLEAN, &ask.bytes())?)
    }
}
