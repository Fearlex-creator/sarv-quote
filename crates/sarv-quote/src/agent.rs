//! Settings-authorized loopback bridge shared by the CLI and MCP adapter.
//! The transport never accepts a database path or an access-control command.
use anyhow::{Context, Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

const MAX_MESSAGE: u64 = 2_000_000;
const POLICY: &str = ".agent-access.json";
pub const CONNECTION: &str = ".agent-connection.json";
const READ: &[&str] = &[
    "workspace",
    "report",
    "detail",
    "ledger",
    "report_csv",
    "profit_csv",
    "opening_csv_preview",
    "draft_preview",
    "purchase_preview",
    "receive_preview",
    "bill_preview",
    "fulfill_preview",
    "operation_options",
    "sale_return_preview",
    "purchase_return_preview",
    "transfer_preview",
    "journal_preview",
    "journal_reverse_preview",
    "payment_options",
    "payment_preview",
    "payment_reverse_preview",
    "banking",
    "banking_preview",
    "contact_detail",
    "product_detail",
];
const WRITE: &[&str] = &[
    "setup_company",
    "initialize_zero",
    "opening_csv_post",
    "resource",
    "draft_save",
    "draft_delete",
    "draft_post",
    "purchase_post",
    "receive_post",
    "bill_post",
    "fulfill_post",
    "sale_return_post",
    "purchase_return_post",
    "transfer_post",
    "journal_post",
    "period_lock",
    "journal_reverse",
    "payment_post",
    "settings",
];

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    #[default]
    ReadOnly,
    Full,
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    enabled: bool,
    access: Access,
    epoch: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Connection {
    protocol: String,
    address: SocketAddr,
    token: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    token: String,
    action: String,
    #[serde(default = "empty")]
    payload: Value,
}
fn empty() -> Value {
    json!({})
}
struct State {
    policy: Policy,
    token: String,
}
struct Inner {
    directory: PathBuf,
    cli: PathBuf,
    state: Mutex<State>,
    /// One owner per process: the GUI keeps the book in memory between
    /// commands and only reloads it when another writer advances the
    /// generation. This removes a full SQLite read + JSON parse per action.
    accounting: Mutex<crate::store::Store>,
    shutdown: AtomicBool,
    connections: AtomicUsize,
    on_change: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}
impl Inner {
    fn dispatch(&self, action: &str, payload: Value) -> Result<Value> {
        let mut store = self
            .accounting
            .lock()
            .map_err(|_| anyhow!("Accounting state interrupted"))?;
        store.refresh_if_stale()?;
        crate::desktop::dispatch_in(&mut store, action, payload)
    }
}
pub struct AgentService {
    inner: Arc<Inner>,
}

fn load_policy(directory: &Path) -> Result<Policy> {
    match fs::read(directory.join(POLICY)) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Policy::default()),
        Err(e) => Err(e.into()),
    }
}
fn private_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary)?;
        file.write_all(&serde_json::to_vec(value)?)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
fn remove_connection(directory: &Path) -> Result<()> {
    match fs::remove_file(directory.join(CONNECTION)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
fn token_matches(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |diff, (x, y)| diff | (x ^ y))
            == 0
}
fn shell_quote(s: &str) -> String {
    #[cfg(windows)]
    {
        format!("'{}'", s.replace('\'', "''"))
    }
    #[cfg(not(windows))]
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl AgentService {
    pub fn start(directory: &Path, cli: PathBuf) -> Result<Self> {
        let accounting = crate::store::Store::open(directory)?;
        let policy = load_policy(directory)?;
        let service = Self {
            inner: Arc::new(Inner {
                directory: directory.to_owned(),
                cli,
                state: Mutex::new(State {
                    policy: Policy {
                        enabled: false,
                        ..policy.clone()
                    },
                    token: String::new(),
                }),
                accounting: Mutex::new(accounting),
                shutdown: AtomicBool::new(false),
                connections: AtomicUsize::new(0),
                on_change: Mutex::new(None),
            }),
        };
        if policy.enabled {
            service.configure(true, policy.access)?;
        } else {
            remove_connection(directory)?;
        }
        Ok(service)
    }
    pub fn status(&self) -> Result<Value> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| anyhow!("Agent service interrupted"))?;
        let connection = self.inner.directory.join(CONNECTION);
        let cli = &self.inner.cli;
        Ok(json!({
            "enabled": state.policy.enabled,
            "access": state.policy.access,
            "transport": "local",
            "cli_available": cli.is_file(),
            "cli_command": format!("{}{} --connection {}", if cfg!(windows) { "& " } else { "" }, shell_quote(&cli.to_string_lossy()), shell_quote(&connection.to_string_lossy())),
            "mcp_config": {"mcpServers":{"sarv":{"command":cli,"args":["--connection",connection,"mcp"]}}},
        }))
    }
    pub fn on_change(&self, handler: impl Fn() + Send + Sync + 'static) {
        if let Ok(mut callback) = self.inner.on_change.lock() {
            *callback = Some(Arc::new(handler));
        }
    }
    pub fn configure(&self, enabled: bool, access: Access) -> Result<Value> {
        // Serialize revocation with accepted calls: no previously queued call
        // can write after disable returns.
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| anyhow!("Agent service interrupted"))?;
        let policy = Policy {
            enabled,
            access,
            epoch: uuid::Uuid::new_v4().to_string(),
        };
        if !enabled {
            private_json(&self.inner.directory.join(POLICY), &policy)?;
            state.policy = policy;
            state.token.clear();
            remove_connection(&self.inner.directory)?;
        } else {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
            listener.set_nonblocking(true)?;
            let token = format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            );
            let connection = Connection {
                protocol: "sarv-local-v1".into(),
                address: listener.local_addr()?,
                token: token.clone(),
            };
            private_json(&self.inner.directory.join(POLICY), &policy)?;
            // A failed connection-file write leaves the service disabled.
            if let Err(e) = private_json(&self.inner.directory.join(CONNECTION), &connection) {
                state.policy = Policy::default();
                state.token.clear();
                private_json(&self.inner.directory.join(POLICY), &state.policy)?;
                return Err(e);
            }
            state.policy = policy.clone();
            state.token = token;
            let inner = self.inner.clone();
            thread::spawn(move || listen(inner, listener, policy.epoch));
        }
        drop(state);
        self.status()
    }
    /// Called only by the trusted native Settings UI (and isolated QA adapter).
    pub fn ui_dispatch(&self, action: &str, p: Value) -> Result<Value> {
        match action {
            "agent_status" => self.status(),
            "agent_configure" => {
                let enabled = p["enabled"]
                    .as_bool()
                    .ok_or_else(|| anyhow!("Enabled must be boolean"))?;
                let access = serde_json::from_value(p["access"].clone())?;
                self.configure(enabled, access)
            }
            _ => self.inner.dispatch(action, p),
        }
    }
}
impl Drop for AgentService {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::Release);
        // Keep the Settings preference; the next app launch issues a fresh grant.
        if let Ok(state) = self.inner.state.lock()
            && let Ok(policy) = load_policy(&self.inner.directory)
            && policy.epoch == state.policy.epoch
        {
            let _ = remove_connection(&self.inner.directory);
        }
    }
}
fn listen(inner: Arc<Inner>, listener: TcpListener, epoch: String) {
    loop {
        if inner.shutdown.load(Ordering::Acquire) {
            break;
        }
        let active = inner
            .state
            .lock()
            .is_ok_and(|s| s.policy.enabled && s.policy.epoch == epoch);
        if !active {
            break;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let count = inner.connections.fetch_add(1, Ordering::AcqRel);
                if count >= 8 {
                    inner.connections.fetch_sub(1, Ordering::AcqRel);
                    continue;
                }
                let worker = inner.clone();
                thread::spawn(move || {
                    let _ = serve(&worker, stream);
                    worker.connections.fetch_sub(1, Ordering::AcqRel);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20))
            }
            Err(_) => break,
        }
    }
}
fn read_message(reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    BufReader::new(reader.take(MAX_MESSAGE + 1)).read_until(b'\n', &mut bytes)?;
    ensure!(
        bytes.len() <= MAX_MESSAGE as usize && bytes.last() == Some(&b'\n'),
        "Invalid or oversized agent message"
    );
    Ok(bytes)
}
fn serve(inner: &Inner, mut stream: TcpStream) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut changed = false;
    let result = (|| {
        let request: Request = serde_json::from_slice(&read_message(&mut stream)?)?;
        let state = inner
            .state
            .lock()
            .map_err(|_| anyhow!("Agent service interrupted"))?;
        let disk = load_policy(&inner.directory)?;
        ensure!(
            !inner.shutdown.load(Ordering::Acquire) && state.policy.enabled && disk.enabled,
            "Agent access is off"
        );
        ensure!(
            disk.epoch == state.policy.epoch && token_matches(&request.token, &state.token),
            "Agent connection expired or unauthorized"
        );
        if request.action == "status" {
            return Ok(json!({"enabled":true,"access":disk.access,"transport":"local"}));
        }
        if request.action == "tools" {
            return Ok(tools());
        }
        let readonly = READ.contains(&request.action.as_str());
        ensure!(
            readonly || WRITE.contains(&request.action.as_str()),
            "Unsupported agent action"
        );
        ensure!(
            readonly || disk.access == Access::Full,
            "Read-only agent access"
        );
        ensure!(request.payload.is_object(), "Payload must be an object");
        let financial_post = matches!(
            request.action.as_str(),
            "opening_csv_post"
                | "receive_post"
                | "bill_post"
                | "fulfill_post"
                | "sale_return_post"
                | "purchase_return_post"
                | "transfer_post"
                | "journal_post"
                | "journal_reverse"
        );
        ensure!(
            !financial_post || request.payload["expected_generation"].as_str().is_some(),
            "Preview first and supply expected_generation"
        );
        let mut payload = request.payload;
        payload["_actor"] = json!("local-agent");
        let result = inner.dispatch(&request.action, payload);
        changed = result.is_ok() && !readonly;
        result
    })();
    if changed {
        let callback = inner.on_change.lock().ok().and_then(|c| c.clone());
        if let Some(callback) = callback {
            callback();
        }
    }
    let response = match result {
        Ok(result) => json!({"result":result}),
        Err(error) => json!({"error":{"code":"agent_error","message":error.to_string()}}),
    };
    serde_json::to_writer(&mut stream, &response)?;
    stream.write_all(b"\n")?;
    Ok(())
}

/// Client transport. Only IPv4 loopback is accepted, even in a modified file.
pub fn call(connection_path: &Path, action: &str, payload: Value) -> Result<Value> {
    let metadata =
        fs::metadata(connection_path).context("Agent access is off or the app is closed")?;
    ensure!(metadata.len() <= 4096, "Invalid agent connection file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "Agent connection file must be private (0600)"
        );
    }
    let connection: Connection = serde_json::from_slice(&fs::read(connection_path)?)?;
    ensure!(
        connection.protocol == "sarv-local-v1"
            && connection.address.ip() == IpAddr::V4(Ipv4Addr::LOCALHOST),
        "Agent connection must use local loopback"
    );
    let message =
        serde_json::to_vec(&json!({"token":connection.token,"action":action,"payload":payload}))?;
    ensure!(
        message.len() < MAX_MESSAGE as usize,
        "Agent message too large"
    );
    let mut stream = TcpStream::connect_timeout(&connection.address, Duration::from_secs(2))
        .context("Open Sarv to connect the agent")?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(&message)?;
    stream.write_all(b"\n")?;
    // Reports can be larger than requests; retain a bounded response.
    let mut response = Vec::new();
    BufReader::new(stream.take(16_000_001)).read_until(b'\n', &mut response)?;
    ensure!(
        response.len() <= 16_000_000 && response.last() == Some(&b'\n'),
        "Invalid agent response"
    );
    let result: Value = serde_json::from_slice(&response)?;
    if let Some(error) = result.get("error") {
        return Err(anyhow!(
            "{}",
            error["message"].as_str().unwrap_or("Agent request failed")
        ));
    }
    Ok(result["result"].clone())
}

/// Executable discovery; every desktop action is listed with JSON schemas.
pub fn tools() -> Value {
    Value::Array(READ.iter().chain(WRITE).map(|action| {
        let readonly = READ.contains(action);
        json!({"name":format!("sarv_{action}"),"description":description(action),"inputSchema":schema(action),"annotations":{"readOnlyHint":readonly,"destructiveHint":!readonly,"openWorldHint":false}})
    }).collect())
}
fn description(action: &str) -> String {
    let extra = match action {
        "workspace" => {
            "Discover company IDs, periods, accounts, resource IDs, settings, drafts and paginated source records. Omit company_id to list companies. All integers and money are strings."
        }
        "draft_preview" => {
            "Validate a raw sales draft without writes. Use supplied_total in draft_save, then draft_post with saved revision. Orders do not move stock; fulfill posts delivery and invoice."
        }
        "operation_options" => {
            "List remaining eligible line IDs and thousandth quantities for receive, bill, fulfill, sale_return or purchase_return. Selected payload quantities are decimal base-unit strings, not thousandths."
        }
        "settings" => {
            "Save complete presentation/document settings obtained from workspace. This cannot change agent access."
        }
        "journal_reverse" => {
            "Create an immutable linked reversal of a supported manual journal. Typed subledger journals must use their return workflow."
        }
        "initialize_zero" => {
            "Explicit confirmed=true declaration for a fresh company with zero opening balances. Existing businesses use opening CSV instead."
        }
        _ if action.ends_with("_preview") => {
            "Read-only validation. Review amounts, costs and journal lines. Copy _generation into the corresponding post's expected_generation and preserve the exact payload and request_key."
        }
        _ if action.ends_with("_post") => {
            "Persist the reviewed operation. Financial posts require expected_generation from preview. Reuse request_key only for an identical retry. Locked periods and all kernel validations apply."
        }
        _ => {
            "Use the same validated accounting operation as the desktop app. Money, quantities and revisions are strings; dates are ISO YYYY-MM-DD."
        }
    };
    format!("{action}: {extra}")
}
fn schema(action: &str) -> Value {
    let text = || json!({"type":"string"});
    let mut properties = serde_json::Map::new();
    let mut required: Vec<&str> = Vec::new();
    let context = [
        "company_id",
        "period_id",
        "source_id",
        "request_key",
        "date",
        "reason",
    ];
    let contextual = matches!(
        action,
        "resource"
            | "journal_preview"
            | "journal_post"
            | "purchase_preview"
            | "purchase_post"
            | "receive_preview"
            | "receive_post"
            | "bill_preview"
            | "bill_post"
            | "fulfill_preview"
            | "fulfill_post"
            | "sale_return_preview"
            | "sale_return_post"
            | "purchase_return_preview"
            | "purchase_return_post"
            | "transfer_preview"
            | "transfer_post"
    );
    if contextual {
        required.extend(context);
    }
    let fields: &[&str] = match action {
        "workspace" => &[],
        "report" | "report_csv" | "profit_csv" => &["company_id", "from", "to"],
        "detail" => &["company_id", "id"],
        "ledger" => &["company_id", "account_id", "from", "to"],
        "setup_company" => &["name", "fiscal_name", "start_date", "end_date"],
        "initialize_zero" => &["company_id", "source_id", "request_key", "confirmed"],
        "opening_csv_preview" | "opening_csv_post" => &["company_id", "request_key", "csv"],
        "resource" => &["kind", "id", "name"],
        "draft_preview" | "draft_save" => &["draft"],
        "draft_delete" => &["id", "revision"],
        "draft_post" => &["id", "revision", "request_key"],
        "purchase_preview" | "purchase_post" => &["supplier_id", "reference", "lines"],
        "receive_preview" | "receive_post" => &["order_id", "reference"],
        "fulfill_preview" | "fulfill_post" => &["order_id", "reference", "due_date"],
        "bill_preview" | "bill_post" => &["receipt_id", "reference", "due_date"],
        "sale_return_preview" | "sale_return_post" => &["record_id", "reference"],
        "purchase_return_preview" | "purchase_return_post" => &["record_id", "reference"],
        "operation_options" => &["company_id", "record_id", "operation"],
        "transfer_preview" | "transfer_post" => &[
            "item_id",
            "source_warehouse_id",
            "target_warehouse_id",
            "unit_id",
            "quantity",
        ],
        "journal_preview" | "journal_post" => &["lines"],
        "period_lock" => &["company_id", "period_id", "locked", "reason"],
        "journal_reverse" | "journal_reverse_preview" => &[
            "company_id",
            "journal_id",
            "period_id",
            "date",
            "request_key",
            "reason",
        ],
        _ => &[],
    };
    required.extend(fields);
    for key in &required {
        properties.insert((*key).into(), text());
    }
    for key in [
        "company_id",
        "offset",
        "expected_generation",
        "expected_revision",
        "reference",
        "due_date",
        "sku",
        "unit_id",
        "discount",
        "timezone",
    ] {
        properties.entry(key).or_insert_with(text);
    }
    if action == "resource" {
        properties.insert("indivisible".into(), json!({"type":"boolean"}));
        properties.insert(
            "kind".into(),
            json!({"type":"string","enum":["customer","vendor","unit","warehouse","item"]}),
        );
    }
    if action == "operation_options" {
        properties.insert("operation".into(), json!({"type":"string","enum":["receive","bill","fulfill","sale_return","purchase_return"]}));
    }
    for key in ["confirmed", "locked"] {
        if required.contains(&key) {
            properties.insert(key.into(), json!({"type":"boolean"}));
        }
    }
    if action == "draft_save" {
        properties.insert(
            "expected_revision".into(),
            json!({"type":["string","null"]}),
        );
    }
    if matches!(action, "draft_save" | "draft_preview") {
        properties.insert("draft".into(), draft_schema());
    }
    if contextual && !matches!(action, "resource" | "transfer_preview" | "transfer_post") {
        let fields: &[&str] = if action.starts_with("journal_") {
            &["account_id", "debit", "credit"]
        } else if action.starts_with("purchase_") && !action.starts_with("purchase_return_") {
            &[
                "id",
                "item_id",
                "warehouse_id",
                "unit_id",
                "quantity",
                "unit_price",
            ]
        } else {
            &["id", "quantity"]
        };
        let mut lp = serde_json::Map::new();
        for key in fields {
            lp.insert((*key).into(), text());
        }
        lp.insert("line_discount".into(), text());
        lp.insert("description".into(), text());
        properties.insert(
            "lines".into(),
            json!({"type":"array","items":{"type":"object","properties":lp,"required":fields}}),
        );
    }
    if matches!(
        action,
        "opening_csv_post"
            | "receive_post"
            | "bill_post"
            | "fulfill_post"
            | "sale_return_post"
            | "purchase_return_post"
            | "transfer_post"
            | "journal_post"
            | "journal_reverse"
    ) {
        required.push("expected_generation");
    }
    if action == "settings" {
        return json!({"type":"object","description":"Complete workspace.settings object, including all existing preference fields."});
    }
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn draft_schema() -> Value {
    let fields = [
        "id",
        "revision",
        "company_id",
        "period_id",
        "setup_id",
        "customer_id",
        "reference",
        "document_date",
        "financial_date",
        "date_display",
        "display_unit",
        "document_discount",
        "supplied_total",
        "reason",
    ];
    let mut p = serde_json::Map::new();
    for key in fields {
        p.insert(key.into(), json!({"type":"string"}));
    }
    p.insert(
        "date_display".into(),
        json!({"type":"string","enum":["Iso","Jalali"]}),
    );
    p.insert(
        "display_unit".into(),
        json!({"type":"string","enum":["Rial","Toman"]}),
    );
    p.insert("quotation".into(), json!({"type":["object","null"]}));
    let line_fields = [
        "id",
        "item_id",
        "warehouse_id",
        "unit_id",
        "quantity",
        "unit_price",
        "line_discount",
        "price_reference",
        "price_date",
        "override_reason",
    ];
    let lp: serde_json::Map<String, Value> = line_fields
        .iter()
        .map(|key| ((*key).into(), json!({"type":"string"})))
        .collect();
    p.insert(
        "lines".into(),
        json!({"type":"array","items":{"type":"object","properties":lp,"required":line_fields}}),
    );
    let mut required = fields.to_vec();
    required.push("lines");
    json!({"type":"object","properties":p,"required":required,"additionalProperties":false})
}

/// JSON-RPC MCP stdio adapter. stdout contains protocol messages only.
pub fn mcp(connection: &Path, input: impl BufRead, mut output: impl Write) -> Result<()> {
    let mut input = input;
    loop {
        let mut line = Vec::new();
        let n = input
            .by_ref()
            .take(MAX_MESSAGE + 1)
            .read_until(b'\n', &mut line)?;
        if n == 0 {
            break;
        }
        let parsed: Result<Value> = (|| {
            ensure!(line.len() <= MAX_MESSAGE as usize, "MCP message too large");
            Ok(serde_json::from_slice(&line)?)
        })();
        let request = match parsed {
            Ok(v) => v,
            Err(_) => {
                writeln!(
                    output,
                    "{}",
                    json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})
                )?;
                output.flush()?;
                if line.len() > MAX_MESSAGE as usize {
                    break;
                }
                continue;
            }
        };
        let Some(id) = request.get("id") else {
            continue;
        };
        let method = request["method"].as_str().unwrap_or("");
        let result: Result<Value> = match method {
            "initialize" => call(connection,"status",empty()).map(|_|json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"sarv","version":env!("CARGO_PKG_VERSION")},"instructions":"Use workspace to discover IDs. Preview financial operations before posting; copy _generation to expected_generation. All money and revisions are strings. Agent access is controlled only in native Settings."})),
            "ping" => call(connection,"status",empty()).map(|_|empty()),
            "tools/list" => call(connection,"tools",empty()).map(|v|json!({"tools":v})),
            "tools/call" => {
                let name = request["params"]["name"].as_str().unwrap_or("");
                let action = name.strip_prefix("sarv_").unwrap_or("");
                let payload = request["params"].get("arguments").cloned().unwrap_or_else(empty);
                Ok(match call(connection,action,payload) {
                    Ok(v) => json!({"content":[{"type":"text","text":serde_json::to_string(&v)?}],"isError":false}),
                    Err(e) => json!({"content":[{"type":"text","text":e.to_string()}],"isError":true}),
                })
            }
            _ => Err(anyhow!("Method not found")),
        };
        let response = match result {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err(e) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":if method.is_empty(){-32600}else if !matches!(method,"initialize"|"ping"|"tools/list"|"tools/call"){-32601}else{-32000},"message":e.to_string()}})
            }
        };
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}
