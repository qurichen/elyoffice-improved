use std::{fs, io, path::Path};

pub(super) fn publish(
    dest: &Path,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let result = (|| {
        let mut staging = tempfile::NamedTempFile::new_in(parent)?;
        write(staging.as_file_mut())?;
        staging.as_file().sync_all()?;
        staging
            .persist_noclobber(dest)
            .map_err(|error| error.error)?;
        Ok(())
    })();
    result.map_err(|error: io::Error| {
        io::Error::new(error.kind(), format!("{}: {error}", dest.display()))
    })
}

pub(super) fn copy(source: &Path, dest: &Path) -> io::Result<()> {
    let mut source = fs::File::open(source)?;
    publish(dest, |target| io::copy(&mut source, target).map(drop))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn a_partial_write_never_publishes_or_leaves_staging() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let dest = dir.path().join("retry.txt");
        let error = publish(&dest, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "injected full disk",
            ))
        })
        .expect_err("injected write failure");
        assert_eq!(error.kind(), io::ErrorKind::StorageFull);
        assert!(!dest.exists());
        assert_eq!(fs::read_dir(dir.path()).expect("the folder").count(), 0);
        publish(&dest, |file| file.write_all(b"complete")).expect("retry");
        assert_eq!(fs::read(&dest).expect("bytes"), b"complete");
    }

    #[test]
    fn a_target_appearing_during_restore_is_never_overwritten() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let dest = dir.path().join("guard.txt");
        let error = publish(&dest, |file| {
            file.write_all(b"restored")?;
            fs::write(&dest, b"other writer")
        })
        .expect_err("another writer claimed the target");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&dest).expect("the guard"), b"other writer");
        assert_eq!(fs::read_dir(dir.path()).expect("the folder").count(), 1);
    }

    #[test]
    fn hardlink_targets_keep_the_original_and_its_alias() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let original = dir.path().join("original.txt");
        let dest = dir.path().join("alias.txt");
        fs::write(&original, b"original").expect("the original");
        fs::hard_link(&original, &dest).expect("the alias");
        assert_eq!(
            publish(&dest, |file| file.write_all(b"replacement"))
                .expect_err("existing alias")
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&original).expect("the original"), b"original");
        assert_eq!(fs::read(&dest).expect("the alias"), b"original");
    }
}
