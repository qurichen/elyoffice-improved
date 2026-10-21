use std::fs;

#[path = "../../../tests/windows_acl/support.rs"]
mod support;

#[test]
fn staging_is_private_before_any_document_bytes_are_written() {
    let dir = tempfile::tempdir().unwrap();
    let user = support::user_sid();
    support::set_dacl(
        dir.path(),
        &format!("D:P(A;OICI;FA;;;{user})(A;OICI;FR;;;BU)"),
    );
    let target = dir.path().join("private.txt");
    fs::write(&target, b"private original").unwrap();
    support::set_dacl(&target, &format!("D:P(A;;FA;;;{user})"));
    let staging = super::super::prepare_staging(&target).expect("prepare private staging");
    assert_eq!(staging.as_file().metadata().unwrap().len(), 0);
    assert_eq!(support::dacl(staging.path()), support::dacl(&target));
}

#[test]
fn invalid_acl_paths_are_errors() {
    use std::os::windows::ffi::OsStringExt;
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("exists.txt");
    fs::write(&target, b"fixture").unwrap();
    let invalid = std::path::PathBuf::from(std::ffi::OsString::from_wide(&[65, 0, 66]));
    assert_eq!(
        super::create_private(&invalid, &target, true)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert_eq!(
        super::create_private(&target, &invalid, true)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert!(super::create_private(&dir.path().join("missing"), &target, true).is_err());
    assert_eq!(fs::read(target).unwrap(), b"fixture");
}
