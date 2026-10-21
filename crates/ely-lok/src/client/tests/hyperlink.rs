//! Hyperlink colors survive import and PPTX export.

use super::*;
use std::path::Path;

#[test]
fn hyperlink_colors_keep_text_and_theme_modes() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = root.join("tests/fixtures/office/hyperlink-colors.pptx");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    let imported = dir.path().join("imported.fodp");
    doc.save_as(&mut worker, &crate::file_url(&imported), "fodp").expect("flattens");
    let saved = dir.path().join("saved.pptx");
    doc.save_as(&mut worker, &crate::file_url(&saved), "pptx").expect("saves");
    worker.close_doc(doc).expect("closes");
    let doc = worker.load(&crate::file_url(&saved)).expect("reopens");
    let reopened = dir.path().join("reopened.fodp");
    doc.save_as(&mut worker, &crate::file_url(&reopened), "fodp").expect("flattens");
    worker.close_doc(doc).expect("closes");
    let reader = root.join("tests/scripts/hyperlink-colors.py");
    let python = if cfg!(windows) { "python" } else { "python3" };
    let output = std::process::Command::new(python)
        .arg(reader).arg(imported).arg(saved).arg(reopened)
        .output().expect("the independent reader");
    assert!(output.status.success(), "{}{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}
