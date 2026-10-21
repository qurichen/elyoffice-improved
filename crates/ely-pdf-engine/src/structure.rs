//! Bookmarks, links, properties, attachments (PDF-07).

use std::{io, path::Path};

use ely_ipc::Unpack;
use serde_json::{Value, json};

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// Where a bookmark or link goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// A page and a point down it.
    Page(usize, f32),
    /// An http, https or mailto address.
    Web(String),
}

/// A bookmark change; paths count children from the top.
#[derive(Clone, Debug, PartialEq)]
pub enum BookmarkChange {
    /// Before the bookmark at `path`, or last past the end.
    Add { path: Vec<usize>, title: String, page: usize, y: f32 },
    /// `expect` is its title as read; another refuses.
    Edit { path: Vec<usize>, expect: String, title: Option<String>, to: Option<(usize, f32)> },
    Delete { path: Vec<usize>, expect: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub rect: [f32; 4],
    pub uri: String,
    /// The page inside the document, if any.
    pub page: Option<usize>,
    pub y: f32,
}

/// A link change; `expect` is the address as read.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkChange {
    Add { rect: [f32; 4], to: Target },
    Edit { index: usize, expect: String, to: Target },
    Delete { index: usize, expect: String },
}

/// The document's properties; none when the file has none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Properties {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
}

/// An embedded file; dates in seconds, if known.
#[derive(Clone, Debug, PartialEq)]
pub struct Attachment {
    /// Names it for reading and removing.
    pub key: String,
    pub name: String,
    /// The page whose annotation holds it, if any.
    pub page: Option<usize>,
    pub size: u64,
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub mime: String,
    pub note: String,
}

fn bad(what: &str, value: &Value) -> io::Error {
    io::Error::other(format!("a bad {what} {value}"))
}

fn target(spec: &mut Value, to: &Target) {
    match to {
        Target::Page(page, y) => {
            spec["page"] = json!(page);
            spec["y"] = json!(y);
        }
        Target::Web(uri) => spec["uri"] = json!(uri),
    }
}

fn parsed(bytes: &[u8]) -> io::Result<Value> {
    serde_json::from_slice(bytes).map_err(io::Error::other)
}

impl Worker {
    pub fn change_bookmark(&mut self, doc: Doc, change: &BookmarkChange) -> io::Result<String> {
        let spec = match change {
            BookmarkChange::Add { path, title, page, y } => json!({ "op": "add", "path": path, "title": title, "page": page, "y": y }),
            BookmarkChange::Edit { path, expect, title, to } => {
                let mut spec = json!({ "op": "edit", "path": path, "expect": expect });
                if let Some(title) = title {
                    spec["title"] = json!(title);
                }
                if let Some((page, y)) = to {
                    target(&mut spec, &Target::Page(*page, *y));
                }
                spec
            }
            BookmarkChange::Delete { path, expect } => json!({ "op": "delete", "path": path, "expect": expect }),
        };
        let ask = self.on(&doc)?.str(&spec.to_string());
        Unpack::new(&self.request(tag::OUTLINE_EDIT, &ask.bytes())?).str()
    }

    pub fn links(&mut self, doc: Doc, page: usize) -> io::Result<Vec<Link>> {
        let reply = parsed(&self.request(tag::LINKS, &self.on(&doc)?.u64(page as u64).bytes())?)?;
        let list = reply.as_array().ok_or_else(|| bad("link list", &reply))?;
        list.iter()
            .map(|value| {
                let rect = value["rect"].as_array().filter(|r| r.len() == 4).ok_or_else(|| bad("link", value))?;
                let mut out = [0f32; 4];
                for (at, side) in rect.iter().enumerate() {
                    out[at] = side.as_f64().ok_or_else(|| bad("link", value))? as f32;
                }
                Ok(Link {
                    rect: out,
                    uri: value["uri"].as_str().ok_or_else(|| bad("link", value))?.to_string(),
                    page: usize::try_from(value["page"].as_i64().ok_or_else(|| bad("link", value))?).ok(),
                    y: value["y"].as_f64().ok_or_else(|| bad("link", value))? as f32,
                })
            })
            .collect()
    }

    pub fn change_link(&mut self, doc: Doc, page: usize, change: &LinkChange) -> io::Result<String> {
        let spec = match change {
            LinkChange::Add { rect, to } => {
                let mut spec = json!({ "op": "add", "rect": rect });
                target(&mut spec, to);
                spec
            }
            LinkChange::Edit { index, expect, to } => {
                let mut spec = json!({ "op": "edit", "index": index, "expect": expect });
                target(&mut spec, to);
                spec
            }
            LinkChange::Delete { index, expect } => json!({ "op": "delete", "index": index, "expect": expect }),
        };
        let ask = self.on(&doc)?.u64(page as u64).str(&spec.to_string());
        Unpack::new(&self.request(tag::LINK_EDIT, &ask.bytes())?).str()
    }

    pub fn properties(&mut self, doc: Doc) -> io::Result<Properties> {
        let reply = parsed(&self.request(tag::METADATA, &self.whole(&doc)?.bytes())?)?;
        let field = |key: &str| match &reply[key] {
            Value::Null => Ok(None),
            Value::String(text) => Ok(Some(text.clone())),
            other => Err(bad(key, other)),
        };
        Ok(Properties {
            title: field("Title")?,
            author: field("Author")?,
            subject: field("Subject")?,
            keywords: field("Keywords")?,
            creator: field("Creator")?,
            producer: field("Producer")?,
            created: field("CreationDate")?,
            modified: field("ModDate")?,
        })
    }

    /// Sets Title, Author, Subject or Keywords, one step.
    pub fn set_properties(&mut self, doc: Doc, changes: &[(&str, &str)]) -> io::Result<String> {
        let spec: serde_json::Map<String, Value> = changes.iter().map(|(key, value)| (key.to_string(), json!(value))).collect();
        let ask = self.whole(&doc)?.str(&Value::Object(spec).to_string());
        Unpack::new(&self.request(tag::SET_METADATA, &ask.bytes())?).str()
    }

    pub fn attachments(&mut self, doc: Doc) -> io::Result<Vec<Attachment>> {
        let reply = parsed(&self.request(tag::ATTACHMENTS, &self.whole(&doc)?.bytes())?)?;
        let list = reply.as_array().ok_or_else(|| bad("attachment list", &reply))?;
        list.iter()
            .map(|value| {
                let text = |key: &str| value[key].as_str().map(str::to_string).ok_or_else(|| bad(key, value));
                let date = |key: &str| value[key].as_i64().map(|at| (at >= 0).then_some(at)).ok_or_else(|| bad(key, value));
                Ok(Attachment {
                    key: text("key")?,
                    name: text("name")?,
                    page: usize::try_from(value["page"].as_i64().ok_or_else(|| bad("page", value))?).ok(),
                    size: value["size"].as_u64().ok_or_else(|| bad("size", value))?,
                    created: date("created")?,
                    modified: date("modified")?,
                    mime: text("type")?,
                    note: text("note")?,
                })
            })
            .collect()
    }

    pub fn attachment(&mut self, doc: Doc, key: &str) -> io::Result<Vec<u8>> {
        self.request(tag::ATTACHMENT, &self.on(&doc)?.str(key).bytes())
    }

    /// Embeds the file at `path` as `name`; the mark.
    pub fn attach(&mut self, doc: Doc, name: &str, path: &Path) -> io::Result<String> {
        let path = path.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        Unpack::new(&self.request(tag::ATTACH, &self.whole(&doc)?.str(name).str(path).bytes())?).str()
    }

    pub fn detach(&mut self, doc: Doc, key: &str) -> io::Result<String> {
        Unpack::new(&self.request(tag::DETACH, &self.on(&doc)?.str(key).bytes())?).str()
    }
}
