use ely_gpui_component::{
    forms::TextInput,
    overlays::{ConfirmDialog, PromptDialog},
};
use gpui::{Context, Entity, IntoElement, SharedString};

use crate::Gallery;

pub(crate) enum Overlay {
    Rename { field: Entity<TextInput> },
    Delete,
}

fn rename_check(empty: &str, failed: &str, text: &str) -> Result<(), SharedString> {
    match text.trim() {
        "" => Err(empty.into()),
        "fail" => Err(failed.into()),
        _ => Ok(()),
    }
}

pub(crate) fn rename_overlay(
    field: Entity<TextInput>,
    cx: &mut Context<Gallery>,
) -> gpui::AnyElement {
    let this = cx.entity();
    PromptDialog::new(
        "rename-dialog",
        ely_ui::i18n::text(cx, "gallery.rename.title", &[]),
        &field,
        {
            let this = this.clone();
            move |_, cx| this.update(cx, |this, cx| this.close_dialog(cx))
        },
    )
    .submit(ely_ui::i18n::text(cx, "gallery.rename.submit", &[]))
    .check({
        let empty = ely_ui::i18n::text(cx, "gallery.rename.empty", &[]);
        let failed = ely_ui::i18n::text(cx, "gallery.rename.failed", &[]);
        move |text| rename_check(&empty, &failed, text)
    })
    .on_submit(move |text, _, cx| {
        this.update(cx, |this, cx| this.rename(text, cx));
    })
    .into_any_element()
}

pub(crate) fn delete_overlay(title: SharedString, cx: &mut Context<Gallery>) -> gpui::AnyElement {
    let this = cx.entity();
    ConfirmDialog::new(
        "delete-dialog",
        ely_ui::i18n::text(cx, "gallery.delete.title", &[("title", &title)]),
        ely_ui::i18n::text(cx, "gallery.delete.message", &[("title", &title)]),
        {
            let this = this.clone();
            move |_, cx| this.update(cx, |this, cx| this.close_dialog(cx))
        },
    )
    .confirm(ely_ui::i18n::text(cx, "gallery.delete.confirm", &[]))
    .destructive()
    .on_confirm(move |_, cx| {
        this.update(cx, |this, cx| this.delete_confirmed(cx));
    })
    .into_any_element()
}
