//! What may go wrong in a show (SHOW-14).

use std::io;

use serde_json::Value;

/// Fonts, links and unreadable objects of every slide and master.
pub const SLIDE_CHECK: &str = ".uno:SlideCheck";

/// One slide's or master's findings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageCheck {
    /// Font families its text asks for.
    pub fonts: Vec<String>,
    /// Linked files, and whether each is missing.
    pub links: Vec<(String, bool)>,
    /// Names of objects the kernel cannot show.
    pub broken: Vec<String>,
}

/// Slides in order, then masters by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Check {
    pub slides: Vec<PageCheck>,
    pub masters: Vec<(String, PageCheck)>,
}

fn page(value: &Value) -> io::Result<PageCheck> {
    let bad = |what: &str| io::Error::other(format!("bad {what} in {value}"));
    let texts = |key: &str| -> io::Result<Vec<String>> {
        (value[key].as_array().ok_or_else(|| bad(key))?.iter())
            .map(|item| item.as_str().map(str::to_string).ok_or_else(|| bad(key)))
            .collect()
    };
    let links = (value["links"].as_array().ok_or_else(|| bad("links"))?.iter())
        .map(|link| {
            let url = link["url"].as_str().ok_or_else(|| bad("url"))?;
            let missing = link["missing"].as_bool().ok_or_else(|| bad("missing"))?;
            Ok((url.to_string(), missing))
        })
        .collect::<io::Result<_>>()?;
    Ok(PageCheck {
        fonts: texts("fonts")?,
        links,
        broken: texts("broken")?,
    })
}

/// Parses `.uno:SlideCheck`.
pub fn check(json: &str) -> io::Result<Check> {
    let value: Value = serde_json::from_str(json).map_err(io::Error::other)?;
    let bad = |what: &str| io::Error::other(format!("bad {what} in {value}"));
    let slides = (value["slides"].as_array().ok_or_else(|| bad("slides"))?.iter())
        .map(page)
        .collect::<io::Result<_>>()?;
    let masters = (value["masters"].as_array().ok_or_else(|| bad("masters"))?.iter())
        .map(|master| {
            let name = master["name"].as_str().ok_or_else(|| bad("name"))?;
            Ok((name.to_string(), page(master)?))
        })
        .collect::<io::Result<_>>()?;
    Ok(Check { slides, masters })
}
