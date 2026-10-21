use super::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}},
};

use crate::key::{Backend, RecoveryKey, load_scoped};

type Stored = Arc<Mutex<HashMap<String, Vec<u8>>>>;

struct Store {
    stored: Stored,
    user: String,
}

impl Backend for Store {
    fn get(&self) -> io::Result<Option<Vec<u8>>> {
        Ok(self.stored.lock().expect("the store").get(&self.user).cloned())
    }

    fn set(&self, bytes: &[u8]) -> io::Result<()> {
        self.stored.lock().expect("the store").insert(self.user.clone(), bytes.to_vec());
        Ok(())
    }
}

fn load(scope: io::Result<Scope>, stored: &Stored) -> io::Result<[u8; 32]> {
    load_scoped(scope, |scope| Box::new(Store {
        stored: Arc::clone(stored),
        user: scope.user.clone(),
    }))
}

#[test]
fn an_unset_profile_keeps_the_legacy_identity_without_loading_it() {
    let scope = Scope::legacy().expect("the account home");
    assert!(!scope.profile);
    assert_eq!(scope.user, "recovery-key");
    assert_eq!(LEGACY_SERVICE, "ElyOffice Recovery");
    assert_eq!(scope.dir, crate::key::shared_dir(&crate::key::account_home().expect("home")));
}

#[test]
fn profiles_keep_separate_persisted_keys_and_local_locks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let a = dir.path().join("profile a");
    let b = dir.path().join("profile b");
    let stored = Stored::default();
    stored.lock().expect("the store").insert(LEGACY_USER.into(), vec![77; 32]);
    let first = load(Scope::explicit(&a), &stored).expect("a stores");
    let other = load(Scope::explicit(&b), &stored).expect("b stores");
    assert_ne!(first, other);
    assert_eq!(load(Scope::explicit(&a), &stored).expect("a reloads"), first);
    assert_eq!(stored.lock().expect("the store").get(LEGACY_USER), Some(&vec![77; 32]));
    for root in [&a, &b] {
        let names: Vec<_> = std::fs::read_dir(root).expect("profile").map(|entry| {
            entry.expect("an entry").file_name()
        }).collect();
        assert_eq!(names, [std::ffi::OsString::from(".recovery-key.lock")]);
    }
}

#[test]
fn invalid_profiles_construct_no_backend_and_never_use_the_legacy_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("not a directory");
    std::fs::write(&file, "sentinel").expect("a file");
    let root = if cfg!(windows) { PathBuf::from("C:/") } else { PathBuf::from("/") };
    let paths = [PathBuf::new(), PathBuf::from("relative"), root,
        dir.path().join("a/../other"), file.clone()];
    let constructed = AtomicUsize::new(0);
    for path in paths {
        let result = load_scoped(Scope::explicit(&path), |_| {
            constructed.fetch_add(1, Ordering::SeqCst);
            panic!("an invalid profile cannot select credentials")
        });
        assert!(result.is_err(), "{path:?}");
        assert!(!RecoveryKey::from(result).lasting);
    }
    assert_eq!(constructed.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read_to_string(file).expect("the sentinel"), "sentinel");
}

#[test]
fn a_recreated_profile_path_retains_its_credential_identity() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("restored profile");
    let first = Scope::explicit(&root).expect("the profile").user;
    std::fs::remove_dir(&root).expect("the empty profile");
    let recreated = Scope::explicit(&root).expect("the recreated profile");
    assert_eq!(recreated.user, first);
    assert_ne!(recreated.user, LEGACY_USER);
    assert_eq!(recreated.user.len(), 72);
    assert!(format!("{}.{}", recreated.user, PROFILE_SERVICE).len() < 128);
}

#[test]
fn an_explicit_profile_uses_its_canonical_directory() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("隔离 profile & 100%");
    let scope = Scope::explicit(&root).expect("a profile");
    assert!(scope.profile);
    assert_eq!(scope.dir, std::fs::canonicalize(&root).expect("canonical profile"));
    assert_eq!(scope.user, Scope::explicit(&root.join(".")).expect("alias").user);
}

fn child_identity(path: &Path) -> String {
    let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args(["--exact", "key::scope::tests::profile_identity_child", "--include-ignored", "--nocapture"])
        .env("ELY_DATA_DIR", path)
        .output().expect("a child");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).lines()
        .find_map(|line| line.strip_prefix("SCOPE ")).expect("the identity").to_owned()
}

#[test]
fn processes_resolve_the_same_profile_identity() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let a = dir.path().join("process a");
    let b = dir.path().join("process b");
    let identity = Scope::explicit(&a).expect("a profile").user;
    assert_eq!(child_identity(&a), identity);
    assert_eq!(child_identity(&a), identity);
    assert_ne!(child_identity(&b), identity);
}

#[test]
fn invalid_profile_environments_construct_no_backend() {
    let root = if cfg!(windows) { "C:/" } else { "/" };
    for path in ["", "relative", root] {
        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args(["--exact", "key::scope::tests::invalid_profile_child", "--include-ignored", "--nocapture"])
            .env("ELY_DATA_DIR", path)
            .output().expect("a child");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("NO_BACKEND"));
    }
}

#[test]
#[ignore]
fn profile_identity_child() {
    let scope = Scope::current().expect("a valid explicit profile");
    assert!(scope.profile);
    println!("SCOPE {}", scope.user);
}

#[test]
#[ignore]
fn invalid_profile_child() {
    let result = load_scoped(Scope::current(), |_| {
        panic!("invalid profiles cannot select credentials")
    });
    assert!(result.is_err());
    assert!(!RecoveryKey::from(result).lasting);
    println!("NO_BACKEND");
}

#[cfg(unix)]
#[test]
fn symlink_aliases_share_identity_without_folding_distinct_case() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let a = dir.path().join("Case");
    let b = dir.path().join("case");
    let alias = dir.path().join("alias");
    let first = Scope::explicit(&a).expect("a profile");
    std::os::unix::fs::symlink(&a, &alias).expect("a directory link");
    assert_eq!(Scope::explicit(&alias).expect("the alias").user, first.user);
    let other = Scope::explicit(&b).expect("another profile");
    assert_eq!(other.user == first.user, other.dir == first.dir);
}

#[cfg(windows)]
#[test]
fn windows_case_and_verbatim_aliases_share_identity() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("MiXeD profile");
    let first = Scope::explicit(&root).expect("a profile");
    let upper = PathBuf::from(root.to_str().expect("UTF-8 temp path").to_uppercase());
    assert_eq!(Scope::explicit(&upper).expect("a case alias").user, first.user);
    assert_eq!(Scope::explicit(&first.dir).expect("a verbatim alias").user, first.user);
}

#[cfg(windows)]
#[test]
fn windows_junction_and_short_names_share_identity() {
    use std::os::windows::{ffi::{OsStrExt, OsStringExt}, process::CommandExt};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("Long profile directory");
    let alias = dir.path().join("junction");
    let first = Scope::explicit(&root).expect("a profile");
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "New-Item -ItemType Junction -Path $env:ELY_KEY_ALIAS -Target $env:ELY_KEY_TARGET -ErrorAction Stop | Out-Null"])
        .env("ELY_KEY_ALIAS", &alias).env("ELY_KEY_TARGET", &root)
        .creation_flags(0x08000000).output().expect("PowerShell");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(Scope::explicit(&alias).expect("the junction").user, first.user);
    let wide: Vec<_> = root.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut buffer = vec![0u16; 32768];
    let length = unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
    assert!(length > 0 && (length as usize) < buffer.len());
    let short = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize]));
    if short == root {
        eprintln!("The test volume has no distinct 8.3 alias");
    }
    assert_eq!(Scope::explicit(&short).expect("the short alias").user, first.user);
}
