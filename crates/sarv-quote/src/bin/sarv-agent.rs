use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};
use std::{
    io::{self, Read},
    path::PathBuf,
};

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut connection = std::env::var_os("SARV_AGENT_CONNECTION").map(PathBuf::from);
    let mut command = args.next().unwrap_or_else(|| "help".into());
    if command == "--connection" {
        connection = Some(PathBuf::from(
            args.next()
                .ok_or_else(|| anyhow!("Missing connection path"))?,
        ));
        command = args.next().unwrap_or_else(|| "status".into());
    }
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!(
            "sarv-agent --connection PATH [status | tools | schema ACTION | call ACTION --json JSON | call ACTION --stdin | mcp]"
        );
        return Ok(());
    }
    let connection =
        connection.ok_or_else(|| anyhow!("Copy the CLI command from Settings → Agents"))?;
    let value = match command.as_str() {
        "status" => {
            ensure!(args.next().is_none(), "Unexpected argument");
            sarv_quote::agent::call(&connection, "status", json!({}))?
        }
        "tools" => {
            ensure!(args.next().is_none(), "Unexpected argument");
            sarv_quote::agent::call(&connection, "tools", json!({}))?
        }
        "schema" => {
            let name = args.next().ok_or_else(|| anyhow!("Missing action"))?;
            ensure!(args.next().is_none(), "Unexpected argument");
            let tools = sarv_quote::agent::call(&connection, "tools", json!({}))?;
            tools
                .as_array()
                .and_then(|tools| tools.iter().find(|t| t["name"] == format!("sarv_{name}")))
                .ok_or_else(|| anyhow!("Unknown action"))?
                .clone()
        }
        "call" => {
            let action = args.next().ok_or_else(|| anyhow!("Missing action"))?;
            let payload: Value = match args.next().as_deref() {
                Some("--json") => {
                    serde_json::from_str(&args.next().ok_or_else(|| anyhow!("Missing JSON"))?)?
                }
                Some("--stdin") => {
                    let mut bytes = Vec::new();
                    io::stdin().take(2_000_001).read_to_end(&mut bytes)?;
                    ensure!(bytes.len() <= 2_000_000, "Payload too large");
                    serde_json::from_slice(&bytes)?
                }
                None => json!({}),
                _ => return Err(anyhow!("Use --json JSON or --stdin")),
            };
            ensure!(args.next().is_none(), "Unexpected argument");
            sarv_quote::agent::call(&connection, &action, payload)?
        }
        "mcp" => {
            ensure!(args.next().is_none(), "Unexpected argument");
            return sarv_quote::agent::mcp(&connection, io::stdin().lock(), io::stdout().lock());
        }
        _ => return Err(anyhow!("Unknown command")),
    };
    println!("{value}");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        println!(
            "{}",
            json!({"error":{"code":"agent_error","message":e.to_string()}})
        );
        std::process::exit(1);
    }
}
