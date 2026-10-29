//! A worker's stderr, carried into the app's log.

use std::{
    io::{self, BufRead, BufReader, Read},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

const RECORD_LIMIT: usize = 64 * 1024;

fn record(reader: &mut impl BufRead, bytes: &mut Vec<u8>) -> io::Result<Option<bool>> {
    bytes.clear();
    let (mut seen, mut oversized) = (false, false);
    loop {
        let chunk = match reader.fill_buf() {
            Ok(chunk) => chunk,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if chunk.is_empty() { return Ok(seen.then_some(!oversized)); }
        let end = chunk.iter().position(|b| *b == b'\n');
        let length = end.unwrap_or(chunk.len());
        if !oversized {
            if length > RECORD_LIMIT - bytes.len() {
                oversized = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&chunk[..length]);
            }
        }
        reader.consume(length + usize::from(end.is_some()));
        seen = true;
        if end.is_some() { return Ok(Some(!oversized)); }
    }
}

/// The relaying thread, and the worker's last error record.
pub struct Relay {
    last: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl Relay {
    /// After the worker exits: its last error record.
    pub fn last_error(&mut self) -> Option<String> {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.last.lock().expect("the relay lock").clone()
    }
}

/// Logs the worker's own records; engine text stays out.
pub fn relay(stderr: impl Read + Send + 'static, who: &'static str) -> Relay {
    let last = Arc::new(Mutex::new(None));
    let kept = Arc::clone(&last);
    let thread = std::thread::spawn(move || {
        let mut withheld = 0u64;
        let (mut reader, mut bytes) = (BufReader::new(stderr), Vec::with_capacity(RECORD_LIMIT));
        loop {
            match record(&mut reader, &mut bytes) {
                Ok(None) => break,
                Ok(Some(true)) => {}
                Ok(Some(false)) => { withheld += 1; continue; }
                Err(error) => return log::warn!(target: who, "stderr unreadable: {error}"),
            }
            // Engines may print other encodings; keep reading past them.
            let line = String::from_utf8_lossy(&bytes);
            let line = line.trim_end_matches(['\n', '\r']);
            match parse(line) {
                Some((level, target, message)) => {
                    if level == log::Level::Error {
                        *kept.lock().expect("the relay lock") = Some(message.to_string());
                    }
                    log::log!(target: target, level, "{message}")
                }
                // A crash stack names code, never documents.
                None if frame(line) => log::error!(target: who, "{line}"),
                None => {
                    withheld += 1;
                }
            }
        }
        if withheld > 0 { log::debug!(target: who, "native output withheld ({withheld} lines)"); }
    });
    Relay { last, thread: Some(thread) }
}

/// `[time LEVEL target] message`, as env_logger writes it.
fn parse(line: &str) -> Option<(log::Level, &str, &str)> {
    let (head, message) = line.strip_prefix('[')?.split_once("] ")?;
    let mut words = head.split_whitespace();
    let (_, level, target) = (words.next()?, words.next()?, words.next()?);
    Some((level.parse().ok()?, target, message))
}

/// `3   ely-lok-worker   0x0001 symbol + 12` from backtrace_symbols_fd.
fn frame(line: &str) -> bool {
    let mut words = line.split_whitespace();
    words.next().is_some_and(|n| n.parse::<u32>().is_ok())
        && words.next().is_some()
        && words.next().is_some_and(|address| address.starts_with("0x"))
}

#[cfg(test)]
mod tests {
    use super::{frame, parse};

    #[test]
    fn record_storage_is_bounded_and_resumes_at_the_next_newline() {
        use std::io::{BufReader, Cursor, Read as _};
        let prefix: &[u8] = b"private prefix ";
        let body = std::io::repeat(b'x').take(8 * 1024 * 1024);
        let tail: &[u8] = b"[time ERROR private] still the same line\nnext\n";
        let mut reader = BufReader::with_capacity(7, prefix.chain(body).chain(tail));
        let mut bytes = Vec::with_capacity(super::RECORD_LIMIT);
        assert_eq!(super::record(&mut reader, &mut bytes).expect("discarded line"), Some(false));
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), super::RECORD_LIMIT);
        assert_eq!(super::record(&mut reader, &mut bytes).expect("next line"), Some(true));
        assert_eq!(bytes, b"next");
        assert_eq!(super::record(&mut reader, &mut bytes).expect("end"), None);
        let exact = vec![b'x'; super::RECORD_LIMIT];
        for newline in [b"".as_slice(), b"\n"] {
            let mut reader = Cursor::new(exact.clone()).chain(newline);
            assert_eq!(super::record(&mut reader, &mut bytes).expect("exact cap"), Some(true));
            assert_eq!(bytes, exact);
        }
    }

    #[test]
    fn split_utf8_and_interrupted_reads_preserve_a_final_record() {
        use std::io::{self, BufReader, Cursor, Read};
        struct Interrupted { first: bool, input: Cursor<Vec<u8>> }
        impl Read for Interrupted {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if self.first { self.first = false; return Err(io::ErrorKind::Interrupted.into()); }
                self.input.read(buffer)
            }
        }
        let text = "[time ERROR test] 你好\r";
        let input = Interrupted { first: true, input: Cursor::new(text.as_bytes().to_vec()) };
        let mut reader = BufReader::with_capacity(1, input);
        let mut bytes = Vec::with_capacity(super::RECORD_LIMIT);
        assert_eq!(super::record(&mut reader, &mut bytes).expect("final record"), Some(true));
        assert_eq!(String::from_utf8(bytes).expect("utf8"), text);
    }

    #[test]
    fn an_oversized_record_is_discarded_without_retaining_its_body() {
        use std::io::Read as _;
        let prefix: &[u8] = b"[time ERROR ely_lok::worker] ";
        let body = std::io::repeat(b'x').take(8 * 1024 * 1024);
        let mut relay = super::relay(prefix.chain(body), "test");
        assert_eq!(relay.last_error(), None);
    }

    #[test]
    fn an_oversized_line_cannot_hide_the_next_error() {
        use std::io::Read as _;
        let body = std::io::repeat(b'x').take(8 * 1024 * 1024);
        let tail: &[u8] = b"[time ERROR ely_lok::worker] not a new line\n[time ERROR ely_lok::worker] next error\n";
        let mut relay = super::relay(body.chain(tail), "test");
        assert_eq!(relay.last_error().as_deref(), Some("next error"));
    }

    #[test]
    fn a_record_keeps_its_level_and_target() {
        let line = "[2026-10-08T12:00:00Z WARN  ely_lok::worker] request failed";
        assert_eq!(parse(line), Some((log::Level::Warn, "ely_lok::worker", "request failed")));
        assert_eq!(parse("warning: syntax error: unknown keyword: 'Zq'"), None);
    }

    #[test]
    fn a_line_in_another_encoding_stops_nothing() {
        let stderr: &[u8] = b"caf\xe9 opened\n[2026-10-08T12:00:00Z ERROR ely_pdf::worker] it broke\n";
        let mut relay = super::relay(stderr, "test");
        assert_eq!(relay.last_error().as_deref(), Some("it broke"));
    }

    #[test]
    fn stack_frames_pass_and_engine_text_does_not() {
        assert!(frame("3   ely-lok-worker    0x0000000104f3c960 report + 52"));
        assert!(!frame("syntax error: unknown keyword: 'ZqxPrivate42'"));
    }
}
