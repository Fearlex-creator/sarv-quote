//! Shop lock for a local profile. Passwords are hashed with Argon2id and never
//! written into the financial snapshot, backups, or audit text. Sessions live in
//! the profile database and expire. There is no shipped default password.
//!
//! The local agent has its own token. Once a shop password exists, that token
//! works only after the owner grants a role again. A grant saved before the
//! lock, or a grant with no role, cannot read or change the books. The SQLite
//! file is not encrypted at rest; the password protects commands, not the disk.
use crate::store::Store;
use anyhow::{Result, anyhow, ensure};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

const SESSION_SECONDS: i64 = 12 * 60 * 60;
const LOCKOUT_SECONDS: i64 = 15 * 60;
const MAX_FAILURES: usize = 8;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Security {
    users: Vec<User>,
    recovery_hash: String,
    sessions: Vec<Session>,
    failures: Vec<i64>,
    locked_until: i64,
    audit: Vec<SecEvent>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct User {
    id: String,
    name: String,
    role: String,
    password_hash: String,
    disabled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    token_hash: String,
    user_id: String,
    expires: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SecEvent {
    at: String,
    actor: String,
    action: String,
    scope: String,
}

pub fn is_auth_command(action: &str) -> bool {
    matches!(
        action,
        "auth_status"
            | "auth_login"
            | "auth_logout"
            | "auth_bootstrap"
            | "auth_create_user"
            | "auth_disable_user"
            | "auth_reset"
    )
}

pub fn configured(store: &Store) -> Result<bool> {
    Ok(!load(store)?.users.is_empty())
}

/// Role check for an already authenticated caller. The caller supplies the
/// actor label. Client-supplied `user:` and `agent:` actors are removed first.
pub fn authorize(role: &str, action: &str, payload: &mut Value, actor: &str) -> Result<()> {
    scrub(payload);
    ensure!(
        role_allows(role, action, payload),
        "Your role cannot perform this action"
    );
    if let Some(object) = payload.as_object_mut() {
        object.insert("_actor".into(), json!(actor));
        object.remove("session_token");
    }
    Ok(())
}

pub fn enforce(store: &Store, action: &str, payload: &mut Value) -> Result<()> {
    scrub(payload);
    let security = load(store)?;
    if security.users.is_empty() {
        ensure!(
            !matches!(
                action,
                "auth_create_user" | "auth_disable_user" | "auth_reset" | "auth_logout"
            ),
            "Set up the shop lock before managing people"
        );
        return Ok(());
    }
    ensure!(
        action != "auth_bootstrap",
        "Shop lock is already configured"
    );
    // Recovery is the credential. A forgotten password cannot require a session.
    if matches!(action, "auth_status" | "auth_login" | "auth_reset") {
        return Ok(());
    }
    let token = payload
        .get("session_token")
        .and_then(Value::as_str)
        .unwrap_or("");
    let user = session_user(&security, token)?;
    authorize(
        &user.role,
        action,
        payload,
        &format!("user:{}:{}", user.role, user.id),
    )
}

pub fn handle(store: &mut Store, action: &str, payload: &Value) -> Result<Value> {
    match action {
        "auth_status" => {
            let security = load(store)?;
            let mut status = public_status(&security);
            let token = payload
                .get("session_token")
                .and_then(Value::as_str)
                .unwrap_or("");
            if session_user(&security, token).is_err() {
                status["people"] = json!([]);
            }
            Ok(status)
        }
        "auth_bootstrap" => bootstrap(store, payload),
        "auth_login" => login(store, payload),
        "auth_logout" => logout(store, payload),
        "auth_create_user" => create_user(store, payload),
        "auth_disable_user" => disable_user(store, payload),
        "auth_reset" => reset(store, payload),
        _ => Err(anyhow!("Unsupported lock command")),
    }
}

fn bootstrap(store: &mut Store, payload: &Value) -> Result<Value> {
    let mut security = load(store)?;
    ensure!(security.users.is_empty(), "Shop lock is already configured");
    let name = required(payload, "name")?;
    let password = required(payload, "password")?;
    validate_password(&password)?;
    let recovery = recovery_code();
    let user = User {
        id: format!("user-{}", &token_hash(&recovery)[..12]),
        name,
        role: "owner".into(),
        password_hash: hash_secret(&password)?,
        disabled: false,
    };
    security.recovery_hash = hash_secret(&recovery)?;
    security.users.push(user.clone());
    remember(
        &mut security,
        &user.id,
        "auth_bootstrap",
        "shop",
    );
    let session = issue(&mut security, &user)?;
    save(store, &security)?;
    Ok(json!({
        "configured": true,
        "session_token": session,
        "role": "owner",
        "name": user.name,
        "recovery_code": recovery,
        "recovery_notice": "Write this recovery code down now. It is not stored and will not be shown again. There is no email reset.",
    }))
}

fn login(store: &mut Store, payload: &Value) -> Result<Value> {
    let mut security = load(store)?;
    ensure!(!security.users.is_empty(), "Shop lock is not configured");
    let now = now_secs();
    ensure!(
        security.locked_until <= now,
        "Too many sign-in attempts. Wait and try again."
    );
    let name = required(payload, "name")?;
    let password = required(payload, "password")?;
    let user = security
        .users
        .iter()
        .find(|u| u.name == name && !u.disabled)
        .cloned();
    let valid = match &user {
        Some(user) => verify_secret(&password, &user.password_hash)?,
        None => {
            // Spend the same hash work so missing names are not a faster answer.
            let _ = verify_secret(&password, &dummy_hash());
            false
        }
    };
    if !valid {
        security.failures.push(now);
        security
            .failures
            .retain(|at| now.saturating_sub(*at) < LOCKOUT_SECONDS);
        if security.failures.len() >= MAX_FAILURES {
            security.locked_until = now + LOCKOUT_SECONDS;
            security.failures.clear();
        }
        remember(&mut security, "anonymous", "auth_login_failed", "shop");
        save(store, &security)?;
        return Err(anyhow!("Name or password is wrong"));
    }
    let user = user.unwrap();
    security.failures.clear();
    security.locked_until = 0;
    remember(&mut security, &user.id, "auth_login", "shop");
    let session = issue(&mut security, &user)?;
    save(store, &security)?;
    Ok(json!({
        "session_token": session,
        "role": user.role,
        "name": user.name,
    }))
}

fn logout(store: &mut Store, payload: &Value) -> Result<Value> {
    let mut security = load(store)?;
    let actor = payload["_actor"].as_str().unwrap_or("").to_owned();
    let user_id = actor.rsplit(':').next().unwrap_or("");
    security.sessions.retain(|s| s.user_id != user_id);
    remember(&mut security, user_id, "auth_logout", "shop");
    save(store, &security)?;
    Ok(json!({"signed_out": true}))
}

fn create_user(store: &mut Store, payload: &Value) -> Result<Value> {
    let mut security = load(store)?;
    let name = required(payload, "name")?;
    let password = required(payload, "password")?;
    let role = required(payload, "role")?;
    ensure!(
        matches!(
            role.as_str(),
            "admin" | "cashier" | "accountant" | "readonly"
        ),
        "Choose admin, cashier, accountant, or readonly"
    );
    validate_password(&password)?;
    ensure!(
        !security.users.iter().any(|u| u.name == name),
        "That name is already in use"
    );
    ensure!(security.users.len() < 50, "Too many people");
    let user = User {
        id: format!("user-{}", &token_hash(&format!("{name}:{role}:{}", now_secs()))[..12]),
        name,
        role,
        password_hash: hash_secret(&password)?,
        disabled: false,
    };
    let actor = payload["_actor"].as_str().unwrap_or("owner");
    remember(&mut security, actor, "auth_create_user", &user.id);
    security.users.push(user.clone());
    save(store, &security)?;
    Ok(json!({"id": user.id, "name": user.name, "role": user.role}))
}

fn disable_user(store: &mut Store, payload: &Value) -> Result<Value> {
    let mut security = load(store)?;
    let id = required(payload, "user_id")?;
    let actor = payload["_actor"].as_str().unwrap_or("");
    ensure!(
        !actor.ends_with(&format!(":{id}")),
        "Disable a different person, not the signed-in owner"
    );
    let user = security
        .users
        .iter_mut()
        .find(|u| u.id == id)
        .ok_or_else(|| anyhow!("Person not found"))?;
    ensure!(user.role != "owner", "The owner account stays enabled");
    user.disabled = true;
    security.sessions.retain(|s| s.user_id != id);
    remember(&mut security, actor, "auth_disable_user", &id);
    save(store, &security)?;
    Ok(json!({"disabled": true}))
}

fn reset(store: &mut Store, payload: &Value) -> Result<Value> {
    let mut security = load(store)?;
    let recovery = required(payload, "recovery_code")?;
    let password = required(payload, "password")?;
    validate_password(&password)?;
    ensure!(
        verify_secret(&recovery, &security.recovery_hash)?,
        "Recovery code is wrong"
    );
    let owner = security
        .users
        .iter_mut()
        .find(|u| u.role == "owner")
        .ok_or_else(|| anyhow!("Owner account is missing"))?;
    owner.password_hash = hash_secret(&password)?;
    owner.disabled = false;
    let owner_id = owner.id.clone();
    security.sessions.clear();
    security.failures.clear();
    security.locked_until = 0;
    remember(&mut security, &owner_id, "auth_reset", "shop");
    save(store, &security)?;
    Ok(json!({
        "reset": true,
        "notice": "The owner password was replaced. Sign in again. The recovery code is unchanged.",
    }))
}

fn public_status(security: &Security) -> Value {
    json!({
        "configured": !security.users.is_empty(),
        "people": security.users.iter().map(|u| json!({"id": u.id, "name": u.name, "role": u.role, "disabled": u.disabled})).collect::<Vec<_>>(),
        "recovery": "one_local_code_shown_once",
        "email_reset": false,
    })
}

fn role_allows(role: &str, action: &str, payload: &Value) -> bool {
    if matches!(action, "auth_logout" | "auth_status") {
        return true;
    }
    if !is_write(action) {
        return true;
    }
    match role {
        "owner" | "admin" => true,
        "accountant" => !matches!(
            action,
            "backup_restore"
                | "auth_create_user"
                | "auth_disable_user"
                | "auth_reset"
                | "agent_configure"
                | "setup_company"
                | "excel_graph_begin"
                | "excel_graph_complete"
                | "excel_graph_revoke"
                | "excel_graph_push"
        ),
        "cashier" => {
            let kind_ok = action != "resource"
                || matches!(
                    payload["kind"].as_str(),
                    Some("customer" | "item" | "unit")
                );
            let receipt_ok = action != "payment_post" || payload["side"] == "receive";
            kind_ok
                && receipt_ok
                && matches!(
                    action,
                    "draft_save"
                        | "draft_delete"
                        | "draft_post"
                        | "fulfill_post"
                        | "payment_post"
                        | "sale_return_post"
                        | "resource"
                        | "product_profile"
                        | "party_profile"
                        | "pos_shift_open"
                        | "pos_shift_close"
                        | "pos_hold"
                        | "pos_hold_delete"
                )
        }
        _ => false,
    }
}

fn is_write(action: &str) -> bool {
    if action.ends_with("_preview")
        || action.ends_with("_status")
        || matches!(
            action,
            "excel_template" | "excel_export" | "excel_graph_status" | "workspace" | "report"
                | "integrity" | "law_data" | "detail" | "ledger" | "report_csv" | "profit_csv"
                | "backup_list" | "payment_options" | "operation_options" | "contact_detail"
                | "banking" | "product_detail"
        )
    {
        return false;
    }
    action.ends_with("_post")
        || action.ends_with("_save")
        || matches!(
            action,
            "resource"
                | "product_profile"
                | "party_profile"
                | "settings"
                | "business_profile"
                | "backup_restore"
                | "backup_create"
                | "backup_export"
                | "period_lock"
                | "setup_company"
                | "initialize_zero"
                | "draft_delete"
                | "journal_reverse"
                | "excel_commit"
                | "excel_graph_sync"
                | "excel_graph_begin"
                | "excel_graph_complete"
                | "excel_graph_revoke"
                | "excel_graph_commit"
                | "excel_graph_push"
                | "pos_shift_open"
                | "pos_shift_close"
                | "pos_hold"
                | "pos_hold_delete"
                | "auth_create_user"
                | "auth_disable_user"
                | "auth_reset"
                | "agent_configure"
                | "cheque_receive"
                | "cheque_issue"
                | "cheque_spend"
                | "cheque_register_opening"
                | "cheque_deposit"
                | "cheque_clear"
                | "cheque_bounce"
                | "cheque_return"
                | "cheque_void"
        )
}

fn issue(security: &mut Security, user: &User) -> Result<String> {
    let now = now_secs();
    security.sessions.retain(|s| s.expires > now);
    ensure!(security.sessions.len() < 32, "Too many open sessions");
    let token = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    security.sessions.push(Session {
        token_hash: token_hash(&token),
        user_id: user.id.clone(),
        expires: now + SESSION_SECONDS,
    });
    Ok(token)
}

fn session_user(security: &Security, token: &str) -> Result<User> {
    let now = now_secs();
    let hash = token_hash(token);
    let session = security
        .sessions
        .iter()
        .find(|s| s.token_hash == hash && s.expires > now)
        .ok_or_else(|| anyhow!("Sign in required"))?;
    security
        .users
        .iter()
        .find(|u| u.id == session.user_id && !u.disabled)
        .cloned()
        .ok_or_else(|| anyhow!("Sign in required"))
}

fn remember(security: &mut Security, actor: &str, action: &str, scope: &str) {
    security.audit.push(SecEvent {
        at: crate::domain::now(),
        actor: actor.to_owned(),
        action: action.to_owned(),
        scope: scope.to_owned(),
    });
    if security.audit.len() > 1_000 {
        let extra = security.audit.len() - 1_000;
        security.audit.drain(0..extra);
    }
}

fn load(store: &Store) -> Result<Security> {
    let payload = store.security_payload()?;
    if payload.trim().is_empty() {
        return Ok(Security::default());
    }
    Ok(serde_json::from_str(&payload)?)
}
fn save(store: &Store, security: &Security) -> Result<()> {
    store.set_security_payload(&serde_json::to_string(security)?)
}

fn hash_secret(secret: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map_err(|e| anyhow!("Could not hash the secret: {e}"))?
        .to_string())
}
fn verify_secret(secret: &str, stored: &str) -> Result<bool> {
    let parsed = PasswordHash::new(stored).map_err(|_| anyhow!("Stored lock is unreadable"))?;
    Ok(Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok())
}
fn dummy_hash() -> String {
    use std::sync::OnceLock;
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY
        .get_or_init(|| hash_secret("not-a-shop-user").unwrap_or_default())
        .clone()
}
fn validate_password(password: &str) -> Result<()> {
    ensure!(
        password.chars().count() >= 10 && password.chars().count() <= 128,
        "Use a password of 10 to 128 characters"
    );
    ensure!(
        !password.chars().any(char::is_control),
        "The password contains a control character"
    );
    Ok(())
}
fn recovery_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let raw = uuid::Uuid::new_v4().as_bytes().to_vec();
    let more = uuid::Uuid::new_v4().as_bytes().to_vec();
    raw.into_iter()
        .chain(more)
        .take(20)
        .map(|b| ALPHABET[(b as usize) % ALPHABET.len()] as char)
        .collect()
}
fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
fn required(payload: &Value, key: &str) -> Result<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("Missing {key}"))
}
fn scrub(payload: &mut Value) {
    if let Some(object) = payload.as_object_mut() {
        if object
            .get("_actor")
            .and_then(Value::as_str)
            .is_some_and(|s| s.starts_with("user:") || s.starts_with("agent:"))
        {
            object.remove("_actor");
        }
    }
}
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
