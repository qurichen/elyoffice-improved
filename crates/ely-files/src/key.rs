use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};

#[cfg(test)]
mod tests;
mod scope;

use scope::Scope;

/// A key store backend, or a test double.
pub trait Backend: Send + Sync {
    /// The stored key bytes, or None when absent.
    fn get(&self) -> io::Result<Option<Vec<u8>>>;
    /// Stores the key bytes.
    fn set(&self, key: &[u8]) -> io::Result<()>;
}

/// The system key store through `keyring` (§18.2).
pub struct KeyringBackend;

impl Backend for KeyringBackend {
    fn get(&self) -> io::Result<Option<Vec<u8>>> {
        read_entry(entry()?)
    }

    fn set(&self, key: &[u8]) -> io::Result<()> {
        write_entry(entry()?, key)
    }
}

/// The credential entry for the recovery key.
fn entry() -> io::Result<keyring::Entry> {
    scoped_entry(scope::LEGACY_SERVICE, scope::LEGACY_USER)
}

struct ProfileBackend(String);

impl Backend for ProfileBackend {
    fn get(&self) -> io::Result<Option<Vec<u8>>> {
        read_entry(scoped_entry(scope::PROFILE_SERVICE, &self.0)?)
    }

    fn set(&self, key: &[u8]) -> io::Result<()> {
        write_entry(scoped_entry(scope::PROFILE_SERVICE, &self.0)?, key)
    }
}

fn scoped_entry(service: &str, user: &str) -> io::Result<keyring::Entry> {
    keyring::Entry::new(service, user)
        .map_err(|error| io::Error::other(format!("no credential entry: {error}")))
}

fn read_entry(entry: keyring::Entry) -> io::Result<Option<Vec<u8>>> {
    match entry.get_secret() {
        Ok(bytes) => Ok(Some(bytes)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("the key store keeps the recovery key: {error}"),
        )),
    }
}

fn write_entry(entry: keyring::Entry, key: &[u8]) -> io::Result<()> {
    entry.set_secret(key)
        .map_err(|error| io::Error::other(format!("the key store refused the key: {error}")))
}

fn load_scoped(
    scope: io::Result<Scope>,
    backend: impl FnOnce(&Scope) -> Box<dyn Backend>,
) -> io::Result<[u8; 32]> {
    let scope = scope?;
    load_with(backend(&scope).as_ref(), &scope.dir)
}

/// The recovery key, and whether it outlives this session (§18.2).
#[derive(Clone, Copy, Debug)]
pub struct RecoveryKey {
    pub key: [u8; 32],
    /// False when persistence failed: this run only.
    pub lasting: bool,
}

impl RecoveryKey {
    /// The stored key, else this session's; read once a run.
    pub fn load() -> Self {
        static LOADED: std::sync::OnceLock<RecoveryKey> = std::sync::OnceLock::new();
        *LOADED.get_or_init(|| {
            Self::from(load_scoped(Scope::current(), |scope| match scope.profile {
                true => Box::new(ProfileBackend(scope.user.clone())),
                false => Box::new(KeyringBackend),
            }))
        })
    }

    /// The loaded key, or this session's when loading failed.
    pub fn from(loaded: io::Result<[u8; 32]>) -> Self {
        match loaded {
            Ok(key) => Self { key, lasting: true },
            Err(error) => {
                log::warn!("recovery: persistent key unavailable, this session's key only: {error}");
                Self { key: *session_key(), lasting: false }
            }
        }
    }
}

/// One random key per run, kept only in memory.
fn session_key() -> &'static [u8; 32] {
    static KEY: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
    KEY.get_or_init(|| {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).expect("system randomness");
        key
    })
}

/// The OS account's home, deaf to environment overrides.
#[cfg(unix)]
fn account_home() -> io::Result<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let mut passwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buffer = vec![0u8; 1024];
    let mut result = std::ptr::null_mut();
    loop {
        let status = unsafe {
            libc::getpwuid_r(
                libc::getuid(),
                &mut passwd,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };
        match status {
            0 => break,
            libc::ERANGE => buffer.resize(buffer.len() * 2, 0),
            error => return Err(io::Error::from_raw_os_error(error)),
        }
    }
    if result.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no account home for this uid",
        ));
    }
    let dir = unsafe { std::ffi::CStr::from_ptr(passwd.pw_dir) };
    Ok(std::path::PathBuf::from(std::ffi::OsString::from_vec(
        dir.to_bytes().to_vec(),
    )))
}

/// The OS account's profile, deaf to environment overrides.
#[cfg(windows)]
fn account_home() -> io::Result<std::path::PathBuf> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_Profile, SHGetKnownFolderPath};
    let mut path = std::ptr::null_mut();
    let result =
        unsafe { SHGetKnownFolderPath(&FOLDERID_Profile, 0, std::ptr::null_mut(), &mut path) };
    if result != 0 || path.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no account profile directory",
        ));
    }
    let text = unsafe {
        let len = (0..).take_while(|i| *path.add(*i) != 0).count();
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(path, len));
        CoTaskMemFree(path.cast());
        text
    };
    Ok(std::path::PathBuf::from(text))
}

/// The lock's home: per user, deaf to data-dir env overrides.
fn shared_dir(home: &Path) -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support/ElyOffice")
    }
    #[cfg(target_os = "windows")]
    {
        home.join("AppData").join("Local").join("ElyOffice")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        home.join(".local").join("share").join("ElyOffice")
    }
    #[cfg(not(any(unix, windows)))]
    {
        home.join(".elyoffice")
    }
}

/// Reads or initializes the key; one winner across processes.
pub fn load_with(backend: &dyn Backend, dir: &Path) -> io::Result<[u8; 32]> {
    std::fs::create_dir_all(dir)?;
    let lock = dir.join(".recovery-key.lock");
    let deadline = Instant::now() + Duration::from_secs(1);
    let guard = loop {
        if let Some(guard) = ely_platform::lock::FileLock::try_take(&lock)? {
            break guard;
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "another process still initializes the recovery key",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let outcome = match backend.get()? {
        Some(bytes) => key_of(bytes),
        None => init(backend),
    };
    drop(guard);
    outcome
}

/// Generates and stores the key, then re-reads the persisted one.
fn init(backend: &dyn Backend) -> io::Result<[u8; 32]> {
    let mut key = [0u8; 32];
    getrandom::fill(&mut key).map_err(|_| io::Error::other("system randomness failed"))?;
    backend.set(&key)?;
    log::info!("recovery: a new recovery key in the system store");
    let Some(bytes) = backend.get()? else {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the key store did not keep the new key",
        ));
    };
    key_of(bytes)
}

/// The key's exact length, or a loud error.
fn key_of(bytes: Vec<u8>) -> io::Result<[u8; 32]> {
    bytes
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "recovery key length"))
}
