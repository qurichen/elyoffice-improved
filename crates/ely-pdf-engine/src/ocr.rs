//! OCR: read pages, then lay their text in hidden (PDF-08).

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_ipc::Unpack;
use serde_json::{Value, json};

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// A line Tesseract read, in page points.
#[derive(Clone, Debug, PartialEq)]
pub struct OcrLine {
    pub rect: [f32; 4],
    pub text: String,
}

/// Beside the app, in its macOS bundle, or this build's.
fn beside(dir: &Path) -> PathBuf {
    [dir.join("tessdata"), dir.join("../Resources/tessdata")]
        .into_iter()
        .find(|found| found.is_dir())
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engines/tessdata"))
}

/// The language data: `ELY_TESSDATA`, the app's, or this build's.
pub fn tessdata() -> io::Result<PathBuf> {
    let found = match std::env::var_os("ELY_TESSDATA") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let exe = std::env::current_exe()?;
            beside(exe.parent().unwrap_or(Path::new(".")))
        }
    };
    match found.join("eng.traineddata").is_file() {
        true => Ok(found),
        false => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no OCR data in {} (run scripts/fetch-tessdata.sh or set ELY_TESSDATA)", found.display()),
        )),
    }
}

fn line(value: &Value) -> io::Result<OcrLine> {
    let bad = || io::Error::other(format!("a bad OCR line {value}"));
    let list = value["rect"].as_array().filter(|list| list.len() == 4).ok_or_else(bad)?;
    let mut rect = [0f32; 4];
    for (at, side) in list.iter().enumerate() {
        rect[at] = side.as_f64().ok_or_else(bad)? as f32;
    }
    Ok(OcrLine { rect, text: value["text"].as_str().ok_or_else(bad)?.to_string() })
}

impl Worker {
    /// Reads `page` in `langs` (as "eng+chi_sim") at `dpi`.
    pub fn ocr_page(&mut self, doc: Doc, page: usize, langs: &str, data: &Path, dpi: f32) -> io::Result<Vec<OcrLine>> {
        let data = data.to_str().ok_or_else(|| io::Error::other("a data path that is not UTF-8"))?;
        let ask = self.on(&doc)?.u64(page as u64).str(langs).str(data).u64(u64::from(dpi.to_bits()));
        let reply: Value = serde_json::from_slice(&self.request(tag::OCR_PAGE, &ask.bytes())?).map_err(io::Error::other)?;
        reply.as_array().ok_or_else(|| io::Error::other("an OCR reply that is not a list"))?.iter().map(line).collect()
    }

    /// Lays `pages` in hidden text, one step; the mark.
    pub fn text_layer(&mut self, doc: Doc, pages: &[(usize, Vec<OcrLine>)], replace: bool) -> io::Result<String> {
        let spec: Vec<Value> = (pages.iter())
            .map(|(page, lines)| json!({ "page": page, "lines": lines.iter().map(|l| json!({ "rect": l.rect, "text": l.text })).collect::<Vec<_>>() }))
            .collect();
        let ask = self.on(&doc)?.str(&Value::Array(spec).to_string()).u64(u64::from(replace));
        Unpack::new(&self.request(tag::TEXT_LAYER, &ask.bytes())?).str()
    }
}

#[cfg(test)]
mod tests {
    use super::beside;

    #[test]
    fn a_bundle_finds_its_own_data() {
        let app = tempfile::tempdir().expect("a temp dir");
        let (exe, data) = (app.path().join("Contents/MacOS"), app.path().join("Contents/Resources/tessdata"));
        std::fs::create_dir_all(&exe).expect("MacOS");
        assert!(beside(&exe).ends_with("engines/tessdata"));
        std::fs::create_dir_all(&data).expect("tessdata");
        assert_eq!(beside(&exe), exe.join("../Resources/tessdata"));
        std::fs::create_dir(exe.join("tessdata")).expect("beside");
        assert_eq!(beside(&exe), exe.join("tessdata"));
    }
}
