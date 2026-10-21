//! Legacy EQ keeps its editable model and field semantics.

use super::*;
use std::{ffi::OsStr, path::Path, process::Command};

fn checked(root: &Path, args: &[&OsStr]) {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let output = Command::new(python)
        .arg(root.join("tests/scripts/word-equation-check.py"))
        .args(args)
        .output()
        .expect("the equation reader");
    assert!(output.status.success(), "{}{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

#[test]
fn legacy_eq_fields_keep_semantics_through_docx_and_odf() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = root.join("tests/fixtures/office/word-equations.docx");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let flat = dir.path().join("imported.fodt");
    let direct = dir.path().join("direct.docx");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads the fixture");
    doc.save_as(&mut worker, &crate::file_url(&flat), "fodt").expect("flattens");
    doc.save_as(&mut worker, &crate::file_url(&direct), "docx").expect("exports EQ");
    worker.close_doc(doc).expect("closes");
    checked(&root, &[OsStr::new("flat"), flat.as_os_str()]);
    checked(&root, &[OsStr::new("docx"), direct.as_os_str()]);
    let mut previous = direct;
    for index in 1..=2 {
        let saved = dir.path().join(format!("reopened-{index}.docx"));
        let doc = worker.load(&crate::file_url(&previous)).expect("reopens EQ");
        doc.save_as(&mut worker, &crate::file_url(&saved), "docx").expect("saves EQ again");
        worker.close_doc(doc).expect("closes");
        checked(&root, &[OsStr::new("docx"), saved.as_os_str()]);
        previous = saved;
    }
    for format in ["odt", "fodt"] {
        let middle = dir.path().join(format!("middle.{format}"));
        let saved = dir.path().join(format!("via-{format}.docx"));
        let doc = worker.load(&crate::file_url(&fixture)).expect("loads for ODF");
        doc.save_as(&mut worker, &crate::file_url(&middle), format).expect("saves ODF");
        worker.close_doc(doc).expect("closes");
        let doc = worker.load(&crate::file_url(&middle)).expect("reopens ODF");
        doc.save_as(&mut worker, &crate::file_url(&saved), "docx").expect("exports from ODF");
        worker.close_doc(doc).expect("closes");
        checked(&root, &[OsStr::new("docx"), saved.as_os_str()]);
    }
}

#[test]
fn edited_eq_exports_the_current_tree_and_refuses_lossy_rewrites() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let fixture = root.join("tests/fixtures/office/word-equations.docx");
    let flat = dir.path().join("before-edit.fodt");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    doc.save_as(&mut worker, &crate::file_url(&flat), "fodt").expect("flattens");
    worker.close_doc(doc).expect("closes");
    for variant in ["edited", "spacing", "mixed", "leading", "blank", "quote"] {
        let edited = dir.path().join(format!("{variant}.fodt"));
        let saved = dir.path().join(format!("{variant}.docx"));
        checked(&root, &[OsStr::new("edit"), flat.as_os_str(), edited.as_os_str(), OsStr::new(variant)]);
        let doc = worker.load(&crate::file_url(&edited)).expect("loads the current model");
        doc.save_as(&mut worker, &crate::file_url(&saved), "docx").expect("exports edits");
        worker.close_doc(doc).expect("closes");
        checked(&root, &[OsStr::new("docx"), saved.as_os_str(), OsStr::new(variant)]);
    }
}

#[test]
fn unmarked_private_eq_metadata_cannot_change_math_styles() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let fixture = root.join("tests/fixtures/office/word-equations.docx");
    let flat = dir.path().join("marked.fodt");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    doc.save_as(&mut worker, &crate::file_url(&flat), "fodt").expect("flattens");
    worker.close_doc(doc).expect("closes");
    let baseline = dir.path().join("plain.odt");
    for variant in ["plain", "absent", "false"] {
        let source = dir.path().join(format!("metadata-{variant}.fodt"));
        let saved = dir.path().join(format!("{variant}.odt"));
        checked(&root, &[OsStr::new("poison"), flat.as_os_str(), source.as_os_str(), OsStr::new(variant)]);
        let doc = worker.load(&crate::file_url(&source)).expect("loads unmarked math");
        doc.save_as(&mut worker, &crate::file_url(&saved), "odt").expect("saves current properties");
        worker.close_doc(doc).expect("closes");
        checked(&root, &[OsStr::new("odf-styles"), baseline.as_os_str(), saved.as_os_str()]);
    }
}
