mod config;
mod dialog;
mod transport;
#[cfg(test)]
mod tests;

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use ely_gpui_component::{forms::TextInput, primitives::Severity};
use ely_office::prelude::*;
use ely_platform::settings::Store;
use gpui::{App, AppContext as _, Context, Entity, Menu, MenuItem, WeakEntity, Window, actions};

use crate::{actions::Dialog, docs::Docs};
use config::Connection;

actions!(doc, [TranslateSelection]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Failure {
    Disabled, Configuration, Empty, TooLong, Credential, Settings, Network, Http(u16), Response,
}

impl Failure {
    fn key(self) -> &'static str {
        match self {
            Self::Disabled => "translation.disabled", Self::Configuration => "translation.configuration",
            Self::Empty => "translation.empty", Self::TooLong => "translation.too_long",
            Self::Credential => "translation.credential", Self::Settings => "translation.settings",
            Self::Network => "translation.network", Self::Http(_) => "translation.http",
            Self::Response => "translation.response",
        }
    }
}

pub(super) struct Translation {
    owner: WeakEntity<Docs>,
    source: String,
    output: String,
    endpoint: Entity<TextInput>,
    model: Entity<TextInput>,
    secret: Entity<TextInput>,
    enabled: bool,
    use_key: bool,
    store: Option<Store>,
    failure: Option<Failure>,
    notice: Option<&'static str>,
    busy: bool,
    cancelled: Arc<AtomicBool>,
}

impl Drop for Translation {
    fn drop(&mut self) { self.cancelled.store(true, Ordering::SeqCst); }
}

impl Translation {
    fn new(owner: WeakEntity<Docs>, source: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let loaded = config::store().map_err(|_| Failure::Settings)
            .and_then(|store| Connection::load(&store).map(|c| (store, c)));
        let failure = loaded.as_ref().err().copied();
        let (store, config) = match loaded { Ok((s, c)) => (Some(s), c), Err(_) => (None, Connection::default()) };
        let endpoint = cx.new(|cx| TextInput::new(window, cx).label(Docs::text(cx, "translation.endpoint", &[])).max_len(2048));
        let model = cx.new(|cx| TextInput::new(window, cx).label(Docs::text(cx, "translation.model", &[])).max_len(256));
        let secret = cx.new(|cx| TextInput::new(window, cx).label(Docs::text(cx, "translation.key", &[])).masked().max_len(8192));
        endpoint.update(cx, |field, cx| field.set_text(config.endpoint.clone(), cx));
        model.update(cx, |field, cx| field.set_text(config.model.clone(), cx));
        cx.subscribe(&endpoint, |panel, _, event, cx| {
            if matches!(event, ely_gpui_component::forms::InputEvent::Changed) {
                panel.secret.update(cx, |field, cx| field.set_text("", cx));
                panel.output.clear();
                cx.notify();
            }
        }).detach();
        Self { owner, source, output: String::new(), endpoint, model, secret,
            enabled: config.enabled, use_key: config.use_key, store, failure, notice: None,
            busy: false, cancelled: Arc::new(AtomicBool::new(false)) }
    }

    fn connection(&self, cx: &App) -> Result<Connection, Failure> {
        if self.store.is_none() { return Err(Failure::Settings); }
        let config = Connection { enabled: self.enabled, use_key: self.use_key,
            endpoint: self.endpoint.read(cx).text().to_string(), model: self.model.read(cx).text().to_string() };
        config.validate()?;
        Ok(config)
    }

    fn start(&mut self, target: Option<&'static str>, forget: bool, cx: &mut Context<Self>) {
        if self.busy || self.cancelled.load(Ordering::SeqCst) { return; }
        self.failure = None;
        self.notice = None;
        let config = match self.connection(cx) { Ok(c) => c, Err(e) => { self.failure = Some(e); cx.notify(); return; } };
        if target.is_some() {
            self.output.clear();
            if !config.enabled { self.failure = Some(Failure::Disabled); cx.notify(); return; }
            if let Err(e) = transport::source(&self.source) { self.failure = Some(e); cx.notify(); return; }
        }
        let store = self.store.clone().expect("validated settings");
        let dir = store.path().parent().expect("settings folder").to_path_buf();
        let secret = self.secret.read(cx).text().to_string();
        let source = self.source.clone();
        let cancelled = self.cancelled.clone();
        self.busy(true, cx);
        log::info!("translation: {} started", if target.is_some() { "request" } else { "configuration" });
        cx.notify();
        let (send, receive) = futures::channel::oneshot::channel();
        let started = std::thread::Builder::new().name("ely-translation".into()).spawn(move || {
            let result = (|| {
                if cancelled.load(Ordering::SeqCst) { return Err(Failure::Disabled); }
                if let Some(target) = target {
                    let saved;
                    let key = if config.use_key {
                        saved = if secret.is_empty() { config::read_key(&dir, &config.endpoint)? } else { secret };
                        Some(saved.as_str())
                    } else { None };
                    transport::translate(&config, &source, target, key, &cancelled).map(Some)
                } else if forget {
                    config::forget_key(&dir, &config.endpoint)?;
                    Ok(None)
                } else {
                    if config.use_key && !secret.is_empty() { config::write_key(&dir, &config.endpoint, &secret)?; }
                    config.save(&store)?;
                    Ok(None)
                }
            })();
            let _ = send.send(result);
        });
        if started.is_err() {
            self.busy(false, cx);
            self.failure = Some(Failure::Network);
            return;
        }
        cx.spawn(async move |this, cx| {
            let result = receive.await.unwrap_or(Err(Failure::Network));
            let _ = this.update(cx, |panel, cx| {
                panel.busy(false, cx);
                if panel.cancelled.load(Ordering::SeqCst) { return; }
                match result {
                    Ok(Some(output)) => { panel.output = output; panel.notice = Some("translation.ready"); }
                    Ok(None) => {
                        panel.secret.update(cx, |field, cx| field.set_text("", cx));
                        panel.notice = Some(if forget { "translation.forgotten" } else { "translation.saved" });
                    }
                    Err(failure) => panel.failure = Some(failure),
                }
                log::info!("translation: operation finished, success={}", panel.failure.is_none());
                cx.notify();
            });
        }).detach();
    }
}

pub fn menu(cx: &App) -> Menu {
    Menu::new(Docs::text(cx, "translation.menu", &[]))
        .items([MenuItem::action(Docs::text(cx, "translation.title", &[]), TranslateSelection)])
}

impl Docs {
    pub fn on_translate(&mut self, _: &TranslateSelection, _: &mut Window, cx: &mut Context<Self>) {
        if self.dialog.is_some() || self.core.dialog.is_some() { return; }
        let Some(id) = self.core.active else { return; };
        self.dialog_asked += 1;
        let asked = self.dialog_asked;
        self.ask_in(id, |worker, doc| doc.selection_text(worker), move |docs, read, window, cx| {
            if docs.dialog_asked != asked || docs.dialog.is_some() || docs.core.dialog.is_some() { return; }
            match read {
                Ok(source) => {
                    if let Err(failure) = transport::source(&source) {
                        let text = Docs::text(cx, failure.key(), &[]);
                        return docs.toast(text, Severity::Warning, cx);
                    }
                    let owner = cx.weak_entity();
                    let panel = cx.new(|cx| Translation::new(owner, source, window, cx));
                    docs.dialog = Some(Dialog::Translation(panel));
                    cx.notify();
                }
                Err(error) => docs.engine_failed(&error.to_string(), cx),
            }
        }, cx);
    }
}
