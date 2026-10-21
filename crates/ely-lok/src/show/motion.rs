//! Transitions and animations of a slide (SHOW-09).

use std::io;

use serde_json::{Value, json};

/// The shown slide's transition.
pub const TRANSITION: &str = ".uno:SlideTransition";
/// The shown slide's animation effects.
pub const ANIMATIONS: &str = ".uno:SlideAnimations";

/// Transitions Ely offers, by kernel preset; empty is none.
pub const TRANSITIONS: [&str; 10] = [
    "",
    "fade-smoothly",
    "fade-through-black",
    "push-left",
    "push-up",
    "cover-left",
    "uncover-left",
    "wipe-right",
    "split-vertical-out",
    "dissolve",
];

/// Effects Ely offers, by kernel preset.
pub const EFFECTS: [&str; 14] = [
    "ooo-entrance-appear",
    "ooo-entrance-fade-in",
    "ooo-entrance-fly-in",
    "ooo-entrance-wipe",
    "ooo-entrance-zoom",
    "ooo-entrance-float",
    "ooo-emphasis-grow-and-shrink",
    "ooo-emphasis-blink",
    "ooo-exit-disappear",
    "ooo-exit-fade-out",
    "ooo-exit-fly-out",
    "ooo-exit-wipe",
    "ooo-exit-zoom",
    "ooo-exit-float",
];

/// A slide's transition; preset "other" is not Ely's.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub preset: String,
    /// Seconds.
    pub duration: f64,
    /// Advances after `time` seconds; else on a click.
    pub auto: bool,
    pub time: f64,
}

/// One effect of the slide's main sequence, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub preset: String,
    /// The kernel's name for the preset, English.
    pub label: String,
    /// entrance, emphasis, exit or other.
    pub class: String,
    /// The target's place in the object list, if listed.
    pub target: Option<usize>,
    /// The target's slideshow layer hash.
    pub hash: String,
    /// click, with or after.
    pub start: String,
    pub duration: f64,
    pub delay: f64,
}

fn error(what: &str, value: &Value) -> io::Error {
    io::Error::other(format!("bad {what} in {value}"))
}

fn text(value: &Value, key: &str) -> io::Result<String> {
    (value[key].as_str().map(str::to_string)).ok_or_else(|| error(key, value))
}

fn number(value: &Value, key: &str) -> io::Result<f64> {
    value[key].as_f64().ok_or_else(|| error(key, value))
}

/// Parses `.uno:SlideTransition`.
pub fn transition(json: &str) -> io::Result<Transition> {
    transition_of(&serde_json::from_str(json).map_err(io::Error::other)?)
}

pub(super) fn transition_of(value: &Value) -> io::Result<Transition> {
    let advance = text(value, "advance")?;
    Ok(Transition {
        preset: text(value, "preset")?,
        duration: number(value, "duration")?,
        auto: match advance.as_str() {
            "auto" => true,
            "click" => false,
            _ => return Err(error("advance", value)),
        },
        time: number(value, "time")?,
    })
}

/// The `Transition` argument; `all` sets every slide.
pub fn transition_json(transition: &Transition, all: bool) -> String {
    json!({
        "preset": transition.preset,
        "duration": transition.duration,
        "advance": if transition.auto { "auto" } else { "click" },
        "time": transition.time,
        "all": all,
    })
    .to_string()
}

/// Parses `.uno:SlideAnimations`.
pub fn effects(json: &str) -> io::Result<Vec<Effect>> {
    effects_of(&serde_json::from_str(json).map_err(io::Error::other)?)
}

pub(super) fn effects_of(value: &Value) -> io::Result<Vec<Effect>> {
    let list = value["effects"]
        .as_array()
        .ok_or_else(|| error("effects", value))?;
    list.iter()
        .map(|item| {
            let target = item["target"]
                .as_i64()
                .ok_or_else(|| error("target", item))?;
            Ok(Effect {
                preset: text(item, "preset")?,
                label: text(item, "label")?,
                class: text(item, "class")?,
                target: usize::try_from(target).ok(),
                hash: text(item, "hash")?,
                start: text(item, "start")?,
                duration: number(item, "duration")?,
                delay: number(item, "delay")?,
            })
        })
        .collect()
}
