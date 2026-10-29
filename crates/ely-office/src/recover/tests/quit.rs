use std::{fs, time::SystemTime};

use ely_files::{Crash, Marker, Recovery};

use super::{Keeping, kept};

fn point(keeping: &Keeping, slot: &str) -> std::path::PathBuf {
    Recovery::new(keeping.folder(slot))
        .write_once(b"complete recovery draft", None, SystemTime::now())
        .expect("a point")
        .expect("written")
}

#[test]
fn a_late_index_cannot_hide_a_held_copy_after_clean_quit() {
    let dir = tempfile::tempdir().expect("temporary data");
    let mut keeping = Keeping::start_in(dir.path().to_owned()).expect("start");
    keeping.crashed.push(kept("1-0", false));
    let point = point(&keeping, "1-0");
    keeping.save_index().expect("old index");
    let pending = keeping.index().expect("queued old snapshot");
    keeping.held = std::mem::take(&mut keeping.crashed);
    keeping.finish(false).expect("final index and clean marker");
    let published = std::thread::spawn(move || pending.publish()).join().unwrap().unwrap();
    let next = Keeping::start_in(dir.path().to_owned()).expect("restart");
    assert_eq!(next.crashed.len(), 1, "the held record must survive a delayed old write");
    assert!(point.exists(), "clean startup must not prune the complete recovery point");
    assert!(!published, "a queued write is explicitly superseded");
}

#[test]
fn a_late_empty_index_cannot_remove_new_held_points() {
    let dir = tempfile::tempdir().expect("temporary data");
    let mut keeping = Keeping::start_in(dir.path().to_owned()).expect("start");
    let pending = keeping.index().expect("queued empty snapshot");
    let point = point(&keeping, "1-1");
    keeping.held.push(kept("1-1", false));
    keeping.finish(false).expect("final index");
    pending.publish().expect("delayed old write completes");
    let next = Keeping::start_in(dir.path().to_owned()).expect("restart");
    assert_eq!(next.crashed.len(), 1, "the final record cannot be replaced by empty");
    assert_eq!(fs::read(point).expect("complete point remains"), b"complete recovery draft");
}

#[test]
fn a_background_write_finishing_before_quit_still_keeps_final_state() {
    let dir = tempfile::tempdir().expect("temporary data");
    let mut keeping = Keeping::start_in(dir.path().to_owned()).expect("start");
    let pending = keeping.index().expect("old snapshot");
    assert!(pending.publish().expect("background first"));
    let point = point(&keeping, "1-0");
    keeping.held.push(kept("1-0", false));
    keeping.finish(false).expect("final write last");
    let next = Keeping::start_in(dir.path().to_owned()).expect("restart");
    assert_eq!(next.crashed.len(), 1);
    assert!(point.exists());
}

#[cfg(windows)]
#[test]
fn a_failed_quit_index_keeps_the_crash_marker_and_previous_index() {
    let dir = tempfile::tempdir().expect("temporary data");
    let mut keeping = Keeping::start_in(dir.path().to_owned()).expect("start");
    keeping.crashed.push(kept("1-0", false));
    let point = point(&keeping, "1-0");
    keeping.save_index().expect("old index");
    let path = dir.path().join("recovery/index.json");
    let old = fs::read(&path).unwrap();
    let permissions = fs::metadata(&path).unwrap().permissions();
    let mut locked = permissions.clone();
    locked.set_readonly(true);
    fs::set_permissions(&path, locked).unwrap();
    let failed = keeping.finish(false);
    fs::set_permissions(&path, permissions).expect("restore test permissions");
    assert_eq!(failed.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(fs::read(path).unwrap(), old, "a failed final write leaves the previous index");
    assert_eq!(Crash::check(dir.path(), |_| false).unwrap(), Marker::Crashed,
        "index failure cannot become a clean exit that prunes points");
    assert!(point.exists());
}
