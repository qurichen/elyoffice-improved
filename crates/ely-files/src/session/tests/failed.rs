use super::*;

#[test]
fn a_failure_before_writing_shows_until_a_save_lands() {
    let mut session = Session::new_file();
    session.edit();
    session.save_failed();
    assert_eq!(session.state(), SaveState::SaveFailed);
    let ticket = session.begin_save().expect("a retry may start");
    assert_eq!(session.state(), SaveState::Saving);
    let _ = ticket;
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("ro.txt");
    std::fs::write(&file, "ORIGINAL").expect("writes");
    let mut read_only = Session::read_only(file.clone()).expect("a read-only view");
    read_only.save_failed();
    assert_eq!(
        read_only.state(),
        SaveState::ReadOnly,
        "a failure keeps read-only"
    );
    assert!(crate::save(&mut read_only, b"OVERWRITTEN").is_err());
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), "ORIGINAL");
}
