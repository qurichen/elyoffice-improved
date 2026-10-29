//! Formats and the kernel half of a save (FILE-04).

use std::{
    io,
    path::{Path, PathBuf},
};

use gpui::SharedString;
use quick_xml::{
    events::Event,
    name::{Namespace, ResolveResult},
    reader::NsReader,
};

use crate::{Engine, Kind};

/// The lowercase extension of `path`.
fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// The format a save of `path` writes: its own (FILE-04).
pub fn kept_format(kind: &Kind, path: &Path) -> io::Result<&'static str> {
    let ext = extension(path);
    kind.kept
        .iter()
        .find(|kept| **kept == ext)
        .copied()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, format!("cannot save .{ext}")))
}

/// The format a Save As to `path` writes, or None.
pub fn save_as_format(kind: &Kind, path: &Path) -> Option<&'static str> {
    let ext = extension(path);
    kind.save_as.iter().find(|format| **format == ext).copied()
}

/// The kernel writes `format` into `work`; the bytes come back.
pub fn kernel_bytes<G: Engine>(
    worker: &mut G::Worker,
    doc: &G::Doc,
    work: &Path,
    format: &str,
) -> io::Result<Vec<u8>> {
    let copy = work.join(format!("save.{format}"));
    G::write(worker, doc, &copy, format)?;
    let bytes = std::fs::read(&copy)?;
    log::info!("office: the kernel wrote {} bytes of {format}", bytes.len());
    Ok(bytes)
}

/// Whether `path` carries macros Ely will not round-trip (LOK-11).
pub fn macro_risk(kind: &Kind, path: &Path) -> io::Result<bool> {
    let Some(macros) = kind.macros else {
        return Ok(false);
    };
    if macros.contains(&extension(path).as_str()) {
        return Ok(true);
    }
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"PK\x03\x04") {
        let names = zip_names(&bytes)?;
        return Ok(names
            .iter()
            .any(|name| name.ends_with("vbaProject.bin") || name.starts_with("Basic/")));
    }
    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return Ok(ole_has_vba(&bytes));
    }
    flat_odf_has_basic(&bytes)
}

/// The office namespace that carries Basic libraries.
const OOO: &[u8] = b"http://openoffice.org/2004/office";

/// Whether flat ODF embeds a Basic library, namespaces in scope.
fn flat_odf_has_basic(bytes: &[u8]) -> io::Result<bool> {
    let text = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if !text.trim_ascii_start().starts_with(b"<") {
        return Ok(false);
    }
    let mut reader = NsReader::from_reader(text);
    let mut buf = Vec::new();
    loop {
        let (bound, event) = reader
            .read_resolved_event_into(&mut buf)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        match (bound, event) {
            (ResolveResult::Bound(Namespace(OOO)), Event::Start(tag) | Event::Empty(tag))
                if tag.local_name().as_ref() == b"library-embedded" =>
            {
                return Ok(true);
            }
            (_, Event::Eof) => return Ok(false),
            _ => {}
        }
        buf.clear();
    }
}

/// The entry names in a zip's central directory.
fn zip_names(bytes: &[u8]) -> io::Result<Vec<String>> {
    let bad = || io::Error::new(io::ErrorKind::InvalidData, "a damaged zip directory");
    let u16_at = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
    };
    let u32_at = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let end = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&at| bytes[at..].starts_with(b"PK\x05\x06"))
        .ok_or_else(bad)?;
    let (count, mut at) = (
        u16_at(end + 10).ok_or_else(bad)?,
        u32_at(end + 16).ok_or_else(bad)?,
    );
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        if !bytes
            .get(at..)
            .is_some_and(|rest| rest.starts_with(b"PK\x01\x02"))
        {
            return Err(bad());
        }
        let name_len = u16_at(at + 28).ok_or_else(bad)?;
        let skip = u16_at(at + 30).ok_or_else(bad)? + u16_at(at + 32).ok_or_else(bad)?;
        let name = bytes.get(at + 46..at + 46 + name_len).ok_or_else(bad)?;
        names.push(String::from_utf8_lossy(name).into_owned());
        at += 46 + name_len + skip;
    }
    Ok(names)
}

/// Whether an OLE file has a `_VBA_PROJECT` directory entry.
fn ole_has_vba(bytes: &[u8]) -> bool {
    let name: Vec<u8> = "_VBA_PROJECT"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    // 128-byte entries: name, byte length, type 2.
    bytes.get(512..).unwrap_or_default().chunks_exact(128)
        .any(|entry| {
            entry.starts_with(&name)
                && u16::from_le_bytes([entry[64], entry[65]]) as usize == name.len() + 2
                && entry[66] == 2
        })
}

/// A path's file name for tab titles.
pub fn file_name(path: &Path) -> SharedString {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .into()
}

/// A fresh scratch folder for one document's kernel copies.
pub fn work_dir(root: &Path, id: u64) -> io::Result<PathBuf> {
    let dir = root.join(format!("doc-{id}"));
    if dir.try_exists()? {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIND: Kind = Kind {
        app: ely_platform::AppId::Doc,
        kept: &["docx", "odt", "doc"],
        save_as: &["docx", "odt"],
        macros: Some(&["docm", "dotm"]),
    };

    #[test]
    fn the_last_complete_ole_directory_slot_is_scanned() {
        let mut bytes = vec![0u8; 1536];
        bytes[..8].copy_from_slice(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]);
        let name: Vec<_> = "_VBA_PROJECT".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let at = bytes.len() - 128;
        bytes[at..at + name.len()].copy_from_slice(&name);
        bytes[at + 64..at + 66].copy_from_slice(&((name.len() + 2) as u16).to_le_bytes());
        bytes[at + 66] = 2;
        let dir = tempfile::tempdir().expect("fixture folder");
        let path = dir.path().join("last-slot.doc");
        std::fs::write(&path, &bytes).expect("fixture bytes");
        assert!(macro_risk(&KIND, &path).expect("macro scan"));
        assert!(ole_has_vba(&bytes));
        bytes.truncate(bytes.len() - 1);
        assert!(!ole_has_vba(&bytes), "an incomplete final slot is ignored");
        for len in [0, 8, 511, 512, 639] {
            assert!(!ole_has_vba(&vec![0; len]), "short input: {len}");
        }
    }

    #[test]
    fn saves_keep_the_format_and_save_as_converts() {
        let kept = |path: &str| kept_format(&KIND, Path::new(path));
        assert_eq!(kept("/a/Contract.DOC").expect("doc"), "doc");
        assert_eq!(kept("/a/b.odt").expect("odt"), "odt");
        assert!(kept("/a/b.pages").is_err(), "unknown formats fail");
        assert_eq!(save_as_format(&KIND, Path::new("/a/b.docx")), Some("docx"));
        assert_eq!(save_as_format(&KIND, Path::new("/a/b.pdf")), None);
    }

    #[test]
    fn macros_are_found_by_structure_never_by_text() {
        let dir = std::env::temp_dir().join(format!("ely-office-risk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a dir");
        let write = |name: &str, body: &[u8]| {
            let path = dir.join(name);
            std::fs::write(&path, body).expect("writes");
            path
        };
        assert!(macro_risk(&KIND, &write("a.docm", b"PK")).expect("docm"));
        let notes = write(
            "notes.txt",
            b"The archive path is Basic/Standard/Module1.xml and vbaProject.bin",
        );
        assert!(
            !macro_risk(&KIND, &notes).expect("text"),
            "words in text are not macros"
        );
        let example = write(
            "xml-example.txt",
            b"Example: <ooo:library-embedded ooo:name=\"Standard\">",
        );
        assert!(
            !macro_risk(&KIND, &example).expect("text with a tag"),
            "unbound tags are text"
        );
        let root = |prefix: &str| {
            format!(
                "<office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:{prefix}=\"http://openoffice.org/2004/office\">"
            )
        };
        let basic = format!(
            "{}<ooo:libraries><ooo:library-embedded ooo:name=\"Standard\"/>",
            root("ooo")
        );
        assert!(macro_risk(&KIND, &write("f.fodt", basic.as_bytes())).expect("flat basic"));
        let alias = format!(
            "{}<basic:libraries><basic:library-embedded basic:name=\"Standard\"/>",
            root("basic")
        );
        assert!(
            macro_risk(&KIND, &write("g.fodt", alias.as_bytes())).expect("aliased"),
            "any bound prefix counts"
        );
        let nested = format!(
            "{}<office:scripts><office:script xmlns:x=\"http://openoffice.org/2004/office\"><x:libraries><x:library-embedded x:name=\"Standard\"/>",
            root("ooo")
        );
        assert!(
            macro_risk(&KIND, &write("n.fods", nested.as_bytes())).expect("nested"),
            "a child may declare the namespace"
        );
        let shadowed = format!(
            "{}<ooo:p xmlns:ooo=\"urn:other\"><ooo:library-embedded/></ooo:p>",
            root("ooo")
        );
        assert!(
            !macro_risk(&KIND, &write("s.fodt", shadowed.as_bytes())).expect("shadowed"),
            "a prefix rebound elsewhere is not Basic"
        );
        assert!(
            macro_risk(&KIND, &write("bad.fodt", b"<office:document><a></b>")).is_err(),
            "broken XML is an error"
        );
        std::fs::remove_dir_all(&dir).expect("cleans");
    }
}
