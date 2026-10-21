//! Chart color defaults and explicit controls.

use super::*;
use std::{path::Path, process::Command};

#[test]
fn chart_point_color_defaults_preserve_explicit_modes_and_fills() {
    color_roundtrip(false);
}

#[test]
fn chart_point_color_defaults_keep_the_office_2007_compatibility_branch() {
    color_roundtrip(true);
}

fn color_roundtrip(legacy: bool) {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("a worker");
    let python = if cfg!(windows) { "python" } else { "python3" };
    let mut fixture = root.join("tests/fixtures/office/chart-appearance.xlsx");
    if legacy {
        let path = dir.path().join("office-2007.xlsx");
        let result = Command::new(python).arg(root.join("tests/scripts/chart-appearance-fixture.py"))
            .arg("--legacy-copy").arg(&fixture).arg(&path).output().expect("fixture metadata");
        assert!(result.status.success(), "{}{}",
            String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        fixture = path;
    }
    let imported = dir.path().join("imported.fods");
    let saved = dir.path().join("saved.xlsx");
    let reopened = dir.path().join("reopened.fods");
    let again = dir.path().join("again.xlsx");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    doc.save_as(&mut worker, &crate::file_url(&imported), "fods").expect("flattens");
    doc.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
    worker.close_doc(doc).expect("closes");
    let doc = worker.load(&crate::file_url(&saved)).expect("reopens");
    doc.save_as(&mut worker, &crate::file_url(&reopened), "fods").expect("flattens");
    doc.save_as(&mut worker, &crate::file_url(&again), "xlsx").expect("saves again");
    worker.close_doc(doc).expect("closes");
    let reader = root.join("tests/scripts/chart-appearance-check.py");
    for workbook in [&saved, &again] {
        let mut command = Command::new(python);
        command.arg(&reader);
        if legacy {
            command.arg("--office-2007");
        }
        let result = command.args([&imported, workbook, &reopened]).output().expect("the reader");
        assert!(result.status.success(), "{}{}",
            String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
    }
}
