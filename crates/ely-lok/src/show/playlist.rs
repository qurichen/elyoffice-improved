//! Every slide as a show plays it (SHOW-11, 12).

use std::io;

use serde_json::Value;

use super::{Effect, Media, Transition, effects_of, media_of, transition_of};

/// All slides: hidden, notes, motion, media.
pub const SLIDE_SHOW: &str = ".uno:SlideShow";

/// One slide for the show.
#[derive(Clone, Debug, PartialEq)]
pub struct ShowSlide {
    pub hidden: bool,
    pub notes: String,
    pub transition: Transition,
    pub effects: Vec<Effect>,
    pub media: Vec<Media>,
}

/// Parses `.uno:SlideShow`.
pub fn slide_show(json: &str) -> io::Result<Vec<ShowSlide>> {
    let value: Value = serde_json::from_str(json).map_err(io::Error::other)?;
    let bad = |what: &str, item: &Value| io::Error::other(format!("bad {what} in {item}"));
    let list = value["slides"]
        .as_array()
        .ok_or_else(|| bad("slides", &value))?;
    list.iter()
        .map(|item| {
            Ok(ShowSlide {
                hidden: item["hidden"].as_bool().ok_or_else(|| bad("hidden", item))?,
                notes: (item["notes"].as_str().map(str::to_string))
                    .ok_or_else(|| bad("notes", item))?,
                transition: transition_of(&item["transition"])?,
                effects: effects_of(item)?,
                media: media_of(item)?,
            })
        })
        .collect()
}
