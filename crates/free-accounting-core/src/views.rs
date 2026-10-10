use super::*;
use std::collections::BTreeMap;

fn rows(conn: &Connection, sql: &str) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let names = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    Ok(stmt
        .query_map([], |r| {
            let mut object = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let value = match r.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(n) => {
                        if name.ends_with("Minor") {
                            Value::String(n.to_string())
                        } else {
                            json!(n)
                        }
                    }
                    rusqlite::types::ValueRef::Text(s) => {
                        Value::String(String::from_utf8_lossy(s).to_string())
                    }
                    _ => Value::Null,
                };
                object.insert(name.clone(), value);
            }
            Ok(Value::Object(object))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

pub(super) fn workspace(conn: &Connection, payload: &Value) -> Result<Value> {
    let s = settings(conn)?;
    let account_rows = rows(
        conn,
        r#"SELECT a.id,a.name,a.kind,a.currency,a.group_name AS 'group',a.archived,a.sort_order AS sortOrder,a.reconciled_through AS reconciledThrough,
       COALESCE(SUM(l.debit-l.credit),0) AS balanceMinor,
       COALESCE(SUM(CASE WHEN t.kind='opening' THEN l.debit-l.credit ELSE 0 END),0) AS openingMinor
       FROM accounts a LEFT JOIN journal_lines l ON l.account_id=a.id LEFT JOIN transactions t ON t.id=l.transaction_id
       WHERE a.system=0 GROUP BY a.id ORDER BY a.sort_order,a.name"#,
    )?;
    let mut totals = BTreeMap::<String, i64>::new();
    let mut cash = 0_i64;
    for a in &account_rows {
        let minor = a["balanceMinor"].as_str().unwrap().parse::<i64>()?;
        let currency = a["currency"].as_str().unwrap();
        let total = totals.entry(currency.into()).or_default();
        *total = total.checked_add(minor).context("Net worth overflow")?;
        if currency == s.currency && !matches!(a["kind"].as_str(), Some("loan" | "savings")) {
            cash = cash.checked_add(minor).context("Cash overflow")?;
        }
    }
    let totals = totals
        .into_iter()
        .map(|(currency, minor)| json!({"currency":currency,"minor":minor.to_string()}))
        .collect::<Vec<_>>();
    let mut contacts = rows(
        conn,
        "SELECT id,name,customer,supplier,phone,email,address,tax_id AS taxId,notes FROM contacts ORDER BY name COLLATE NOCASE",
    )?;
    let mut balance_query=conn.prepare("SELECT t.contact_id,a.currency,a.kind,COALESCE(SUM(CASE WHEN a.kind='receivable' THEN l.debit-l.credit ELSE l.credit-l.debit END),0) FROM journal_lines l JOIN accounts a ON a.id=l.account_id JOIN transactions t ON t.id=l.transaction_id WHERE a.kind IN ('receivable','payable') AND t.contact_id IS NOT NULL GROUP BY t.contact_id,a.currency,a.kind")?;
    let balance_rows = balance_query
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut lots_query=conn.prepare("SELECT t.contact_id,a.currency,a.kind,COALESCE(t.due_date,t.date),SUM(CASE WHEN a.kind='receivable' THEN l.debit-l.credit ELSE l.credit-l.debit END) FROM journal_lines l JOIN accounts a ON a.id=l.account_id JOIN transactions t ON t.id=l.transaction_id WHERE a.kind IN ('receivable','payable') AND t.kind IN ('income','expense') AND NOT EXISTS(SELECT 1 FROM transactions r WHERE r.reverses_id=t.id) GROUP BY t.id,a.kind ORDER BY COALESCE(t.due_date,t.date) DESC,t.seq DESC")?;
    let lots = lots_query
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut receivables = Vec::new();
    let mut payables = Vec::new();
    for contact in &mut contacts {
        let contact_id = contact["id"].as_str().unwrap().to_string();
        let mut balances = BTreeMap::<String, Value>::new();
        for (_, currency, kind, balance) in balance_rows.iter().filter(|b| b.0 == contact_id) {
            let mut buckets = [0_i64; 5];
            let mut remaining = (*balance).max(0);
            // Allocate settlements FIFO. Reading newest lots first leaves the
            // correct residual debt without adding a separate balance source.
            for (_, _, _, due, amount) in lots
                .iter()
                .filter(|l| l.0 == contact_id && l.1 == *currency && l.2 == *kind)
            {
                let outstanding = remaining.min((*amount).max(0));
                remaining -= outstanding;
                let due = NaiveDate::parse_from_str(due, "%Y-%m-%d")?;
                let days = (chrono::Local::now().date_naive() - due).num_days();
                let bucket = if days <= 0 {
                    0
                } else if days <= 30 {
                    1
                } else if days <= 60 {
                    2
                } else if days <= 90 {
                    3
                } else {
                    4
                };
                buckets[bucket] = buckets[bucket]
                    .checked_add(outstanding)
                    .context("Aging overflow")?;
            }
            let row=balances.entry(currency.clone()).or_insert_with(||json!({"currency":currency,"receivableMinor":"0","payableMinor":"0","receivableAging":["0","0","0","0","0"],"payableAging":["0","0","0","0","0"]}));
            row[format!("{kind}Minor")] = json!(balance.to_string());
            row[format!("{kind}Aging")] = json!(buckets.map(|b| b.to_string()));
            if *balance > 0 && *currency == s.currency {
                let item = json!({"id":contact_id,"name":contact["name"],"minor":balance.to_string(),"currency":currency});
                if kind == "receivable" {
                    receivables.push(item);
                } else {
                    payables.push(item);
                }
            }
        }
        contact["balances"] = json!(balances.into_values().collect::<Vec<_>>());
    }
    let range = payload
        .get("range")
        .and_then(Value::as_u64)
        .unwrap_or(30)
        .clamp(1, 3660) as i64;
    let end = payload
        .get("to")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .unwrap_or("");
    let end = if end.is_empty() {
        chrono::Local::now().date_naive()
    } else {
        valid_date(end)?;
        NaiveDate::parse_from_str(end, "%Y-%m-%d")?
    };
    let start = payload
        .get("from")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty());
    let start = if let Some(start) = start {
        valid_date(start)?;
        NaiveDate::parse_from_str(start, "%Y-%m-%d")?
    } else {
        end - chrono::Duration::days(range - 1)
    };
    ensure!(start <= end, "Date range is reversed");
    let days = (end - start).num_days() + 1;
    let previous_end = start - chrono::Duration::days(1);
    let previous_start = start - chrono::Duration::days(days);
    let summary = |from: NaiveDate, to: NaiveDate| -> Result<(i64, i64)> {
        Ok(conn.query_row("SELECT COALESCE(SUM(CASE WHEN a.kind='income' THEN l.credit-l.debit ELSE 0 END),0),COALESCE(SUM(CASE WHEN a.kind='expense' THEN l.debit-l.credit ELSE 0 END),0) FROM journal_lines l JOIN accounts a ON a.id=l.account_id JOIN transactions t ON t.id=l.transaction_id WHERE t.date BETWEEN ?1 AND ?2 AND t.currency=?3",params![from.to_string(),to.to_string(),s.currency],|r|Ok((r.get(0)?,r.get(1)?)))?)
    };
    let (income, expense) = summary(start, end)?;
    let (prev_income, prev_expense) = summary(previous_start, previous_end)?;
    let profit = income.checked_sub(expense).context("Profit overflow")?;
    let change = |current: i64, prev: i64| -> String {
        if prev == 0 {
            if current == 0 {
                "0%".into()
            } else {
                "New".into()
            }
        } else {
            let value = ((current as i128 - prev as i128) * 100) / (prev as i128).abs();
            format!("{value:+}%")
        }
    };
    let mut trend_query=conn.prepare("SELECT substr(t.date,1,7),COALESCE(SUM(CASE WHEN a.kind='income' THEN l.credit-l.debit ELSE 0 END),0),COALESCE(SUM(CASE WHEN a.kind='expense' THEN l.debit-l.credit ELSE 0 END),0) FROM journal_lines l JOIN accounts a ON a.id=l.account_id JOIN transactions t ON t.id=l.transaction_id WHERE t.date BETWEEN ?1 AND ?2 AND t.currency=?3 GROUP BY substr(t.date,1,7) ORDER BY 1")?;
    let trend_rows = trend_query
        .query_map(
            params![start.to_string(), end.to_string(), s.currency],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let max = trend_rows
        .iter()
        .flat_map(|r| [r.1.abs(), r.2.abs()])
        .max()
        .unwrap_or(1)
        .max(1) as i128;
    let trend=trend_rows.iter().map(|(month,i,e)|json!({"month":month,"incomeMinor":i.to_string(),"expenseMinor":e.to_string(),"incomeHeight":((*i as i128).abs()*100/max) as u32,"expenseHeight":((*e as i128).abs()*100/max) as u32})).collect::<Vec<_>>();
    let recent = ledger(
        conn,
        Filters {
            currency: s.currency.clone(),
            limit: 6,
            ..Default::default()
        },
    )?["rows"]
        .clone();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transactions WHERE state='posted'",
        [],
        |r| r.get(0),
    )?;
    let mut saved = rows(
        conn,
        "SELECT id,name,filters FROM saved_views ORDER BY name",
    )?;
    for row in &mut saved {
        row["filters"] = serde_json::from_str(row["filters"].as_str().unwrap())?;
    }
    let memories = rows(
        conn,
        "SELECT payee,json_extract(input_json,'$.categoryId') AS categoryId,json_extract(input_json,'$.accountId') AS accountId FROM transactions WHERE kind IN ('income','expense') GROUP BY payee HAVING seq=MAX(seq) ORDER BY seq DESC LIMIT 100",
    )?;
    let audits = rows(
        conn,
        "SELECT id,at,action,entity_id AS entityId,detail FROM audit_events ORDER BY id DESC LIMIT 80",
    )?;
    Ok(
        json!({"settings":s,"currencies":money::CURRENCIES.iter().map(|(code,places)|json!({"code":code,"decimals":places})).collect::<Vec<_>>(),"accounts":account_rows,"contacts":contacts,"categories":rows(conn,"SELECT id,name,kind FROM categories ORDER BY kind,name")?,"totals":totals,"savedViews":saved,"memories":memories,"audit":audits,"transactionCount":count,"dashboard":{"currency":s.currency,"from":start.to_string(),"to":end.to_string(),"cashMinor":cash.to_string(),"incomeMinor":income.to_string(),"expenseMinor":expense.to_string(),"profitMinor":profit.to_string(),"incomeChange":change(income,prev_income),"expenseChange":change(expense,prev_expense),"profitChange":change(profit,prev_income.checked_sub(prev_expense).context("Previous profit overflow")?),"previousIncomeMinor":prev_income.to_string(),"previousExpenseMinor":prev_expense.to_string(),"trend":trend,"receivables":receivables,"payables":payables,"recent":recent}}),
    )
}

pub(super) fn ledger(conn: &Connection, mut f: Filters) -> Result<Value> {
    if f.currency.is_empty() {
        f.currency = settings(conn)?.currency;
    }
    money::decimals(&f.currency)?;
    if !f.from.is_empty() {
        valid_date(&f.from)?;
    }
    if !f.to.is_empty() {
        valid_date(&f.to)?;
    }
    ensure!(
        f.from.is_empty() || f.to.is_empty() || f.from <= f.to,
        "Date range is reversed"
    );
    ensure!(f.offset <= 10_000_000, "Offset exceeds supported range");
    let limit = if f.limit == 0 { 150 } else { f.limit.min(500) };
    let amount_search = money::parse(&f.search, &f.currency)
        .ok()
        .map(|n| n.abs().to_string())
        .unwrap_or_default();
    // Only the 150 visible documents receive expensive names/JSON projection.
    // The balance window processes narrow numeric rows, not 100k whole views.
    let predicate = r#"t.state='posted' AND t.currency=?3
      AND (?1='' OR EXISTS(SELECT 1 FROM journal_lines WHERE transaction_id=t.id AND account_id=?1))
      AND (?2='' OR EXISTS(SELECT 1 FROM journal_lines WHERE transaction_id=t.id AND category_id=?2))
      AND (?4='' OR t.date>=?4) AND (?5='' OR t.date<=?5)
      AND (?6='' OR t.contact_id=?6)
      AND (?7='' OR (CASE WHEN t.kind='reversal' THEN 'reversal' WHEN EXISTS(SELECT 1 FROM transactions rev WHERE rev.reverses_id=t.id) THEN 'reversed' ELSE 'posted' END)=?7)
      AND (?8='' OR instr(lower(t.payee||' '||t.memo||' '||t.tags),lower(?8))>0 OR (?9!='' AND CAST(t.amount_minor AS TEXT)=?9))"#;
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM transactions t WHERE {predicate}"),
        params![
            f.account,
            f.category,
            f.currency,
            f.from,
            f.to,
            f.contact,
            f.status,
            f.search,
            amount_search
        ],
        |r| r.get(0),
    )?;
    let sql = format!(
        r#"
      WITH candidates AS MATERIALIZED (
        SELECT t.* FROM transactions t WHERE {predicate} ORDER BY t.date DESC,t.seq DESC LIMIT ?10 OFFSET ?11
      ), deltas AS (
        SELECT l.transaction_id,SUM(l.debit-l.credit) AS delta FROM journal_lines l JOIN accounts a ON a.id=l.account_id
        WHERE a.system=0 AND a.currency=?3 AND (?1='' OR a.id=?1) GROUP BY l.transaction_id
      ), running AS (
        SELECT t.id,COALESCE(d.delta,0) AS delta,SUM(COALESCE(d.delta,0)) OVER (ORDER BY t.date,t.seq ROWS UNBOUNDED PRECEDING) AS balance
        FROM transactions t LEFT JOIN deltas d ON d.transaction_id=t.id WHERE t.currency=?3 AND t.state='posted'
      ) SELECT c.*,r.delta,r.balance,
        (SELECT GROUP_CONCAT(DISTINCT a.name) FROM journal_lines l JOIN accounts a ON a.id=l.account_id WHERE l.transaction_id=c.id AND a.system=0) AS account_names,
        (SELECT GROUP_CONCAT(DISTINCT cat.name) FROM journal_lines l JOIN categories cat ON cat.id=l.category_id WHERE l.transaction_id=c.id) AS category_names,
        CASE WHEN c.kind='reversal' THEN 'reversal' WHEN EXISTS(SELECT 1 FROM transactions rev WHERE rev.reverses_id=c.id) THEN 'reversed' ELSE 'posted' END AS status
        FROM candidates c JOIN running r ON r.id=c.id ORDER BY c.date DESC,c.seq DESC
    "#
    );
    let mut q = conn.prepare(&sql)?;
    let mut query = q.query(params![
        f.account,
        f.category,
        f.currency,
        f.from,
        f.to,
        f.contact,
        f.status,
        f.search,
        amount_search,
        limit,
        f.offset
    ])?;
    let mut result = Vec::new();
    while let Some(r) = query.next()? {
        let kind: String = r.get("kind")?;
        let amount: i64 = r.get("amount_minor")?;
        let delta: i64 = r.get("delta")?;
        let signed = if kind == "expense" {
            -amount
        } else if matches!(kind.as_str(), "income" | "opening") {
            amount
        } else {
            delta
        };
        result.push(json!({"id":r.get::<_,String>("id")?,"date":r.get::<_,String>("date")?,"kind":kind,"payee":r.get::<_,String>("payee")?,"memo":r.get::<_,String>("memo")?,"tags":r.get::<_,String>("tags")?,"category":r.get::<_,Option<String>>("category_names")?.unwrap_or_else(||if kind=="opening"{"Opening balance".into()}else if kind=="payment"{"Contact payment".into()}else{"Transfer".into()}),"account":r.get::<_,Option<String>>("account_names")?.unwrap_or_else(||"Unpaid".into()),"currency":r.get::<_,String>("currency")?,"amountMinor":signed.to_string(),"paidMinor":r.get::<_,i64>("paid_minor")?.to_string(),"movementMinor":delta.to_string(),"balanceMinor":r.get::<_,i64>("balance")?.to_string(),"status":r.get::<_,String>("status")?,"recordedAt":r.get::<_,String>("recorded_at")?}));
    }
    Ok(
        json!({"rows":result,"total":total,"offset":f.offset,"nextOffset":if result.len()==limit as usize && ((f.offset as i64)+(limit as i64))<total{Some(f.offset+limit)}else{None},"balanceCurrency":f.currency}),
    )
}

pub(super) fn account(conn: &Connection, account_id: &str) -> Result<Value> {
    let currency: String = conn.query_row(
        "SELECT currency FROM accounts WHERE id=?1 AND system=0",
        [account_id],
        |r| r.get(0),
    )?;
    let mut q=conn.prepare("SELECT t.date,SUM(l.debit-l.credit) AS delta FROM journal_lines l JOIN transactions t ON t.id=l.transaction_id WHERE l.account_id=?1 GROUP BY t.date ORDER BY t.date")?;
    let points = q
        .query_map([account_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut balance = 0_i64;
    let mut history = Vec::new();
    for (date, delta) in points {
        balance = balance.checked_add(delta).context("Balance overflow")?;
        history.push(json!({"date":date,"minor":balance.to_string()}));
    }
    Ok(
        json!({"currency":currency,"history":history,"recent":ledger(conn,Filters{account:account_id.into(),currency,limit:20,..Default::default()})?}),
    )
}

pub(super) fn detail(conn: &Connection, tx_id: &str) -> Result<Value> {
    let (input,currency,amount,paid,recorded,reverses,replaces):(String,String,i64,i64,String,Option<String>,Option<String>)=conn.query_row("SELECT input_json,currency,amount_minor,paid_minor,recorded_at,reverses_id,replaces_id FROM transactions WHERE id=?1",[tx_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).context("Transaction not found")?;
    let input: TransactionInput = serde_json::from_str(&input)?;
    let reversed = optional_string(
        conn,
        "SELECT id FROM transactions WHERE reverses_id=?1",
        tx_id,
    )?;
    let mut lines_query=conn.prepare("SELECT a.name,a.kind,l.debit,l.credit,c.name FROM journal_lines l JOIN accounts a ON a.id=l.account_id LEFT JOIN categories c ON c.id=l.category_id WHERE transaction_id=?1 ORDER BY l.id")?;
    let lines=lines_query.query_map([tx_id],|r|Ok(json!({"account":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"debitMinor":r.get::<_,i64>(2)?.to_string(),"creditMinor":r.get::<_,i64>(3)?.to_string(),"category":r.get::<_,Option<String>>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut attachment_query =
        conn.prepare("SELECT id,name,mime,length(data) FROM attachments WHERE transaction_id=?1")?;
    let attachments=attachment_query.query_map([tx_id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"mime":r.get::<_,String>(2)?,"size":r.get::<_,i64>(3)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut history_query=conn.prepare("SELECT id,at,action,entity_id,detail FROM audit_events WHERE entity_id=?1 OR entity_id IN (SELECT id FROM transactions WHERE reverses_id=?1 OR replaces_id=?1) ORDER BY id")?;
    let history=history_query.query_map([tx_id],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"at":r.get::<_,String>(1)?,"action":r.get::<_,String>(2)?,"entityId":r.get::<_,String>(3)?,"detail":r.get::<_,String>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut balance_history = Vec::new();
    if !input.account_id.is_empty() {
        let mut q=conn.prepare("SELECT t.date,SUM(l.debit-l.credit) AS delta FROM journal_lines l JOIN transactions t ON t.id=l.transaction_id WHERE l.account_id=?1 GROUP BY t.date ORDER BY t.date")?;
        let points = q
            .query_map([&input.account_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut balance = 0_i64;
        for (date, delta) in points {
            balance = balance.checked_add(delta).context("Balance overflow")?;
            balance_history.push(json!({"date":date,"minor":balance.to_string()}));
        }
    }
    Ok(
        json!({"id":tx_id,"input":input,"currency":currency,"amountMinor":amount.to_string(),"paidMinor":paid.to_string(),"recordedAt":recorded,"reverses":reverses,"replaces":replaces,"reversedBy":reversed,"lines":lines,"attachments":attachments,"history":history,"balanceHistory":balance_history}),
    )
}

pub(super) fn attachment(conn: &Connection, attachment_id: &str) -> Result<Value> {
    Ok(conn.query_row("SELECT name,mime,data FROM attachments WHERE id=?1",[attachment_id],|r|Ok(json!({"name":r.get::<_,String>(0)?,"mime":r.get::<_,String>(1)?,"bytes":r.get::<_,Vec<u8>>(2)?})))?)
}

pub(super) fn export(conn: &Connection, payload: &Value) -> Result<Value> {
    let scope = text(payload, "scope")?;
    if matches!(scope, "ledger_pdf" | "statement_pdf") {
        return super::pdf::export_pdf(conn, payload);
    }
    if scope == "backup" {
        // Open, complete JSON export including both sides and the immutable audit.
        let tables = [
            "accounts",
            "contacts",
            "categories",
            "transactions",
            "journal_lines",
            "settings",
            "saved_views",
            "audit_events",
        ];
        let mut output = serde_json::Map::new();
        for table in tables {
            output.insert(
                table.into(),
                json!(rows(conn, &format!("SELECT * FROM {table}"))?),
            );
        }
        let mut q = conn.prepare("SELECT id,transaction_id,name,mime,data FROM attachments")?;
        let attachments=q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"transaction_id":r.get::<_,String>(1)?,"name":r.get::<_,String>(2)?,"mime":r.get::<_,String>(3)?,"bytes":r.get::<_,Vec<u8>>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        output.insert("attachments".into(), json!(attachments));
        output.insert("schemaVersion".into(), json!(1));
        // Integer amounts are explicitly strings even in exports.
        for name in ["journal_lines", "transactions"] {
            if let Some(Value::Array(rows)) = output.get_mut(name) {
                for r in rows {
                    for key in ["debit", "credit", "amount_minor", "paid_minor"] {
                        if let Some(n) = r.get(key).and_then(Value::as_i64) {
                            r[key] = json!(n.to_string());
                        }
                    }
                }
            }
        }
        return Ok(
            json!({"name":format!("sarv-{}.json",today()),"mime":"application/json","content":serde_json::to_string_pretty(&output)?}),
        );
    }
    if scope == "contacts" {
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer.write_record([
            "name", "customer", "supplier", "phone", "email", "address", "tax_id", "notes",
        ])?;
        let mut q=conn.prepare("SELECT name,customer,supplier,phone,email,address,tax_id,notes FROM contacts ORDER BY name")?;
        let values = q
            .query_map([], |r| {
                Ok(vec![
                    r.get::<_, String>(0)?,
                    r.get::<_, bool>(1)?.to_string(),
                    r.get::<_, bool>(2)?.to_string(),
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ])
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for row in values {
            writer.write_record(row.iter().map(|s| safe_cell(s)))?;
        }
        return Ok(
            json!({"name":"contacts.csv","mime":"text/csv;charset=utf-8","content":String::from_utf8(writer.into_inner()?)?}),
        );
    }
    ensure!(scope == "ledger", "Unknown export scope");
    let mut f: Filters =
        serde_json::from_value(payload.get("filters").cloned().unwrap_or(json!({})))?;
    f.offset = 0;
    f.limit = 500;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "id",
        "date",
        "kind",
        "payee",
        "category",
        "account",
        "currency",
        "amount_minor",
        "movement_minor",
        "balance_minor",
        "status",
        "memo",
        "tags",
    ])?;
    loop {
        let page = ledger(conn, f.clone())?;
        for row in page["rows"].as_array().unwrap() {
            let cells = [
                "id",
                "date",
                "kind",
                "payee",
                "category",
                "account",
                "currency",
                "amountMinor",
                "movementMinor",
                "balanceMinor",
                "status",
                "memo",
                "tags",
            ]
            .map(|key| safe_cell(row[key].as_str().unwrap_or("")));
            writer.write_record(cells)?;
        }
        if let Some(next) = page["nextOffset"].as_u64() {
            f.offset = next as u32;
        } else {
            break;
        }
    }
    Ok(
        json!({"name":format!("ledger-{}.csv",today()),"mime":"text/csv;charset=utf-8","content":String::from_utf8(writer.into_inner()?)?}),
    )
}

fn safe_cell(s: &str) -> String {
    if s.trim_start().starts_with(['=', '+', '@'])
        || (s.trim_start().starts_with('-') && s.parse::<i64>().is_err())
    {
        format!("'{s}")
    } else {
        s.into()
    }
}
