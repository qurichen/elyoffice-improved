//! Annotations and form fields: read, add, change (PDF-05, PDF-06).

use std::io;

use ely_ipc::Unpack;
use serde_json::{Value, json};

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// An annotation as the page holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct Annot {
    /// The object number; stays through edits.
    pub id: u64,
    /// The PDF subtype: Highlight, Text, Ink and so on.
    pub kind: String,
    pub rect: [f32; 4],
    pub rgb: Option<u32>,
    pub contents: String,
    pub author: String,
    pub icon: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Checkbox,
    Radio,
    Choice,
    List,
    Signature,
    Button,
}

/// One widget of a form field.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub id: u64,
    pub kind: FieldKind,
    pub name: String,
    pub rect: [f32; 4],
    pub value: String,
    /// A button's on state.
    pub on: String,
    pub checked: bool,
    pub readonly: bool,
    /// Choice values, and what each shows.
    pub options: Vec<String>,
    pub labels: Vec<String>,
}

/// What a page holds besides content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Marks {
    pub annots: Vec<Annot>,
    pub fields: Vec<Field>,
}

/// A new annotation's shape, in page points.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Highlight(Vec<[f32; 4]>),
    Underline(Vec<[f32; 4]>),
    StrikeOut(Vec<[f32; 4]>),
    Note((f32, f32)),
    Square([f32; 4]),
    Circle([f32; 4]),
    Line([f32; 4]),
    Ink(Vec<Vec<(f32, f32)>>),
    Stamp([f32; 4], String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewAnnot {
    pub page: usize,
    pub shape: Shape,
    pub rgb: Option<u32>,
    pub contents: String,
    pub author: String,
}

/// A new field; radios of one name form a group.
#[derive(Clone, Debug, PartialEq)]
pub struct NewField {
    pub kind: FieldKind,
    pub name: String,
    pub rect: [f32; 4],
    pub options: Vec<String>,
}

impl Field {
    /// The last part of the name, which a rename sets.
    pub fn leaf(&self) -> &str {
        self.name.rfind('.').map_or(self.name.as_str(), |dot| &self.name[dot + 1..])
    }
}

impl FieldKind {
    fn name(self) -> &'static str {
        match self {
            FieldKind::Text => "text",
            FieldKind::Checkbox => "checkbox",
            FieldKind::Radio => "radio",
            FieldKind::Choice => "choice",
            FieldKind::List => "list",
            FieldKind::Signature => "signature",
            FieldKind::Button => "button",
        }
    }
}

fn spec(annot: &NewAnnot) -> Value {
    let (kind, geometry) = match &annot.shape {
        Shape::Highlight(rects) => ("highlight", json!({ "rects": rects })),
        Shape::Underline(rects) => ("underline", json!({ "rects": rects })),
        Shape::StrikeOut(rects) => ("strikeout", json!({ "rects": rects })),
        Shape::Note(at) => ("note", json!({ "at": [at.0, at.1] })),
        Shape::Square(rect) => ("square", json!({ "rect": rect })),
        Shape::Circle(rect) => ("circle", json!({ "rect": rect })),
        Shape::Line(line) => ("line", json!({ "line": line })),
        Shape::Ink(strokes) => {
            let flat: Vec<Vec<f32>> = strokes.iter().map(|s| s.iter().flat_map(|p| [p.0, p.1]).collect()).collect();
            ("ink", json!({ "strokes": flat }))
        }
        Shape::Stamp(rect, icon) => ("stamp", json!({ "rect": rect, "icon": icon })),
    };
    let mut spec = json!({ "page": annot.page, "kind": kind, "contents": annot.contents, "author": annot.author });
    if let Some(rgb) = annot.rgb {
        spec["color"] = json!(rgb);
    }
    spec.as_object_mut().expect("an object").extend(geometry.as_object().expect("an object").clone());
    spec
}

fn bad(what: &str, value: &Value) -> io::Error {
    io::Error::other(format!("a bad {what} {value}"))
}

fn rect(value: &Value) -> io::Result<[f32; 4]> {
    let list = value.as_array().filter(|list| list.len() == 4).ok_or_else(|| bad("rectangle", value))?;
    let mut out = [0f32; 4];
    for (at, side) in list.iter().enumerate() {
        out[at] = side.as_f64().ok_or_else(|| bad("rectangle", value))? as f32;
    }
    Ok(out)
}

fn text(value: &Value, key: &str) -> io::Result<String> {
    value[key].as_str().map(str::to_string).ok_or_else(|| bad(key, value))
}

fn texts(value: &Value, key: &str) -> io::Result<Vec<String>> {
    let list = value[key].as_array().ok_or_else(|| bad(key, value))?;
    list.iter().map(|item| item.as_str().map(str::to_string).ok_or_else(|| bad(key, value))).collect()
}

fn annot(value: &Value) -> io::Result<Annot> {
    let color = value["color"].as_i64().ok_or_else(|| bad("annotation", value))?;
    Ok(Annot {
        id: value["id"].as_u64().ok_or_else(|| bad("annotation", value))?,
        kind: text(value, "kind")?,
        rect: rect(&value["rect"])?,
        rgb: u32::try_from(color).ok(),
        contents: text(value, "contents")?,
        author: text(value, "author")?,
        icon: text(value, "icon")?,
    })
}

fn field(value: &Value) -> io::Result<Field> {
    let kind = match value["kind"].as_str() {
        Some("text") => FieldKind::Text,
        Some("checkbox") => FieldKind::Checkbox,
        Some("radio") => FieldKind::Radio,
        Some("choice") => FieldKind::Choice,
        Some("list") => FieldKind::List,
        Some("signature") => FieldKind::Signature,
        Some("button") => FieldKind::Button,
        _ => return Err(bad("field", value)),
    };
    let flag = |key: &str| value[key].as_bool().ok_or_else(|| bad(key, value));
    Ok(Field {
        id: value["id"].as_u64().ok_or_else(|| bad("field", value))?,
        kind,
        name: text(value, "name")?,
        rect: rect(&value["rect"])?,
        value: text(value, "value")?,
        on: text(value, "on")?,
        checked: flag("checked")?,
        readonly: flag("readonly")?,
        options: texts(value, "options")?,
        labels: texts(value, "labels")?,
    })
}

impl Worker {
    pub fn marks(&mut self, doc: Doc, page: usize) -> io::Result<Marks> {
        let reply = self.request(tag::ANNOTS, &self.on(&doc)?.u64(page as u64).bytes())?;
        let value: Value = serde_json::from_slice(&reply).map_err(io::Error::other)?;
        let list = |key: &str| value[key].as_array().ok_or_else(|| bad(key, &value));
        Ok(Marks {
            annots: list("annots")?.iter().map(annot).collect::<io::Result<_>>()?,
            fields: list("fields")?.iter().map(field).collect::<io::Result<_>>()?,
        })
    }

    /// Every field name in the form, widgets or not.
    pub fn field_names(&mut self, doc: Doc) -> io::Result<Vec<String>> {
        let reply = self.request(tag::FIELD_NAMES, &self.whole(&doc)?.bytes())?;
        let value: Value = serde_json::from_slice(&reply).map_err(io::Error::other)?;
        let list = value.as_array().ok_or_else(|| bad("name list", &value))?;
        list.iter().map(|name| name.as_str().map(str::to_string).ok_or_else(|| bad("field name", name))).collect()
    }

    /// Adds `annots` as one step; the mark.
    pub fn annotate(&mut self, doc: Doc, annots: &[NewAnnot]) -> io::Result<String> {
        let specs = Value::Array(annots.iter().map(spec).collect());
        Unpack::new(&self.request(tag::ANNOTATE, &self.on(&doc)?.str(&specs.to_string()).bytes())?).str()
    }

    fn on_object(&mut self, kind: u8, doc: Doc, page: usize, id: u64, value: &str) -> io::Result<String> {
        let ask = self.on(&doc)?.u64(page as u64).u64(id).str(value);
        Unpack::new(&self.request(kind, &ask.bytes())?).str()
    }

    pub fn change_annot(&mut self, doc: Doc, page: usize, id: u64, rgb: Option<u32>, contents: Option<&str>) -> io::Result<String> {
        let mut change = json!({});
        if let Some(rgb) = rgb {
            change["color"] = json!(rgb);
        }
        if let Some(contents) = contents {
            change["contents"] = json!(contents);
        }
        self.on_object(tag::CHANGE_ANNOT, doc, page, id, &change.to_string())
    }

    pub fn delete_annot(&mut self, doc: Doc, page: usize, id: u64) -> io::Result<String> {
        self.on_object(tag::DELETE_ANNOT, doc, page, id, "annot")
    }

    pub fn delete_field(&mut self, doc: Doc, page: usize, id: u64) -> io::Result<String> {
        self.on_object(tag::DELETE_ANNOT, doc, page, id, "field")
    }

    /// Sets a value; buttons take their on state or Off.
    pub fn fill(&mut self, doc: Doc, page: usize, id: u64, value: &str) -> io::Result<String> {
        self.on_object(tag::FILL, doc, page, id, value)
    }

    pub fn change_field(&mut self, doc: Doc, page: usize, id: u64, name: Option<&str>, options: Option<&[String]>) -> io::Result<String> {
        let mut change = json!({});
        if let Some(name) = name {
            change["name"] = json!(name);
        }
        if let Some(options) = options {
            change["options"] = json!(options);
        }
        self.on_object(tag::CHANGE_FIELD, doc, page, id, &change.to_string())
    }

    pub fn add_field(&mut self, doc: Doc, page: usize, field: &NewField) -> io::Result<String> {
        let spec = json!({ "kind": field.kind.name(), "name": field.name, "rect": field.rect, "options": field.options });
        let ask = self.on(&doc)?.u64(page as u64).str(&spec.to_string());
        Unpack::new(&self.request(tag::ADD_FIELD, &ask.bytes())?).str()
    }
}
