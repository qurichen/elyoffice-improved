use super::*;
use std::cell::Cell;

const CONTENT: &[u8] = b"private unsaved document text";

#[test]
fn plain_points_never_load_a_recovery_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let calls = Cell::new(0);
    let session_only = write(dir.path().to_path_buf(), CONTENT, false, 20, 1024, || {
        calls.set(calls.get() + 1);
        panic!("plain recovery cannot access credentials")
    }).expect("a plain point");
    assert_eq!(calls.get(), 0);
    assert!(!session_only);
    let recovery = Recovery::new(dir.path().to_path_buf());
    let points = recovery.points().expect("the points");
    assert_eq!(points.len(), 1);
    assert!(!points[0].sealed);
}

#[test]
fn encrypted_points_return_only_the_key_lifetime_and_stay_sealed() {
    for lasting in [false, true] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let folder = dir.path().join("recovery");
        let calls = Cell::new(0);
        let key = RecoveryKey { key: [42; 32], lasting };
        let session_only = write(folder.clone(), CONTENT, true, 20, 1024, || {
            calls.set(calls.get() + 1);
            key
        }).expect("a sealed point");
        assert_eq!(calls.get(), 1);
        assert_eq!(session_only, !lasting);
        let recovery = Recovery::sensitive(folder.clone(), key.key);
        let points = recovery.points().expect("the points");
        assert_eq!(points.len(), 1);
        assert!(points[0].sealed);
        assert_eq!(points[0].content.extension().and_then(|name| name.to_str()), Some("enc"));
        let bytes = std::fs::read(&points[0].content).expect("ciphertext");
        assert!(!bytes.windows(CONTENT.len()).any(|window| window == CONTENT));
        let files: Vec<_> = std::fs::read_dir(folder.join("points")).expect("the point folder")
            .map(|entry| entry.expect("a point").path()).collect();
        assert_eq!(files, [points[0].content.clone()]);
        let restored = dir.path().join("restored.docx");
        recovery.restore(&points[0], &restored).expect("same-session recovery");
        assert_eq!(std::fs::read(restored).expect("the restored content"), CONTENT);
        let wrong = Recovery::sensitive(folder, [43; 32]);
        assert!(wrong.restore(&points[0], &dir.path().join("wrong.docx")).is_err());
    }
}

#[test]
fn failed_points_preserve_the_error_instead_of_reporting_success() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let folder = dir.path().join("blocked");
    std::fs::write(&folder, b"sentinel").expect("a blocker");
    let error = write(folder.clone(), CONTENT, true, 20, 1024, || {
        RecoveryKey { key: [42; 32], lasting: false }
    }).expect_err("the point must fail");
    assert_ne!(error.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(std::fs::read(folder).expect("the blocker"), b"sentinel");
}

#[test]
fn point_limits_still_reject_oversized_encrypted_content() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let error = write(dir.path().join("recovery"), CONTENT, true, 20, 1, || {
        RecoveryKey { key: [42; 32], lasting: false }
    }).expect_err("over the cap");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}
