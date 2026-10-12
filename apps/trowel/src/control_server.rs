//! Loopback JSON-lines control server: one request per line, one reply per line.
//! This is the transport the MCP server drives Trowel through.
//!
//! The server only ever reads requests: a line that is not a JSON object with a string `method`
//! (an HTTP request line from a browser's cross-origin `fetch`, a stray `nc`, binary junk) gets
//! one error reply and the connection is closed, so nothing after it is executed. Lines are
//! bounded ([`MAX_LINE`]) and so is the number of open connections ([`MAX_CONNECTIONS`]); junk
//! never reaches the UI thread. The protocol itself is `docs/control-protocol.md`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use serde_json::{Value, json};
use vectorcraft_ui_egui::ControlRequest;

/// Longest accepted request line (bytes, without the newline). Requests are small JSON objects;
/// anything longer is refused and the connection closed.
pub const MAX_LINE: usize = 4 * 1024 * 1024;
/// Connections served at once; further ones get an error line and are closed.
pub const MAX_CONNECTIONS: usize = 16;
/// How long one request may take before the peer is answered with a timeout.
const REPLY_TIMEOUT: Duration = Duration::from_secs(60);

/// Start the control server on `port`; requests arrive on the receiver the app drains each frame.
/// A port that cannot be bound is reported and the app carries on without the channel.
pub fn start(port: u16, ctx: egui::Context) -> Receiver<ControlRequest> {
    let (tx, rx) = channel::<ControlRequest>();
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("trowel: control server failed to bind 127.0.0.1:{port}: {e}");
            return rx;
        }
    };
    eprintln!("trowel: control server listening on 127.0.0.1:{port}");
    std::thread::spawn(move || accept_loop(listener, tx, ctx));
    rx
}

fn accept_loop(listener: TcpListener, tx: Sender<ControlRequest>, ctx: egui::Context) {
    let open = Arc::new(AtomicUsize::new(0));
    for mut stream in listener.incoming().flatten() {
        if open.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            open.fetch_sub(1, Ordering::SeqCst);
            // Best effort: the connection is dropped either way, and a peer that
            // can't read the reason loses nothing.
            let _ = writeln!(stream, "{}", json!({"ok": false, "error": "too many control connections"}));
            continue;
        }
        let tx = tx.clone();
        let ctx = ctx.clone();
        let open = Arc::clone(&open);
        std::thread::spawn(move || {
            serve(stream, &tx, &ctx);
            open.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

fn serve(stream: TcpStream, tx: &Sender<ControlRequest>, ctx: &egui::Context) {
    let Ok(read) = stream.try_clone() else { return };
    serve_lines(read, stream, |method, params| {
        let (req, rrx) = ControlRequest::new(method, params);
        tx.send(req).ok()?;
        ctx.request_repaint();
        Some(rrx.recv_timeout(REPLY_TIMEOUT).unwrap_or_else(|_| json!({"ok": false, "error": "timeout"})))
    });
}

/// One parsed line.
enum Line {
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    Blank,
    /// Not a request: reply with this error and close the connection.
    Reject(String),
}

fn parse_line(bytes: &[u8]) -> Line {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Line::Reject("not a control request (invalid UTF-8); closing the connection".into());
    };
    let text = text.trim();
    if text.is_empty() {
        return Line::Blank;
    }
    let msg = match serde_json::from_str::<Value>(text) {
        Ok(v) => v,
        Err(e) => {
            return Line::Reject(format!("bad JSON ({e}); expected one JSON request object per line; closing the connection"));
        }
    };
    let Some(obj) = msg.as_object() else {
        return Line::Reject("not a control request (expected a JSON object); closing the connection".into());
    };
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        return Line::Reject("not a control request (missing string `method`); closing the connection".into());
    };
    Line::Request {
        id: obj.get("id").cloned().unwrap_or(Value::Null),
        method: method.to_string(),
        params: obj.get("params").cloned().unwrap_or(json!({})),
    }
}

/// Read one `\n`-terminated line of at most `max` bytes into `buf` (newline stripped).
/// `Ok(false)` at end of stream; `Err` when the line is too long or the read fails.
fn read_bounded_line(reader: &mut impl BufRead, buf: &mut Vec<u8>, max: usize) -> Result<bool, String> {
    buf.clear();
    // `take` bounds the bytes buffered for one line: a peer can't make us allocate more than `max`.
    let limit = u64::try_from(max).unwrap_or(u64::MAX).saturating_add(1);
    let n = reader.take(limit).read_until(b'\n', buf).map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(false);
    }
    if buf.last() == Some(&b'\n') {
        buf.pop();
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        return Ok(true);
    }
    if buf.len() > max {
        return Err(format!("request line longer than {max} bytes; closing the connection"));
    }
    // Last line without a newline, then end of stream.
    Ok(true)
}

/// Serve requests from `read`, answering on `out`, until the peer closes, sends something that is
/// not a request, or `handle` returns `None` (app gone). `handle` gets `(method, params)` and
/// returns the reply object (the request's `id` is added here).
fn serve_lines<R: Read, W: Write>(read: R, mut out: W, mut handle: impl FnMut(String, Value) -> Option<Value>) {
    let mut reader = BufReader::new(read);
    let mut buf = Vec::new();
    loop {
        let reject = match read_bounded_line(&mut reader, &mut buf, MAX_LINE) {
            Ok(false) => return,
            Err(e) => e,
            Ok(true) => match parse_line(&buf) {
                Line::Blank => continue,
                Line::Reject(e) => e,
                Line::Request { id, method, params } => {
                    let Some(mut r) = handle(method, params) else { return };
                    if let Some(o) = r.as_object_mut() {
                        o.insert("id".into(), id);
                    }
                    if writeln!(out, "{r}").and_then(|()| out.flush()).is_err() {
                        return;
                    }
                    continue;
                }
            },
        };
        let _ = writeln!(out, "{}", json!({"ok": false, "error": reject}));
        let _ = out.flush();
        return;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn rejected(bytes: &[u8]) -> String {
        match parse_line(bytes) {
            Line::Reject(e) => e,
            _ => panic!("expected a rejection"),
        }
    }

    #[test]
    fn a_request_carries_its_id_method_and_params() {
        let Line::Request { id, method, params } = parse_line(br#"{"id":7,"method":"ui.inspect","params":{"a":1}}"#) else {
            panic!("expected a request");
        };
        assert_eq!(id, json!(7));
        assert_eq!(method, "ui.inspect");
        assert_eq!(params, json!({"a": 1}));
    }

    #[test]
    fn params_default_to_an_empty_object() {
        let Line::Request { params, .. } = parse_line(br#"{"method":"ui.inspect"}"#) else {
            panic!("expected a request");
        };
        assert_eq!(params, json!({}));
    }

    #[test]
    fn blank_lines_are_skipped_not_rejected() {
        assert!(matches!(parse_line(b"   "), Line::Blank));
    }

    #[test]
    fn junk_is_rejected_with_a_reason() {
        assert!(rejected(b"GET / HTTP/1.1").contains("bad JSON"));
        assert!(rejected(b"[]").contains("expected a JSON object"));
        assert!(rejected(br#"{"params":{}}"#).contains("missing string `method`"));
        assert!(rejected(&[0xff, 0xfe]).contains("invalid UTF-8"));
    }

    /// A peer cannot make the server buffer more than `max` bytes for one line.
    #[test]
    fn an_over_long_line_is_refused() {
        let long = format!("{{\"method\":\"{}\"}}\n", "x".repeat(64));
        let mut reader = BufReader::new(long.as_bytes());
        let mut buf = Vec::new();
        assert!(read_bounded_line(&mut reader, &mut buf, 16).is_err());
    }

    #[test]
    fn lines_are_read_whole_and_end_at_the_stream() {
        let mut reader = BufReader::new(&b"one\ntwo\r\nthree"[..]);
        let mut buf = Vec::new();
        assert_eq!(read_bounded_line(&mut reader, &mut buf, MAX_LINE), Ok(true));
        assert_eq!(buf, b"one");
        assert_eq!(read_bounded_line(&mut reader, &mut buf, MAX_LINE), Ok(true));
        assert_eq!(buf, b"two");
        assert_eq!(read_bounded_line(&mut reader, &mut buf, MAX_LINE), Ok(true));
        assert_eq!(buf, b"three");
        assert_eq!(read_bounded_line(&mut reader, &mut buf, MAX_LINE), Ok(false));
    }

    /// Two requests on one connection are both answered, each with its own id; junk
    /// after them gets one error and closes the connection without running anything.
    #[test]
    fn the_connection_answers_each_request_with_its_id() {
        let input = "{\"id\":1,\"method\":\"a\"}\n{\"id\":2,\"method\":\"b\"}\n";
        let mut out: Vec<u8> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        serve_lines(input.as_bytes(), &mut out, |method, _| {
            seen.push(method);
            Some(json!({"ok": true}))
        });
        let text = String::from_utf8(out).unwrap();
        assert_eq!(seen, vec!["a".to_string(), "b".to_string()]);
        assert!(text.lines().any(|l| l.contains("\"id\":1")));
        assert!(text.lines().any(|l| l.contains("\"id\":2")));
    }

    /// Junk is answered once and the connection closes: nothing after it runs.
    #[test]
    fn junk_ends_the_connection() {
        let input = "GET / HTTP/1.1\n{\"id\":2,\"method\":\"a\"}\n";
        let mut out: Vec<u8> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        serve_lines(input.as_bytes(), &mut out, |method, _| {
            seen.push(method);
            Some(json!({"ok": true}))
        });
        let text = String::from_utf8(out).unwrap();
        assert!(seen.is_empty(), "nothing after the junk may run: {seen:?}");
        assert!(text.contains("\"ok\":false"));
        assert_eq!(text.lines().count(), 1);
    }
}
