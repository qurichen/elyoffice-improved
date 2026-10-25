//! Requested deep profiles remain usable on Windows.

use super::*;
use std::path::Path;

/// Counts Windows path units, including surrogate pairs.
fn units(path: &Path) -> usize {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().count()
}

fn deep(root: &Path, target: usize) -> PathBuf {
    let mut path = root.to_path_buf();
    while units(&path) < target {
        let remaining = target - units(&path);
        if remaining <= 1 {
            break;
        }
        path.push("d".repeat((remaining - 1).min(40)));
    }
    std::fs::create_dir_all(&path).expect("the requested directory exists");
    path
}

#[test]
fn deep_profiles_keep_their_location_and_save_reopened_content() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let root = tempfile::tempdir().expect("an isolated directory");
    for (index, target) in [180, 240, 300, 520, 320].into_iter().enumerate() {
        let base = root.path().join(format!("case-{index}"));
        let base = if index == 4 { base.join("资料 空格%😀") } else { base };
        let profile = deep(&base, target);
        assert!(units(&profile) >= target - 1);
        let mut worker = Worker::spawn(&install, &profile).expect("the deep profile starts");
        let source = root.path().join(format!("input-{index}.fodt"));
        std::fs::write(&source, FODT).expect("a synthetic document writes");
        let doc = worker.load(&crate::file_url(&source)).expect("the document opens");
        let output = profile.join("saved.fodt");
        doc.save_as(&mut worker, &crate::file_url(&output), "fodt")
            .expect("the deep output saves");
        worker.close_doc(doc).expect("the source closes");
        let doc = worker.load(&crate::file_url(&output)).expect("the deep output reopens");
        worker.run(&doc, "edit.select_all", &[]).expect("the text selects");
        assert_eq!(doc.selection_text(&mut worker).expect("the text reads").trim(), "Hello 世界");
        doc.paint_tile(&mut worker, (0, 0, 3840, 3840), (256, 256))
            .expect("the reopened document paints");
        worker.close_doc(doc).expect("the reopened document closes");
        worker.shutdown().expect("the worker exits normally");
        assert!(profile.join("user").is_dir(), "the requested profile was used");
        assert!(output.is_file(), "the saved file stays at the requested path");
    }
}
