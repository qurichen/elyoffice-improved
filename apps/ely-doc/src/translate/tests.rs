use std::{io::{Read, Write}, net::{TcpListener, TcpStream}, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::Duration};

use super::{Failure, config::{self, Connection}, transport};
use ely_platform::settings::Store;
use serde_json::{Value, json};

mod window;

fn connection(endpoint: String) -> Connection {
    Connection { enabled: true, endpoint, model: "fixture-model".into(), use_key: false }
}

fn answer(text: &str) -> String {
    json!({"choices":[{"finish_reason":"stop", "message":{"role":"assistant","content":text}}]}).to_string()
}

fn request(stream: &mut TcpStream) -> String {
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut bytes = Vec::new();
    let mut header_end = None;
    let mut length = 0;
    loop {
        let mut buf = [0; 4096];
        let n = stream.read(&mut buf).unwrap();
        assert_ne!(n, 0);
        bytes.extend_from_slice(&buf[..n]);
        if header_end.is_none() {
            if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                length = headers.lines().find_map(|s| s.to_lowercase().strip_prefix("content-length:")
                    .map(|s| s.trim().parse::<usize>().unwrap())).unwrap();
                header_end = Some(end + 4);
            }
        }
        if header_end.is_some_and(|end| bytes.len() >= end + length) { break; }
    }
    String::from_utf8(bytes).unwrap()
}

fn serve(status: u16, body: String, extra: String) -> (Connection, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/chat/completions", listener.local_addr().unwrap());
    let job = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let read = request(&mut stream);
        write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}", body.len()).unwrap();
        drop(stream);
        listener.set_nonblocking(true).unwrap();
        thread::sleep(Duration::from_millis(30));
        assert!(listener.accept().is_err(), "no retry or redirect");
        read
    });
    (connection(url), job)
}

#[test]
fn endpoint_policy_refuses_credentials_queries_and_remote_plaintext() {
    for url in ["http://example.com/v1/chat/completions", "http://localhost/v1/chat/completions", "https://user:secret@host/v1", "https://host/v1?key=secret", "https://host/v1#x", "file:///x", "https://host", "https://host/a b", "http://127.0.0.1.evil/v1"] {
        assert_eq!(config::endpoint(url).unwrap_err(), Failure::Configuration, "{url}");
    }
    for url in ["http://127.0.0.1:11434/v1/chat/completions", "http://[::1]:11434/v1/chat/completions", "https://api.example.com/v1/chat/completions"] {
        assert!(config::endpoint(url).is_ok(), "{url}");
    }
}

#[test]
fn settings_default_off_and_roundtrip_preserves_other_fields_without_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::at(dir.path().join("translation.json"));
    assert!(!Connection::load(&store).unwrap().enabled);
    assert!(!store.path().exists());
    store.set("unrelated", json!({"keep":17})).unwrap();
    let config = connection("https://api.example.com/v1/chat/completions".into());
    config.save(&store).unwrap();
    assert_eq!(Connection::load(&store).unwrap(), config);
    let data = store.read().unwrap();
    assert_eq!(data["unrelated"]["keep"], 17);
    let fields = data["translation"].as_object().unwrap();
    assert_eq!(fields.len(), 4);
    assert!(!fields.contains_key("secret"));
    assert!(!fields.contains_key("key"));
}

#[test]
fn malformed_settings_fail_without_overwriting_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::at(dir.path().join("translation.json"));
    for text in ["{broken", "{\"translation\":{\"enabled\":\"true\"}}", "{\"version\":999}"] {
        std::fs::write(store.path(), text).unwrap();
        assert_eq!(Connection::load(&store).unwrap_err(), Failure::Settings);
        assert_eq!(std::fs::read_to_string(store.path()).unwrap(), text);
    }
}

#[test]
fn credential_identity_is_bound_to_profile_and_exact_endpoint() {
    let a = tempfile::tempdir().unwrap(); let b = tempfile::tempdir().unwrap();
    let url = "https://api.example.com/v1/chat/completions";
    let id = config::credential_id(a.path(), url).unwrap();
    assert_eq!(id, config::credential_id(a.path(), url).unwrap());
    assert_ne!(id, config::credential_id(b.path(), url).unwrap());
    assert_ne!(id, config::credential_id(a.path(), "https://else.example/v1/chat/completions").unwrap());
    assert_ne!(id, config::credential_id(a.path(), "https://api.example.com/other").unwrap());
    let invalid = a.path().join("must-not-be-created");
    assert!(config::credential_id(&invalid, "https://user:secret@host/api").is_err());
    assert!(!invalid.exists());
}

#[test]
fn request_sends_exact_selected_text_and_returns_unicode_without_rewriting_it() {
    let selected = "  你好 😀\n00123  ";
    let (config, job) = serve(200, answer("  Hello 😀\n00123  "), String::new());
    let got = transport::translate(&config, selected, "English", None, &AtomicBool::new(false)).unwrap();
    assert_eq!(got, "  Hello 😀\n00123  ");
    let read = job.join().unwrap();
    assert!(read.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
    assert!(!read.to_lowercase().contains("authorization:"));
    let body: Value = serde_json::from_str(read.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["model"], "fixture-model");
    assert_eq!(body["stream"], false); assert_eq!(body["store"], false);
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    assert_eq!(body["messages"][1], json!({"role":"user", "content":selected}));
    assert!(body["messages"][0]["content"].as_str().unwrap().contains("English"));
    assert!(!body.to_string().contains("file_path"));
}

#[test]
fn authorization_is_explicit_and_error_bodies_never_become_messages() {
    let (mut config, job) = serve(401, "SECRET provider echo".into(), String::new());
    config.use_key = true;
    let result = transport::translate(&config, "hello", "Simplified Chinese", Some("fixture-secret"), &AtomicBool::new(false));
    assert_eq!(result, Err(Failure::Http(401)));
    assert!(!format!("{result:?}").contains("SECRET"));
    assert!(job.join().unwrap().to_lowercase().contains("authorization: bearer fixture-secret\r\n"));
}

#[test]
fn disabled_cancelled_empty_oversized_and_invalid_requests_open_no_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut config = connection(format!("http://{}/v1/chat/completions", listener.local_addr().unwrap()));
    let off = AtomicBool::new(false);
    config.enabled = false;
    assert_eq!(transport::translate(&config, "hello", "English", None, &off), Err(Failure::Disabled));
    config.enabled = true;
    assert_eq!(transport::translate(&config, "hello", "English", None, &AtomicBool::new(true)), Err(Failure::Disabled));
    assert_eq!(transport::translate(&config, " \r\n", "English", None, &off), Err(Failure::Empty));
    assert_eq!(transport::translate(&config, &"中".repeat(6000), "English", None, &off), Err(Failure::TooLong));
    assert_eq!(transport::translate(&config, "hello", "invalid", None, &off), Err(Failure::Configuration));
    config.use_key = true;
    for key in [None, Some("x\r\nAuthorization: leak")] {
        assert_eq!(transport::translate(&config, "hello", "English", key, &off), Err(Failure::Credential));
    }
    listener.set_nonblocking(true).unwrap();
    assert!(listener.accept().is_err());
}

#[test]
fn redirects_are_refused_without_forwarding_text_or_authorization() {
    let (config, job) = serve(307, String::new(), "Location: https://else.example/leak\r\n".into());
    assert_eq!(transport::translate(&config, "private text", "English", None, &AtomicBool::new(false)), Err(Failure::Http(307)));
    job.join().unwrap();
}

#[test]
fn malformed_partial_refused_and_tool_responses_are_not_complete_translations() {
    let bad = ["not json".into(), answer(" "),
        answer("hello").replace("\"stop\"", "\"length\""),
        answer("hello").replace("\"assistant\"", "\"tool\""),
        json!({"choices":[{"finish_reason":"stop", "message":{"role":"assistant", "content":null,"refusal":"No"}}]}).to_string(),
        json!({"choices":[{"finish_reason":"stop", "message":{"role":"assistant", "content":"partial","refusal":"No"}}]}).to_string(),
        json!({"choices":[{"finish_reason":"stop", "message":{"role":"assistant", "content":"hello","tool_calls":[]}}]}).to_string(),
        answer(&"x".repeat(65537)), answer("zero\0byte")];
    for body in bad {
        let (config, job) = serve(200, body, String::new());
        assert_eq!(transport::translate(&config, "hello", "English", None, &AtomicBool::new(false)), Err(Failure::Response));
        job.join().unwrap();
    }
}

#[test]
fn a_closed_preview_discards_an_already_sent_response() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let config = connection(format!("http://{}/v1/chat/completions", listener.local_addr().unwrap()));
    let cancelled = Arc::new(AtomicBool::new(false)); let cancel = cancelled.clone();
    let job = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap(); request(&mut stream);
        cancel.store(true, Ordering::SeqCst);
        let body = answer("discarded");
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    assert_eq!(transport::translate(&config, "hello", "English", None, &cancelled), Err(Failure::Disabled));
    job.join().unwrap();
}

#[cfg(windows)]
#[test]
fn isolated_windows_credential_roundtrip_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://fixture.invalid/v1/chat/completions";
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup { fn drop(&mut self) { config::forget_key(&self.0, "https://fixture.invalid/v1/chat/completions").unwrap(); } }
    let _cleanup = Cleanup(dir.path().to_path_buf());
    config::write_key(dir.path(), url, "only-a-test-secret").unwrap();
    assert_eq!(config::read_key(dir.path(), url).unwrap(), "only-a-test-secret");
    assert!(config::read_key(dir.path(), "https://another.invalid/v1/chat/completions").is_err());
    assert!(!dir.path().join("translation.json").exists());
    config::forget_key(dir.path(), url).unwrap();
    assert!(config::read_key(dir.path(), url).is_err());
}
