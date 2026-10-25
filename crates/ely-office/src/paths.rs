use std::{
    io,
    path::{Path, PathBuf},
};

fn canonical_target(path: &Path) -> io::Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(real) => Ok(real),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let name = path
                .file_name()
                .ok_or_else(|| io::Error::other("no file name"))?;
            Ok(std::fs::canonicalize(parent)?.join(name))
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn identity(path: &Path) -> io::Result<PathBuf> {
    Ok(dunce::simplified(&canonical_target(path)?).to_path_buf())
}

pub(crate) fn inside(root: &Path, target: &Path) -> io::Result<bool> {
    Ok(canonical_target(target)?.starts_with(canonical_target(root)?))
}

pub(crate) fn scratch(root: &Path, target: &Path) -> io::Result<PathBuf> {
    let root = std::fs::canonicalize(root)?;
    let target = std::fs::canonicalize(target)?;
    if target == root || !target.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scratch target is outside its owned root",
        ));
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_bounds_exclude_the_root_and_prefix_siblings() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let root = dir.path().join("work");
        let child = root.join("doc-0");
        let sibling = dir.path().join("work-other");
        std::fs::create_dir_all(&child).expect("the work folder");
        std::fs::create_dir(&sibling).expect("the sibling");
        assert!(inside(&root, &child.join("new.docx")).expect("inside"));
        assert!(!inside(&root, &sibling.join("new.docx")).expect("outside"));
        assert!(scratch(&root, &child).is_ok());
        assert!(scratch(&root, &root).is_err());
        assert!(scratch(&root, &sibling).is_err());
    }

    #[test]
    fn a_missing_scratch_root_can_be_compared_without_creating_it() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let root = dir.path().join("work");
        let outside = dir.path().join("new.docx");
        assert!(!inside(&root, &outside).expect("missing work can be identified"));
        assert!(!root.exists());
    }

    #[cfg(windows)]
    #[test]
    fn windows_case_aliases_and_long_targets_stay_inside_scratch() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let root = dir.path().join("WorkCase");
        std::fs::create_dir(&root).expect("the work root");
        let alias = PathBuf::from(root.to_string_lossy().to_uppercase());
        assert!(inside(&root, &alias.join("new.docx")).expect("a case alias"));
        let long = std::fs::canonicalize(&root)
            .expect("canonical root")
            .join("d".repeat(150));
        std::fs::create_dir(&long).expect("long folder");
        assert!(
            inside(&root, &long.join(format!("{}.docx", "x".repeat(120)))).expect("long target")
        );
    }

    #[cfg(windows)]
    #[test]
    fn junctions_are_resolved_before_scratch_boundaries_are_decided() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let root = dir.path().join("work");
        let child = root.join("doc-0");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&child).expect("the work folder");
        std::fs::create_dir(&outside).expect("the outside control");
        let junction = |link: &Path, target: &Path| {
            let output = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .output()
                .expect("mklink");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        let escaping = root.join("escaping");
        junction(&escaping, &outside);
        assert!(!inside(&root, &escaping.join("new.docx")).expect("outside target"));
        assert!(scratch(&root, &escaping).is_err());
        let incoming = dir.path().join("alias");
        junction(&incoming, &child);
        assert!(inside(&root, &incoming.join("new.docx")).expect("internal alias"));
    }
}
