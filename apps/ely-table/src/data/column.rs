use ely_gpui_component::tables::SortKey;
use ely_lok::{Request, sheet::{self, Entry}};
use ely_office::prelude::*;
use gpui::{Context, SharedString};

use super::sort_values;
use crate::{book::Book, dialog::Dialog};

/// The query keeping the picked column entries.
pub fn column_query(column: u32, entries: &[Entry], picked: &[SharedString]) -> String {
    if entries.iter().all(|entry| picked.iter().any(|p| *p == entry.text)) {
        return serde_json::json!({ "column": column }).to_string();
    }
    let values: Vec<&str> = picked.iter().map(SharedString::as_ref).collect();
    serde_json::json!({
        "column": column,
        "conditions": [{ "op": "values", "values": values }],
    }).to_string()
}

impl Book {
    /// Reads the column menu and its original context.
    pub(crate) fn ask_column(&mut self, id: u64, request: &Request, cx: &mut Context<Self>) {
        let Request::Filter { column, row } = *request else { return };
        let Some(tab) = self.tab(id) else { return };
        let view = tab.view.clone();
        let read_context = self.tab_mut(id).expect("the tab lives").extra.filter_read.take()
            .map(|(_, read)| read).unwrap_or_else(|| view.read(cx).request_read());
        let at = match view.read(cx).checked_request_read(&read_context) {
            Ok(at) => at,
            Err(error) => return self.column_failed(id, error, cx),
        };
        self.ask_in(
            id,
            move |worker, doc| {
                if worker.now(doc)? != at {
                    return Err(std::io::Error::other(format!("{}the menu read changed", ely_lok::STALE)));
                }
                let json = worker.values(&at, &sheet::cell_query(sheet::FILTER, column, row))?;
                Ok((sheet::filter_values(&json)?, at))
            },
            move |book, read, _, cx| match read {
                Ok((filter, at)) => {
                    if view.read(cx).checked_request_read(&read_context).ok().as_ref() != Some(&at) {
                        return book.column_failed(id, std::io::Error::other(format!("{}the menu read changed", ely_lok::STALE)), cx);
                    }
                    let picked = filter.entries.iter().filter(|entry| entry.selected)
                        .map(|entry| SharedString::from(entry.text.clone())).collect();
                    book.open_dialog(Dialog::Column { tab: id, at, column, row, entries: filter.entries, picked });
                    cx.notify();
                }
                Err(error) => book.column_failed(id, error, cx),
            },
            cx,
        );
    }

    fn column_failed(&self, id: u64, error: std::io::Error, cx: &mut Context<Self>) {
        if ely_lok::is_stale(&error) {
            if let Some(tab) = self.tab(id) {
                tab.view.update(cx, |_, cx| cx.emit(ely_lok::LokError(error.to_string())));
            }
        } else {
            self.engine_failed(&error.to_string(), cx);
        }
    }

    /// Locates and changes the original menu target as one batch.
    pub(crate) fn column_asked(&mut self, sort: Option<bool>, cx: &mut Context<Self>) {
        let Some(Dialog::Column { tab, at, column, row, entries, picked }) = self.dialog.take() else { return };
        let (id, values) = match sort {
            Some(rising) => {
                let key = SortKey { column: column.to_string().into(), rising };
                ("data.sort", sort_values(true, &[key]))
            }
            None => ("data.filter", vec![column_query(column, &entries, &picked)]),
        };
        let address = sheet::address(column, row);
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        self.run_all_on(tab, &at, &[("cell.goto", &[address.as_str()]), (id, &values)], cx);
    }
}
