use super::*;
use gpui::{AppContext as _, Entity, IntoElement, ParentElement as _, Styled as _, TestAppContext, VisualTestContext};
use ely_office::prelude::*;
use crate::{docs::Docs, files::Template, translate::{Translation, TranslateSelection}, tests::{docs, settle, type_into, with_window, written}};

struct Host(Entity<Docs>);
impl gpui::Render for Host {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        gpui::div().size_full().relative().children(self.0.update(cx, |docs, cx| docs.dialog_element(window, cx)))
    }
}

fn frames(shown: &mut VisualTestContext) {
    for _ in 0..3 { shown.update(|window, _| window.refresh()); shown.run_until_parked(); }
}

fn panel(cx: &TestAppContext, docs: &Entity<Docs>) -> Entity<Translation> {
    cx.update(|app| match docs.read(app).dialog.as_ref().unwrap() {
        crate::actions::Dialog::Translation(panel) => panel.clone(), _ => panic!("translation dialog"),
    })
}

#[test]
fn selected_text_preview_sends_only_the_selection_and_copy_keeps_the_document_clean() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("built LOK required");
    cx.update(ely_gpui_component::init_for_tests);
    with_window(&mut cx, &docs, |docs, _, cx| docs.new_from(Template::Blank, cx));
    settle(&cx, |app| docs.read(app).core.active.is_some());
    type_into(&mut cx, &docs, "Hello world\nDO_NOT_SEND_THIS");
    let original = dir.path().join("original.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(docs.core.active.unwrap(), original.clone(), None, window, cx);
    });
    written(&cx, &original, None);
    settle(&cx, |app| !docs.read(app).active_tab().unwrap().dirty(app));
    let before = std::fs::read(&original).unwrap();
    let (worker, doc) = cx.update(|app| {
        let docs = docs.read(app); (docs.core.worker.clone(), docs.active_tab().unwrap().doc.clone())
    });
    {
        let mut worker = worker.lock().unwrap();
        worker.post_key(&doc, 0, 1028 | 8192).unwrap();
        for _ in 0..5 { worker.post_key(&doc, 0, 1027 | 4096).unwrap(); }
        assert_eq!(doc.selection_text(&mut worker).unwrap(), "Hello");
    }
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_translate(&TranslateSelection, window, cx));
    settle(&cx, |app| matches!(docs.read(app).dialog, Some(crate::actions::Dialog::Translation(_))));
    let panel = panel(&cx, &docs);
    cx.update(|app| {
        let panel = panel.read(app);
        assert_eq!(panel.source, "Hello");
        assert!(!panel.enabled);
        assert!(!panel.use_key);
        assert!(panel.secret.read(app).is_masked());
        assert!(panel.output.is_empty());
    });
    let (config, job) = serve(200, answer("你好"), String::new());
    panel.update(&mut cx, |panel, cx| {
        panel.store = Some(Store::at(dir.path().join("translation.json")));
        panel.endpoint.update(cx, |field, cx| field.set_text(config.endpoint, cx));
        panel.model.update(cx, |field, cx| field.set_text(config.model, cx));
        panel.enabled = true;
        cx.notify();
    });
    let window = cx.add_window(|_, _| Host(docs.clone()));
    cx.update(|app| docs.update(app, |docs, _| docs.core.window = Some(window.into())));
    let mut shown = VisualTestContext::from_window(window.into(), &mut cx);
    frames(&mut shown);
    let button = shown.debug_bounds("translation-chinese").expect("rendered translate button");
    shown.simulate_click(button.center(), gpui::Modifiers::none());
    frames(&mut shown);
    settle(&cx, |app| !panel.read(app).busy);
    assert_eq!(cx.update(|app| panel.read(app).output.clone()), "你好");
    let request = job.join().unwrap();
    let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["messages"][1]["content"], "Hello");
    assert!(!request.contains("DO_NOT_SEND_THIS"));
    assert!(body["messages"][0]["content"].as_str().unwrap().contains("Simplified Chinese"));
    frames(&mut shown);
    let copy = shown.debug_bounds("translation-copy").expect("copy button");
    shown.simulate_click(copy.center(), gpui::Modifiers::none());
    assert_eq!(cx.update(|app| app.read_from_clipboard().unwrap().text()), Some("你好".into()));
    cx.update(|app| assert!(!docs.read(app).active_tab().unwrap().dirty(app)));
    assert_eq!(std::fs::read(&original).unwrap(), before);
    assert_eq!(doc.selection_text(&mut worker.lock().unwrap()).unwrap(), "Hello");
    assert!(!dir.path().join("translation.json").exists());
}

#[test]
fn disabling_translation_blocks_a_direct_request_and_corrupt_settings_block_saving() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("built LOK required");
    cx.update(ely_gpui_component::init_for_tests);
    let window = cx.add_window(|_, _| Host(docs.clone()));
    let translation = cx.update_window(window.into(), |_, window, app| {
        app.new(|cx| Translation::new(docs.downgrade(), "Hello".into(), window, cx))
    }).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    translation.update(&mut cx, |panel, cx| {
        panel.store = Some(Store::at(dir.path().join("translation.json")));
        panel.endpoint.update(cx, |field, cx| field.set_text(format!("http://{}/api", listener.local_addr().unwrap()), cx));
        panel.model.update(cx, |field, cx| field.set_text("fixture-model", cx));
        panel.enabled = false;
        panel.start(Some("English"), false, cx);
        assert_eq!(panel.failure, Some(Failure::Disabled));
        assert!(!panel.busy);
        panel.store = None;
        panel.start(None, false, cx);
        assert_eq!(panel.failure, Some(Failure::Settings));
        assert!(!panel.busy);
    });
    listener.set_nonblocking(true).unwrap();
    assert!(listener.accept().is_err());
    assert!(!dir.path().join("translation.json").exists());
}

#[test]
fn closing_the_rendered_dialog_rejects_a_late_response_even_when_the_panel_is_retained() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("built LOK required");
    cx.update(ely_gpui_component::init_for_tests);
    let window = cx.add_window(|_, _| Host(docs.clone()));
    let translation = cx.update_window(window.into(), |_, window, app| {
        app.new(|cx| Translation::new(docs.downgrade(), "Hello".into(), window, cx))
    }).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/chat/completions", listener.local_addr().unwrap());
    let received = Arc::new(AtomicBool::new(false)); let sent = received.clone();
    let (release, wait) = std::sync::mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap(); request(&mut stream);
        sent.store(true, Ordering::SeqCst);
        wait.recv_timeout(Duration::from_secs(10)).unwrap();
        let body = answer("late translation");
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    translation.update(&mut cx, |panel, cx| {
        panel.store = Some(Store::at(dir.path().join("translation.json")));
        panel.endpoint.update(cx, |field, cx| field.set_text(url, cx));
        panel.model.update(cx, |field, cx| field.set_text("fixture-model", cx));
        panel.enabled = true;
    });
    docs.update(&mut cx, |docs, cx| { docs.dialog = Some(crate::actions::Dialog::Translation(translation.clone())); cx.notify(); });
    let mut shown = VisualTestContext::from_window(window.into(), &mut cx);
    frames(&mut shown);
    let go = shown.debug_bounds("translation-english").unwrap();
    shown.simulate_click(go.center(), gpui::Modifiers::none());
    settle(&cx, |_| received.load(Ordering::SeqCst));
    frames(&mut shown);
    let close = shown.debug_bounds("translation-close").unwrap();
    shown.simulate_click(close.center(), gpui::Modifiers::none());
    frames(&mut shown);
    cx.update(|app| assert!(docs.read(app).dialog.is_none()));
    release.send(()).unwrap();
    server.join().unwrap();
    settle(&cx, |app| !translation.read(app).busy);
    cx.update(|app| {
        let panel = translation.read(app);
        assert!(panel.cancelled.load(Ordering::SeqCst));
        assert!(panel.output.is_empty());
        assert_ne!(panel.notice, Some("translation.ready"));
    });
}

#[cfg(windows)]
#[test]
fn editing_the_endpoint_clears_unsaved_and_saved_keys_before_a_different_service_can_receive_them() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let docs = docs(&mut cx, dir.path()).expect("built LOK required");
    cx.update(ely_gpui_component::init_for_tests);
    let window = cx.add_window(|_, _| Host(docs.clone()));
    let translation = cx.update_window(window.into(), |_, window, app| {
        app.new(|cx| Translation::new(docs.downgrade(), "Hello".into(), window, cx))
    }).unwrap();
    let a = "https://only-fixture.invalid/v1/chat/completions";
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let b = format!("http://{}/v1/chat/completions", listener.local_addr().unwrap());
    translation.update(&mut cx, |panel, cx| {
        panel.store = Some(Store::at(dir.path().join("translation.json")));
        panel.endpoint.update(cx, |field, cx| field.set_text(a, cx));
        panel.model.update(cx, |field, cx| field.set_text("fixture-model", cx));
        panel.enabled = true;
        panel.use_key = true;
    });
    cx.run_until_parked();
    translation.update(&mut cx, |panel, cx| {
        panel.secret.update(cx, |field, cx| field.set_text("unsaved-fixture-key", cx));
        panel.endpoint.update(cx, |field, cx| field.set_text(b.clone(), cx));
    });
    cx.run_until_parked();
    cx.update(|app| assert!(translation.read(app).secret.read(app).text().is_empty()));
    translation.update(&mut cx, |panel, cx| panel.endpoint.update(cx, |field, cx| field.set_text(a, cx)));
    cx.run_until_parked();
    translation.update(&mut cx, |panel, cx| {
        panel.secret.update(cx, |field, cx| field.set_text("saved-fixture-key", cx));
        panel.start(None, false, cx);
    });
    settle(&cx, |app| !translation.read(app).busy);
    cx.update(|app| {
        let panel = translation.read(app);
        assert_eq!(panel.notice, Some("translation.saved"));
        assert!(panel.secret.read(app).text().is_empty());
    });
    config::forget_key(dir.path(), a).unwrap();
    translation.update(&mut cx, |panel, cx| panel.endpoint.update(cx, |field, cx| field.set_text(b, cx)));
    cx.run_until_parked();
    translation.update(&mut cx, |panel, cx| panel.start(Some("English"), false, cx));
    settle(&cx, |app| !translation.read(app).busy);
    cx.update(|app| assert_eq!(translation.read(app).failure, Some(Failure::Credential)));
    listener.set_nonblocking(true).unwrap();
    assert!(listener.accept().is_err(), "service B never receives key A");
}
