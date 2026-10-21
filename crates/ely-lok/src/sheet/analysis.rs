//! Charts and pivot tables read from the kernel (TABLE-10, 11).

use std::io;

use serde_json::{Value, json};

/// The one selected chart.
pub const CHART: &str = ".uno:Chart";
/// The pivot table at the cursor.
pub const PIVOT: &str = ".uno:PivotTable";

/// Chart kinds the kernel takes; it reads others as "other".
pub const CHART_KINDS: [&str; 5] = ["column", "bar", "line", "scatter", "pie"];
/// Pivot data functions.
pub const PIVOT_FUNCTIONS: [&str; 5] = ["sum", "count", "average", "max", "min"];

pub(super) fn parse(json: &str, what: &str) -> io::Result<Value> {
    serde_json::from_str(json).map_err(|error| io::Error::other(format!("bad {what}: {error}")))
}

pub(super) fn text(value: &Value, key: &str) -> io::Result<String> {
    value[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

pub(super) fn flag(value: &Value, key: &str) -> io::Result<bool> {
    value[key]
        .as_bool()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

pub(super) fn texts(value: &Value, key: &str) -> io::Result<Vec<String>> {
    value[key]
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| io::Error::other(format!("bad {key} in {value}")))
        })
        .collect()
}

/// A chart's kind, titles and legend.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub kind: String,
    pub title: String,
    pub legend: bool,
    pub x_title: String,
    pub y_title: String,
}

impl Look {
    /// The JSON the kernel's `Chart` argument takes.
    pub fn to_json(&self) -> String {
        json!({
            "type": self.kind,
            "title": self.title,
            "legend": self.legend,
            "xtitle": self.x_title,
            "ytitle": self.y_title,
        })
        .to_string()
    }
}

/// A selected chart: its object name, data and look.
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    pub name: String,
    pub range: String,
    pub col_headers: bool,
    pub row_headers: bool,
    pub look: Look,
}

/// Parses `.uno:Chart`; none when no chart is selected.
pub fn chart(json: &str) -> io::Result<Option<Chart>> {
    let value = parse(json, "chart")?;
    if value.get("name").is_none() {
        return Ok(None);
    }
    Ok(Some(Chart {
        name: text(&value, "name")?,
        range: text(&value, "range")?,
        col_headers: flag(&value, "colHeaders")?,
        row_headers: flag(&value, "rowHeaders")?,
        look: look(&value)?,
    }))
}

/// The look members of a chart read.
pub(crate) fn look(value: &Value) -> io::Result<Look> {
    Ok(Look {
        kind: text(value, "type")?,
        title: text(value, "title")?,
        legend: flag(value, "legend")?,
        x_title: text(value, "xtitle")?,
        y_title: text(value, "ytitle")?,
    })
}

/// A pivot table's layout: fields by area, data with functions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub rows: Vec<String>,
    pub columns: Vec<String>,
    pub pages: Vec<String>,
    pub data: Vec<(String, String)>,
}

impl Layout {
    /// The `Pivot` argument: from `source` to `dest`, or in place.
    pub fn to_json(&self, source: &str, dest: &str) -> String {
        let data: Vec<Value> = self
            .data
            .iter()
            .map(|(field, function)| json!({ "field": field, "function": function }))
            .collect();
        json!({
            "source": source,
            "dest": dest,
            "rows": self.rows,
            "columns": self.columns,
            "pages": self.pages,
            "data": data,
        })
        .to_string()
    }
}

/// The pivot table at the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct Pivot {
    pub name: String,
    /// Where it shows.
    pub range: String,
    /// Its data; empty when not from cells.
    pub source: String,
    pub fields: Vec<String>,
    pub layout: Layout,
}

/// Parses `.uno:PivotTable`; none off a pivot table.
pub fn pivot(json: &str) -> io::Result<Option<Pivot>> {
    let value = parse(json, "pivot table")?;
    if value.get("name").is_none() {
        return Ok(None);
    }
    let data = value["data"]
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no data in {value}")))?
        .iter()
        .map(|item| Ok((text(item, "field")?, text(item, "function")?)))
        .collect::<io::Result<_>>()?;
    Ok(Some(Pivot {
        name: text(&value, "name")?,
        range: text(&value, "range")?,
        source: text(&value, "source")?,
        fields: texts(&value, "fields")?,
        layout: Layout {
            rows: texts(&value, "rows")?,
            columns: texts(&value, "columns")?,
            pages: texts(&value, "pages")?,
            data,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_reads_parse() {
        assert_eq!(
            chart(r#"{"commandName":".uno:Chart"}"#).expect("none"),
            None
        );
        let one = r#"{"name":"Object 1","range":"$S.$A$1:$B$4","colHeaders":true,"rowHeaders":false,"type":"bar","title":"T","legend":false,"xtitle":"","ytitle":"Y"}"#;
        let one = chart(one).expect("a chart").expect("selected");
        assert_eq!(
            (one.look.kind.as_str(), one.look.y_title.as_str()),
            ("bar", "Y")
        );
        assert!(chart(r#"{"name":"Object 1"}"#).is_err(), "parts missing");
        let table = r#"{"name":"P","range":"$S.$E$1:$G$5","source":"$S.$A$1:$C$9","fields":["Region","Year","Sales"],"rows":["Region"],"columns":[],"pages":[],"data":[{"field":"Sales","function":"sum"}]}"#;
        let table = pivot(table).expect("a pivot").expect("here");
        assert_eq!(table.layout.data, [("Sales".into(), "sum".into())]);
        assert_eq!(
            pivot(r#"{"commandName":".uno:PivotTable"}"#).expect("none"),
            None
        );
    }
}
