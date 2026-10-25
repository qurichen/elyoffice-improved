use super::*;

#[test]
fn a_missing_point_leaves_no_failed_copy_and_allows_retry() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"retained body", None, at(100))
        .expect("the point");
    let point = recovery.points().expect("points").remove(0);
    let bytes = std::fs::read(&point.content).expect("point bytes");
    std::fs::remove_file(&point.content).expect("simulate a missing point");
    for copy in [false, true] {
        let dest = dir.path().join(format!("retry-{copy}.txt"));
        let failed = if copy {
            restore_copy(&point, &dest)
        } else {
            recovery.restore(&point, &dest)
        };
        assert_eq!(
            failed.expect_err("the point is missing").kind(),
            io::ErrorKind::NotFound
        );
        assert!(
            !dest.exists(),
            "a failed restore must not reserve its destination"
        );
        std::fs::write(&point.content, &bytes).expect("the point returns");
        if copy {
            restore_copy(&point, &dest)
        } else {
            recovery.restore(&point, &dest)
        }
        .expect("retry");
        assert_eq!(std::fs::read(&dest).expect("the restored body"), bytes);
        std::fs::remove_file(&point.content).expect("simulate the missing point again");
    }
}

#[test]
fn a_wrong_key_leaves_no_empty_target_and_the_right_key_can_retry() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::sensitive(dir.path().to_path_buf(), key());
    recovery
        .write_once(b"synthetic secret", Some("2:3"), at(100))
        .expect("the point");
    let point = recovery.points().expect("points").remove(0);
    let dest = dir.path().join("retry.txt");
    let wrong = Recovery::sensitive(dir.path().to_path_buf(), [8; 32]);
    assert_eq!(
        wrong.restore(&point, &dest).expect_err("wrong key").kind(),
        io::ErrorKind::InvalidData
    );
    assert!(
        !dest.exists(),
        "authentication failure must not leave an empty target"
    );
    recovery
        .restore(&point, &dest)
        .expect("the right key retries");
    assert_eq!(
        std::fs::read(&dest).expect("restored bytes"),
        b"synthetic secret"
    );
}

#[test]
fn authenticated_malformed_bundles_fail_without_panicking_or_creating_files() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let recovery = Recovery::sensitive(dir.path().to_path_buf(), key());
    let malformed = [
        vec![],
        vec![0],
        vec![1, 4, 0, 0, 0, b'x'],
        vec![0, 1, 0, 0, 0],
        vec![2, 0, 0, 0, 0],
    ];
    for (n, body) in malformed.into_iter().enumerate() {
        let (sealed, nonce) = seal(&key(), &body).expect("test encryption");
        let mut bytes = nonce.to_vec();
        bytes.extend(sealed);
        let source = dir.path().join(format!("{n}.enc"));
        std::fs::write(&source, bytes).expect("synthetic encrypted point");
        let point = RecoveryPoint {
            at: at(100),
            content: source,
            cursor: None,
            history: false,
            sealed: true,
        };
        let dest = dir.path().join(format!("invalid-{n}.txt"));
        assert_eq!(
            recovery
                .restore(&point, &dest)
                .expect_err("invalid bundle")
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert!(!dest.exists(), "invalid metadata must not publish content");
    }
}

#[test]
fn newest_authenticates_only_the_latest_without_hiding_list_errors() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut old = Recovery::sensitive(dir.path().to_path_buf(), key());
    old.write_once(b"old", Some("1:1"), at(100))
        .expect("older point");
    let mut current = Recovery::sensitive(dir.path().to_path_buf(), [9; 32]);
    current
        .write_once(b"latest", Some("2:2"), at(101))
        .expect("current point");
    assert!(
        current.points().is_err(),
        "listing still reports unreadable older data"
    );
    let point = current
        .newest()
        .expect("the newest is valid")
        .expect("a point");
    assert_eq!(point.at, at(101));
    assert_eq!(point.cursor.as_deref(), Some("2:2"));
    let dest = dir.path().join("restored.txt");
    current.restore(&point, &dest).expect("the newest restores");
    assert_eq!(std::fs::read(dest).expect("restored bytes"), b"latest");
}

#[test]
fn unreadable_latest_metadata_is_not_silently_replaced_by_an_older_point() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    assert!(recovery.newest().expect("an empty folder").is_none());
    recovery
        .write_once(b"older", None, at(100))
        .expect("older point");
    let latest = recovery
        .write_once(b"latest", Some("2:2"), at(101))
        .expect("newest point")
        .expect("a write");
    std::fs::write(latest.with_extension("cursor"), [0xff]).expect("bad latest cursor");
    assert_eq!(
        recovery.newest().expect_err("bad newest metadata").kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(recovery.stamps().expect("both points stay").len(), 2);
}
