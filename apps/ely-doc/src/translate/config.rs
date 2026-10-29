use std::{io, path::Path};

use ely_platform::settings::Store;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ureq::http::Uri;

use super::Failure;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct Connection {
    pub enabled: bool,
    pub endpoint: String,
    pub model: String,
    pub use_key: bool,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://127.0.0.1:11434/v1/chat/completions".into(),
            model: String::new(),
            use_key: false,
        }
    }
}

impl Connection {
    pub fn validate(&self) -> Result<(), Failure> {
        endpoint(&self.endpoint)?;
        if self.model.len() > 256 || self.model.chars().any(char::is_control)
            || (self.enabled && self.model.trim().is_empty())
        {
            return Err(Failure::Configuration);
        }
        Ok(())
    }

    pub fn load(store: &Store) -> Result<Self, Failure> {
        let fields = store.read().map_err(|_| Failure::Settings)?;
        let Some(value) = fields.get("translation") else { return Ok(Self::default()) };
        let result: Self = serde_json::from_value(value.clone()).map_err(|_| Failure::Settings)?;
        result.validate().map_err(|_| Failure::Settings)?;
        Ok(result)
    }

    pub fn save(&self, store: &Store) -> Result<(), Failure> {
        self.validate()?;
        let value = serde_json::to_value(self).map_err(|_| Failure::Settings)?;
        store.set("translation", value).map_err(|_| Failure::Settings)
    }
}

pub(super) fn endpoint(text: &str) -> Result<Uri, Failure> {
    if text.len() > 2048 || text.contains(['?', '#']) || text.chars().any(char::is_whitespace) {
        return Err(Failure::Configuration);
    }
    let uri: Uri = text.parse().map_err(|_| Failure::Configuration)?;
    let authority = uri.authority().ok_or(Failure::Configuration)?;
    let local = authority.host().trim_matches(['[', ']'])
        .parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if authority.as_str().contains('@') || uri.path() == "/"
        || !(uri.scheme_str() == Some("https") || (uri.scheme_str() == Some("http") && local))
    {
        return Err(Failure::Configuration);
    }
    Ok(uri)
}

pub(super) fn credential_id(dir: &Path, url: &str) -> Result<String, Failure> {
    endpoint(url)?;
    std::fs::create_dir_all(dir).map_err(|_| Failure::Settings)?;
    let dir = std::fs::canonicalize(dir).map_err(|_| Failure::Settings)?;
    let mut hash = Sha256::new();
    hash.update(b"ElyOffice Translation\0");
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        for unit in dir.as_os_str().encode_wide() { hash.update(unit.to_le_bytes()); }
    }
    #[cfg(not(windows))]
    hash.update(dir.as_os_str().as_encoded_bytes());
    hash.update(b"\0");
    hash.update(url.as_bytes());
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn key(dir: &Path, url: &str) -> Result<keyring::Entry, Failure> {
    keyring::Entry::new("ElyOffice Translation", &credential_id(dir, url)?)
        .map_err(|_| Failure::Credential)
}

pub(super) fn read_key(dir: &Path, url: &str) -> Result<String, Failure> {
    key(dir, url)?.get_password().map_err(|_| Failure::Credential)
}

pub(super) fn write_key(dir: &Path, url: &str, secret: &str) -> Result<(), Failure> {
    if secret.is_empty() || secret.len() > 8192 || secret.chars().any(char::is_control) {
        return Err(Failure::Credential);
    }
    key(dir, url)?.set_password(secret).map_err(|_| Failure::Credential)
}

pub(super) fn forget_key(dir: &Path, url: &str) -> Result<(), Failure> {
    match key(dir, url)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(Failure::Credential),
    }
}

pub(super) fn store() -> io::Result<Store> {
    let dir = ely_platform::data_dir(ely_platform::AppId::Doc).map_err(io::Error::other)?;
    Ok(Store::at(dir.join("translation.json")))
}
