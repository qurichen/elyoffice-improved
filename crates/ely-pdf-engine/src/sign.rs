//! Signatures: listed with their checks, and new ones (PDF-12).

use std::{io, path::Path};

use serde_json::Value;

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// Whether the signed bytes are as signed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Digest {
    Intact,
    Changed,
    /// No certificate, or a signature that will not read.
    #[default]
    Unreadable,
}

/// One signature field, signed or empty.
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    /// None for a field drawn on no page.
    pub page: Option<usize>,
    pub object: u32,
    pub rect: [f32; 4],
    pub name: String,
    pub signed: Option<Signed>,
}

/// What a signed field says and how it checks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Signed {
    pub digest: Digest,
    /// Updates were saved after it.
    pub later: bool,
    /// Those updates go past what it allows.
    pub broken: bool,
    pub signer: String,
    pub common: String,
    /// The PDF date the signer claims.
    pub date: String,
    pub reason: String,
    pub location: String,
    /// DER, the signer's first.
    pub certs: Vec<Vec<u8>>,
    /// Why it could not be read.
    pub error: Option<String>,
}

/// A new signature: a field, a certificate, words.
#[derive(Clone, Debug, PartialEq)]
pub struct Signing {
    pub page: usize,
    /// An empty field's object, or a new field at `rect`.
    pub object: Option<u32>,
    pub rect: [f32; 4],
    pub name: String,
    /// A PKCS #12 file and its password.
    pub pfx: String,
    pub pfx_password: String,
    pub reason: String,
    pub location: String,
}

fn bad(what: &str, value: &Value) -> io::Error {
    io::Error::other(format!("a bad {what} in {value}"))
}

fn text(value: &Value, key: &str) -> io::Result<String> {
    value[key].as_str().map(str::to_string).ok_or_else(|| bad(key, value))
}

fn hex(value: &Value) -> io::Result<Vec<u8>> {
    let digits = value.as_str().filter(|s| s.len() % 2 == 0).ok_or_else(|| bad("certificate", value))?;
    (0..digits.len()).step_by(2).map(|at| u8::from_str_radix(&digits[at..at + 2], 16).map_err(io::Error::other)).collect()
}

fn signed(value: &Value) -> io::Result<Signed> {
    let digest = match value["digest"].as_str() {
        Some("ok") => Digest::Intact,
        Some("changed") => Digest::Changed,
        Some("no_certificate" | "unreadable") => Digest::Unreadable,
        _ => return Err(bad("digest", value)),
    };
    if let Some(error) = value["error"].as_str() {
        return Ok(Signed { digest, error: Some(error.into()), ..Signed::default() });
    }
    let certs = value["certs"].as_array().ok_or_else(|| bad("certs", value))?.iter().map(hex).collect::<io::Result<_>>()?;
    Ok(Signed {
        digest,
        later: value["later"].as_i64().ok_or_else(|| bad("later", value))? != 0,
        broken: value["broken"].as_i64().ok_or_else(|| bad("broken", value))? != 0,
        signer: text(value, "signer")?,
        common: text(value, "common")?,
        date: text(value, "date")?,
        reason: text(value, "reason")?,
        location: text(value, "location")?,
        certs,
        error: None,
    })
}

fn signature(value: &Value) -> io::Result<Signature> {
    let rect = value["rect"].as_array().filter(|r| r.len() == 4).ok_or_else(|| bad("rect", value))?;
    let mut out = [0f32; 4];
    for (at, side) in rect.iter().enumerate() {
        out[at] = side.as_f64().ok_or_else(|| bad("rect", value))? as f32;
    }
    Ok(Signature {
        page: usize::try_from(value["page"].as_i64().ok_or_else(|| bad("page", value))?).ok(),
        object: value["object"].as_u64().and_then(|n| u32::try_from(n).ok()).ok_or_else(|| bad("object", value))?,
        rect: out,
        name: text(value, "name")?,
        signed: match value["signed"].as_bool().ok_or_else(|| bad("signed", value))? {
            true => Some(signed(value)?),
            false => None,
        },
    })
}

impl Worker {
    /// Every signature field in the file at `path`, checked.
    pub fn signatures(&mut self, doc: Doc, path: &Path) -> io::Result<Vec<Signature>> {
        let path = path.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        let reply = self.request(tag::SIGNATURES, &self.whole(&doc)?.str(path).bytes())?;
        let value: Value = serde_json::from_slice(&reply).map_err(io::Error::other)?;
        value.as_array().ok_or_else(|| bad("list", &value))?.iter().map(signature).collect()
    }

    /// Fields holding a signature, counted without checking.
    pub fn signed(&mut self, doc: Doc) -> io::Result<usize> {
        let reply = self.request(tag::SIGNED, &self.whole(&doc)?.bytes())?;
        Ok(ely_ipc::Unpack::new(&reply).u64()? as usize)
    }

    /// Writes `out` signed by an update; `doc` is untouched.
    pub fn sign(&mut self, doc: Doc, out: &Path, how: &Signing) -> io::Result<()> {
        let out = out.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        let object = how.object.map_or(-1, i64::from);
        let mut ask = self.on(&doc)?.str(out).u64(how.page as u64).i64(object);
        for side in how.rect {
            ask = ask.u64(u64::from(side.to_bits()));
        }
        let ask = ask.str(&how.name).str(&how.pfx).str(&how.pfx_password).str(&how.reason).str(&how.location);
        self.request(tag::SIGN, &ask.bytes()).map(|_| ())
    }
}
