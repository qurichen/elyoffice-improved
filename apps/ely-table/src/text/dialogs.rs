//! The import dialog: encoding, separator, quote, column kinds.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    feedback::InlineMessage,
    forms::{Choice, Select, Switch},
    layout::ScrollArea,
    overlays::Dialog as Card,
    primitives::Severity,
    theme::{ActiveTheme as _, ControlSize},
};
use ely_lok::sheet::csv::{COLUMNS, Column, ENCODINGS};
use gpui::{App, Entity, ParentElement as _, SharedString, Styled as _, Window, div, rems};

use super::{Pending, SEPARATORS};
use crate::book::Book;
use crate::dialog::{Dialog, edit};

fn pending_of(dialog: &mut Dialog) -> &mut Pending {
    match dialog {
        Dialog::Import(pending) => pending,
        _ => unreachable!("an import dialog"),
    }
}

/// Changes a choice that reads the file anew.
fn reread(this: &Entity<Book>, cx: &mut App, change: impl FnOnce(&mut Pending)) {
    edit(this, cx, |dialog, _, _| change(pending_of(dialog)));
    this.update(cx, |book, cx| book.scan_import(cx));
}

fn column_key(column: Column) -> &'static str {
    match column {
        Column::Standard => "table.import.standard",
        Column::Text => "table.import.text",
        Column::DateMdy => "table.import.mdy",
        Column::DateDmy => "table.import.dmy",
        Column::DateYmd => "table.import.ymd",
        Column::Skip => "table.import.skip",
    }
}

fn separator_key(separator: char) -> &'static str {
    match separator {
        ',' => "table.import.comma",
        ';' => "table.import.semicolon",
        '\t' => "table.import.tab",
        '|' => "table.import.pipe",
        _ => "table.import.space",
    }
}

/// One column: its kind above its first cells.
fn column(this: &Entity<Book>, at: usize, kind: Column, rows: &[Vec<String>], off: bool, cx: &App) -> gpui::Div {
    let label = |key: &str| Book::text(cx, key, &[]);
    let picking = this.clone();
    let kinds = Select::new(
        ("table-import-kind", at),
        COLUMNS
            .iter()
            .enumerate()
            .map(|(index, kind)| Choice::new(index.to_string(), label(column_key(*kind)))),
    )
    .label(Book::text(cx, "table.import.kind_of", &[("n", &(at + 1).to_string())]))
    .selected(COLUMNS.iter().position(|each| *each == kind).expect("listed").to_string())
    .disabled(off)
    .on_change(move |value, _, cx| {
        let Ok(index) = value.parse::<usize>() else {
            return;
        };
        edit(&picking, cx, |dialog, _, _| {
            let pending = pending_of(dialog);
            if pending.text_only || pending.reading { return; }
            if let Some(column) = pending.how.columns.get_mut(at) { *column = COLUMNS[index]; }
        })
    });
    let fg = cx.theme().colors.fg;
    let cells = rows.iter().map(|row| {
        div()
            .h_6()
            .truncate()
            .text_color(fg)
            .child(SharedString::from(row.get(at).cloned().unwrap_or_default()))
    });
    div().w_32().flex_none().flex().flex_col().gap_1().child(kinds).children(cells)
}

pub fn import_dialog(
    this: Entity<Book>,
    pending: &Pending,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let label = |key: &str| Book::text(cx, key, &[]);
    let options = pending.import_options();
    let how = &options;
    let picking = this.clone();
    let encoding = Select::new(
        "table-import-encoding",
        ENCODINGS
            .iter()
            .enumerate()
            .map(|(index, encoding)| Choice::new(index.to_string(), encoding.label())),
    )
    .label(label("table.import.encoding"))
    .selected(ENCODINGS.iter().position(|e| *e == how.encoding).expect("listed").to_string())
    .on_change(move |value, _, cx| {
        if let Ok(index) = value.parse::<usize>() {
            reread(&picking, cx, |pending| pending.how.encoding = ENCODINGS[index]);
        }
    });
    let picking = this.clone();
    let separator = Select::new(
        "table-import-separator",
        SEPARATORS
            .iter()
            .map(|each| Choice::new(each.to_string(), label(separator_key(*each)))),
    )
    .label(label("table.import.separator"))
    .selected(how.separator.to_string())
    .on_change(move |value, _, cx| {
        if let Some(separator) = value.chars().next() {
            reread(&picking, cx, |pending| pending.how.separator = separator);
        }
    });
    let picking = this.clone();
    let quoted = how.quote.map_or(String::new(), String::from);
    let quote = SegmentedControl::new("table-import-quote", quoted)
        .size(ControlSize::Sm)
        .segment("\"", "\"", None)
        .segment("'", "'", None)
        .segment("", label("table.import.no_quote"), None)
        .on_change(move |value, _, cx| {
            let quote = value.chars().next();
            reread(&picking, cx, |pending| pending.how.quote = quote);
        });
    let choices = DescriptionList::new()
        .item(label("table.import.encoding"), div().w_full().max_w_48().child(encoding))
        .item(label("table.import.separator"), div().w_full().max_w_48().child(separator))
        .item(label("table.import.quote"), quote);
    let picking = this.clone();
    let text_only = Switch::new("table-import-text-only", pending.text_only)
        .label(label("table.import.text_only"))
        .on_change(move |on, _, cx| {
            edit(&picking, cx, |dialog, _, _| pending_of(dialog).text_only = on);
        });
    let text_hint = pending.text_only.then(|| {
        InlineMessage::new(Severity::Info, label("table.import.text_only_hint"))
    });
    let columns = how
        .columns
        .iter()
        .enumerate()
        .map(|(at, kind)| column(&this, at, *kind, &pending.rows, pending.text_only || pending.reading, cx));
    // As wide as its columns, so it scrolls.
    let shown = how.columns.len() as f32;
    let row = div().w(rems(shown * 8.5)).flex().gap_2().pb_2().children(columns);
    let preview = ScrollArea::new("table-import-preview").h_64().child(row);
    let broken = pending
        .broken
        .then(|| InlineMessage::new(Severity::Danger, label("table.import.broken")));
    let off = pending.reading || pending.broken;
    let name = ely_office::files::file_name(&pending.path);
    let go = label("table.import.go");
    Card::new(
        "table-import",
        Book::text(cx, "table.import.title", &[("name", &name)]),
        dismiss,
    )
    .child(choices)
    .child(text_only)
    .children(text_hint)
    .children(broken)
    .child(preview)
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("table-import-go", go)
            .variant(ButtonVariant::Primary)
            .disabled(off)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.import_asked(cx)))
    })
}
