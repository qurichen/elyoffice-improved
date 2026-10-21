//! The edit panel: text, font, size, colour; objects (PDF-03, PDF-04).

use std::sync::Arc;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, ColorPalette, Input, NumberInput, Select, TextInput},
    theme::ControlSize,
};
use ely_office::Tabs as _;
use ely_pdf_engine::Writing;
use gpui::{
    AppContext as _, Context, Entity, Hsla, IntoElement, ParentElement as _, Render, RenderImage, SharedString,
    Styled as _, WeakEntity, Window, div, rgb,
};

use crate::content::Aim;
use crate::viewer::Viewer;

/// Built-in fonts by value and label key.
pub(crate) const FONTS: [(&str, &str); 8] = [
    ("original", "pdf.font.original"),
    ("helvetica", "pdf.font.helvetica"),
    ("times", "pdf.font.times"),
    ("courier", "pdf.font.courier"),
    ("cjk-sc", "pdf.font.sc"),
    ("cjk-tc", "pdf.font.tc"),
    ("cjk-jp", "pdf.font.jp"),
    ("cjk-kr", "pdf.font.kr"),
];

/// A change that alters layout, shown before it is made.
pub struct Preview {
    pub tab: u64,
    /// The view's version when asked; any edit since voids it.
    pub version: u64,
    pub page: usize,
    pub writing: Writing,
    pub image: Arc<RenderImage>,
}

pub struct EditPanel {
    viewer: WeakEntity<Viewer>,
    /// The tab, view version and aim the fields show.
    shown: Option<(u64, u64, Aim)>,
    /// Made on the first draw, which has a window.
    pub(crate) field: Option<Entity<TextInput>>,
    pub font: SharedString,
    pub size: f32,
    pub rgb: u32,
    /// The text box width, in points.
    pub width: f32,
    /// Why the last write did not happen.
    pub note: Option<SharedString>,
}

impl EditPanel {
    pub fn new(viewer: WeakEntity<Viewer>) -> Self {
        EditPanel { viewer, shown: None, field: None, font: "original".into(), size: 12.0, rgb: 0, width: 200.0, note: None }
    }

    /// Fills the fields for what the view aims at now.
    fn follow(&mut self, aimed: (u64, u64, Aim), cx: &mut Context<Self>) {
        if self.shown == Some(aimed) {
            return;
        }
        let Some(viewer) = self.viewer.upgrade() else {
            return;
        };
        let Some(view) = viewer.read(cx).active_view() else {
            return;
        };
        let view = view.read(cx);
        let (text, font, size, rgb, width) = match aimed.2 {
            Aim::Text { page, index } => match view.layers.get(&page).and_then(|layer| layer.blocks.get(index)) {
                Some(b) => (b.text.clone(), "original", b.size, b.rgb, b.rect[2] - b.rect[0]),
                None => return,
            },
            Aim::New { page, at } => (String::new(), "helvetica", 12.0, 0, (view.sizes[page].0 - at.0 - 36.0).max(72.0)),
            Aim::Object { .. } => (String::new(), "original", 12.0, 0, 200.0),
            Aim::Note(_) => return,
        };
        (self.shown, self.font, self.size, self.rgb, self.width, self.note) = (Some(aimed), font.into(), size, rgb, width, None);
        if let Some(field) = &self.field {
            field.update(cx, |field, cx| field.set_text(&text, cx));
        }
    }

    /// The text to write for `aim` on `view`.
    pub fn writing(&self, view: &crate::view::PdfView, aim: Aim, cx: &gpui::App) -> Option<(usize, Writing)> {
        let text = self.field.as_ref()?.read(cx).text().to_string();
        let (page, block, expected, bx) = match aim {
            Aim::Text { page, index } => {
                let b = view.layers.get(&page)?.blocks.get(index)?;
                (page, Some(index), b.text.clone(), [b.rect[0], b.rect[1], self.width])
            }
            Aim::New { page, at } => (page, None, String::new(), [at.0, at.1, self.width]),
            Aim::Object { .. } | Aim::Note(_) => return None,
        };
        let writing = Writing { block, expected, text, font: self.font.to_string(), size: self.size, rgb: self.rgb, bx };
        Some((page, writing))
    }
}

/// A row's label, as the viewer says it.
fn label(cx: &gpui::App, key: &str) -> SharedString {
    Viewer::text(cx, key, &[])
}

impl Render for EditPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.field.is_none() {
            self.field = Some(cx.new(|cx| TextInput::new(window, cx).multi_line(3, 12)));
        }
        let Some(viewer) = self.viewer.upgrade() else {
            return div();
        };
        let Some((tab, view)) = viewer.read(cx).active_tab().map(|tab| (tab.id, tab.view.clone())) else {
            return div();
        };
        let Some(aim) = view.read(cx).aim else {
            return div();
        };
        self.follow((tab, view.read(cx).version, aim), cx);
        let this = cx.entity();
        let buttons = div().flex().gap_2();
        let body = match aim {
            Aim::Object { .. } => {
                let image = matches!(aim, Aim::Object { page, index } if view.read(cx).layers.get(&page).and_then(|l| l.drawn.get(index)).is_some_and(|d| !d.form));
                let (swap, gone) = (viewer.downgrade(), viewer.downgrade());
                buttons
                    .children(image.then(|| {
                        Button::new("pdf-object-replace", label(cx, "pdf.object.replace"))
                            .size(ControlSize::Sm)
                            .on_click(move |_, _, cx| {
                                swap.update(cx, |viewer, cx| viewer.replace_image(cx)).ok();
                            })
                    }))
                    .child(
                        Button::new("pdf-object-delete", label(cx, "pdf.object.delete"))
                            .size(ControlSize::Sm)
                            .variant(ButtonVariant::Danger)
                            .on_click(move |_, _, cx| {
                                gone.update(cx, |viewer, cx| viewer.delete_object(cx)).ok();
                            }),
                    )
                    .into_any_element()
            }
            _ => self.text_rows(&this, &viewer, aim, cx),
        };
        div().p_3().flex().flex_col().gap_3().child(body)
    }
}

impl EditPanel {
    fn text_rows(&self, this: &Entity<Self>, viewer: &Entity<Viewer>, aim: Aim, cx: &mut Context<Self>) -> gpui::AnyElement {
        let set = |this: &Entity<Self>, change: fn(&mut EditPanel, f64)| {
            let this = this.clone();
            move |value: f64, _: &mut Window, cx: &mut gpui::App| this.update(cx, |panel, cx| {
                change(panel, value);
                cx.notify();
            })
        };
        let fonts = FONTS
            .iter()
            .filter(|(value, _)| *value != "original" || matches!(aim, Aim::Text { .. }))
            .map(|(value, key)| Choice::new(*value, label(cx, key)));
        let picked = this.clone();
        let font = Select::new("pdf-text-font", fonts.collect::<Vec<_>>())
            .selected(self.font.clone())
            .label(label(cx, "pdf.text.font"))
            .on_change(move |value, _, cx| picked.update(cx, |panel, cx| {
                (panel.font, panel.note) = (value.clone(), None);
                cx.notify();
            }));
        let size = NumberInput::new("pdf-text-size", f64::from(self.size))
            .range(1.0, 400.0)
            .label(label(cx, "pdf.text.size"))
            .on_commit(set(this, |panel, value| panel.size = value as f32));
        let width = NumberInput::new("pdf-text-width", f64::from(self.width))
            .range(10.0, 2000.0)
            .suffix("pt")
            .label(label(cx, "pdf.text.width"))
            .on_commit(set(this, |panel, value| panel.width = value as f32));
        let swatches = ely_office::panel::COLORS.map(|color| (format!("#{color:06X}"), Hsla::from(rgb(color))));
        let painted = this.clone();
        let mut color = ColorPalette::new("pdf-text-color", swatches.clone()).columns(5).on_change(move |color, _, cx| {
            let at = swatches.iter().position(|(_, swatch)| *swatch == color).expect("the palette hands back its own colors");
            painted.update(cx, |panel, cx| {
                panel.rgb = ely_office::panel::COLORS[at];
                cx.notify();
            })
        });
        if ely_office::panel::COLORS.contains(&self.rgb) {
            color = color.selected(rgb(self.rgb));
        }
        let rows = DescriptionList::new()
            .item(label(cx, "pdf.text.font"), font)
            .item(label(cx, "pdf.text.size"), size)
            .item(label(cx, "pdf.text.color"), color)
            .item(label(cx, "pdf.text.width"), width);
        let (apply, erase) = (viewer.downgrade(), viewer.downgrade());
        let mut buttons = div().flex().gap_2().child(
            Button::new("pdf-text-apply", label(cx, "pdf.text.apply"))
                .size(ControlSize::Sm)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| {
                    apply.update(cx, |viewer, cx| viewer.write_text(false, cx)).ok();
                }),
        );
        if matches!(aim, Aim::Text { .. }) {
            buttons = buttons.child(
                Button::new("pdf-text-erase", label(cx, "pdf.text.erase"))
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Danger)
                    .on_click(move |_, _, cx| {
                        erase.update(cx, |viewer, cx| viewer.write_text(true, cx)).ok();
                    }),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .children(self.field.as_ref().map(Input::new))
            .child(rows)
            .children(self.note.clone().map(|note| div().text_sm().child(note)))
            .child(buttons)
            .into_any_element()
    }
}
