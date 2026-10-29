use super::*;

#[test]
fn attachment_names_cannot_escape_the_destination() {
    let root = tempfile::tempdir().expect("fixture root");
    let dir = root.path().join("attachments");
    std::fs::create_dir(&dir).expect("destination");
    let outside = root.path().join("outside.txt");
    let nested = dir.join("nested");
    std::fs::create_dir(&nested).expect("nested folder");
    let mut names = vec!["".to_owned(), ".".into(), "..".into(), "../outside.txt".into(),
        "nested/file.txt".into(), outside.to_string_lossy().into_owned()];
    #[cfg(windows)]
    names.extend([r"..\outside.txt", r"nested\file.txt", "note.txt:stream", "NUL.txt",
        "report. ", "COM1", "AUX", "LPT9.csv"].map(str::to_owned));
    for name in names {
        let result = write_attachment(&name, b"must stay inside", &dir);
        assert!(result.is_err(), "accepted unsafe name: {name:?}");
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput, "{name:?}");
    }
    assert!(!outside.exists());
    assert_eq!(std::fs::read_dir(&nested).expect("nested listing").count(), 0);
    assert_eq!(std::fs::read_dir(&dir).expect("listing").count(), 1);
}

#[test]
fn attachment_names_keep_unicode_spaces_and_collision_copies() {
    let root = tempfile::tempdir().expect("fixture root");
    let dir = root.path().join("attachments");
    let name = "报告 Final (1) & 50%.txt";
    let first = write_attachment(name, b"original", &dir).expect("literal name");
    let next = write_attachment(name, b"copy", &dir).expect("collision copy");
    assert_eq!(first, dir.join(name));
    assert_ne!(first, next);
    assert_eq!(std::fs::read(first).expect("original"), b"original");
    assert_eq!(std::fs::read(next).expect("copy"), b"copy");
}
