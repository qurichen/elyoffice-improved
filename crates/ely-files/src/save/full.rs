//! A full disk refuses the save; the original stays (§20.2).

use std::{path::Path, process::Command};

use super::save_atomic;

/// A 4 MB disk image, attached; detached when dropped.
struct Volume(tempfile::TempDir);

impl Volume {
    fn new() -> Option<Self> {
        let dir = tempfile::tempdir().expect("a temp dir");
        let image = dir.path().join("full.dmg");
        let made = Command::new("hdiutil")
            .args(["create", "-quiet", "-size", "4m", "-fs", "HFS+", "-volname", "elyfull"])
            .arg(&image)
            .status();
        if !made.is_ok_and(|status| status.success()) {
            return None;
        }
        let mount = dir.path().join("mount");
        std::fs::create_dir(&mount).expect("the mount point");
        let attached = Command::new("hdiutil")
            .args(["attach", "-quiet", "-nobrowse", "-mountpoint"])
            .arg(&mount)
            .arg(&image)
            .status()
            .expect("hdiutil runs");
        assert!(attached.success(), "the image attaches");
        Some(Volume(dir))
    }

    fn root(&self) -> std::path::PathBuf {
        self.0.path().join("mount")
    }
}

impl Drop for Volume {
    fn drop(&mut self) {
        let detached = Command::new("hdiutil").args(["detach", "-quiet", "-force"]).arg(self.root()).status();
        if !detached.is_ok_and(|status| status.success()) {
            eprintln!("the test image stays attached at {}", self.root().display());
        }
    }
}

/// Writes until the volume refuses, then frees nothing.
fn fill(root: &Path) {
    let block = vec![7u8; 64 * 1024];
    let mut filler = std::fs::File::create(root.join("filler")).expect("the filler");
    while std::io::Write::write_all(&mut filler, &block).is_ok() {}
}

#[test]
fn a_full_disk_refuses_the_save_and_keeps_the_original() {
    let Some(volume) = Volume::new() else {
        return eprintln!("SKIP: no hdiutil");
    };
    let target = volume.root().join("keep.md");
    std::fs::write(&target, "original").expect("the original");
    fill(&volume.root());
    let failed = save_atomic(&target, &vec![b'x'; 256 * 1024], 1).expect_err("no room");
    assert_eq!(failed.kind(), std::io::ErrorKind::StorageFull, "{failed}");
    assert_eq!(std::fs::read_to_string(&target).expect("reads"), "original");
    let left: Vec<_> = std::fs::read_dir(volume.root()).expect("lists").flatten().map(|entry| entry.file_name()).collect();
    assert_eq!(left.len(), 2, "no half-written file stays: {left:?}");
}
