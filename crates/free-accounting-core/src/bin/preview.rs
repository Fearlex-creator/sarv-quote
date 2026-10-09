//! Development-only browser transport, using the real Rust ledger.
use anyhow::{Result, ensure};
use free_accounting_core::Ledger;
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};

fn handle(mut stream: TcpStream, ledger: &Ledger, token: &str) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let header_end = loop {
        let count = stream.read(&mut buffer)?;
        ensure!(count > 0, "Incomplete request");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        ensure!(bytes.len() < 16384, "Header too large");
    };
    let header = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    ensure!(
        header.starts_with("POST /api/accounting "),
        "POST /api/accounting required"
    );
    let length = header
        .lines()
        .find_map(|l| {
            l.to_lowercase()
                .strip_prefix("content-length:")
                .map(|n| n.trim().parse::<usize>())
        })
        .transpose()?
        .unwrap_or(0);
    ensure!(length > 0 && length <= 40 * 1024 * 1024, "Body too large");
    while bytes.len() < header_end + length {
        let count = stream.read(&mut buffer)?;
        ensure!(count > 0, "Incomplete request");
        bytes.extend_from_slice(&buffer[..count]);
    }
    let request: Value = serde_json::from_slice(&bytes[header_end..header_end + length])?;
    ensure!(
        request["token"].as_str() == Some(token),
        "Invalid development token"
    );
    let action = request["action"].as_str().unwrap_or("");
    let result = if request["mutation"].as_bool() == Some(true) {
        ledger.mutate(action, request["payload"].clone())
    } else {
        ledger.read(action, request["payload"].clone())
    };
    let (status, body) = match result {
        Ok(value) => ("200 OK", json!({"data":value})),
        Err(error) => ("400 Bad Request", json!({"error":format!("{error:#}")})),
    };
    let body = body.to_string();
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

fn main() -> Result<()> {
    let path = std::env::var("FREE_ACCOUNTING_PREVIEW_DB")?;
    let token = std::env::var("FREE_ACCOUNTING_PREVIEW_TOKEN")?;
    ensure!(
        token.len() >= 16,
        "Use a development token of at least 16 characters"
    );
    let ledger = Ledger::open(path)?;
    let port = std::env::var("SARV_PREVIEW_LEDGER_PORT")
        .unwrap_or_else(|_| "4783".into())
        .parse::<u16>()?;
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("Rust ledger preview listening on 127.0.0.1:{port}");
    for stream in listener.incoming() {
        if let Err(e) = handle(stream?, &ledger, &token) {
            eprintln!("Preview request: {e:#}");
        }
    }
    Ok(())
}
