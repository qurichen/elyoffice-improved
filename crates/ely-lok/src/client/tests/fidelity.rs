//! OOXML dimensions, attachment modes and theme settings.

use super::*;
use std::{path::Path, process::Command};

fn checked(script: &Path, arguments: &[&std::ffi::OsStr]) {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let output = Command::new(python).arg(script).args(arguments).output().expect("the reader");
    assert!(output.status.success(), "{}{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

#[test]
fn manual_row_heights_keep_fractional_points_and_hidden_state() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let fixture = root.join("tests/fixtures/office/fractional-row-heights.xlsx");
    let saved = dir.path().join("saved.xlsx");
    let again = dir.path().join("again.xlsx");
    let reader = root.join("tests/scripts/row-height-check.py");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    doc.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
    worker.close_doc(doc).expect("closes");
    checked(&reader, &[saved.as_os_str()]);
    let doc = worker.load(&crate::file_url(&saved)).expect("reopens");
    doc.save_as(&mut worker, &crate::file_url(&again), "xlsx").expect("saves again");
    worker.close_doc(doc).expect("closes");
    checked(&reader, &[again.as_os_str()]);
}

#[test]
fn word_character_compression_survives_import_and_export() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let reader = root.join("tests/scripts/compression-check.py");
    for mode in ["doNotCompress", "compressPunctuation", "compressPunctuationAndJapaneseKana", "absent"] {
        let fixture = root.join(format!("tests/fixtures/office/compression-{mode}.docx"));
        let imported = dir.path().join("imported.fodt");
        let saved = dir.path().join("saved.docx");
        let reopened = dir.path().join("reopened.fodt");
        let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
        doc.save_as(&mut worker, &crate::file_url(&imported), "fodt").expect("flattens");
        doc.save_as(&mut worker, &crate::file_url(&saved), "docx").expect("saves");
        worker.close_doc(doc).expect("closes");
        let doc = worker.load(&crate::file_url(&saved)).expect("reopens");
        doc.save_as(&mut worker, &crate::file_url(&reopened), "fodt").expect("flattens");
        worker.close_doc(doc).expect("closes");
        checked(&reader, &[std::ffi::OsStr::new(mode), imported.as_os_str(), saved.as_os_str(), reopened.as_os_str()]);
    }
}

#[test]
fn chart_placement_keeps_page_cell_and_resize_modes() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let fixture = root.join("tests/fixtures/office/chart-anchors.xlsx");
    let imported = dir.path().join("imported.fods");
    let saved = dir.path().join("saved.xlsx");
    let reopened = dir.path().join("reopened.fods");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    doc.save_as(&mut worker, &crate::file_url(&imported), "fods").expect("flattens");
    doc.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
    worker.close_doc(doc).expect("closes");
    let doc = worker.load(&crate::file_url(&saved)).expect("reopens");
    doc.save_as(&mut worker, &crate::file_url(&reopened), "fods").expect("flattens");
    worker.close_doc(doc).expect("closes");
    checked(&root.join("tests/scripts/chart-anchor-check.py"),
        &[imported.as_os_str(), saved.as_os_str(), reopened.as_os_str()]);
}

#[test]
fn theme_fonts_keep_major_and_minor_script_mappings() {
    theme_roundtrip("docx");
}

#[test]
fn workbook_theme_fonts_keep_script_mappings() {
    theme_roundtrip("xlsx");
}

#[test]
fn presentation_theme_fonts_keep_script_mappings() {
    theme_roundtrip("pptx");
}

fn theme_roundtrip(format: &str) {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let fixture = root.join(format!("tests/fixtures/office/theme-fonts.{format}"));
    let saved = dir.path().join(format!("saved.{format}"));
    let again = dir.path().join(format!("again.{format}"));
    let reader = root.join("tests/scripts/theme-font-check.py");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    doc.save_as(&mut worker, &crate::file_url(&saved), format).expect("saves");
    worker.close_doc(doc).expect("closes");
    checked(&reader, &[saved.as_os_str()]);
    let doc = worker.load(&crate::file_url(&saved)).expect("reopens");
    doc.save_as(&mut worker, &crate::file_url(&again), format).expect("saves again");
    worker.close_doc(doc).expect("closes");
    checked(&reader, &[again.as_os_str()]);
}
