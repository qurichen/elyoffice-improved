//! The pivot table and chart dialogs.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, Input, Select},
    layout::ScrollArea,
    overlays::Dialog as Card,
};
use ely_lok::sheet::{Chart, Layout, Look, PIVOT_FUNCTIONS};
use gpui::{App, Entity, ParentElement as _, Styled as _, Window, div};

use super::ChartFields;
use crate::book::Book;
use crate::dialog::{Dialog, edit};

/// Where a field goes; empty for nowhere.
const ROLES: [(&str, &str); 5] = [
    ("", "table.pivot.unused"),
    ("rows", "table.pivot.rows"),
    ("columns", "table.pivot.columns"),
    ("pages", "table.pivot.pages"),
    ("data", "table.pivot.data"),
];

fn role_of(layout: &Layout, field: &str) -> &'static str {
    let has = |list: &[String]| list.iter().any(|name| name == field);
    if has(&layout.rows) {
        "rows"
    } else if has(&layout.columns) {
        "columns"
    } else if has(&layout.pages) {
        "pages"
    } else if layout.data.iter().any(|(name, _)| name == field) {
        "data"
    } else {
        ""
    }
}

/// Moves `field` to the end of `role`'s list.
fn place(layout: &mut Layout, field: &str, role: &str) {
    for list in [&mut layout.rows, &mut layout.columns, &mut layout.pages] {
        list.retain(|name| name != field);
    }
    layout.data.retain(|(name, _)| name != field);
    let field = field.to_string();
    match role {
        "rows" => layout.rows.push(field),
        "columns" => layout.columns.push(field),
        "pages" => layout.pages.push(field),
        "data" => layout.data.push((field, "sum".into())),
        _ => {}
    }
}

fn layout_of(dialog: &mut Dialog) -> &mut Layout {
    match dialog {
        Dialog::Pivot { layout, .. } => layout,
        _ => unreachable!("a pivot dialog"),
    }
}

/// One field's area and, for values, its function.
fn field_row(this: &Entity<Book>, at: usize, field: &str, layout: &Layout, cx: &App) -> gpui::Div {
    let label = |key: &str| Book::text(cx, key, &[]);
    let role = role_of(layout, field);
    let (picking, name) = (this.clone(), field.to_string());
    let roles = Select::new(
        ("table-pivot-role", at),
        ROLES
            .iter()
            .map(|(value, key)| Choice::new(*value, label(key))),
    )
    .label(field.to_string())
    .selected(role.to_string())
    .on_change(move |value, _, cx| {
        let value = value.to_string();
        edit(&picking, cx, |dialog, _, _| {
            place(layout_of(dialog), &name, &value)
        })
    });
    let mut row = div().flex().gap_2().child(div().w_full().max_w_32().child(roles));
    if let Some((_, function)) = layout.data.iter().find(|(name, _)| name == field) {
        let mut functions: Vec<&str> = PIVOT_FUNCTIONS.to_vec();
        // The kernel's own function stays until changed.
        if !functions.contains(&function.as_str()) {
            functions.push("other");
        }
        let (picking, name) = (this.clone(), field.to_string());
        let shown = if PIVOT_FUNCTIONS.contains(&function.as_str()) {
            function.clone()
        } else {
            "other".into()
        };
        let choices = functions
            .into_iter()
            .map(|value| Choice::new(value, label(&format!("table.pivot.{value}"))));
        row = row.child(
            div().w_full().max_w_32().child(
                Select::new(("table-pivot-function", at), choices)
                    .label(Book::text(
                        cx,
                        "table.pivot.function_of",
                        &[("field", field)],
                    ))
                    .selected(shown)
                    .on_change(move |value, _, cx| {
                        let value = value.to_string();
                        edit(&picking, cx, |dialog, _, _| {
                            let data = &mut layout_of(dialog).data;
                            if let Some(entry) = data.iter_mut().find(|(field, _)| *field == name) {
                                entry.1 = value;
                            }
                        })
                    }),
            ),
        );
    }
    row
}

pub fn pivot_dialog(
    this: Entity<Book>,
    (source, fields, layout, editing): (&str, &[String], &Layout, bool),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let label = |key: &str| Book::text(cx, key, &[]);
    let rows = fields
        .iter()
        .enumerate()
        .fold(DescriptionList::new(), |rows, (at, field)| {
            rows.item(field.clone(), field_row(&this, at, field, layout, cx))
        });
    let go = label(if editing {
        "table.pivot.apply"
    } else {
        "table.pivot.go"
    });
    let off = layout.data.is_empty() || layout.data.iter().any(|(_, f)| f == "other");
    Card::new("table-pivot", label("table.pivot.title"), dismiss)
        .detail(Book::text(cx, "table.pivot.source", &[("range", source)]))
        .child(
            div().max_h_96().child(
                ScrollArea::new("table-pivot-scroll")
                    .size_full()
                    .child(rows),
            ),
        )
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("table-pivot-go", go)
                .variant(ButtonVariant::Primary)
                .disabled(off)
                .on_click(move |_, _, cx| this.update(cx, |book, cx| book.pivot_asked(cx)))
        })
}

fn look_of(dialog: &mut Dialog) -> &mut Look {
    match dialog {
        Dialog::Chart { look, .. } => look,
        _ => unreachable!("a chart dialog"),
    }
}

pub fn chart_dialog(
    this: Entity<Book>,
    (chart, look, fields): (Option<&Chart>, &Look, &ChartFields),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let label = |key: &str| Book::text(cx, key, &[]);
    let (picking, toggling) = (this.clone(), this.clone());
    let range = DescriptionList::new().item(
        label("table.chart.range"),
        div().w_full().max_w_48().child(Input::new(&fields.range)),
    );
    let (rows, legend) = ely_office::chart::look_rows(
        "table-chart",
        range,
        (look, &fields.look),
        move |kind, cx| edit(&picking, cx, |dialog, _, _| look_of(dialog).kind = kind),
        move |on, cx| edit(&toggling, cx, |dialog, _, _| look_of(dialog).legend = on),
        cx,
    );
    let (title, go) = if chart.is_some() {
        ("office.chart.edit_title", "office.chart.apply")
    } else {
        ("office.chart.insert_title", "office.chart.go")
    };
    let go = label(go);
    Card::new("table-chart", label(title), dismiss)
        .child(rows)
        .child(legend)
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("table-chart-go", go)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| this.update(cx, |book, cx| book.chart_asked(cx)))
        })
}
