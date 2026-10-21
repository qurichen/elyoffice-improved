use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::{PropertyGrid, PropertyGroup},
    feedback::EmptyState,
    primitives::IconName,
    theme::ActiveTheme,
};
use gpui::{
    Context, FocusHandle, IntoElement, ParentElement, Render, Styled, WeakEntity, Window, div,
};

use crate::{Gallery, anchor};

pub(crate) struct Props {
    pub(crate) anchor: FocusHandle,
    pub(crate) gallery: WeakEntity<Gallery>,
    pub(crate) rows: Vec<(String, String)>,
    pub(crate) empty: bool,
}

impl Render for Props {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        if self.empty {
            return div()
                .h_full()
                .child(EmptyState::new(
                    "no-selection",
                    IconName::File,
                    ely_ui::i18n::text(cx, "gallery.empty.pages", &[]),
                ))
                .into_any_element();
        }
        let gallery = self.gallery.clone();
        let rename = Button::new(
            "rename-button",
            ely_ui::i18n::text(cx, "gallery.rename", &[]),
        )
        .variant(ButtonVariant::Outline)
        .on_click(move |_, window, cx| {
            if let Err(error) = gallery.update(cx, |this, cx| this.open_rename(window, cx)) {
                log::error!("props: the gallery is gone: {error:#}");
            }
        });
        let gallery = self.gallery.clone();
        let delete = Button::new(
            "delete-button",
            ely_ui::i18n::text(cx, "gallery.delete", &[]),
        )
        .variant(ButtonVariant::Outline)
        .on_click(move |_, _, cx| {
            if let Err(error) = gallery.update(cx, |this, cx| this.open_delete(cx)) {
                log::error!("props: the gallery is gone: {error:#}");
            }
        });
        let group = self.rows.iter().fold(
            PropertyGroup::new(ely_ui::i18n::text(cx, "gallery.group.selection", &[])),
            |group, (name, value)| group.row(name.clone(), value.clone()),
        );
        div()
            .h_full()
            .p_2()
            .flex()
            .flex_col()
            .gap_2()
            .text_color(theme.colors.fg)
            .child(anchor(
                "prop-anchor",
                ely_ui::i18n::text(cx, "gallery.props", &[]),
                &self.anchor,
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(PropertyGrid::new("selection-grid").group(group)),
            )
            .child(div().flex_none().flex().gap_2().child(rename).child(delete))
            .into_any_element()
    }
}
