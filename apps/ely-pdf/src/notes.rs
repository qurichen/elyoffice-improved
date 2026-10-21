//! The note panel: annotations, notes, stamps, fields (PDF-05, PDF-06).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, ColorPalette, Input, Select, TextInput},
    theme::ControlSize,
};
use ely_office::Tabs as _;
use ely_pdf_engine::{Annot, Field, FieldKind};
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, Hsla, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, WeakEntity, Window, div, rgb,
};

use crate::annotate::Pick;
use crate::content::{Aim, Tool};
use crate::marking::Change;
use crate::viewer::Viewer;

/// The standard stamps, by PDF name.
pub const STAMPS: [&str; 14] = [
    "Approved",
    "AsIs",
    "Confidential",
    "Departmental",
    "Draft",
    "Experimental",
    "Expired",
    "Final",
    "ForComment",
    "ForPublicRelease",
    "NotApproved",
    "NotForPublicRelease",
    "Sold",
    "TopSecret",
];

/// Annotation kinds with a name of their own.
const KINDS: [&str; 15] = [
    "Highlight", "Underline", "StrikeOut", "Squiggly", "Text", "FreeText", "Square", "Circle", "Line", "Polygon",
    "PolyLine", "Ink", "Stamp", "Caret", "FileAttachment",
];

/// An annotation kind's name; unknown kinds as written.
pub fn kind_label(cx: &App, kind: &str) -> SharedString {
    match KINDS.contains(&kind) {
        true => Viewer::text(cx, &format!("pdf.annot.{kind}"), &[]),
        false => SharedString::from(kind.to_string()),
    }
}

fn label(cx: &App, key: &str) -> SharedString {
    Viewer::text(cx, key, &[])
}

pub struct NotePanel {
    viewer: WeakEntity<Viewer>,
    /// The tab, view version and pick the fields show.
    pub shown: Option<(u64, u64, Pick)>,
    /// Contents, value or name; made when first drawn.
    pub(crate) text: Option<Entity<TextInput>>,
    /// A field's name, and a choice's options, one a line.
    pub(crate) name: Option<Entity<TextInput>>,
    pub(crate) options: Option<Entity<TextInput>>,
    pub stamp: SharedString,
}

impl NotePanel {
    pub fn new(viewer: WeakEntity<Viewer>) -> Self {
        NotePanel { viewer, shown: None, text: None, name: None, options: None, stamp: STAMPS[0].into() }
    }

    /// What the view's notes say of `pick`.
    fn found(view: &crate::view::PdfView, pick: Pick) -> (Option<&Annot>, Option<&Field>) {
        match pick {
            Pick::Annot { page, id } => (view.notes.get(&page).and_then(|m| m.annots.iter().find(|a| a.id == id)), None),
            Pick::Field { page, id } => (None, view.notes.get(&page).and_then(|m| m.fields.iter().find(|f| f.id == id))),
            _ => (None, None),
        }
    }

    /// Fills the inputs for what the view picks now.
    fn follow(&mut self, aimed: (u64, u64, Pick), view: &Entity<crate::view::PdfView>, cx: &mut Context<Self>) {
        if self.shown == Some(aimed) {
            return;
        }
        let view = view.read(cx);
        let (annot, field) = Self::found(view, aimed.2);
        let designing = matches!(view.tool, Tool::Field(_));
        let (text, name, options) = match aimed.2 {
            Pick::Annot { .. } => (annot.map_or_else(String::new, |a| a.contents.clone()), String::new(), String::new()),
            Pick::Field { .. } => match field {
                Some(f) if designing => (String::new(), f.leaf().to_string(), f.options.join("\n")),
                Some(f) => (f.value.clone(), String::new(), String::new()),
                None => (String::new(), String::new(), String::new()),
            },
            Pick::NewField { page, kind, .. } => (String::new(), suggested(view, page, kind), String::new()),
            Pick::Link { page, index } => {
                let link = view.links.get(&page).and_then(|links| links.get(index));
                (String::new(), link.map_or_else(String::new, crate::links::typed), String::new())
            }
            Pick::NewNote { .. } | Pick::NewStamp { .. } | Pick::NewLink { .. } => (String::new(), String::new(), String::new()),
        };
        // Inputs are named for assistive tech.
        let text_name = match (aimed.2, field) {
            (Pick::Field { .. }, Some(f)) => SharedString::from(f.name.clone()),
            _ => label(cx, "pdf.note.contents"),
        };
        (self.shown, self.stamp) = (Some(aimed), STAMPS[0].into());
        let name_name = match aimed.2 {
            Pick::Link { .. } | Pick::NewLink { .. } => label(cx, "pdf.link.target"),
            _ => label(cx, "pdf.field.name"),
        };
        let named = [text_name, name_name, label(cx, "pdf.field.options")];
        let inputs = [(&self.text, text), (&self.name, name), (&self.options, options)];
        for ((input, value), named) in inputs.into_iter().zip(named) {
            if let Some(input) = input {
                input.update(cx, |input, cx| {
                    input.set_text(value, cx);
                    input.set_label(named, cx);
                });
            }
        }
    }

    /// Asks the viewer to make `change` on what is shown.
    fn send(this: &Entity<Self>, change: Change, cx: &mut App) {
        let (Some(shown), Some(viewer)) = (this.read(cx).shown, this.read(cx).viewer.upgrade()) else {
            return;
        };
        viewer.update(cx, |viewer, cx| viewer.change_note(shown, change, cx));
    }

    fn typed(input: &Option<Entity<TextInput>>, cx: &App) -> String {
        input.as_ref().map_or_else(String::new, |input| input.read(cx).text().to_string())
    }

    fn button(id: &'static str, text: SharedString, this: &Entity<Self>, make: fn(&NotePanel, &App) -> Change) -> Button {
        let this = this.clone();
        Button::new(id, text).size(ControlSize::Sm).on_click(move |_, _, cx| {
            let change = make(this.read(cx), cx);
            Self::send(&this, change, cx);
        })
    }
}

/// A free name for a new field.
fn suggested(view: &crate::view::PdfView, page: usize, kind: FieldKind) -> String {
    let on_page = view.notes.get(&page).map_or(&[][..], |m| m.fields.as_slice());
    // A new radio joins the page's last top group.
    if kind == FieldKind::Radio
        && let Some(last) = on_page.iter().rev().find(|f| f.kind == FieldKind::Radio && !f.name.contains('.'))
    {
        return last.name.clone();
    }
    let stem = match kind {
        FieldKind::Checkbox => "Check",
        FieldKind::Radio => "Group",
        FieldKind::Choice | FieldKind::List => "Choice",
        _ => "Text",
    };
    let taken = view.form_names.as_deref().expect("names are read first");
    (1..).map(|n| format!("{stem}{n}")).find(|name| !taken.contains(name)).expect("a free name")
}

impl Render for NotePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.text.is_none() {
            self.text = Some(cx.new(|cx| TextInput::new(window, cx).multi_line(3, 12)));
            self.name = Some(cx.new(|cx| TextInput::new(window, cx)));
            self.options = Some(cx.new(|cx| TextInput::new(window, cx).multi_line(3, 8)));
        }
        let Some(viewer) = self.viewer.upgrade() else {
            return div();
        };
        let Some((tab, view)) = viewer.read(cx).active_tab().map(|tab| (tab.id, tab.view.clone())) else {
            return div();
        };
        let Some(Aim::Note(pick)) = view.read(cx).aim else {
            return div();
        };
        // A new field's name is free in the whole form.
        if matches!(pick, Pick::NewField { .. }) && view.read(cx).form_names.is_none() {
            view.update(cx, |view, cx| view.ask_names(cx));
            return div();
        }
        self.follow((tab, view.read(cx).version, pick), &view, cx);
        let this = cx.entity();
        let designing = matches!(view.read(cx).tool, Tool::Field(_));
        let (annot, field) = Self::found(view.read(cx), pick);
        let (annot, field) = (annot.cloned(), field.cloned());
        let body = match (pick, annot, field) {
            (Pick::Annot { .. }, Some(annot), _) => self.annot_rows(&this, &annot, cx),
            (Pick::Field { .. }, _, Some(field)) if designing => self.design_rows(&this, field.kind, cx),
            (Pick::Field { .. }, _, Some(field)) => self.fill_rows(&this, &field, cx),
            (Pick::NewField { kind, .. }, _, _) => self.design_rows(&this, kind, cx),
            (Pick::Link { .. } | Pick::NewLink { .. }, _, _) => self.link_rows(&this, matches!(pick, Pick::NewLink { .. }), cx),
            (Pick::NewNote { .. }, _, _) => div()
                .flex()
                .flex_col()
                .gap_3()
                .children(self.text.as_ref().map(Input::new))
                .child(div().flex().child(Self::button("pdf-note-add", label(cx, "pdf.note.add"), &this, |p, cx| Change::Note(Self::typed(&p.text, cx))).variant(ButtonVariant::Primary)))
                .into_any_element(),
            (Pick::NewStamp { .. }, _, _) => {
                let picked = this.clone();
                let stamps = STAMPS.map(|name| Choice::new(name, label(cx, &format!("pdf.stamp.{name}"))));
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Select::new("pdf-stamp", stamps.to_vec())
                            .selected(self.stamp.clone())
                            .label(label(cx, "pdf.tool.stamp"))
                            .on_change(move |value, _, cx| picked.update(cx, |panel, cx| {
                                panel.stamp = value.clone();
                                cx.notify();
                            })),
                    )
                    .child(div().flex().child(Self::button("pdf-stamp-add", label(cx, "pdf.note.add"), &this, |p, _| Change::Stamp(p.stamp.to_string())).variant(ButtonVariant::Primary)))
                    .into_any_element()
            }
            _ => div().into_any_element(),
        };
        div().p_3().flex().flex_col().gap_3().child(body)
    }
}

impl NotePanel {
    fn annot_rows(&self, this: &Entity<Self>, annot: &Annot, cx: &mut Context<Self>) -> AnyElement {
        let swatches = ely_office::panel::COLORS.map(|color| (format!("#{color:06X}"), Hsla::from(rgb(color))));
        let painted = this.clone();
        let mut color = ColorPalette::new("pdf-note-color", swatches.clone()).columns(5).on_change(move |color, _, cx| {
            let at = swatches.iter().position(|(_, swatch)| *swatch == color).expect("the palette hands back its own colors");
            Self::send(&painted, Change::Color(ely_office::panel::COLORS[at]), cx);
        });
        if let Some(now) = annot.rgb.filter(|rgb| ely_office::panel::COLORS.contains(rgb)) {
            color = color.selected(rgb(now));
        }
        let mut rows = DescriptionList::new().item(label(cx, "pdf.note.kind"), kind_label(cx, &annot.kind));
        if !annot.author.is_empty() {
            rows = rows.item(label(cx, "pdf.note.author"), annot.author.clone());
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(rows.item(label(cx, "pdf.text.color"), color))
            .children(self.text.as_ref().map(Input::new))
            .child(self.actions(this, ("pdf-note-apply", "pdf-note-delete"), |p, cx| Change::Contents(Self::typed(&p.text, cx)), cx))
            .into_any_element()
    }

    /// Name and options of a field, new or picked.
    fn design_rows(&self, this: &Entity<Self>, kind: FieldKind, cx: &mut Context<Self>) -> AnyElement {
        let choice = matches!(kind, FieldKind::Choice | FieldKind::List);
        let make = |p: &NotePanel, cx: &App| {
            let options: Vec<String> = Self::typed(&p.options, cx).lines().map(str::trim).filter(|o| !o.is_empty()).map(String::from).collect();
            Change::Design(Self::typed(&p.name, cx).trim().to_string(), options)
        };
        let new = matches!(self.shown, Some((_, _, Pick::NewField { .. })));
        let buttons = match new {
            true => div().flex().child(Self::button("pdf-field-add", label(cx, "pdf.note.add"), this, make).variant(ButtonVariant::Primary)),
            false => self.actions(this, ("pdf-field-apply", "pdf-field-delete"), make, cx),
        };
        let mut rows = DescriptionList::new();
        if let Some(name) = &self.name {
            rows = rows.item(label(cx, "pdf.field.name"), div().w_full().child(Input::new(name)));
        }
        if let Some(options) = self.options.as_ref().filter(|_| choice) {
            rows = rows.item(label(cx, "pdf.field.options"), div().w_full().child(Input::new(options)));
        }
        div().flex().flex_col().gap_3().child(rows).child(buttons).into_any_element()
    }

    /// A field's value: typed, or picked from its options.
    fn fill_rows(&self, this: &Entity<Self>, field: &Field, cx: &mut Context<Self>) -> AnyElement {
        if field.kind == FieldKind::Text {
            let apply = Self::button("pdf-fill-apply", label(cx, "pdf.text.apply"), this, |p, cx| Change::Fill(Self::typed(&p.text, cx)));
            return div()
                .flex()
                .flex_col()
                .gap_3()
                .child(div().text_sm().child(field.name.clone()))
                .children(self.text.as_ref().map(Input::new))
                .child(div().flex().child(apply.variant(ButtonVariant::Primary)))
                .into_any_element();
        }
        let choices: Vec<Choice> = (field.options.iter().zip(&field.labels)).map(|(value, shown)| Choice::new(value.clone(), shown.clone())).collect();
        let picked = this.clone();
        Select::new("pdf-fill-choice", choices)
            .selected(SharedString::from(field.value.clone()))
            .label(SharedString::from(field.name.clone()))
            .on_change(move |value, _, cx| Self::send(&picked, Change::Fill(value.to_string()), cx))
            .into_any_element()
    }

    /// A link's target: a page number or an address.
    fn link_rows(&self, this: &Entity<Self>, new: bool, cx: &mut Context<Self>) -> AnyElement {
        let make = |p: &NotePanel, cx: &App| Change::Target(Self::typed(&p.name, cx));
        let buttons = match new {
            true => div().flex().child(Self::button("pdf-link-add", label(cx, "pdf.note.add"), this, make).variant(ButtonVariant::Primary)),
            false => self.actions(this, ("pdf-link-apply", "pdf-link-delete"), make, cx),
        };
        let mut rows = DescriptionList::new();
        if let Some(name) = &self.name {
            rows = rows.item(label(cx, "pdf.link.target"), div().w_full().child(Input::new(name)));
        }
        div().flex().flex_col().gap_3().child(rows).child(buttons).into_any_element()
    }

    /// Apply and Delete for what is picked.
    fn actions(&self, this: &Entity<Self>, (apply, delete): (&'static str, &'static str), make: fn(&NotePanel, &App) -> Change, cx: &App) -> gpui::Div {
        let apply = Self::button(apply, label(cx, "pdf.text.apply"), this, make);
        let delete = Self::button(delete, label(cx, "pdf.object.delete"), this, |_, _| Change::Delete);
        div().flex().gap_2().child(apply.variant(ButtonVariant::Primary)).child(delete.variant(ButtonVariant::Danger))
    }
}
