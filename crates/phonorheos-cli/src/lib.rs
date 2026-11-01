// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `phonorheos` command: `phonorheos say "…"` sends text to the running app and
//! waits until it has been spoken. A plain HTTP/1.1 client, since the command needs
//! nothing more, over the app's Unix socket on macOS and Linux and over 127.0.0.1 on
//! Windows, whose Rust standard library has no Unix socket.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::ExitCode;

/// Where the command reaches the running app.
#[derive(Debug, Clone)]
pub enum Endpoint {
    /// The app's Unix socket (macOS and Linux).
    #[cfg(unix)]
    Socket(std::path::PathBuf),
    /// The app's loopback listener (Windows).
    Loopback(SocketAddr),
}

trait Stream: Read + Write {}
impl<T: Read + Write> Stream for T {}

impl Endpoint {
    /// The socket on macOS and Linux, `127.0.0.1` at phonorheos's port on Windows.
    pub fn default_for_this_system() -> Self {
        #[cfg(unix)]
        {
            Self::Socket(phonorheos_core::paths::socket_path())
        }
        #[cfg(not(unix))]
        {
            Self::Loopback(SocketAddr::from((
                std::net::Ipv4Addr::LOCALHOST,
                phonorheos_core::PORT,
            )))
        }
    }

    /// The `Host` the app's access rule wants: anything on the socket, the loopback
    /// address and port on TCP.
    fn host(&self) -> String {
        match self {
            #[cfg(unix)]
            Self::Socket(_) => "localhost".into(),
            Self::Loopback(addr) => addr.to_string(),
        }
    }

    fn connect(&self) -> Result<Box<dyn Stream>, String> {
        match self {
            #[cfg(unix)]
            Self::Socket(path) => std::os::unix::net::UnixStream::connect(path)
                .map(|s| Box::new(s) as Box<dyn Stream>)
                .map_err(|e| format!("phonorheos is not running ({}: {e})", path.display())),
            Self::Loopback(addr) => TcpStream::connect(addr)
                .map(|s| Box::new(s) as Box<dyn Stream>)
                .map_err(|e| format!("phonorheos is not running ({addr}: {e})")),
        }
    }
}

const USAGE: &str = "usage:
  phonorheos say [--voice <voice>] [--speed <0.5..2>] [--by-clause | --whole] [--cache] [--silent]
                 [--no-wait] <text>…
    --cache   keep the speech, so that the same text plays at once next time
    --silent  with --cache, make the speech without playing it
  phonorheos stop
  phonorheos status
  phonorheos --version";

/// Runs the command with its arguments (without the program name).
pub fn run(args: &[String]) -> ExitCode {
    let socket = Endpoint::default_for_this_system();
    match args.first().map(String::as_str) {
        Some("--version") => {
            println!("phonorheos {}", phonorheos_core::version());
            ExitCode::SUCCESS
        }
        Some("say") => report(say(&socket, &args[1..])),
        Some("stop") => report(
            request(&socket, "POST", "/stop", Some("{}")).map(|(_, body)| println!("{body}")),
        ),
        Some("status") => {
            report(request(&socket, "GET", "/status", None).map(|(_, body)| println!("{body}")))
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn report(r: Result<(), String>) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("phonorheos: {e}");
            ExitCode::FAILURE
        }
    }
}

struct Say {
    text: String,
    voice: Option<String>,
    speed: Option<f32>,
    by_clause: Option<bool>,
    cache: bool,
    silent: bool,
    wait: bool,
}

fn parse_say(args: &[String]) -> Result<Say, String> {
    let mut s = Say {
        text: String::new(),
        voice: None,
        speed: None,
        by_clause: None,
        cache: false,
        silent: false,
        wait: true,
    };
    let mut words = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--voice" => s.voice = Some(it.next().ok_or("--voice needs a voice")?.clone()),
            "--speed" => {
                let v = it.next().ok_or("--speed needs a number")?;
                s.speed = Some(v.parse().map_err(|_| format!("not a speed: {v}"))?);
            }
            "--by-clause" => s.by_clause = Some(true),
            "--whole" => s.by_clause = Some(false),
            "--cache" => s.cache = true,
            "--silent" => s.silent = true,
            "--no-wait" => s.wait = false,
            "--" => words.extend(it.by_ref().cloned()),
            _ => words.push(a.clone()),
        }
    }
    s.text = words.join(" ");
    if s.text.trim().is_empty() {
        return Err(format!("nothing to say\n{USAGE}"));
    }
    if s.silent && !s.cache {
        return Err("--silent needs --cache, or it does nothing".into());
    }
    Ok(s)
}

fn say(socket: &Endpoint, args: &[String]) -> Result<(), String> {
    let s = parse_say(args)?;
    let mut body = serde_json::json!({ "text": s.text, "client": "phonorheos say" });
    if let Some(v) = s.voice {
        body["voice"] = v.into();
    }
    if let Some(sp) = s.speed {
        body["speed"] = sp.into();
    }
    if let Some(b) = s.by_clause {
        body["by_clause"] = b.into();
    }
    if s.cache {
        body["cache"] = true.into();
    }
    if s.silent {
        body["speak"] = false.into();
    }
    let (code, reply) = request(socket, "POST", "/speak", Some(&body.to_string()))?;
    let reply: serde_json::Value =
        serde_json::from_str(&reply).map_err(|_| format!("unexpected reply: {reply}"))?;
    if code != 201 {
        return Err(reply["error"].as_str().unwrap_or("refused").to_string());
    }
    if !s.wait {
        return Ok(());
    }
    let id = reply["id"].as_str().ok_or("no id in the reply")?;
    let after = reply["event"].as_u64().ok_or("no event in the reply")?;
    wait_for_end(socket, id, after)
}

/// Reads the event stream from just after the item's `queued` event until its end.
fn wait_for_end(socket: &Endpoint, id: &str, after: u64) -> Result<(), String> {
    let mut stream = socket.connect()?;
    let head = format!(
        "GET /events?item={id}&last_id={after} HTTP/1.1\r\nHost: {}\r\nUser-Agent: phonorheos say\r\nAccept: text/event-stream\r\n\r\n",
        socket.host()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    let reader = BufReader::new(stream);
    let mut event = String::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if let Some(name) = line.strip_prefix("event: ") {
            event = name.trim().to_string();
        } else if let Some(data) = line.strip_prefix("data: ") {
            match event.as_str() {
                "finished" => return Ok(()),
                "cancelled" => return Err("cancelled before it was spoken to the end".into()),
                "failed" => {
                    let v: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
                    return Err(format!(
                        "failed: {}",
                        v["reason"].as_str().unwrap_or("unknown")
                    ));
                }
                _ => {}
            }
        }
    }
    Err("the app closed the stream before the text was spoken".into())
}

/// One request, read to its end; returns the status code and the body.
fn request(
    socket: &Endpoint,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let mut stream = socket.connect()?;
    let body = body.unwrap_or("");
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: phonorheos\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        socket.host(),
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("malformed reply")?;
    let head = String::from_utf8_lossy(&raw[..split]);
    let body = &raw[split + 4..];
    let code = head
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or("malformed status line")?;
    let chunked = head.lines().any(|l| {
        l.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    });
    let body = if chunked {
        dechunk(body)
    } else {
        body.to_vec()
    };
    Ok((code, String::from_utf8_lossy(&body).into_owned()))
}

/// The body of a chunked reply, put back together as bytes (a chunk may end inside a
/// character, so decoding waits until the end).
fn dechunk(mut s: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(eol) = s.windows(2).position(|w| w == b"\r\n") {
        let size = String::from_utf8_lossy(&s[..eol]);
        let Ok(n) = usize::from_str_radix(size.trim(), 16) else {
            break;
        };
        let rest = &s[eol + 2..];
        if n == 0 || rest.len() < n {
            break;
        }
        out.extend_from_slice(&rest[..n]);
        s = rest[n..].strip_prefix(b"\r\n").unwrap_or(&rest[n..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn say_takes_its_words_and_options() {
        let s = parse_say(&args(&[
            "--voice",
            "bm_george",
            "Hello,",
            "sir.",
            "--no-wait",
        ]))
        .unwrap();
        assert_eq!(
            (s.text.as_str(), s.voice.as_deref(), s.wait),
            ("Hello, sir.", Some("bm_george"), false)
        );
        let s = parse_say(&args(&["--speed", "1.2", "--", "--not-an-option"])).unwrap();
        assert_eq!((s.text.as_str(), s.speed), ("--not-an-option", Some(1.2)));
        assert!(parse_say(&args(&[])).is_err());
        assert!(parse_say(&args(&["--speed", "fast", "x"])).is_err());
        assert_eq!(
            parse_say(&args(&["--by-clause", "x"])).unwrap().by_clause,
            Some(true)
        );
        assert_eq!(
            parse_say(&args(&["--whole", "x"])).unwrap().by_clause,
            Some(false)
        );
        assert_eq!(parse_say(&args(&["x"])).unwrap().by_clause, None);
        let s = parse_say(&args(&["--cache", "--silent", "Thinking..."])).unwrap();
        assert_eq!(
            (s.cache, s.silent, s.text.as_str()),
            (true, true, "Thinking...")
        );
        let s = parse_say(&args(&["x"])).unwrap();
        assert!(!s.cache && !s.silent);
        assert!(parse_say(&args(&["--silent", "x"])).is_err());
    }

    #[test]
    fn the_loopback_endpoint_sends_the_loopback_host() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = Endpoint::Loopback(listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut head = Vec::new();
            let mut byte = [0];
            while !head.ends_with(b"\r\n\r\n") {
                conn.read_exact(&mut byte).unwrap();
                head.push(byte[0]);
            }
            conn.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .unwrap();
            String::from_utf8(head).unwrap()
        });
        let (code, body) = request(&endpoint, "GET", "/status", None).unwrap();
        let head = server.join().unwrap();
        assert_eq!((code, body.as_str()), (200, "{}"));
        assert!(
            head.contains(&format!("\r\nHost: {}\r\n", endpoint.host())),
            "{head}"
        );
        assert!(endpoint.host().starts_with("127.0.0.1:"));
    }

    #[test]
    fn a_chunked_body_is_put_back_together() {
        assert_eq!(
            dechunk(b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"),
            b"hello world"
        );
        // "naïve" with the chunk boundary inside the two bytes of "ï".
        let split = b"3\r\nna\xc3\r\n3\r\n\xafve\r\n0\r\n\r\n";
        assert_eq!(String::from_utf8(dechunk(split)).unwrap(), "naïve");
    }
}
