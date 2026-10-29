use std::{
    collections::HashSet,
    io,
    path::{Component, Path},
};

use super::Kept;

mod write;
pub(super) use write::{Pending, Writer};

pub(super) fn read(dir: &Path) -> io::Result<Vec<Kept>> {
    let bytes = match std::fs::read(dir.join("recovery").join("index.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let listed: Vec<Kept> = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mut slots = HashSet::new();
    for kept in &listed {
        let slot = kept.slot.split_once('-').is_some_and(|(pid, id)| {
            !pid.is_empty()
                && !id.is_empty()
                && pid.bytes().all(|b| b.is_ascii_digit())
                && id.bytes().all(|b| b.is_ascii_digit())
                && pid.parse::<u32>().is_ok()
                && id.parse::<u64>().is_ok()
        });
        let format =
            !kept.format.is_empty() && kept.format.bytes().all(|b| b.is_ascii_alphanumeric());
        if !slot || !format || !filename(&kept.name) || !slots.insert(kept.slot.clone()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid or duplicate recovery record",
            ));
        }
    }
    Ok(listed)
}

fn filename(name: &str) -> bool {
    let path = Path::new(name);
    if !matches!(path.components().next(), Some(Component::Normal(part)) if part == path.as_os_str())
        || name.contains('\0')
    {
        return false;
    }
    #[cfg(windows)]
    if name.ends_with(['.', ' ']) || name.chars().any(|c| c < ' ' || r#"<>:"/\|?*"#.contains(c)) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn load(record: serde_json::Value) -> io::Result<Vec<Kept>> {
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::create_dir(dir.path().join("recovery")).expect("the recovery folder");
        std::fs::write(
            dir.path().join("recovery/index.json"),
            serde_json::to_vec(&record).expect("json"),
        )
        .expect("the index");
        read(dir.path())
    }

    fn record() -> serde_json::Value {
        json!({"slot":"123-0", "name":"中文文档 😀.docx", "format":"docx", "sealed":false})
    }

    #[test]
    fn normal_records_keep_unicode_and_legacy_held_defaults() {
        let listed = load(json!([record()])).expect("valid index");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "中文文档 😀.docx");
        assert!(!listed[0].held);
        let mut next = record();
        next["slot"] = json!("123-1");
        next["held"] = json!(true);
        let listed = load(json!([record(), next])).expect("two slots");
        assert!(listed[1].held);
    }

    #[test]
    fn path_components_in_recovery_metadata_are_rejected() {
        for (field, value) in [
            ("slot", "../escape"),
            ("slot", ""),
            ("name", "../outside.docx"),
            ("name", "/outside.docx"),
            ("name", "."),
            ("name", ""),
            ("format", "../docx"),
            ("format", "docx/escape"),
            ("format", ""),
        ] {
            let mut item = record();
            item[field] = json!(value);
            assert_eq!(
                load(json!([item])).expect_err("invalid component").kind(),
                io::ErrorKind::InvalidData,
                "{field}={value}"
            );
        }
    }

    #[test]
    fn duplicate_slots_are_rejected_instead_of_restored_twice() {
        assert_eq!(
            load(json!([record(), record()]))
                .expect_err("duplicate slot")
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn invalid_index_stops_startup_before_cleanup_or_lock_replacement() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let recovery = dir.path().join("recovery");
        std::fs::create_dir(&recovery).expect("the recovery folder");
        let guard = recovery.join("keep.bin");
        std::fs::write(&guard, b"retained recovery").expect("the guard");
        let mut invalid = record();
        invalid["slot"] = json!("../escape");
        let bytes = serde_json::to_vec(&json!([invalid])).expect("json");
        std::fs::write(recovery.join("index.json"), &bytes).expect("the index");
        let error = super::super::Keeping::start_in(dir.path().to_path_buf())
            .err()
            .expect("invalid index");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::read(&guard).expect("the guard remains"),
            b"retained recovery"
        );
        assert_eq!(
            std::fs::read(recovery.join("index.json")).expect("the index remains"),
            bytes
        );
        assert!(!dir.path().join("session.lock").exists());
    }

    #[cfg(windows)]
    #[test]
    fn windows_drive_paths_streams_and_normalized_aliases_are_rejected() {
        for name in [
            r"C:\outside.docx",
            r"C:outside.docx",
            r"..\outside.docx",
            r"\\server\share\outside.docx",
            "report:stream",
            "report.docx.",
            "report.docx ",
        ] {
            let mut item = record();
            item["name"] = json!(name);
            assert_eq!(
                load(json!([item]))
                    .expect_err("invalid Windows name")
                    .kind(),
                io::ErrorKind::InvalidData,
                "{name}"
            );
        }
    }
}
