use super::*;

pub(super) fn dispatch(conn: &Connection, action: &str, payload: Value) -> Result<Value> {
    match action {
        "transaction" => {
            let input: TransactionInput = serde_json::from_value(payload)?;
            let result = create_transaction(conn, input)?;
            Ok(json!({"id":result}))
        }
        "reverse" => Ok(
            json!({"id":reverse(conn,text(&payload,"id")?,text(&payload,"date")?,text(&payload,"reason")?)?}),
        ),
        "account" => save_account(conn, serde_json::from_value(payload)?),
        "contact" => save_contact(conn, serde_json::from_value(payload)?),
        "settings" => update_settings(conn, serde_json::from_value(payload)?),
        "reconcile" => reconcile(conn, &payload),
        "settle" => settle(conn, &payload),
        "categorize" => categorize(conn, &payload),
        "merge" => merge(conn, &payload),
        "saved_view" => {
            let name = text(&payload, "name")?.trim();
            ensure!(!name.is_empty() && name.len() <= 100, "Enter a view name");
            let filters: Filters = serde_json::from_value(payload["filters"].clone())?;
            let view_id = id();
            conn.execute(
                "INSERT INTO saved_views VALUES(?1,?2,?3)",
                params![view_id, name, serde_json::to_string(&filters)?],
            )?;
            audit(conn, "view.saved", &view_id, &payload)?;
            Ok(json!({"id":view_id}))
        }
        "delete_view" => {
            let view_id = text(&payload, "id")?;
            conn.execute("DELETE FROM saved_views WHERE id=?1", [view_id])?;
            audit(conn, "view.removed", view_id, &payload)?;
            Ok(json!({}))
        }
        "category" => {
            let name = text(&payload, "name")?.trim();
            let kind = text(&payload, "kind")?;
            ensure!(
                !name.is_empty() && name.len() <= 100 && matches!(kind, "income" | "expense"),
                "Invalid category"
            );
            let category_id = id();
            conn.execute(
                "INSERT INTO categories VALUES(?1,?2,?3)",
                params![category_id, name, kind],
            )?;
            audit(conn, "category.created", &category_id, &payload)?;
            Ok(json!({"id":category_id}))
        }
        "import_contacts" => import_contacts(conn, text(&payload, "csv")?),
        "import_transactions" => import_transactions(conn, text(&payload, "csv")?),
        "demo" => demo(conn),
        _ => anyhow::bail!("Unknown mutation: {action}"),
    }
}

pub(super) fn create_transaction(conn: &Connection, mut input: TransactionInput) -> Result<String> {
    ensure!(
        matches!(input.kind.as_str(), "income" | "expense" | "transfer"),
        "Choose income, expense, or transfer"
    );
    input.payee = input.payee.trim().to_string();
    ensure!(
        !input.payee.is_empty() && input.payee.chars().count() <= 160,
        "Enter a payee (up to 160 characters)"
    );
    ensure!(
        input.memo.len() <= 4000 && input.tags.len() <= 500 && input.splits.len() <= 50,
        "Transaction is too large"
    );
    let (_, currency) = wallet(conn, &input.account_id)?;
    let amount = money::parse(&input.amount, &currency)?;
    ensure!(amount > 0, "Amount must be positive");
    let paid = if input.paid_amount.trim().is_empty() {
        amount
    } else {
        money::parse(&input.paid_amount, &currency)?
    };
    ensure!(
        (0..=amount).contains(&paid),
        "Paid amount must be between zero and the total"
    );
    unlocked(conn, &input.date)?;
    if !input.due_date.is_empty() {
        valid_date(&input.due_date)?;
        ensure!(
            input.due_date >= input.date,
            "Due date cannot precede the transaction"
        );
    }
    if !input.contact_id.is_empty() {
        ensure!(
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM contacts WHERE id=?1)",
                [&input.contact_id],
                |r| r.get::<_, bool>(0)
            )?,
            "Contact does not exist"
        );
    }
    let mut lines = Vec::new();
    if input.kind == "transfer" {
        ensure!(
            paid == amount && input.splits.is_empty(),
            "Transfers cannot have a partial payment or category splits"
        );
        let (_, target_currency) = wallet(conn, &input.to_account_id)?;
        ensure!(
            input.account_id != input.to_account_id,
            "Choose different source and destination accounts"
        );
        ensure!(
            currency == target_currency,
            "Transfers require matching currencies. Foreign exchange belongs to a later phase."
        );
        lines.push(line(input.account_id.clone(), -amount, None));
        lines.push(line(input.to_account_id.clone(), amount, None));
    } else {
        let expense = input.kind == "expense";
        if paid > 0 {
            lines.push(line(
                input.account_id.clone(),
                if expense { -paid } else { paid },
                None,
            ));
        }
        if amount > paid {
            ensure!(
                !input.contact_id.is_empty(),
                "Select a contact for an unpaid balance"
            );
            let system = system_account(
                conn,
                if expense { "payable" } else { "receivable" },
                &currency,
                "",
            )?;
            lines.push(line(
                system,
                if expense {
                    -(amount - paid)
                } else {
                    amount - paid
                },
                None,
            ));
            if input.due_date.is_empty() {
                input.due_date = input.date.clone();
            }
        }
        let splits = if input.splits.is_empty() {
            vec![Split {
                category_id: input.category_id.clone(),
                amount: input.amount.clone(),
            }]
        } else {
            input.splits.clone()
        };
        let mut sum = 0_i64;
        for split in splits {
            let split_amount = money::parse(&split.amount, &currency)?;
            ensure!(split_amount > 0, "Every split must have a positive amount");
            sum = sum
                .checked_add(split_amount)
                .context("Split amount overflow")?;
            let category_kind: String = conn
                .query_row(
                    "SELECT kind FROM categories WHERE id=?1",
                    [&split.category_id],
                    |r| r.get(0),
                )
                .context("Select a category for every split")?;
            ensure!(
                category_kind == input.kind,
                "Category does not match the transaction type"
            );
            let account = system_account(conn, &input.kind, &currency, &split.category_id)?;
            lines.push(line(
                account,
                if expense { split_amount } else { -split_amount },
                Some(split.category_id),
            ));
        }
        ensure!(
            sum == amount,
            "Category splits must sum exactly to the transaction total"
        );
    }
    let replaces = if input.correction_id.is_empty() {
        None
    } else {
        Some(input.correction_id.clone())
    };
    if let Some(old) = &replaces {
        let correction_date = if input.correction_date.is_empty() {
            input.date.clone()
        } else {
            input.correction_date.clone()
        };
        ensure!(
            correction_date == input.date,
            "The reversal and replacement must use the same correction date"
        );
        reverse(
            conn,
            old,
            &correction_date,
            "Correction: reverse and repost",
        )?;
    }
    let new_id = post(
        conn,
        &input,
        &currency,
        amount,
        paid,
        lines,
        None,
        replaces.as_deref(),
    )?;
    if input.receipt.is_none()
        && let Some(old) = &replaces
    {
        let mut q =
            conn.prepare("SELECT name,mime,data FROM attachments WHERE transaction_id=?1")?;
        let rows = q
            .query_map([old], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (name, mime, data) in rows {
            conn.execute(
                "INSERT INTO attachments VALUES(?1,?2,?3,?4,?5)",
                params![id(), new_id, name, mime, data],
            )?;
        }
    }
    let mut s = settings(conn)?;
    s.last_account = input.account_id;
    save_settings(conn, &s)?;
    Ok(new_id)
}

fn save_account(conn: &Connection, input: AccountInput) -> Result<Value> {
    ensure!(
        !input.name.trim().is_empty() && input.name.len() <= 160,
        "Enter an account name"
    );
    ensure!(
        matches!(
            input.kind.as_str(),
            "cash" | "bank" | "card" | "wallet" | "savings" | "loan"
        ),
        "Invalid account type"
    );
    money::decimals(&input.currency)?;
    ensure!(input.group.len() <= 100, "Group is too long");
    let account_id = if input.id.is_empty() {
        id()
    } else {
        input.id.clone()
    };
    let duplicate: bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE system=0 AND name=?1 COLLATE NOCASE AND id!=?2)",params![input.name.trim(),account_id],|r|r.get(0))?;
    ensure!(!duplicate, "An account with this name already exists");
    if input.id.is_empty() {
        conn.execute("INSERT INTO accounts(id,name,kind,currency,group_name,archived,sort_order,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![account_id,input.name.trim(),input.kind,input.currency,input.group,input.archived,input.sort_order,now()])?;
        let opening = if input.opening_balance.trim().is_empty() {
            0
        } else {
            money::parse(&input.opening_balance, &input.currency)?
        };
        if opening != 0 {
            let equity = system_account(conn, "equity", &input.currency, "")?;
            let date = if input.opening_date.is_empty() {
                today()
            } else {
                input.opening_date.clone()
            };
            let opening_input = TransactionInput {
                date,
                kind: "opening".into(),
                payee: "Opening balance".into(),
                memo: input.name.clone(),
                account_id: account_id.clone(),
                ..Default::default()
            };
            post(
                conn,
                &opening_input,
                &input.currency,
                opening.abs(),
                opening.abs(),
                vec![
                    line(account_id.clone(), opening, None),
                    line(equity, -opening, None),
                ],
                None,
                None,
            )?;
        }
    } else {
        ensure!(
            input.opening_balance.trim().is_empty(),
            "Opening balances are journal entries; use a correction rather than editing them"
        );
        let affected=conn.execute("UPDATE accounts SET name=?1,kind=?2,currency=?3,group_name=?4,archived=?5,sort_order=?6 WHERE id=?7 AND system=0",params![input.name.trim(),input.kind,input.currency,input.group,input.archived,input.sort_order,account_id])?;
        ensure!(affected == 1, "Account does not exist");
    }
    audit(
        conn,
        if input.id.is_empty() {
            "account.created"
        } else {
            "account.updated"
        },
        &account_id,
        &serde_json::to_value(&input)?,
    )?;
    Ok(json!({"id":account_id}))
}

fn save_contact(conn: &Connection, input: ContactInput) -> Result<Value> {
    ensure!(
        !input.name.trim().is_empty() && input.name.len() <= 200,
        "Enter a contact name"
    );
    ensure!(
        input.customer || input.supplier,
        "Choose customer, supplier, or both"
    );
    ensure!(
        input.notes.len() <= 4000
            && input.email.len() <= 250
            && input.address.len() <= 1000
            && input.phone.len() <= 100
            && input.tax_id.len() <= 100,
        "Contact field is too long"
    );
    if !input.email.is_empty() {
        ensure!(input.email.contains('@'), "Enter a valid email address");
    }
    let contact_id = if input.id.is_empty() {
        id()
    } else {
        input.id.clone()
    };
    if !input.id.is_empty() {
        ensure!(
            optional_string(conn, "SELECT id FROM contacts WHERE id=?1", &contact_id)?.is_some(),
            "Contact does not exist"
        );
    }
    conn.execute("INSERT INTO contacts VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET name=excluded.name,customer=excluded.customer,supplier=excluded.supplier,phone=excluded.phone,email=excluded.email,address=excluded.address,tax_id=excluded.tax_id,notes=excluded.notes",params![contact_id,input.name.trim(),input.customer,input.supplier,input.phone,input.email,input.address,input.tax_id,input.notes])?;
    audit(
        conn,
        if input.id.is_empty() {
            "contact.created"
        } else {
            "contact.updated"
        },
        &contact_id,
        &serde_json::to_value(&input)?,
    )?;
    Ok(json!({"id":contact_id}))
}

fn update_settings(conn: &Connection, s: Settings) -> Result<Value> {
    ensure!(
        !s.profile.trim().is_empty() && s.profile.len() <= 200,
        "Enter a profile name"
    );
    money::decimals(&s.currency)?;
    ensure!(
        (1..=12).contains(&s.fiscal_month),
        "Fiscal start month must be between 1 and 12"
    );
    ensure!(
        matches!(s.calendar.as_str(), "gregorian" | "jalali")
            && matches!(s.language.as_str(), "en" | "fa")
            && matches!(s.direction.as_str(), "ltr" | "rtl")
            && matches!(s.digits.as_str(), "latin" | "persian")
            && matches!(s.theme.as_str(), "light" | "dark" | "system")
            && matches!(s.mode.as_str(), "personal" | "business")
            && matches!(s.date_format.as_str(), "medium" | "iso" | "short")
            && matches!(s.tax_format.as_str(), "exclusive" | "inclusive"),
        "Invalid localization settings"
    );
    ensure!(
        s.accent.len() == 7
            && s.accent.starts_with('#')
            && s.accent[1..].chars().all(|c| c.is_ascii_hexdigit()),
        "Accent must be a six-digit hex color"
    );
    ensure!(
        s.logo.len() <= 1_500_000
            && (s.logo.is_empty()
                || s.logo.starts_with("data:image/png;base64,")
                || s.logo.starts_with("data:image/jpeg;base64,")),
        "Use a PNG or JPEG logo under 1 MB"
    );
    let old = settings(conn)?;
    if !s.closed_through.is_empty() {
        valid_date(&s.closed_through)?;
        ensure!(s.closed_through <= today(), "Cannot close a future period");
    }
    ensure!(
        old.closed_through.is_empty() || s.closed_through >= old.closed_through,
        "Closed periods cannot be reopened in Phase 1"
    );
    let allowed = ["trend", "balances", "recent", "upcoming"];
    ensure!(
        s.widgets.len() == 4
            && allowed
                .iter()
                .all(|w| s.widgets.iter().filter(|v| v.as_str() == *w).count() == 1)
            && s.hidden_widgets
                .iter()
                .all(|w| allowed.contains(&w.as_str())),
        "Invalid widget layout"
    );
    save_settings(conn, &s)?;
    audit(
        conn,
        "settings.updated",
        "profile",
        &json!({"before":old,"after":s}),
    )?;
    Ok(serde_json::to_value(s)?)
}

fn reconcile(conn: &Connection, payload: &Value) -> Result<Value> {
    let account = text(payload, "accountId")?;
    let date = text(payload, "date")?;
    valid_date(date)?;
    ensure!(date <= today().as_str(), "Cannot reconcile a future date");
    let (_, currency) = wallet(conn, account)?;
    let statement = money::parse(text(payload, "balance")?, &currency)?;
    let actual: i64=conn.query_row("SELECT COALESCE(SUM(l.debit-l.credit),0) FROM journal_lines l JOIN transactions t ON t.id=l.transaction_id WHERE l.account_id=?1 AND t.date<=?2 AND t.state='posted'",params![account,date],|r|r.get(0))?;
    ensure!(
        statement == actual,
        "Statement does not match the ledger ({} minor units). Add a correction first.",
        actual
    );
    conn.execute(
        "UPDATE accounts SET reconciled_through=?1 WHERE id=?2",
        params![date, account],
    )?;
    audit(
        conn,
        "account.reconciled",
        account,
        &json!({"through":date,"balance":actual.to_string()}),
    )?;
    Ok(json!({}))
}

fn settle(conn: &Connection, payload: &Value) -> Result<Value> {
    let contact = text(payload, "contactId")?;
    let account = text(payload, "accountId")?;
    let direction = text(payload, "direction")?;
    let date = text(payload, "date")?;
    ensure!(
        matches!(direction, "receivable" | "payable"),
        "Invalid settlement direction"
    );
    let (_, currency) = wallet(conn, account)?;
    let amount = money::parse(text(payload, "amount")?, &currency)?;
    let outstanding: i64=conn.query_row("SELECT COALESCE(SUM(CASE WHEN a.kind='receivable' THEN l.debit-l.credit ELSE l.credit-l.debit END),0) FROM journal_lines l JOIN accounts a ON a.id=l.account_id JOIN transactions t ON t.id=l.transaction_id WHERE t.contact_id=?1 AND a.kind=?2 AND a.currency=?3",params![contact,direction,currency],|r|r.get(0))?;
    ensure!(
        amount > 0 && amount <= outstanding,
        "Settlement must be positive and cannot exceed the outstanding balance in this currency"
    );
    let name: String = conn.query_row("SELECT name FROM contacts WHERE id=?1", [contact], |r| {
        r.get(0)
    })?;
    let receive = direction == "receivable";
    let system = system_account(conn, direction, &currency, "")?;
    let input = TransactionInput {
        date: date.into(),
        kind: "payment".into(),
        payee: name,
        contact_id: contact.into(),
        account_id: account.into(),
        amount: text(payload, "amount")?.into(),
        memo: if receive {
            "Payment received"
        } else {
            "Payment sent"
        }
        .into(),
        ..Default::default()
    };
    let tx_id = post(
        conn,
        &input,
        &currency,
        amount,
        amount,
        vec![
            line(account.into(), if receive { amount } else { -amount }, None),
            line(system, if receive { -amount } else { amount }, None),
        ],
        None,
        None,
    )?;
    Ok(json!({"id":tx_id}))
}

fn selected(payload: &Value) -> Result<Vec<String>> {
    let ids: Vec<String> = serde_json::from_value(payload["ids"].clone())?;
    ensure!(
        !ids.is_empty() && ids.len() <= 200,
        "Select between 1 and 200 transactions"
    );
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    ensure!(unique.len() == ids.len(), "Duplicate selection IDs");
    Ok(ids)
}
fn categorize(conn: &Connection, payload: &Value) -> Result<Value> {
    let ids = selected(payload)?;
    let category = text(payload, "categoryId")?;
    let date = text(payload, "date")?;
    for tx_id in &ids {
        is_active(conn, tx_id)?;
        let raw: String = conn.query_row(
            "SELECT input_json FROM transactions WHERE id=?1",
            [tx_id],
            |r| r.get(0),
        )?;
        let mut input: TransactionInput = serde_json::from_str(&raw)?;
        ensure!(
            input.splits.is_empty(),
            "Edit category splits in the detail drawer"
        );
        input.category_id = category.into();
        input.correction_id = tx_id.clone();
        input.correction_date = date.into();
        input.date = date.into();
        create_transaction(conn, input)?;
    }
    Ok(json!({"count":ids.len()}))
}
fn merge(conn: &Connection, payload: &Value) -> Result<Value> {
    let ids = selected(payload)?;
    ensure!(ids.len() >= 2, "Select at least two duplicate transactions");
    let date = text(payload, "date")?;
    let mut fingerprint = None;
    for (i, tx_id) in ids.iter().enumerate() {
        is_active(conn, tx_id)?;
        let raw: String = conn.query_row(
            "SELECT input_json FROM transactions WHERE id=?1",
            [tx_id],
            |r| r.get(0),
        )?;
        let input: TransactionInput = serde_json::from_str(&raw)?;
        let key = json!([
            input.date,
            input.kind,
            input.amount,
            input.paid_amount,
            input.account_id,
            input.to_account_id,
            input.contact_id,
            input.category_id,
            input.payee,
            input.splits,
            input.due_date
        ]);
        if let Some(first) = &fingerprint {
            ensure!(
                first == &key,
                "Selected transactions are not exact economic duplicates"
            );
        } else {
            fingerprint = Some(key);
        }
        if i > 0 {
            reverse(conn, tx_id, date, &format!("Duplicate of {}", ids[0]))?;
        }
    }
    audit(conn, "duplicates.merged", &ids[0], payload)?;
    Ok(json!({"kept":ids[0],"reversed":ids.len()-1}))
}

fn import_contacts(conn: &Connection, csv: &str) -> Result<Value> {
    ensure!(csv.len() <= 5 * 1024 * 1024, "CSV exceeds 5 MB");
    let mut reader = csv::Reader::from_reader(csv.as_bytes());
    let headers = reader.headers()?.clone();
    let index = |name: &str| headers.iter().position(|v| v.trim() == name);
    ensure!(index("name").is_some(), "CSV must have a name column");
    let mut count = 0;
    for (row, result) in reader.records().enumerate() {
        ensure!(row < 5000, "Import is limited to 5,000 contacts at a time");
        let record = result?;
        let get = |name: &str| {
            index(name)
                .and_then(|i| record.get(i))
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let customer = get("customer");
        let supplier = get("supplier");
        let input = ContactInput {
            name: get("name"),
            customer: customer == "true"
                || customer == "1"
                || (customer.is_empty() && supplier.is_empty()),
            supplier: supplier == "true" || supplier == "1",
            phone: get("phone"),
            email: get("email"),
            address: get("address"),
            tax_id: get("tax_id"),
            notes: get("notes"),
            ..Default::default()
        };
        save_contact(conn, input).with_context(|| format!("CSV row {}", row + 2))?;
        count += 1;
    }
    audit(
        conn,
        "contacts.imported",
        "workspace",
        &json!({"count":count}),
    )?;
    Ok(json!({"count":count}))
}

fn import_transactions(conn: &Connection, csv: &str) -> Result<Value> {
    ensure!(csv.len() <= 5 * 1024 * 1024, "CSV exceeds 5 MB");
    let mut reader = csv::Reader::from_reader(csv.as_bytes());
    let headers = reader.headers()?.clone();
    let index = |name: &str| headers.iter().position(|v| v.trim() == name);
    ensure!(
        ["date", "kind", "amount", "account", "payee", "category"]
            .iter()
            .all(|k| index(k).is_some()),
        "Required CSV columns: date,kind,amount,account,payee,category (optional: memo,tags,to_account)"
    );
    let mut count = 0;
    for (row, result) in reader.records().enumerate() {
        ensure!(
            row < 5000,
            "Import is limited to 5,000 transactions at a time"
        );
        let record = result?;
        let get = |name: &str| {
            index(name)
                .and_then(|i| record.get(i))
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let account = optional_string(
            conn,
            "SELECT id FROM accounts WHERE name=?1 COLLATE NOCASE AND system=0 AND archived=0",
            &get("account"),
        )?
        .context("Create matching accounts before importing")?;
        let category = optional_string(
            conn,
            "SELECT id FROM categories WHERE name=?1 COLLATE NOCASE",
            &get("category"),
        )?
        .unwrap_or_default();
        let target = optional_string(
            conn,
            "SELECT id FROM accounts WHERE name=?1 COLLATE NOCASE AND system=0 AND archived=0",
            &get("to_account"),
        )?
        .unwrap_or_default();
        let input = TransactionInput {
            date: get("date"),
            kind: get("kind"),
            amount: get("amount"),
            account_id: account,
            payee: get("payee"),
            category_id: category,
            to_account_id: target,
            memo: get("memo"),
            tags: get("tags"),
            ..Default::default()
        };
        create_transaction(conn, input).with_context(|| format!("CSV row {}", row + 2))?;
        count += 1;
    }
    audit(
        conn,
        "transactions.imported",
        "workspace",
        &json!({"count":count}),
    )?;
    Ok(json!({"count":count}))
}

fn demo(conn: &Connection) -> Result<Value> {
    ensure!(
        conn.query_row("SELECT COUNT(*) FROM transactions", [], |r| r
            .get::<_, i64>(0))?
            == 0
            && conn.query_row("SELECT COUNT(*) FROM accounts WHERE system=0", [], |r| r
                .get::<_, i64>(0))?
                == 0,
        "Sample data is only available in an empty workspace"
    );
    let opening_date = (chrono::Local::now().date_naive() - chrono::Duration::days(90)).to_string();
    let bank = save_account(
        conn,
        AccountInput {
            name: "Everyday account".into(),
            kind: "bank".into(),
            currency: "USD".into(),
            group: "Daily money".into(),
            opening_balance: "8500.00".into(),
            opening_date: opening_date.clone(),
            ..Default::default()
        },
    )?["id"]
        .as_str()
        .unwrap()
        .to_string();
    let cash = save_account(
        conn,
        AccountInput {
            name: "Cash wallet".into(),
            kind: "cash".into(),
            currency: "USD".into(),
            group: "Daily money".into(),
            opening_balance: "320.00".into(),
            opening_date: opening_date.clone(),
            sort_order: 1,
            ..Default::default()
        },
    )?["id"]
        .as_str()
        .unwrap()
        .to_string();
    save_account(
        conn,
        AccountInput {
            name: "Rainy day fund".into(),
            kind: "savings".into(),
            currency: "USD".into(),
            group: "Savings".into(),
            opening_balance: "4200.00".into(),
            opening_date,
            sort_order: 2,
            ..Default::default()
        },
    )?;
    let acme = save_contact(
        conn,
        ContactInput {
            name: "Studio North".into(),
            customer: true,
            email: "hello@example.com".into(),
            phone: "+1 202 555 0142".into(),
            address: "24 Market Street".into(),
            notes: "Monthly design retainer".into(),
            ..Default::default()
        },
    )?["id"]
        .as_str()
        .unwrap()
        .to_string();
    let supply = save_contact(
        conn,
        ContactInput {
            name: "Paper & Co.".into(),
            supplier: true,
            email: "orders@example.com".into(),
            ..Default::default()
        },
    )?["id"]
        .as_str()
        .unwrap()
        .to_string();
    let day = |n: i64| {
        (chrono::Local::now().date_naive() - chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };
    for (days, kind, amount, payee, category, account, contact, paid) in [
        (
            24,
            "income",
            "2400.00",
            "Studio North",
            "sales",
            &bank,
            &acme,
            "2400.00",
        ),
        (
            20,
            "expense",
            "89.50",
            "Paper & Co.",
            "office",
            &bank,
            &supply,
            "89.50",
        ),
        (
            16,
            "expense",
            "42.80",
            "Neighborhood café",
            "food",
            &cash,
            &String::new(),
            "42.80",
        ),
        (
            12,
            "income",
            "1850.00",
            "Studio North",
            "sales",
            &bank,
            &acme,
            "850.00",
        ),
        (
            9,
            "expense",
            "125.00",
            "City transit",
            "travel",
            &bank,
            &String::new(),
            "125.00",
        ),
        (
            5,
            "expense",
            "340.00",
            "Paper & Co.",
            "office",
            &bank,
            &supply,
            "140.00",
        ),
        (
            3,
            "expense",
            "67.24",
            "Internet service",
            "utilities",
            &bank,
            &String::new(),
            "67.24",
        ),
        (
            1,
            "income",
            "650.00",
            "Freelance project",
            "sales",
            &bank,
            &String::new(),
            "650.00",
        ),
    ] {
        create_transaction(
            conn,
            TransactionInput {
                date: day(days),
                kind: kind.into(),
                amount: amount.into(),
                paid_amount: paid.into(),
                payee: payee.into(),
                category_id: category.into(),
                account_id: account.clone(),
                contact_id: contact.clone(),
                due_date: if paid != amount {
                    day(2)
                } else {
                    String::new()
                },
                memo: "Sample transaction".into(),
                tags: "sample".into(),
                ..Default::default()
            },
        )?;
    }
    let mut s = settings(conn)?;
    s.profile = "Alex Morgan".into();
    save_settings(conn, &s)?;
    audit(conn, "demo.loaded", "workspace", &json!({"sample":true}))?;
    Ok(json!({}))
}
