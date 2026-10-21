#![cfg(windows)]

#[path = "windows_acl/support.rs"]
mod support;

use ely_files::{save_atomic, save_with_backup};
use std::{fs, os::windows::fs::OpenOptionsExt, path::Path};
use support::{dacl, set_dacl, user_sid};

fn private_file(dir: &Path) -> std::path::PathBuf {
    let user = user_sid();
    set_dacl(dir, &format!("D:P(A;OICI;FA;;;{user})(A;OICI;FR;;;BU)"));
    let target = dir.join("private.txt");
    fs::write(&target, b"original secret").expect("create the fixture");
    set_dacl(&target, &format!("D:P(A;;FA;;;{user})"));
    target
}

#[test]
fn replacement_preserves_a_protected_private_dacl() {
    let dir = tempfile::tempdir().expect("create the test directory");
    let target = private_file(dir.path());
    let before = dacl(&target);
    assert!(before.starts_with("D:P"), "protected fixture: {before}");
    assert!(!before.contains(";;;BU)"), "private fixture: {before}");
    save_atomic(&target, b"updated secret", 1).expect("save the private file");
    assert_eq!(fs::read(&target).unwrap(), b"updated secret");
    assert_eq!(dacl(&target), before, "replacement must not widen access");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn backup_and_replacement_preserve_private_dacls() {
    let dir = tempfile::tempdir().expect("create the test directory");
    let target = private_file(dir.path());
    let before = dacl(&target);
    let (_, backup) = save_with_backup(&target, b"new secret", 2).expect("save with backup");
    let backup = backup.expect("previous bytes have a backup");
    assert_eq!(fs::read(&backup).unwrap(), b"original secret");
    assert_eq!(fs::read(&target).unwrap(), b"new secret");
    assert_eq!(dacl(&backup), before, "backup must not widen access");
    assert_eq!(dacl(&target), before, "replacement must not widen access");
}

#[test]
fn replacement_preserves_explicit_and_inherited_access() {
    let dir = tempfile::tempdir().expect("create the test directory");
    let user = user_sid();
    set_dacl(
        dir.path(),
        &format!("D:P(A;OICI;FA;;;{user})(A;OICI;FR;;;BU)"),
    );
    let target = dir.path().join("shared.txt");
    fs::write(&target, b"old shared bytes").unwrap();
    set_dacl(&target, &format!("D:(A;;FR;;;SY)"));
    let before = dacl(&target);
    assert!(before.contains(";;;SY)"), "explicit access: {before}");
    assert!(before.contains(";ID;"), "inherited access: {before}");
    save_atomic(&target, b"shared bytes", 3).expect("save shared file");
    assert_eq!(dacl(&target), before);
    assert_eq!(fs::read(&target).unwrap(), b"shared bytes");
}

#[test]
fn new_files_inherit_their_destination_policy() {
    let dir = tempfile::tempdir().expect("create the test directory");
    let user = user_sid();
    set_dacl(
        dir.path(),
        &format!("D:P(A;OICI;FA;;;{user})(A;OICI;FR;;;BU)"),
    );
    let reference = dir.path().join("reference.txt");
    fs::write(&reference, b"ordinary creation").unwrap();
    let target = dir.path().join("new.txt");
    save_atomic(&target, b"new file", 4).expect("save new file");
    assert_eq!(dacl(&target), dacl(&reference));
    assert_eq!(fs::read(&target).unwrap(), b"new file");
}

#[test]
fn a_sharing_violation_preserves_original_bytes_and_dacl() {
    let dir = tempfile::tempdir().expect("create the test directory");
    let target = private_file(dir.path());
    let before = dacl(&target);
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&target)
        .unwrap();
    let error = save_atomic(&target, b"must not replace", 5).expect_err("deny delete sharing");
    assert!(error.to_string().contains("private.txt"));
    assert_eq!(fs::read(&target).unwrap(), b"original secret");
    assert_eq!(dacl(&target), before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    drop(held);
}

#[test]
fn a_readonly_target_keeps_its_bytes_and_dacl() {
    let dir = tempfile::tempdir().expect("create the test directory");
    let target = private_file(dir.path());
    let before = dacl(&target);
    let mut permissions = fs::metadata(&target).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&target, permissions.clone()).unwrap();
    let failed = save_atomic(&target, b"must not replace", 6);
    let bytes = fs::read(&target).unwrap();
    let after = dacl(&target);
    permissions.set_readonly(false);
    fs::set_permissions(&target, permissions).unwrap();
    assert!(failed.is_err());
    assert_eq!(bytes, b"original secret");
    assert_eq!(after, before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn null_dacls_are_not_confused_with_empty_dacls() {
    for policy in ["D:NO_ACCESS_CONTROL", "D:PNO_ACCESS_CONTROL"] {
        let dir = tempfile::tempdir().expect("create the test directory");
        let target = dir.path().join("null-dacl.txt");
        fs::write(&target, b"original").unwrap();
        set_dacl(&target, policy);
        let before = dacl(&target);
        assert!(
            before.contains("NO_ACCESS_CONTROL"),
            "null fixture: {before}"
        );
        let (_, backup) = save_with_backup(&target, b"updated", 7).expect("save null DACL");
        assert_eq!(dacl(&target), before);
        assert_eq!(dacl(&backup.unwrap()), before);
        assert_eq!(fs::read(&target).unwrap(), b"updated");
    }
}
