//! Opt-in loopback-only adapter for browser QA of the real Rust kernel.
//! Not a product backend. Requires a separate profile and explicit token.
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
};
fn serve(
    mut stream: TcpStream,
    agents: &sarv_quote::agent::AgentService,
    token: &str,
) -> anyhow::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(15)))?;
    let mut bytes = vec![];
    let mut buf = [0; 8192];
    let head_end = loop {
        let n = stream.read(&mut buf)?;
        anyhow::ensure!(n > 0, "closed");
        bytes.extend_from_slice(&buf[..n]);
        anyhow::ensure!(bytes.len() < 2_100_000, "too large");
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..head_end]).into_owned();
    let method = headers.split_whitespace().next().unwrap_or("");
    let length = headers
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse::<usize>())
        })
        .transpose()?
        .unwrap_or(0);
    anyhow::ensure!(length <= 2_000_000, "body too large");
    let authorized = headers.lines().any(|l| {
        l.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("x-sarv-qa-token") && value.trim() == token
        })
    });
    let origin = headers
        .lines()
        .find_map(|l| {
            l.strip_prefix("Origin: ")
                .or_else(|| l.strip_prefix("origin: "))
        })
        .unwrap_or("");
    let allowed = matches!(
        origin,
        "http://localhost:1420" | "http://127.0.0.1:1420" | ""
    );
    let (status, result) = if !allowed {
        (
            "403 Forbidden",
            json!({"error":{"message":"Origin denied"}}),
        )
    } else if method == "OPTIONS" {
        ("200 OK", json!({}))
    } else if !authorized || method != "POST" {
        (
            "403 Forbidden",
            json!({"error":{"message":"QA access denied"}}),
        )
    } else {
        while bytes.len() < head_end + length {
            let n = stream.read(&mut buf)?;
            anyhow::ensure!(n > 0, "closed body");
            bytes.extend_from_slice(&buf[..n]);
        }
        let r: Value = serde_json::from_slice(&bytes[head_end..head_end + length])?;
        match agents.ui_dispatch(r["action"].as_str().unwrap_or(""), r["payload"].clone()) {
            Ok(v) => ("200 OK", json!({"result":v})),
            Err(e) => (
                "400 Bad Request",
                json!({"error":{"code":"accounting_error","message":e.to_string()}}),
            ),
        }
    };
    let body = serde_json::to_vec(&result)?;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: {origin}\r\nVary: Origin\r\nAccess-Control-Allow-Headers: content-type,x-sarv-qa-token\r\nAccess-Control-Allow-Methods: POST,OPTIONS\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)?;
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let dir = PathBuf::from(std::env::var("SARV_QA_DATA_DIR")?);
    let token = std::env::var("SARV_QA_TOKEN")?;
    anyhow::ensure!(
        token.len() >= 24,
        "QA token must contain at least 24 characters"
    );
    if std::env::var("SARV_QA_SEED").as_deref() == Ok("1") {
        sarv_quote::desktop::seed_fixture(&dir)?;
    }
    let listener = TcpListener::bind("127.0.0.1:1421")?;
    let agents = std::sync::Arc::new(sarv_quote::agent::AgentService::start(
        &dir,
        std::env::current_dir()?.join("target/release/sarv-agent"),
    )?);
    println!("Rust QA adapter listening on 127.0.0.1:1421; isolated profile");
    for s in listener.incoming().flatten() {
        let agents = agents.clone();
        let token = token.clone();
        std::thread::spawn(move || {
            if let Err(e) = serve(s, &agents, &token) {
                eprintln!("QA request failed: {e}");
            }
        });
    }
    Ok(())
}
