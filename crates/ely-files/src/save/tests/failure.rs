use std::{fs, io, os::windows::fs::OpenOptionsExt, path::Path};

use crate::{SaveError, SaveState, Session, save, save_as};
use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

fn prevent_replace(path: &Path) -> fs::File {
    fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(path)
        .expect("the owned fixture is held without delete sharing")
}

fn dirty(path: &Path) -> Session {
    let mut session = Session::load(path.to_owned()).expect("load fixture");
    session.edit();
    session
}

fn failed(result: Result<crate::Saved, SaveError>, session: &Session) {
    assert!(matches!(result, Err(SaveError::Io(ref error))
        if matches!(error.kind(), io::ErrorKind::PermissionDenied | io::ErrorKind::Other)),
        "the real filesystem must refuse replacement");
    assert_eq!(session.state(), SaveState::SaveFailed, "failure cannot report saved");
}

#[test]
fn an_in_use_target_keeps_original_and_draft_then_retries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("busy.docx");
    fs::write(&path, b"original document").unwrap();
    let mut session = dirty(&path);
    let revision = session.revision();
    let held = prevent_replace(&path);
    failed(save(&mut session, b"edited document"), &session);
    assert_eq!(fs::read(&path).unwrap(), b"original document");
    assert_eq!(session.revision(), revision, "the draft revision remains unsaved");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1, "no partial file remains");
    drop(held);
    save(&mut session, b"edited document").expect("retry after releasing owned lock");
    assert_eq!(fs::read(&path).unwrap(), b"edited document");
    assert_eq!(session.state(), SaveState::Saved);
}

#[test]
fn a_target_becoming_read_only_keeps_draft_until_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("readonly.xlsx");
    fs::write(&path, b"original workbook").unwrap();
    let mut session = dirty(&path);
    let permissions = fs::metadata(&path).unwrap().permissions();
    let mut locked = permissions.clone();
    locked.set_readonly(true);
    fs::set_permissions(&path, locked).unwrap();
    let result = save(&mut session, b"unsaved workbook");
    fs::set_permissions(&path, permissions).expect("restore fixture permissions");
    failed(result, &session);
    assert_eq!(fs::read(&path).unwrap(), b"original workbook");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    save(&mut session, b"unsaved workbook").expect("retry");
    assert_eq!(fs::read(&path).unwrap(), b"unsaved workbook");
    assert_eq!(session.state(), SaveState::Saved);
}

#[test]
fn failed_save_as_keeps_source_identity_and_target_until_retry() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pptx");
    let target = dir.path().join("target.pptx");
    fs::write(&source, b"original presentation").unwrap();
    fs::write(&target, b"other presentation").unwrap();
    let mut session = dirty(&source);
    let held = prevent_replace(&target);
    failed(save_as(&mut session, &target, b"draft presentation"), &session);
    assert_eq!(session.path(), Some(source.as_path()));
    assert_eq!(fs::read(&source).unwrap(), b"original presentation");
    assert_eq!(fs::read(&target).unwrap(), b"other presentation");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    drop(held);
    save_as(&mut session, &target, b"draft presentation").expect("retry");
    assert_eq!(session.path(), Some(target.as_path()));
    assert_eq!(fs::read(&source).unwrap(), b"original presentation");
    assert_eq!(fs::read(&target).unwrap(), b"draft presentation");
    assert_eq!(session.state(), SaveState::Saved);
}

#[test]
fn an_external_change_is_kept_when_the_draft_saves_as_a_copy() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("changed.docx");
    let copy = dir.path().join("my-copy.docx");
    fs::write(&source, b"original").unwrap();
    let mut session = dirty(&source);
    fs::write(&source, b"external version with a different length").unwrap();
    assert!(matches!(save(&mut session, b"my unsaved draft"), Err(SaveError::Conflict)));
    assert_eq!(session.state(), SaveState::ExternalModified);
    assert!(!copy.exists());
    save_as(&mut session, &copy, b"my unsaved draft").expect("keep draft in another file");
    assert_eq!(fs::read(&source).unwrap(), b"external version with a different length");
    assert_eq!(fs::read(&copy).unwrap(), b"my unsaved draft");
    assert_eq!(session.state(), SaveState::Saved);
}
