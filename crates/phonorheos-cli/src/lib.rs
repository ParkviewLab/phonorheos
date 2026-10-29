// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `phonorheos` command: `phonorheos say "…"` sends text to the running app over
//! its Unix socket and waits until it has been spoken. A plain HTTP/1.1 client over
//! the socket, since the command needs nothing more.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage:
  phonorheos say [--voice <voice>] [--speed <0.5..2>] [--no-wait] <text>…
  phonorheos stop
  phonorheos status
  phonorheos --version";

/// Runs the command with its arguments (without the program name).
pub fn run(args: &[String]) -> ExitCode {
    let socket = phonorheos_core::paths::socket_path();
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
    wait: bool,
}

fn parse_say(args: &[String]) -> Result<Say, String> {
    let mut s = Say {
        text: String::new(),
        voice: None,
        speed: None,
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
            "--no-wait" => s.wait = false,
            "--" => words.extend(it.by_ref().cloned()),
            _ => words.push(a.clone()),
        }
    }
    s.text = words.join(" ");
    if s.text.trim().is_empty() {
        return Err(format!("nothing to say\n{USAGE}"));
    }
    Ok(s)
}

fn say(socket: &Path, args: &[String]) -> Result<(), String> {
    let s = parse_say(args)?;
    let mut body = serde_json::json!({ "text": s.text, "client": "phonorheos say" });
    if let Some(v) = s.voice {
        body["voice"] = v.into();
    }
    if let Some(sp) = s.speed {
        body["speed"] = sp.into();
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
fn wait_for_end(socket: &Path, id: &str, after: u64) -> Result<(), String> {
    let mut stream = connect(socket)?;
    let head = format!(
        "GET /events?item={id}&last_id={after} HTTP/1.1\r\nHost: localhost\r\nUser-Agent: phonorheos say\r\nAccept: text/event-stream\r\n\r\n"
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

fn connect(socket: &Path) -> Result<UnixStream, String> {
    UnixStream::connect(socket)
        .map_err(|e| format!("phonorheos is not running ({}: {e})", socket.display()))
}

/// One request, read to its end; returns the status code and the body.
fn request(
    socket: &Path,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let mut stream = connect(socket)?;
    let body = body.unwrap_or("");
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nUser-Agent: phonorheos\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&raw);
    let (head, rest) = text.split_once("\r\n\r\n").ok_or("malformed reply")?;
    let code = head
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or("malformed status line")?;
    let chunked = head.lines().any(|l| {
        l.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    });
    Ok((
        code,
        if chunked {
            dechunk(rest)
        } else {
            rest.to_string()
        },
    ))
}

fn dechunk(mut s: &str) -> String {
    let mut out = String::new();
    while let Some((size, rest)) = s.split_once("\r\n") {
        let Ok(n) = usize::from_str_radix(size.trim(), 16) else {
            break;
        };
        if n == 0 || rest.len() < n {
            break;
        }
        out.push_str(&rest[..n]);
        s = rest[n..].trim_start_matches("\r\n");
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
    }

    #[test]
    fn a_chunked_body_is_put_back_together() {
        assert_eq!(
            dechunk("5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"),
            "hello world"
        );
    }
}
