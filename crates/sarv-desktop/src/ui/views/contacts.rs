//! Contacts — customers & suppliers with kit Table, role filters, and a full PartyProfile suite.

use std::borrow::Cow;
use std::collections::HashMap;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, SortDir, Switch, Table, TableColumn, TableState};
use serde_json::{Value, json};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, DIALOG_MD, Ui};
use crate::ui::icons;
use crate::ui::rows::{FilterCache, Row, SortKey, fold_text, reapply_sort, sort_rows};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ContactFilter {
    #[default]
    All,
    Customers,
    Suppliers,
}

#[derive(Default)]
pub struct ContactsState {
    pub modal_open: bool,
    pub is_supplier: bool,
    pub filter: ContactFilter,
    pub table: TableState,
    pub open_id: String,
    pub selected: String,
    pub generation: String,
    pub detail: Value,
    pub new_snapshot: String,
    pub profile_snapshot: String,
    pub rows: Vec<Row>,
    pub search_blobs: HashMap<String, String>,
    pub filter_cache: FilterCache,
    pub rows_gen: u64,
    pub rows_stamp: u64,
}

fn all_parties(s: &State) -> &[serde_json::Value] {
    s.books.workspace["resources"]["parties"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

fn profile<'a>(s: &'a State, id: &str) -> &'a Value {
    &s.books.workspace["party_profiles"][id]
}

pub fn party_matches(s: &State, party: &serde_json::Value, query: &str) -> bool {
    let q = crate::i18n::fold(query.trim());
    if q.is_empty() {
        return true;
    }
    let id = crate::data::s(&party["id"]);
    let name = s.books.party_name(&id);
    let p = profile(s, &id);
    let phone = crate::data::s(&p["phone"]);
    let mobile = crate::data::s(&p["mobile"]);
    let email = crate::data::s(&p["email"]);
    let org = crate::data::s(&p["organization"]);
    let city = crate::data::s(&p["city"]);
    [
        name.as_str(),
        phone.as_str(),
        mobile.as_str(),
        email.as_str(),
        org.as_str(),
        city.as_str(),
        id.as_str(),
    ]
    .iter()
    .any(|field| crate::i18n::fold(field).contains(&q))
}

fn balance_for(s: &State, id: &str, filter: ContactFilter) -> String {
    let Some(party_balances) = s.books.report["party_balances"].as_object() else {
        return "0".into();
    };
    let get = |key: String| party_balances.get(&key).map(crate::data::s);
    match filter {
        ContactFilter::Customers => get(format!("customer:{id}")).unwrap_or_else(|| "0".into()),
        ContactFilter::Suppliers => get(format!("vendor:{id}")).unwrap_or_else(|| "0".into()),
        ContactFilter::All => match (get(format!("customer:{id}")), get(format!("vendor:{id}"))) {
            (Some(c), Some(v)) => {
                let c_num: i128 = c.parse().unwrap_or(0);
                let v_num: i128 = v.parse().unwrap_or(0);
                c_num.saturating_sub(v_num).to_string()
            }
            (Some(c), None) => c,
            (None, Some(v)) => v,
            (None, None) => "0".into(),
        },
    }
}

pub fn open_new_contact(st: &mut State, supplier: bool) {
    let search = st.forms.get("contact-search");
    st.forms.clear_prefix("contact-");
    if !search.is_empty() {
        st.forms.set("contact-search", &search);
    }
    st.contacts.is_supplier = supplier;
    st.contacts.new_snapshot = st.forms.snapshot_prefix("contact-");
    st.contacts.modal_open = true;
}

pub fn close_or_confirm(st: &mut State) {
    if st.forms.snapshot_prefix("contact-") == st.contacts.new_snapshot {
        st.contacts.modal_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.contacts.modal_open = false);
}

pub fn close_profile_or_confirm(st: &mut State) {
    if st.forms.snapshot_prefix("profile-") == st.contacts.profile_snapshot {
        st.contacts.open_id.clear();
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.contacts.open_id.clear());
}

fn col_index(col: &str) -> Option<usize> {
    match col {
        "name" => Some(0),
        "org" => Some(1),
        "role" => Some(2),
        "phone" => Some(3),
        "city" => Some(4),
        "balance" => Some(5),
        _ => None,
    }
}

fn build_rows(s: &State) -> (Vec<Row>, HashMap<String, String>) {
    let mut rows = Vec::new();
    let mut blobs = HashMap::new();
    for party in all_parties(s) {
        let is_cust = party["customer"].as_bool().unwrap_or(false);
        let is_supp = party["supplier"].as_bool().unwrap_or(false);
        let role_ok = match s.contacts.filter {
            ContactFilter::All => true,
            ContactFilter::Customers => is_cust,
            ContactFilter::Suppliers => is_supp,
        };
        if !role_ok {
            continue;
        }
        let id = crate::data::s(&party["id"]);
        let row_id = format!("contact-{id}");
        let name = s.books.party_name(&id);
        let p = profile(s, &id);
        let org_raw = crate::data::s(&p["organization"]);
        let org = if org_raw.is_empty() { "—".into() } else { org_raw.clone() };
        let role = match (is_cust, is_supp) {
            (true, true) => s.fmt.t("Both"),
            (true, false) => s.fmt.t("Customer"),
            (false, true) => s.fmt.t("Supplier"),
            (false, false) => s.fmt.t("Contact"),
        };
        let phone_raw = crate::data::s(&p["phone"]);
        let mobile_raw = crate::data::s(&p["mobile"]);
        let phone = if !mobile_raw.is_empty() {
            mobile_raw.clone()
        } else if !phone_raw.is_empty() {
            phone_raw.clone()
        } else {
            "—".into()
        };
        let phone_display = if phone == "—" {
            phone.clone()
        } else {
            crate::i18n::isolate(&phone)
        };
        let city_raw = crate::data::s(&p["city"]);
        let city = if city_raw.is_empty() { "—".into() } else { city_raw.clone() };
        let bal_raw = balance_for(s, &id, s.contacts.filter);
        let email_raw = crate::data::s(&p["email"]);
        let blob = fold_text(&format!(
            "{name}\n{phone_raw}\n{mobile_raw}\n{email_raw}\n{org_raw}\n{city_raw}\n{id}"
        ));
        blobs.insert(row_id.clone(), blob);
        rows.push(Row {
            id: row_id,
            cells: vec![
                name.clone(),
                org.clone(),
                role.clone(),
                phone_display,
                city.clone(),
                s.fmt.money(&bal_raw),
            ],
            keys: vec![
                SortKey::text(&name),
                SortKey::text(&org),
                SortKey::text(&role),
                SortKey::text(&phone),
                SortKey::text(&city),
                SortKey::num_str(&bal_raw),
            ],
        });
    }
    (rows, blobs)
}

pub fn ensure(st: &mut State) {
    if st.contacts.rows_stamp == st.stamp {
        return;
    }
    let (rows, blobs) = build_rows(st);
    st.contacts.rows = rows;
    st.contacts.search_blobs = blobs;
    st.contacts.rows_stamp = st.stamp;
    st.contacts.rows_gen += 1;
    reapply_sort(&st.contacts.table, &mut st.contacts.rows, col_index);
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.contacts.table.set_sort(col, dir);
    if let Some(idx) = col_index(col) {
        sort_rows(&mut st.contacts.rows, idx, dir);
        st.contacts.rows_gen += 1;
    }
}

fn visible_indices(s: &State) -> std::cell::Ref<'_, Vec<usize>> {
    let q = s.forms.get("contact-search");
    s.contacts
        .filter_cache
        .visible(&s.contacts.rows, s.contacts.rows_gen, &q, |row, q_folded| {
            let blob = s.contacts.search_blobs.get(&row.id).map(String::as_str).unwrap_or("");
            blob.contains(q_folded)
        })
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let visible_count = visible_indices(s).len();
    let table = Table::new(
        |s: &State| &s.contacts.table,
        |s: &mut State| &mut s.contacts.table,
        |s: &State| visible_indices(s).len(),
        |s: &State, i| {
            let vis = visible_indices(s);
            Cow::Owned(
                vis.get(i)
                    .and_then(|&idx| s.contacts.rows.get(idx))
                    .map(|r| r.id.clone())
                    .unwrap_or_default(),
            )
        },
    )
    .name(ui.t("Contacts"))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("org", ui.t("Organization")).flex(1.4).min(110.0).sortable())
    .column(TableColumn::new("role", ui.t("Role")).width(110.0).min(80.0).sortable())
    .column(TableColumn::new("phone", ui.t("Phone")).width(130.0).min(96.0).sortable())
    .column(TableColumn::new("city", ui.t("City")).width(110.0).min(80.0).sortable())
    .column(
        TableColumn::new("balance", ui.t("Balance"))
            .width(150.0)
            .min(110.0)
            .align(Align::End)
            .sortable(),
    )
    .cell(|s: &State, i, col| {
        let vis = visible_indices(s);
        let Some(row) = vis.get(i).and_then(|&idx| s.contacts.rows.get(idx)) else {
            return Cell::text("");
        };
        match col {
            "name" => Cell::text(&row.cells[0]),
            "org" => Cell::text(&row.cells[1]).muted(),
            "role" => Cell::text(&row.cells[2]).muted(),
            "phone" => Cell::text(&row.cells[3]).muted(),
            "city" => Cell::text(&row.cells[4]).muted(),
            _ => Cell::text(&row.cells[5]).mono(),
        }
    })
    .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
    .on_select(|st, id, _| st.contacts.selected = id.to_string())
    .on_open(|st, id, _| open_contact(st, id))
    .zebra(true)
    .build(s, &ui.k)
    .key("contacts-table")
    .grow(1.0)
    .min_h(0.0);

    let all_l = ui.t("All contacts");
    let cust_l = ui.t("Customers");
    let supp_l = ui.t("Suppliers");
    let filter_tabs = [
        (all_l.as_str(), s.contacts.filter == ContactFilter::All),
        (cust_l.as_str(), s.contacts.filter == ContactFilter::Customers),
        (supp_l.as_str(), s.contacts.filter == ContactFilter::Suppliers),
    ];
    let filter_bar = ui.tabs("contact-filter", &filter_tabs, |st, idx| {
        st.contacts.filter = match idx {
            1 => ContactFilter::Customers,
            2 => ContactFilter::Suppliers,
            _ => ContactFilter::All,
        };
        st.contacts.rows_stamp = 0;
        ensure(st);
    });

    let add_btn = ui.btn(Btn::Primary, "New customer", Some(&icons::PLUS), |st| open_new_contact(st, false));
    let add_supp_btn = ui.btn(Btn::Glass, "Add supplier", Some(&icons::PLUS), |st| open_new_contact(st, true));

    if !s.contacts.open_id.is_empty() {
        mount_profile(s, ui, layer);
    }
    if s.contacts.modal_open {
        mount_create(s, ui, layer);
    }

    let parties = all_parties(s);
    let cust_count = parties.iter().filter(|p| p["customer"].as_bool() == Some(true)).count();
    let supp_count = parties.iter().filter(|p| p["supplier"].as_bool() == Some(true)).count();
    let contact_kpis = row()
        .w_full()
        .gap(12.0)
        .child(ui.metric(
            "contacts-kpi-customers",
            &icons::CONTACTS,
            &ui.t("Customers"),
            ui.f.digits(&cust_count.to_string()),
            ui.f.money(&s.books.position("receivables")),
            Tone::Success,
        ))
        .child(ui.metric(
            "contacts-kpi-suppliers",
            &icons::PURCHASES,
            &ui.t("Suppliers"),
            ui.f.digits(&supp_count.to_string()),
            ui.f.money(&s.books.position("payables")),
            Tone::Warning,
        ));

    let empty_title = if parties.is_empty() { "No contacts" } else { "Nothing to show" };
    let mut filter_row = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(filter_bar)
        .child(ui.field(s, "contact-search", &ui.t("Search")).grow(1.0).min_w(0.0));
    if !s.contacts.selected.is_empty() {
        filter_row = filter_row.child(ui.btn(Btn::Glass, "Contact details", None, |st| {
            let id = st.contacts.selected.clone();
            open_contact(st, &id);
        }));
    }
    let body = col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(contact_kpis)
        .child(filter_row)
        .child(ui.fill_or_empty(
            visible_count,
            empty_title,
            "People and companies you sell to or buy from",
            &icons::CONTACTS,
            table,
        ));

    ui.page(
        "contacts-screen",
        &ui.t("Contacts"),
        &ui.t("People and companies you sell to or buy from"),
        vec![add_supp_btn, add_btn],
        body,
    )
}

pub(crate) fn national_id_hint(ui: &Ui, raw: &str) -> Option<(Tone, String)> {
    use sarv_quote::iran::ids::{IdError, legal_id, national_id, normalize_digits};
    let norm = normalize_digits(raw.trim());
    if norm.is_empty() {
        return None;
    }
    let res = match norm.chars().count() {
        10 => national_id(&norm),
        11 => legal_id(&norm),
        _ => return Some((Tone::Warning, ui.t("A national ID has 10 digits; a legal ID has 11."))),
    };
    match res {
        Ok(()) => None,
        Err(IdError::Checksum | IdError::Repeated) => Some((Tone::Warning, ui.t("This national ID does not pass its check digit."))),
        Err(IdError::NotDigits | IdError::Length) => Some((Tone::Warning, ui.t("A national ID has 10 digits; a legal ID has 11."))),
    }
}

pub(crate) fn postal_code_hint(ui: &Ui, raw: &str) -> Option<(Tone, String)> {
    use sarv_quote::iran::ids::{normalize_digits, postal_code};
    let norm = normalize_digits(raw.trim());
    if norm.is_empty() {
        return None;
    }
    postal_code(&norm)
        .err()
        .map(|_| (Tone::Warning, ui.t("A postal code has 10 digits.")))
}

pub(crate) fn sheba_hint(ui: &Ui, raw: &str) -> Option<(Tone, String)> {
    use sarv_quote::iran::sheba::{bank_name, canonical, validate};
    let canon = canonical(raw.trim());
    if !canon.starts_with("IR") {
        return None;
    }
    match validate(&canon) {
        Ok(()) => bank_name(&canon).map(|name| (Tone::Info, name.to_string())),
        Err(_) => Some((Tone::Warning, ui.t("This Sheba number is not valid."))),
    }
}

fn contact_form_fields(s: &State, ui: &Ui, prefix: &str) -> El {
    let name = format!("{prefix}-name");
    let first = format!("{prefix}-first");
    let last = format!("{prefix}-last");
    let org = format!("{prefix}-org");
    let phone = format!("{prefix}-phone");
    let mobile = format!("{prefix}-mobile");
    let email = format!("{prefix}-email");
    let website = format!("{prefix}-website");
    let address = format!("{prefix}-address");
    let city = format!("{prefix}-city");
    let postal = format!("{prefix}-postal");
    let national = format!("{prefix}-national");
    let tax = format!("{prefix}-tax");
    let notes = format!("{prefix}-notes");

    col()
        .gap(16.0)
        .child(
            ui.section(
                &format!("{prefix}-identity"),
                &ui.t("Identity"),
                col()
                    .gap(12.0)
                    .child(ui.labeled("Name", ui.field(s, &name, &ui.t("Contact name"))))
                    .child(ui.form_grid(
                        ui.labeled("First name", ui.field(s, &first, "")),
                        ui.labeled("Last name", ui.field(s, &last, "")),
                    ))
                    .child(ui.labeled("Organization", ui.field(s, &org, &ui.t("Organization")))),
            ),
        )
        .child(
            ui.section(
                &format!("{prefix}-reach"),
                &ui.t("Contact details"),
                col()
                    .gap(12.0)
                    .child(ui.form_grid(
                        ui.labeled("Phone", ui.field(s, &phone, &s.fmt.digits("021-88888888"))),
                        ui.labeled("Mobile", ui.field(s, &mobile, "")),
                    ))
                    .child(ui.form_grid(
                        ui.labeled("Email", ui.field(s, &email, "name@example.com")),
                        ui.labeled("Website", ui.field(s, &website, "")),
                    )),
            ),
        )
        .child(
            ui.section(
                &format!("{prefix}-place"),
                &ui.t("Address"),
                col()
                    .gap(12.0)
                    .child(ui.labeled("Address", ui.field(s, &address, "")))
                    .child(ui.form_grid(
                        ui.labeled("City", ui.field(s, &city, "")),
                        ui.hinted("Postal code", ui.field(s, &postal, ""), postal_code_hint(ui, &s.forms.get(&postal))),
                    )),
            ),
        )
        .child(
            ui.section(
                &format!("{prefix}-legal"),
                &ui.t("Identity & registration"),
                col()
                    .gap(12.0)
                    .child(ui.form_grid(
                        ui.hinted(
                            "National / registration ID",
                            ui.field(s, &national, ""),
                            national_id_hint(ui, &s.forms.get(&national)),
                        ),
                        ui.labeled("Tax / economic ID", ui.field(s, &tax, "")),
                    ))
                    .child(ui.labeled("Contact notes", ui.field(s, &notes, &ui.t("Note (optional)")))),
            ),
        )
}

fn profile_json(st: &State, prefix: &str, company_id: &str, display_fallback: &str) -> Value {
    let display = st.forms.get(&format!("{prefix}-name"));
    let display_name = if display.trim().is_empty() {
        display_fallback.to_string()
    } else {
        display
    };
    json!({
        "company_id": company_id,
        "display_name": display_name,
        "first_name": st.forms.get(&format!("{prefix}-first")),
        "last_name": st.forms.get(&format!("{prefix}-last")),
        "organization": st.forms.get(&format!("{prefix}-org")),
        "phone": st.forms.get(&format!("{prefix}-phone")),
        "mobile": st.forms.get(&format!("{prefix}-mobile")),
        "email": st.forms.get(&format!("{prefix}-email")),
        "address": st.forms.get(&format!("{prefix}-address")),
        "city": st.forms.get(&format!("{prefix}-city")),
        "postal_code": st.forms.get(&format!("{prefix}-postal")),
        "national_id": st.forms.get(&format!("{prefix}-national")),
        "tax_id": st.forms.get(&format!("{prefix}-tax")),
        "website": st.forms.get(&format!("{prefix}-website")),
        "notes": st.forms.get(&format!("{prefix}-notes")),
    })
}

fn mount_create(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let is_supp = s.contacts.is_supplier;
    let title = if is_supp { ui.t("New supplier") } else { ui.t("New customer") };
    let role_switch = Switch::new(is_supp)
        .label(ui.t("Supplier"))
        .description(ui.t("People and companies you sell to or buy from"))
        .on_toggle(|st: &mut State| st.contacts.is_supplier = !st.contacts.is_supplier)
        .build(&ui.k)
        .key("contact-role-switch");
    let form = col().gap(14.0).child(role_switch).child(contact_form_fields(s, ui, "contact"));
    let save_btn = ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), move |st| {
        let name = st.forms.get("contact-name").trim().to_string();
        if name.is_empty() {
            st.toast(Tone::Danger, st.fmt.t("Enter a contact name."));
            return;
        }
        let kind = if st.contacts.is_supplier { "vendor" } else { "customer" };
        let id = st.uid();
        let profile = profile_json(st, "contact", &st.books.company_id, &name);
        if st
            .run(
                "resource",
                json!({
                    "kind": kind,
                    "id": id,
                    "name": name,
                    "profile": profile,
                    "reason": "Register contact"
                }),
                Some("Contact saved"),
            )
            .is_ok()
        {
            st.contacts.modal_open = false;
        }
    });
    let mut portals = Vec::new();
    ui.form_dialog(
        layer,
        &mut portals,
        "contact",
        &title,
        &ui.t("Save contact details for sales and purchases."),
        true,
        DIALOG_MD,
        form,
        vec![ui.btn(Btn::Ghost, "Cancel", None, close_or_confirm), save_btn],
        close_or_confirm,
    );
}

pub(crate) fn open_contact(st: &mut State, row_id: &str) {
    let party_id = row_id.strip_prefix("contact-").unwrap_or(row_id).to_string();
    match st.query("contact_detail", json!({ "party_id": party_id, "as_of": crate::data::today() })) {
        Ok(v) => {
            let profile = &v["profile"];
            let display = crate::data::s(&profile["display_name"]);
            let fallback = st.books.party_name(&party_id);
            st.forms.set("profile-name", if display.is_empty() { &fallback } else { &display });
            st.forms.set("profile-first", &crate::data::s(&profile["first_name"]));
            st.forms.set("profile-last", &crate::data::s(&profile["last_name"]));
            st.forms.set("profile-org", &crate::data::s(&profile["organization"]));
            st.forms.set("profile-phone", &crate::data::s(&profile["phone"]));
            st.forms.set("profile-mobile", &crate::data::s(&profile["mobile"]));
            st.forms.set("profile-email", &crate::data::s(&profile["email"]));
            st.forms.set("profile-website", &crate::data::s(&profile["website"]));
            st.forms.set("profile-address", &crate::data::s(&profile["address"]));
            st.forms.set("profile-city", &crate::data::s(&profile["city"]));
            st.forms.set("profile-postal", &crate::data::s(&profile["postal_code"]));
            st.forms.set("profile-national", &crate::data::s(&profile["national_id"]));
            st.forms.set("profile-tax", &crate::data::s(&profile["tax_id"]));
            st.forms.set("profile-notes", &crate::data::s(&profile["notes"]));
            st.contacts.generation = crate::data::s(&v["_generation"]);
            st.contacts.detail = v;
            st.contacts.profile_snapshot = st.forms.snapshot_prefix("profile-");
            st.contacts.open_id = party_id;
        }
        Err(e) => st.toast_err(e),
    }
}

fn mount_profile(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let stats = &s.contacts.detail["stats"];
    let outstanding = ui.f.money(&crate::data::s(&stats["outstanding"]));
    let overdue = ui.f.money(&crate::data::s(&stats["overdue"]));
    let net = ui.f.money(&crate::data::s(&stats["net_sales"]));
    let invoices = s.fmt.digits(&crate::data::s(&stats["invoices"]));
    let metrics = row()
        .w_full()
        .gap(10.0)
        .child(ui.metric(
            "contact-outstanding",
            &icons::PAYMENTS,
            &ui.t("Balance"),
            outstanding,
            ui.t("Open customer invoices"),
            Tone::Warning,
        ))
        .child(ui.metric(
            "contact-overdue",
            &icons::PAYMENTS,
            &ui.t("Overdue"),
            overdue,
            ui.t("Past due"),
            Tone::Danger,
        ))
        .child(ui.metric(
            "contact-net",
            &icons::SALES,
            &ui.t("Net sales"),
            net,
            format!("{} {invoices}", ui.t("Invoices")),
            Tone::Success,
        ));

    let form = col().gap(14.0).child(metrics).child(contact_form_fields(s, ui, "profile"));
    let mut portals = Vec::new();
    ui.form_dialog(
        layer,
        &mut portals,
        "contact-profile",
        &ui.t("Contact details"),
        &ui.t("Contact details, linked documents and performance for this business."),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_profile_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                let id = st.contacts.open_id.clone();
                if id.is_empty() {
                    return;
                }
                let profile = profile_json(st, "profile", &st.books.company_id, &st.books.party_name(&id));
                let payload = json!({
                    "party_id": id,
                    "profile": profile,
                    "expected_generation": st.contacts.generation
                });
                if st.run("party_profile", payload, Some("Contact saved")).is_ok() {
                    st.contacts.open_id.clear();
                }
            }),
        ],
        close_profile_or_confirm,
    );
}
