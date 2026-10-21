//! Audio and video on the shown slide (SHOW-10).

use std::io;

use serde_json::Value;

/// The shown slide's media objects.
pub const MEDIA: &str = ".uno:SlideMedia";

/// One media object as the kernel keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct Media {
    /// Its place in the object list.
    pub index: usize,
    pub url: String,
    /// Linked to a file outside the document.
    pub link: bool,
    /// The file that plays: the link, or the kernel's copy.
    pub file: String,
    /// The file to play is not there.
    pub missing: bool,
    pub repeat: bool,
    /// x, y, width, height on the slide, 1/100 mm.
    pub bounds: (i64, i64, i64, i64),
}

/// Parses `.uno:SlideMedia`.
pub fn media(json: &str) -> io::Result<Vec<Media>> {
    media_of(&serde_json::from_str(json).map_err(io::Error::other)?)
}

pub(super) fn media_of(value: &Value) -> io::Result<Vec<Media>> {
    let bad = |what: &str, item: &Value| io::Error::other(format!("bad {what} in {item}"));
    let list = value["media"]
        .as_array()
        .ok_or_else(|| bad("media", value))?;
    list.iter()
        .map(|item| {
            let text = |key: &str| {
                item[key]
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| bad(key, item))
            };
            let flag = |key: &str| item[key].as_bool().ok_or_else(|| bad(key, item));
            let index = item["index"].as_u64().ok_or_else(|| bad("index", item))?;
            let at = |key: &str| item[key].as_i64().ok_or_else(|| bad(key, item));
            Ok(Media {
                index: usize::try_from(index).map_err(io::Error::other)?,
                url: text("url")?,
                link: flag("link")?,
                file: text("file")?,
                missing: flag("missing")?,
                repeat: flag("loop")?,
                bounds: (at("x")?, at("y")?, at("width")?, at("height")?),
            })
        })
        .collect()
}
