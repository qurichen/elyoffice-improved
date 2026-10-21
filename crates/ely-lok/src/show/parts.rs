//! A slide's chart, notes and comments (SHOW-07, 08).

use std::io;

use serde_json::{Value, json};

use crate::Rect;
use crate::sheet::Look;

/// The shown slide's notes.
pub const NOTES: &str = ".uno:SlideNotes";

/// A chart's own table: one value per category and series.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartData {
    pub categories: Vec<String>,
    /// Name and values; `None` where a cell is empty.
    pub series: Vec<(String, Vec<Option<f64>>)>,
}

/// The selected chart on a slide.
#[derive(Clone, Debug, PartialEq)]
pub struct SlideChart {
    pub name: String,
    pub look: Look,
    /// None when the data is not one table.
    pub data: Option<ChartData>,
}

/// The `Chart` argument: the look, and the data if given.
pub fn chart_json(look: &Look, data: Option<&ChartData>) -> String {
    let mut value: Value = serde_json::from_str(&look.to_json()).expect("a look is JSON");
    if let Some(data) = data {
        let series: Vec<Value> = (data.series.iter())
            .map(|(name, values)| json!({ "name": name, "values": values }))
            .collect();
        value["data"] = json!({ "categories": data.categories, "series": series });
    }
    value.to_string()
}

fn error(what: &str, value: &Value) -> io::Error {
    io::Error::other(format!("bad {what} in {value}"))
}

/// Parses `.uno:Chart` on a slide; none without a chart.
pub fn slide_chart(json: &str) -> io::Result<Option<SlideChart>> {
    let value: Value = serde_json::from_str(json).map_err(io::Error::other)?;
    let Some(name) = value.get("name") else {
        return Ok(None);
    };
    let data = value.get("data").map(chart_data).transpose()?;
    Ok(Some(SlideChart {
        name: name
            .as_str()
            .ok_or_else(|| error("name", &value))?
            .to_string(),
        look: crate::sheet::look(&value)?,
        data,
    }))
}

fn chart_data(data: &Value) -> io::Result<ChartData> {
    let categories = data["categories"]
        .as_array()
        .ok_or_else(|| error("categories", data))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| error("category", item))
        })
        .collect::<io::Result<_>>()?;
    let series = data["series"]
        .as_array()
        .ok_or_else(|| error("series", data))?
        .iter()
        .map(|item| {
            let name = item["name"].as_str().ok_or_else(|| error("name", item))?;
            let values = item["values"]
                .as_array()
                .ok_or_else(|| error("values", item))?
                .iter()
                .map(|value| match value {
                    Value::Null => Ok(None),
                    value => value
                        .as_f64()
                        .map(Some)
                        .ok_or_else(|| error("value", value)),
                })
                .collect::<io::Result<_>>()?;
            Ok((name.to_string(), values))
        })
        .collect::<io::Result<_>>()?;
    Ok(ChartData { categories, series })
}

/// The shown slide's notes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notes {
    /// The slide they belong to; none on a master.
    pub part: Option<usize>,
    /// The slide has a notes text box.
    pub editable: bool,
    pub text: String,
    /// Their look varies, which plain text would lose.
    pub rich: bool,
}

/// Parses `.uno:SlideNotes`.
pub fn notes(json: &str) -> io::Result<Notes> {
    let value: Value = serde_json::from_str(json).map_err(io::Error::other)?;
    let flag = |key| value[key].as_bool().ok_or_else(|| error(key, &value));
    let part = value["part"]
        .as_i64()
        .ok_or_else(|| error("part", &value))?;
    Ok(Notes {
        part: usize::try_from(part).ok(),
        editable: flag("editable")?,
        text: value["text"]
            .as_str()
            .ok_or_else(|| error("text", &value))?
            .to_string(),
        rich: flag("rich")?,
    })
}

/// A comment on a slide.
#[derive(Clone, Debug, PartialEq)]
pub struct Remark {
    pub id: String,
    pub author: String,
    pub text: String,
    /// ISO 8601, as the kernel gives it.
    pub at: String,
    pub part: usize,
    /// Where it is pinned, in twips.
    pub anchor: Rect,
}

/// Parses `.uno:ViewAnnotations` from a presentation.
pub fn remarks(json: &str) -> io::Result<Vec<Remark>> {
    let value: Value = serde_json::from_str(json).map_err(io::Error::other)?;
    let Some(list) = value["comments"].as_object() else {
        return Ok(Vec::new());
    };
    list.values()
        .map(|item| {
            let text = |key: &str| {
                item[key]
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| error(key, item))
            };
            let id = item["id"].as_u64().ok_or_else(|| error("id", item))?;
            let part = item["part"].as_u64().ok_or_else(|| error("part", item))?;
            let anchor: Vec<i32> = text("rectangle")?
                .split(',')
                .map(|n| {
                    n.trim()
                        .parse::<i32>()
                        .map_err(|_| error("rectangle", item))
                })
                .collect::<io::Result<_>>()?;
            let [x, y, w, h] = anchor[..] else {
                return Err(error("rectangle", item));
            };
            Ok(Remark {
                id: id.to_string(),
                author: text("author")?,
                text: text("text")?,
                at: text("dateTime")?,
                part: usize::try_from(part).map_err(io::Error::other)?,
                anchor: (x, y, w, h),
            })
        })
        .collect()
}
