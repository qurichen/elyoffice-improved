use ely_gpui_component::{
    buttons::Button, forms::{Input, Switch}, overlays::Dialog, theme::{ActiveTheme as _, TextSize},
};
use gpui::{App, ClipboardItem, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, Window, div};

use super::Translation;
use crate::docs::Docs;

impl Translation {
    pub(super) fn busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        for field in [&self.endpoint, &self.model, &self.secret] {
            field.update(cx, |field, cx| field.set_disabled(busy, cx));
        }
        cx.notify();
    }
}

impl Render for Translation {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text = |key| Docs::text(cx, key, &[]);
        let this = cx.entity();
        let owner = self.owner.clone();
        let enabled = Switch::new("translation-enabled", self.enabled)
            .label(text("translation.enable")).disabled(self.busy || self.store.is_none())
            .on_change({let this = this.clone(); move |on, _, cx| this.update(cx, |panel, cx| {
                if !panel.busy && panel.store.is_some() { panel.enabled = on; panel.output.clear(); cx.notify(); }
            })});
        let use_key = Switch::new("translation-use-key", self.use_key)
            .label(text("translation.use_key")).disabled(self.busy)
            .on_change({let this = this.clone(); move |on, _, cx| this.update(cx, |panel, cx| {
                if !panel.busy { panel.use_key = on; cx.notify(); }
            })});
        let fields = div().flex().flex_col().gap_2()
            .child(enabled)
            .child(text("translation.endpoint")).child(Input::new(&self.endpoint))
            .child(text("translation.model")).child(Input::new(&self.model))
            .child(use_key)
            .children(self.use_key.then(|| div().flex().flex_col().gap_1()
                .child(text("translation.key")).child(Input::new(&self.secret))
                .child(text("translation.key_hint"))));
        let mut row = div().flex().flex_wrap().gap_2();
        for (id, key, target) in [("translation-english", "translation.english", "English"),
            ("translation-chinese", "translation.chinese", "Simplified Chinese")]
        {
            let this = this.clone();
            row = row.child(div().debug_selector(move || id.into())
                .child(Button::new(id, text(key)).disabled(self.busy || !self.enabled || self.store.is_none())
                .on_click(move |_, _, cx| this.update(cx, |panel, cx| panel.start(Some(target), false, cx)))));
        }
        let copy = this.clone();
        row = row.child(div().debug_selector(|| "translation-copy".into()).child(Button::new("translation-copy", text("translation.copy"))
            .disabled(self.busy || self.output.is_empty())
            .on_click(move |_, _, cx| copy.update(cx, |panel, cx| {
                if !panel.busy && !panel.output.is_empty() && !panel.cancelled.load(super::Ordering::SeqCst) {
                    cx.write_to_clipboard(ClipboardItem::new_string(panel.output.clone()));
                    panel.notice = Some("translation.copied");
                    cx.notify();
                }
            }))));
        let message = self.failure.map(|failure| {
            let code = match failure { super::Failure::Http(n) => n.to_string(), _ => String::new() };
            Docs::text(cx, failure.key(), &[("code", &code)])
        }).or_else(|| self.notice.map(text));
        let body = div().id("translation-body").flex().flex_col().gap_3()
            .text_color(cx.theme().colors.fg).text_size(cx.theme().text_size(TextSize::Sm))
            .child(fields)
            .child(text("translation.source"))
            .child(div().id("translation-source").max_h_24().overflow_y_scroll().child(self.source.clone()))
            .child(text("translation.disclosure"))
            .child(row)
            .children(self.busy.then(|| text("translation.busy")))
            .children(message)
            .children((!self.output.is_empty()).then(|| div().flex().flex_col().gap_1()
                .child(text("translation.output"))
                .child(div().id("translation-output").max_h_32().overflow_y_scroll().child(self.output.clone()))));
        let save = this.clone();
        let forget = this.clone();
        let can_save = !self.busy && self.store.is_some();
        let can_forget = can_save && self.use_key;
        let cancelled = self.cancelled.clone();
        let id = this.entity_id();
        let (save_text, forget_text, close_text) = (text("translation.save"), text("translation.forget"), text("translation.close"));
        Dialog::new("translation-dialog", text("translation.title"), move |_, cx: &mut App| {
            cancelled.store(true, super::Ordering::SeqCst);
            let _ = owner.update(cx, |docs, cx| {
                if matches!(docs.dialog.as_ref(), Some(crate::actions::Dialog::Translation(panel)) if panel.entity_id() == id) {
                    docs.dialog = None;
                    cx.notify();
                }
            });
        })
            .child(body)
            .action(move |_| Button::new("translation-save", save_text).disabled(!can_save)
                .on_click(move |_, _, cx| save.update(cx, |panel, cx| panel.start(None, false, cx))))
            .action(move |_| Button::new("translation-forget", forget_text).disabled(!can_forget)
                .on_click(move |_, _, cx| forget.update(cx, |panel, cx| panel.start(None, true, cx))))
            .action(move |close| div().debug_selector(|| "translation-close".into())
                .child(Button::new("translation-close", close_text).on_click(move |_, window, cx| close(window, cx))))
    }
}
