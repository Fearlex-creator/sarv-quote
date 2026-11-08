//! Optional Microsoft Graph connector for one workbook the owner names.
//!
//! Nothing here runs unless `SARV_GRAPH_CLIENT_ID` is set on this computer.
//! Tokens stay in a mode-0600 file next to the profile. They are not in the
//! financial backup, not in status JSON, and not encrypted beyond the file
//! permissions. This is not SQLCipher. Live calls are not claimed verified
//! unless this process actually completed them.
use crate::store::Store;
use anyhow::{Result, anyhow, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Per-file delegated scope. `Files.ReadWrite.Selected` is an Office file-handler
/// scope and Microsoft says it must not be used to call Microsoft Graph.
/// `Files.SelectedOperations.Selected` is the current per-file scope. Consent
/// does not grant a typed drive item id; OneDrive or SharePoint must permission
/// this application on that file, and admin consent is required.
const SCOPES: &str = "Files.SelectedOperations.Selected offline_access";
const PENDING: &str = ".graph-pending.json";
const GRANT: &str = ".graph-grant.json";
const MAX_DOWNLOAD: u64 = 1_000_000;

#[derive(Serialize, Deserialize)]
struct Pending {
    state: String,
    verifier: String,
    expires: i64,
}
#[derive(Serialize, Deserialize)]
struct Grant {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

struct Endpoints {
    client_id: String,
    tenant: String,
    redirect: String,
    authorize: String,
    token_url: String,
    api: String,
}

pub fn status(store: &Store) -> Value {
    let configured = endpoints().is_some();
    let authenticated = read_grant(&store.directory).is_some();
    json!({
        "configured": configured,
        "status": if !configured { "not_configured" } else if authenticated { "authenticated" } else { "ready" },
        "authenticated": authenticated,
        "live_verified": false,
        "scopes": if configured { vec![SCOPES] } else { vec![] },
        "instructions": if configured {
            "Open the connect link and sign in. Files.SelectedOperations.Selected does not grant a typed item id. The workbook must already be permissioned to this app in OneDrive or SharePoint, and an administrator must consent. A refusal means that grant is missing. Sarv does not list the drive. live_verified stays false until a real tenant call succeeds, and this build has not made one."
        } else {
            "Local .xlsx import and export work with no Microsoft 365 subscription. A client id does not connect a workbook. The connector requests Files.SelectedOperations.Selected. Admin consent is required, and typing an item id is not a file grant. This build does not register an application."
        },
    })
}

pub fn begin(store: &Store) -> Result<Value> {
    let endpoints = endpoints().ok_or_else(|| anyhow!("Microsoft 365 is not configured"))?;
    let verifier = random_token();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_token();
    write_private(
        &store.directory.join(PENDING),
        &serde_json::to_vec(&Pending {
            state: state.clone(),
            verifier,
            expires: now() + 600,
        })?,
    )?;
    let url = format!(
        "{}/{}/oauth2/v2.0/authorize?client_id={}&response_type=code&redirect_uri={}&response_mode=query&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        endpoints.authorize.trim_end_matches('/'),
        encode(&endpoints.tenant),
        encode(&endpoints.client_id),
        encode(&endpoints.redirect),
        encode(SCOPES),
        encode(&state),
        encode(&challenge),
    );
    Ok(json!({
        "status": "authorization_required",
        "authorize_url": url,
        "scopes": SCOPES,
        "live_verified": false,
    }))
}

pub fn complete(store: &Store, payload: &Value) -> Result<Value> {
    let endpoints = endpoints().ok_or_else(|| anyhow!("Microsoft 365 is not configured"))?;
    let redirected = payload["redirect_url"].as_str().unwrap_or("");
    let code = query(redirected, "code").ok_or_else(|| anyhow!("The redirected address has no code"))?;
    let state = query(redirected, "state").ok_or_else(|| anyhow!("The redirected address has no state"))?;
    let pending: Pending = serde_json::from_slice(&fs::read(store.directory.join(PENDING)).map_err(|_| anyhow!("Start the connection again"))?)?;
    ensure!(now() <= pending.expires, "The connection attempt expired. Start again.");
    ensure!(state == pending.state, "The state does not match. Start again.");
    let body = format!(
        "client_id={}&grant_type=authorization_code&code={}&redirect_uri={}&code_verifier={}",
        encode(&endpoints.client_id),
        encode(&code),
        encode(&endpoints.redirect),
        encode(&pending.verifier),
    );
    let token = post_form(&endpoints.token_url, &body)?;
    store_token(&store.directory, &token)?;
    let _ = fs::remove_file(store.directory.join(PENDING));
    Ok(json!({"authenticated": true, "status": "authenticated", "live_verified": false}))
}

pub fn revoke(store: &Store) -> Result<Value> {
    if let (Some(endpoints), Some(grant)) = (endpoints(), read_grant(&store.directory)) {
        let body = format!(
            "client_id={}&token={}",
            encode(&endpoints.client_id),
            encode(&grant.refresh_token)
        );
        let revoke = endpoints.token_url.replace("/token", "/revoke");
        let _ = post_form(&revoke, &body);
    }
    let _ = fs::remove_file(store.directory.join(GRANT));
    let _ = fs::remove_file(store.directory.join(PENDING));
    Ok(json!({"authenticated": false, "status": "ready", "live_verified": false}))
}

pub fn download(store: &Store, payload: &Value) -> Result<Vec<u8>> {
    let item = item_id(payload)?;
    let token = access_token(store)?;
    let endpoints = endpoints().ok_or_else(|| anyhow!("Microsoft 365 is not configured"))?;
    let url = format!(
        "{}/me/drive/items/{}/content",
        endpoints.api.trim_end_matches('/'),
        encode(&item)
    );
    http_bytes("GET", &url, &token, None)
}

pub fn push(store: &Store, payload: &Value) -> Result<Value> {
    ensure!(payload["confirm"] == true, "Push needs explicit confirmation");
    let item = item_id(payload)?;
    let bytes = crate::excel::exported_bytes(store, payload)?;
    ensure!(bytes.len() as u64 <= MAX_DOWNLOAD, "Export is larger than 1 MB");
    let token = access_token(store)?;
    let endpoints = endpoints().ok_or_else(|| anyhow!("Microsoft 365 is not configured"))?;
    let url = format!(
        "{}/me/drive/items/{}/content",
        endpoints.api.trim_end_matches('/'),
        encode(&item)
    );
    let _ = http_bytes("PUT", &url, &token, Some(&bytes))?;
    Ok(json!({
        "pushed": true,
        "item_id": item,
        "bytes": bytes.len(),
        "live_verified": false,
        "note": "Replaced only the named workbook. The drive was not listed.",
    }))
}

fn access_token(store: &Store) -> Result<String> {
    let endpoints = endpoints().ok_or_else(|| anyhow!("Microsoft 365 is not configured"))?;
    let mut grant = read_grant(&store.directory).ok_or_else(|| anyhow!("Microsoft 365 is not signed in"))?;
    if grant.expires_at <= now() + 30 {
        let body = format!(
            "client_id={}&grant_type=refresh_token&refresh_token={}&scope={}",
            encode(&endpoints.client_id),
            encode(&grant.refresh_token),
            encode(SCOPES),
        );
        let token = post_form(&endpoints.token_url, &body)?;
        store_token(&store.directory, &token)?;
        grant = read_grant(&store.directory).ok_or_else(|| anyhow!("Microsoft 365 is not signed in"))?;
    }
    Ok(grant.access_token)
}

fn store_token(directory: &Path, token: &Value) -> Result<()> {
    let access = token["access_token"].as_str().unwrap_or("");
    let refresh = token["refresh_token"].as_str().filter(|s| !s.is_empty()).map(str::to_owned).or_else(|| {
        read_grant(directory).map(|g| g.refresh_token)
    });
    ensure!(!access.is_empty(), "The token response had no access token");
    let refresh = refresh.ok_or_else(|| anyhow!("The token response had no refresh token"))?;
    let expires_in = token["expires_in"].as_u64().unwrap_or(3600);
    write_private(
        &directory.join(GRANT),
        &serde_json::to_vec(&Grant {
            access_token: access.to_owned(),
            refresh_token: refresh,
            expires_at: now() + expires_in as i64,
        })?,
    )
}

fn endpoints() -> Option<Endpoints> {
    let client_id = std::env::var("SARV_GRAPH_CLIENT_ID").ok().filter(|s| !s.trim().is_empty())?;
    let tenant = std::env::var("SARV_GRAPH_TENANT").unwrap_or_else(|_| "common".into());
    let redirect = std::env::var("SARV_GRAPH_REDIRECT_URI")
        .unwrap_or_else(|_| "http://127.0.0.1:47821/sarv-graph".into());
    let authorize = std::env::var("SARV_GRAPH_AUTHORIZE_BASE")
        .unwrap_or_else(|_| "https://login.microsoftonline.com".into());
    let token_url = std::env::var("SARV_GRAPH_TOKEN_URL").unwrap_or_else(|_| {
        format!(
            "{}/{}/oauth2/v2.0/token",
            authorize.trim_end_matches('/'),
            tenant
        )
    });
    let api = std::env::var("SARV_GRAPH_API_BASE")
        .unwrap_or_else(|_| "https://graph.microsoft.com/v1.0".into());
    Some(Endpoints {
        client_id,
        tenant,
        redirect,
        authorize,
        token_url,
        api,
    })
}

fn item_id(payload: &Value) -> Result<String> {
    let id = payload["item_id"].as_str().unwrap_or("").trim().to_owned();
    ensure!(
        !id.is_empty()
            && id.len() <= 128
            && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '!' | '.' | '_' | '-')),
        "Workbook item id is missing or not a single item"
    );
    Ok(id)
}

fn post_form(url: &str, body: &str) -> Result<Value> {
    let agent = ureq::AgentBuilder::new().redirects(0).build();
    let attempted = agent
        .post(url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("Accept", "application/json")
        .send_string(body);
    let response = match attempted {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let _ = response.into_string();
            return Err(graph_status(code));
        }
        Err(error) => {
            return Err(anyhow!(
                "Token request failed: {}",
                redact(&error.to_string())
            ));
        }
    };
    let text = response
        .into_string()
        .map_err(|_| anyhow!("Token response was not readable"))?;
    serde_json::from_str(&text).map_err(|_| anyhow!("Token response was not JSON"))
}

fn graph_status(code: u16) -> anyhow::Error {
    match code {
        401 => anyhow!("Microsoft refused the sign-in. Start the connection again."),
        403 => anyhow!(
            "This workbook is not granted to the app. Files.SelectedOperations.Selected does not authorize a typed item id."
        ),
        429 => anyhow!("Microsoft asked Sarv to wait. Try the workbook again later."),
        _ => anyhow!("Workbook request was refused ({code})"),
    }
}

fn allowed_redirect(location: &str) -> bool {
    let (secure, rest) = if let Some(rest) = location.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = location.strip_prefix("http://") {
        (false, rest)
    } else {
        return false;
    };
    let hostport = rest.split(['/', '?', '#']).next().unwrap_or("");
    if hostport.is_empty() || hostport.contains('@') {
        return false;
    }
    let host = hostport
        .rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map(|(host, _)| host)
        .unwrap_or(hostport);
    if !secure {
        return host == "127.0.0.1";
    }
    if host == "localhost" || host == "metadata.google.internal" {
        return false;
    }
    match host.parse::<std::net::Ipv4Addr>() {
        Ok(ip) => {
            !(ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast())
        }
        Err(_) => true,
    }
}

fn http_bytes(method: &str, url: &str, token: &str, body: Option<&[u8]>) -> Result<Vec<u8>> {
    let agent = ureq::AgentBuilder::new().redirects(0).build();
    let request = match method {
        "PUT" => agent.put(url),
        _ => agent.get(url),
    }
    .set("Authorization", &format!("Bearer {token}"));
    let attempted = if method == "PUT" {
        request
            .set(
                "Content-Type",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            )
            .send_bytes(body.unwrap_or_default())
    } else {
        request.call()
    };
    let response = match attempted {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let _ = response.into_string();
            return Err(graph_status(code));
        }
        Err(error) => {
            return Err(anyhow!(
                "Workbook request failed: {}",
                redact(&error.to_string())
            ));
        }
    };
    let status = response.status();
    if status == 301 || status == 302 || status == 303 || status == 307 {
        let location = response
            .header("location")
            .unwrap_or("")
            .to_owned();
        ensure!(allowed_redirect(&location), "Redirect was not allowed");
        let follow = ureq::AgentBuilder::new().redirects(0).build();
        let file = follow
            .get(&location)
            .call()
            .map_err(|_| anyhow!("Workbook download failed"))?;
        return read_limited(file);
    }
    ensure!(status < 400, "Workbook request was refused");
    read_limited(response)
}

fn read_limited(response: ureq::Response) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_DOWNLOAD + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow!("Workbook could not be read"))?;
    ensure!(bytes.len() as u64 <= MAX_DOWNLOAD, "Workbook is larger than 1 MB");
    Ok(bytes)
}

fn read_grant(directory: &Path) -> Option<Grant> {
    let bytes = fs::read(directory.join(GRANT)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn random_token() -> String {
    URL_SAFE_NO_PAD.encode(uuid::Uuid::new_v4().as_bytes())
}

fn query(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            return Some(decode(value));
        }
    }
    None
}

fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&value[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn redact(text: &str) -> String {
    let mut out = text.to_owned();
    for secret in ["access_token", "refresh_token", "code", "code_verifier"] {
        if let Some(start) = out.find(secret) {
            out.replace_range(start.., secret);
        }
    }
    if out.chars().count() > 180 {
        out.truncate(180);
    }
    out
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
