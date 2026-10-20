#![allow(dead_code)]
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cityloom_prepare::progress::testing::SharedBuf;
use cityloom_prepare::progress::{Progress, Spec};

/// A script named `name` in `dir` that appends its arguments, one invocation per line, to `dir/<name>.log` and then
/// runs `body` (shell, `$LOG` is that file). Returns the directory to put on a command line as the program's path.
pub fn fake_tool(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    let log = dir.join(format!("{name}.log"));
    fs::write(&path, format!("#!/bin/sh\nLOG='{}'\necho \"$@\" >> \"$LOG\"\n{body}\n", log.display())).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

pub fn calls(dir: &Path, name: &str) -> Vec<String> {
    fs::read_to_string(dir.join(format!("{name}.log"))).unwrap_or_default().lines().map(str::to_string).collect()
}

/// A `Progress` that writes plain lines to a shared buffer and keeps its logs under `dir/logs`.
pub fn quiet_progress(specs: Vec<Spec>, dir: &Path) -> (Progress, SharedBuf) {
    let sink = SharedBuf::default();
    let clock = Arc::new(|| 0.0);
    let progress = Progress::with(specs, dir.join("logs"), dir.to_path_buf(), Box::new(sink.clone()), false, clock);
    (progress, sink)
}

/// What a test server was asked.
pub struct Request {
    pub path: String,
    /// `N` of a `Range: bytes=N-` header.
    pub range_from: Option<u64>,
    /// The `Host` header, `127.0.0.1:port`: what a test writes an absolute address from.
    pub host: String,
    /// The `Accept-Encoding` header.
    pub accept_encoding: Option<String>,
}

pub enum Reply {
    /// `content_length` is what the header says; it may be larger than the body sent, to make a truncated answer.
    Body {
        status: u16,
        body: Vec<u8>,
        content_length: Option<usize>,
    },
    /// A 302 to this address, which may be relative.
    Redirect(String),
    Status(u16),
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let path = line.split_whitespace().nth(1)?.to_string();
    let (mut range_from, mut host, mut accept_encoding) = (None, String::new(), None);
    loop {
        line.clear();
        reader.read_line(&mut line).ok()?;
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            let value = value.trim();
            match name.to_ascii_lowercase().as_str() {
                "range" => range_from = value.strip_prefix("bytes=").and_then(|v| v.strip_suffix('-')).and_then(|v| v.parse().ok()),
                "host" => host = value.to_string(),
                "accept-encoding" => accept_encoding = Some(value.to_string()),
                _ => {}
            }
        }
    }
    Some(Request { path, range_from, host, accept_encoding })
}

fn answer(mut stream: TcpStream, reply: Reply) {
    let head = |status: u16| {
        let reason = match status {
            200 => "OK",
            206 => "Partial Content",
            302 => "Found",
            404 => "Not Found",
            416 => "Range Not Satisfiable",
            _ => "Other",
        };
        format!("HTTP/1.1 {status} {reason}\r\n")
    };
    let _ = match reply {
        Reply::Body { status, body, content_length } => {
            let mut out = head(status);
            if let Some(length) = content_length {
                out.push_str(&format!("Content-Length: {length}\r\n"));
            }
            out.push_str("Connection: close\r\n\r\n");
            stream.write_all(out.as_bytes()).and_then(|()| stream.write_all(&body))
        }
        Reply::Redirect(to) => stream.write_all(format!("{}Location: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", head(302)).as_bytes()),
        Reply::Status(status) => stream.write_all(format!("{}Content-Length: 0\r\nConnection: close\r\n\r\n", head(status)).as_bytes()),
    };
    let _ = stream.flush();
}

/// A one-thread-per-connection HTTP server on 127.0.0.1 for the download tests. `handler(request)` answers each
/// request. Returns the address (`http://127.0.0.1:port`) and the accepting thread, which runs until the test ends.
pub fn serve(handler: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let handler = Arc::new(handler);
    let thread = std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let handler = Arc::clone(&handler);
            std::thread::spawn(move || {
                if let Some(request) = read_request(&stream) {
                    answer(stream, handler(&request));
                }
            });
        }
    });
    (address, thread)
}
