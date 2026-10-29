use super::{Keeping, Kept, Slot};

mod quit;
mod process;

#[test]
fn a_failing_point_says_so_once_until_the_reason_changes() {
    let mut slot = Slot { kept: kept("s", false), written: None, writing: false, failed: None };
    let full = std::io::Error::other("disk full");
    assert_eq!(slot.fail(1, &full).as_deref(), Some("disk full"));
    assert_eq!(slot.fail(2, &full), None, "the same reason stays quiet");
    let big = std::io::Error::new(std::io::ErrorKind::InvalidInput, "over the cap");
    assert!(slot.fail(3, &big).is_some(), "a new reason shows");
    assert_eq!(slot.written, Some(3), "too big waits for the next edit");
}

#[test]
fn session_only_points_warn_once_without_plain_points_consuming_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut keeping = Keeping::start_in(dir.path().to_path_buf()).expect("starts");
    assert!(!keeping.session_notice(false));
    assert!(keeping.session_notice(true));
    assert!(!keeping.session_notice(true));
    assert!(!keeping.session_notice(false));
}

fn kept(slot: &str, held: bool) -> Kept {
    Kept { slot: slot.into(), name: "a.docx".into(), format: "docx".into(), sealed: false, held }
}

#[cfg(unix)]
#[test]
fn a_failed_index_write_keeps_the_last_one() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut keeping = Keeping::start_in(dir.path().to_path_buf()).expect("starts");
    keeping.crashed.push(kept("1-0", false));
    keeping.save_index().expect("the first index");
    let recovery = dir.path().join("recovery");
    std::fs::set_permissions(&recovery, std::fs::Permissions::from_mode(0o555)).expect("locks");
    keeping.crashed.push(kept("1-1", false));
    let failed = keeping.save_index();
    std::fs::set_permissions(&recovery, std::fs::Permissions::from_mode(0o755)).expect("unlocks");
    assert!(failed.is_err(), "the folder refused the write");
    let read = super::read_index(dir.path()).expect("the old index still reads");
    assert_eq!(read.len(), 1);
}

#[test]
fn a_clean_end_keeps_only_what_never_reopened() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut keeping = Keeping::start_in(dir.path().to_path_buf()).expect("starts");
    for slot in ["1-0", "1-1"] {
        std::fs::create_dir_all(keeping.folder(slot).join("points")).expect("a slot");
    }
    keeping.crashed.push(kept("1-0", false));
    keeping.held.push(kept("1-1", false));
    keeping.save_index().expect("the index");
    keeping.end();
    let again = Keeping::start_in(dir.path().to_path_buf()).expect("starts again");
    let slots: Vec<&str> = again.crashed.iter().map(|kept| kept.slot.as_str()).collect();
    assert_eq!(slots, ["1-1"], "only the held slot comes back");
    assert!(again.folder("1-1").exists() && !again.folder("1-0").exists());
}
