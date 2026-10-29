use super::*;

fn recovery(dir: &Path, sealed: bool, keep: usize) -> Recovery {
    let points = if sealed {
        Recovery::sensitive(dir.to_path_buf(), key())
    } else {
        Recovery::new(dir.to_path_buf())
    };
    points.with_limits(keep, 4096)
}

fn restored(points: &Recovery, point: &RecoveryPoint, dest: &Path) -> Vec<u8> {
    points.restore(point, dest).expect("a separate restored copy");
    std::fs::read(dest).expect("restored bytes")
}

fn equal_clock(sealed: bool) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let folder = dir.path().join("slot");
    let mut points = recovery(&folder, sealed, 20);
    let first = points.write_once(b"first", Some("1:2"), at(100))
        .expect("first point").expect("a write");
    let original = std::fs::read(&first).expect("first disk bytes");
    let second = points.write_once(b"second", Some("3:4"), at(100))
        .expect("second point").expect("a write");
    assert_ne!(first, second, "an equal clock never replaces a complete point");
    assert_eq!(std::fs::read(first).expect("first stays"), original);
    let listed = points.points().expect("both points");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].cursor.as_deref(), Some("3:4"));
    assert_eq!(listed[1].cursor.as_deref(), Some("1:2"));
    assert_eq!(restored(&points, &listed[0], &dir.path().join("new")), b"second");
    assert_eq!(restored(&points, &listed[1], &dir.path().join("old")), b"first");
    assert_eq!(points.last_ok(), Some(at(100)));
}

#[test]
fn equal_plain_clocks_keep_both_contents_and_their_own_cursors() {
    equal_clock(false);
}

#[test]
fn equal_sealed_clocks_keep_both_complete_bundles() {
    equal_clock(true);
}

#[test]
fn a_clock_rollback_never_prunes_the_point_just_reported_saved() {
    for sealed in [false, true] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let folder = dir.path().join("slot");
        let mut points = recovery(&folder, sealed, 1);
        points.write_once(b"old", None, at(200)).expect("first point");
        let saved = points.write_once(b"current", None, at(100))
            .expect("rollback write").expect("a write");
        assert!(saved.is_file(), "success must name a retained point");
        let listed = points.points().expect("retained points");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].content, saved);
        assert_eq!(restored(&points, &listed[0], &dir.path().join("copy")), b"current");
        assert_eq!(points.last_ok(), Some(at(100)));
    }
}

#[test]
fn reopening_after_a_clock_rollback_still_restores_the_latest_edit() {
    for sealed in [false, true] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let folder = dir.path().join("slot");
        recovery(&folder, sealed, 20).write_once(b"old", None, at(200))
            .expect("previous run");
        let mut points = recovery(&folder, sealed, 20);
        points.write_once(b"latest", None, at(100)).expect("reopened write");
        let newest = points.newest().expect("newest reads").expect("a point");
        assert_eq!(restored(&points, &newest, &dir.path().join("copy")), b"latest");
        let listed = points.points().expect("both retained");
        assert_eq!(listed.len(), 2);
        assert_eq!(restored(&points, &listed[1], &dir.path().join("older")), b"old");
    }
}

#[test]
fn orphan_metadata_never_attaches_to_a_new_point() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let folder = dir.path().join("slot");
    let files = folder.join("points");
    std::fs::create_dir_all(&files).expect("points folder");
    let stamp = stamp_of(at(100)).expect("timestamp");
    let cursor = files.join(format!("{stamp}.cursor"));
    let history = files.join(format!("{stamp}.history"));
    std::fs::write(&cursor, "old cursor").expect("orphan cursor");
    std::fs::write(&history, b"old history").expect("orphan history");
    let mut points = recovery(&folder, false, 20);
    points.write_once(b"new content", None, at(100)).expect("new point");
    let listed = points.points().expect("points");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].cursor, None, "unrelated cursor is not recovered");
    assert!(!listed[0].history, "unrelated history is not recovered");
    assert_eq!(restored(&points, &listed[0], &dir.path().join("copy")), b"new content");
    assert_eq!(std::fs::read_to_string(cursor).expect("orphan stays"), "old cursor");
    assert_eq!(std::fs::read(history).expect("orphan stays"), b"old history");
}

fn while_locked(action: impl FnOnce(&mut Recovery) -> io::Result<()>) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let folder = dir.path().join("slot");
    let mut points = recovery(&folder, false, 20);
    let first = points.write_once(b"kept", None, at(100))
        .expect("first point").expect("a write");
    let lock = ely_platform::lock::FileLock::take(&folder.join(".points.lock"))
        .expect("another holder");
    let error = action(&mut points).expect_err("mutation refuses another holder");
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(std::fs::read(first).expect("original point stays"), b"kept");
    assert_eq!(points.last_ok(), Some(at(100)));
    assert!(!points.discarded().expect("not discarded"));
    drop(lock);
    points.write_once(b"retry", None, at(101)).expect("retry after unlock");
    assert_eq!(points.points().expect("both points").len(), 2);
}

#[test]
fn a_second_writer_refuses_the_slot_until_unlocked() {
    while_locked(|points| points.write_once(b"busy", None, at(101)).map(drop));
}

#[test]
fn maintenance_never_prunes_while_a_writer_owns_the_slot() {
    while_locked(|points| points.maintain(at(100 + 8 * 24 * 3600)));
}

#[test]
fn discard_never_removes_points_while_a_writer_owns_the_slot() {
    while_locked(Recovery::discard);
}
