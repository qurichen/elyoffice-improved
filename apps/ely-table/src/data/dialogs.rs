//! The data dialogs: sort, filter, duplicates, columns.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Checkbox, CheckboxGroup, Choice, Input, Switch, TextInput},
    layout::ScrollArea,
    overlays::{Close, Dialog as Card},
    primitives::Severity,
    tables::{FilterBuilder, FilterRule, SortBuilder, SortKey},
    theme::ControlSize,
};
use ely_lok::sheet::{self, Entry, Selection};
use ely_office::prelude::*;
use gpui::{App, Entity, ParentElement as _, SharedString, Styled as _, Window, div};

use super::title;
use crate::book::Book;
use crate::dialog::{Dialog, edit};

/// The kernel sorts by up to three keys.
const KEYS: usize = 3;

/// The primary button that runs what the dialog asks.
fn go(
    this: Entity<Book>,
    (id, label): (&'static str, SharedString),
    off: bool,
    run: fn(&mut Book, &mut gpui::Context<Book>),
) -> impl FnOnce(Close) -> Button + 'static {
    move |_| {
        Button::new(id, label)
            .variant(ButtonVariant::Primary)
            .disabled(off)
            .on_click(move |_, _, cx| this.update(cx, run))
    }
}

fn header_switch(this: &Entity<Book>, id: &'static str, on: bool, cx: &App) -> Switch {
    let this = this.clone();
    Switch::new(id, on)
        .label(Book::text(cx, "table.data.header", &[]))
        .on_change(move |on, _, cx| {
            edit(&this, cx, |dialog, _, _| match dialog {
                Dialog::Sort { header, .. } | Dialog::Dedupe { header, .. } => *header = on,
                _ => {}
            })
        })
}

pub fn sort_dialog(
    this: Entity<Book>,
    (picked, keys, header): (&Selection, &[SortKey], bool),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let (first, _, last, _) = picked.area.expect("a data area");
    let columns: Vec<_> = (first..=last)
        .map(|column| (column.to_string(), title(picked, column, header)))
        .collect();
    let keyed = this.clone();
    let builder = SortBuilder::new("table-sort-keys", columns)
        .keys(keys.to_vec())
        .on_change(move |keys, _, cx| {
            let mut keys = keys.to_vec();
            let over = keys.len() > KEYS;
            keys.truncate(KEYS);
            edit(&keyed, cx, |dialog, book, cx| {
                if let Dialog::Sort { keys: held, .. } = dialog {
                    *held = keys;
                }
                if over {
                    let text = Book::text(cx, "table.data.three", &[]);
                    book.toast(text, Severity::Info, cx);
                }
            })
        });
    let label = Book::text(cx, "table.data.sort_go", &[]);
    Card::new(
        "table-sort",
        Book::text(cx, "table.data.sort_title", &[]),
        dismiss,
    )
    .child(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(header_switch(&this, "table-sort-header", header, cx))
            .child(builder),
    )
    .action(ely_office::cancel(cx))
    .action(go(
        this,
        ("table-sort-go", label),
        keys.is_empty(),
        Book::data_asked,
    ))
}

pub fn filter_dialog(
    this: Entity<Book>,
    (picked, rules, any): (&Selection, &[FilterRule], bool),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let (first, _, last, _) = picked.area.expect("a data area");
    let columns: Vec<_> = (first..=last)
        .map(|column| (column.to_string(), title(picked, column, true)))
        .collect();
    let ruled = this.clone();
    let builder = FilterBuilder::new("table-filter-rules", columns)
        .rules(rules.to_vec(), any)
        .on_change(move |rules, any, _, cx| {
            let rules = rules.to_vec();
            edit(&ruled, cx, |dialog, _, _| {
                if let Dialog::Filter {
                    rules: held,
                    any: joined,
                    ..
                } = dialog
                {
                    *held = rules;
                    *joined = any;
                }
            })
        });
    let label = Book::text(cx, "table.data.filter_go", &[]);
    Card::new(
        "table-filter",
        Book::text(cx, "table.data.filter_title", &[]),
        dismiss,
    )
    .child(builder)
    .action(ely_office::cancel(cx))
    .action(go(
        this,
        ("table-filter-go", label),
        rules.is_empty(),
        Book::data_asked,
    ))
}

pub fn dedupe_dialog(
    this: Entity<Book>,
    (picked, header): (&Selection, bool),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let detail = Book::text(cx, "table.data.dedupe_detail", &[("range", &picked.range)]);
    let label = Book::text(cx, "table.data.dedupe_go", &[]);
    let removing = this.clone();
    Card::new(
        "table-dedupe",
        Book::text(cx, "table.data.dedupe_title", &[]),
        dismiss,
    )
    .detail(detail)
    .child(header_switch(&this, "table-dedupe-header", header, cx))
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("table-dedupe-go", label)
            .variant(ButtonVariant::Danger)
            .on_click(move |_, _, cx| removing.update(cx, |book, cx| book.data_asked(cx)))
    })
}

/// Separators as kernel character codes.
const SEPARATORS: [(&str, &str); 4] = [
    ("9", "table.columns.tab"),
    ("44", "table.columns.comma"),
    ("59", "table.columns.semicolon"),
    ("32", "table.columns.space"),
];

pub fn columns_dialog(
    this: Entity<Book>,
    (separators, other): (&[SharedString], &Entity<TextInput>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let choices = SEPARATORS
        .iter()
        .map(|(code, key)| Choice::new(*code, Book::text(cx, key, &[])));
    let picked = this.clone();
    let group = CheckboxGroup::new("table-columns-separators", choices)
        .selected(separators.to_vec())
        .on_change(move |values, _, cx| {
            let values = values.to_vec();
            edit(&picked, cx, |dialog, _, _| {
                if let Dialog::Columns { separators, .. } = dialog {
                    *separators = values;
                }
            })
        });
    let other_row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(Book::text(cx, "table.columns.other", &[]))
        .child(div().w_16().child(Input::new(other).size(ControlSize::Sm)));
    let none = separators.is_empty() && other.read(cx).text().is_empty();
    let label = Book::text(cx, "table.columns.go", &[]);
    Card::new(
        "table-columns",
        Book::text(cx, "table.columns.title", &[]),
        dismiss,
    )
    .child(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(group)
            .child(other_row),
    )
    .action(ely_office::cancel(cx))
    .action(go(
        this,
        ("table-columns-go", label),
        none,
        Book::columns_asked,
    ))
}

pub fn column_dialog(
    this: Entity<Book>,
    (column, entries, picked): (u32, &[Entry], &[SharedString]),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let empty = Book::text(cx, "table.filter.empty", &[]);
    let choices = entries.iter().map(|entry| {
        let label = match entry.text.is_empty() {
            true => empty.clone(),
            false => entry.text.clone().into(),
        };
        Choice::new(entry.text.clone(), label)
    });
    let checked = this.clone();
    let group = CheckboxGroup::new("table-column-entries", choices)
        .selected(picked.to_vec())
        .on_change(move |values, _, cx| {
            let values = values.to_vec();
            edit(&checked, cx, |dialog, _, _| {
                if let Dialog::Column { picked, .. } = dialog {
                    *picked = values;
                }
            })
        });
    let every = this.clone();
    let all: Vec<SharedString> = entries
        .iter()
        .map(|entry| entry.text.clone().into())
        .collect();
    let whole = Checkbox::new("table-column-all", picked.len() == entries.len())
        .label(Book::text(cx, "table.filter.all", &[]))
        .on_change(move |on, _, cx| {
            let values = if on { all.clone() } else { Vec::new() };
            edit(&every, cx, |dialog, _, _| {
                if let Dialog::Column { picked, .. } = dialog {
                    *picked = values;
                }
            })
        });
    let sort = |id: &'static str, key: &str, rising: bool| {
        let this = this.clone();
        Button::new(id, Book::text(cx, key, &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                this.update(cx, |book, cx| book.column_asked(Some(rising), cx))
            })
    };
    let title = Book::text(
        cx,
        "table.filter.title",
        &[("column", &sheet::column_name(column))],
    );
    let label = Book::text(cx, "table.filter.go", &[]);
    let (applying, none) = (this.clone(), picked.is_empty());
    Card::new("table-column", title, dismiss)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(sort("table-column-up", "table.filter.up", true))
                        .child(sort("table-column-down", "table.filter.down", false)),
                )
                .child(whole)
                .child(
                    div().h_64().child(
                        ScrollArea::new("table-column-scroll")
                            .size_full()
                            .child(group),
                    ),
                ),
        )
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("table-column-go", label)
                .variant(ButtonVariant::Primary)
                .disabled(none)
                .on_click(move |_, _, cx| {
                    applying.update(cx, |book, cx| book.column_asked(None, cx))
                })
        })
}
