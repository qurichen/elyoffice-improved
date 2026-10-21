//! The look rows chart dialogs share: kind, titles, legend.

use ely_gpui_component::{
    data_display::DescriptionList,
    forms::{Choice, Input, Select, Switch, TextInput},
};
use ely_lok::sheet::{CHART_KINDS, Look};
use gpui::{App, AppContext as _, Entity, ParentElement as _, Styled as _, Window, div};

fn text(cx: &App, key: &str) -> gpui::SharedString {
    ely_ui::i18n::text(cx, key, &[])
}

/// A new chart's look: columns with a legend.
pub fn fresh() -> Look {
    Look {
        kind: "column".into(),
        legend: true,
        ..Look::default()
    }
}

/// The look's text fields.
pub struct LookFields {
    pub title: Entity<TextInput>,
    pub x_title: Entity<TextInput>,
    pub y_title: Entity<TextInput>,
}

/// A labelled text field holding `value`.
pub fn field(value: &str, key: &str, window: &mut Window, cx: &mut App) -> Entity<TextInput> {
    let (value, name) = (value.to_string(), text(cx, key));
    cx.new(|cx| {
        let mut input = TextInput::new(window, cx).label(name);
        input.set_text(value, cx);
        input
    })
}

impl LookFields {
    pub fn new(look: &Look, window: &mut Window, cx: &mut App) -> Self {
        LookFields {
            title: field(&look.title, "office.chart.heading", window, cx),
            x_title: field(&look.x_title, "office.chart.x", window, cx),
            y_title: field(&look.y_title, "office.chart.y", window, cx),
        }
    }

    /// `look` with the fields' text.
    pub fn read(&self, look: &Look, cx: &App) -> Look {
        let text = |field: &Entity<TextInput>| field.read(cx).text().trim().to_string();
        Look {
            title: text(&self.title),
            x_title: text(&self.x_title),
            y_title: text(&self.y_title),
            ..look.clone()
        }
    }
}

/// `rows` with kind and titles added, and the legend switch.
pub fn look_rows(
    id: &str,
    rows: DescriptionList,
    (look, fields): (&Look, &LookFields),
    on_kind: impl Fn(String, &mut App) + 'static,
    on_legend: impl Fn(bool, &mut App) + 'static,
    cx: &App,
) -> (DescriptionList, Switch) {
    let label = |key: &str| text(cx, key);
    let mut kinds: Vec<&str> = CHART_KINDS.to_vec();
    if !kinds.contains(&look.kind.as_str()) {
        kinds.push("other");
    }
    let kind = Select::new(
        format!("{id}-kind"),
        kinds
            .into_iter()
            .map(|value| Choice::new(value, label(&format!("office.chart.{value}")))),
    )
    .label(label("office.chart.kind"))
    .selected(look.kind.clone())
    .on_change(move |value, _, cx| on_kind(value.to_string(), cx));
    let legend = Switch::new(format!("{id}-legend"), look.legend)
        .label(label("office.chart.legend"))
        .on_change(move |on, _, cx| on_legend(on, cx));
    let input = |field| div().w_full().max_w_48().child(Input::new(field));
    let mut rows = rows
        .item(label("office.chart.kind"), div().w_full().max_w_48().child(kind))
        .item(label("office.chart.heading"), input(&fields.title));
    // A pie has no axes.
    if look.kind != "pie" {
        rows = rows
            .item(label("office.chart.x"), input(&fields.x_title))
            .item(label("office.chart.y"), input(&fields.y_title));
    }
    (rows, legend)
}
