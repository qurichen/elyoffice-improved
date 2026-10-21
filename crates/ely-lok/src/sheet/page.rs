//! Print layout and sheet protection (TABLE-14, 15).

use std::io;

use serde_json::{Value, json};

use super::analysis::{flag, parse, texts};

/// The shown sheet's print layout.
pub const LAYOUT: &str = ".uno:PrintLayout";
/// The shown sheet's protection.
pub const PROTECTION: &str = ".uno:SheetProtection";

/// Actions a protected sheet may allow, in kernel names.
pub const PROTECT_OPTIONS: [&str; 8] = [
    "selectLocked",
    "selectUnlocked",
    "insertColumns",
    "insertRows",
    "deleteColumns",
    "deleteRows",
    "autofilter",
    "pivot",
];

/// How printing scales the sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Percent(u16),
    /// Pages wide and tall; 0 leaves one free.
    Fit(u16, u16),
    /// Pages in all.
    Pages(u16),
}

/// Rows or columns, first to last, from 1.
pub type Span = (u32, u32);

/// What the kernel prints of the shown sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintLayout {
    /// Print ranges; none prints the used area.
    pub areas: Vec<String>,
    pub rows: Option<Span>,
    pub columns: Option<Span>,
    pub landscape: bool,
    pub scale: Scale,
    /// Sheets sharing the page style.
    pub sheets: Vec<String>,
    /// Manual breaks at the cursor.
    pub row_break: bool,
    pub column_break: bool,
}

/// The JSON `page.setup` takes.
pub fn page_json(landscape: bool, scale: Scale) -> String {
    match scale {
        Scale::Percent(percent) => json!({ "landscape": landscape, "scale": percent }),
        Scale::Fit(wide, tall) => json!({ "landscape": landscape, "fit": [wide, tall] }),
        Scale::Pages(pages) => json!({ "landscape": landscape, "pages": pages }),
    }
    .to_string()
}

/// Repeated rows as `print.area` takes them.
pub fn rows_ref(span: Option<Span>) -> String {
    span.map_or_else(String::new, |(first, last)| format!("${first}:${last}"))
}

/// Repeated columns as `print.area` takes them.
pub fn columns_ref(span: Option<Span>) -> String {
    span.map_or_else(String::new, |(first, last)| {
        let name = |index: u32| super::column_name(index - 1);
        format!("${}:${}", name(first), name(last))
    })
}

fn number(value: &Value, key: &str) -> io::Result<u64> {
    value[key]
        .as_u64()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

fn span(value: &Value, key: &str) -> io::Result<Option<Span>> {
    let Some(node) = value.get(key) else {
        return Ok(None);
    };
    let bound = |end| -> io::Result<u32> {
        u32::try_from(number(node, end)?).map_err(|_| io::Error::other(format!("bad {key}")))
    };
    Ok(Some((bound("first")?, bound("last")?)))
}

/// Parses `.uno:PrintLayout`.
pub fn print_layout(json: &str) -> io::Result<PrintLayout> {
    let value = parse(json, "print layout")?;
    let small = |key| -> io::Result<u16> {
        u16::try_from(number(&value, key)?).map_err(|_| io::Error::other(format!("bad {key}")))
    };
    let (percent, wide, tall, pages) = (
        small("scale")?,
        small("fitWidth")?,
        small("fitHeight")?,
        small("pages")?,
    );
    // The kernel's order: pages, then fit, then percent.
    let scale = match () {
        _ if pages > 0 => Scale::Pages(pages),
        _ if wide > 0 || tall > 0 => Scale::Fit(wide, tall),
        _ if percent > 0 => Scale::Percent(percent),
        _ => return Err(io::Error::other(format!("no scale in {value}"))),
    };
    Ok(PrintLayout {
        areas: texts(&value, "areas")?,
        rows: span(&value, "rows")?,
        columns: span(&value, "columns")?,
        landscape: flag(&value, "landscape")?,
        scale,
        sheets: texts(&value, "sheets")?,
        row_break: flag(&value, "rowBreak")?,
        column_break: flag(&value, "columnBreak")?,
    })
}

/// A sheet's protection as the kernel reports it.
#[derive(Clone, Debug, PartialEq)]
pub struct Protection {
    pub protected: bool,
    /// Unprotecting needs a password.
    pub password: bool,
    pub allow: Vec<String>,
}

/// Parses `.uno:SheetProtection`.
pub fn protection(json: &str) -> io::Result<Protection> {
    let value = parse(json, "protection")?;
    Ok(Protection {
        protected: flag(&value, "protected")?,
        password: flag(&value, "password")?,
        allow: texts(&value, "allow")?,
    })
}

/// The JSON `sheet.protect` takes to protect.
pub fn protect_json(password: &str, allow: &[String]) -> String {
    json!({ "protect": true, "password": password, "allow": allow }).to_string()
}

/// The JSON `sheet.protect` takes to unprotect.
pub fn unprotect_json(password: &str) -> String {
    json!({ "protect": false, "password": password }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_read_each_scale() {
        let json = r#"{"areas":["$A$1:$C$4"],"rows":{"first":1,"last":2},"rowBreak":false,
            "columnBreak":true,"landscape":true,"scale":0,"pages":0,"fitWidth":1,"fitHeight":0,
            "sheets":["Sheet1","Sheet2"]}"#;
        let read = print_layout(json).unwrap();
        assert_eq!(read.areas, ["$A$1:$C$4"]);
        assert_eq!((read.rows, read.columns), (Some((1, 2)), None));
        assert_eq!(read.scale, Scale::Fit(1, 0));
        assert!(read.landscape && read.column_break && !read.row_break);
        let both = json.replace(r#""scale":0"#, r#""scale":100"#);
        assert_eq!(
            print_layout(&both).unwrap().scale,
            Scale::Fit(1, 0),
            "fit wins"
        );
        let none = json.replace(r#""fitWidth":1"#, r#""fitWidth":0"#);
        assert!(print_layout(&none).is_err(), "no scale is an error");
    }

    #[test]
    fn spans_become_references() {
        assert_eq!(rows_ref(Some((1, 3))), "$1:$3");
        assert_eq!(columns_ref(Some((1, 28))), "$A:$AB");
        assert_eq!(rows_ref(None), "");
    }
}
