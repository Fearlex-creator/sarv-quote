//! Contacts — customers & suppliers with kit Table, role filters, and a full PartyProfile suite.

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, Switch, Table, TableColumn, TableState};
use serde_json::{Value, json};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

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
}

fn all_parties(s: &State) -> &[serde_json::Value] {
    s.books.workspace["resources"]["parties"].as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

fn profile<'a>(s: &'a State, id: &str) -> &'a Value {
    &s.books.workspace["party_profiles"][id]
}

pub fn party_matches(s: &State, party: &serde_json::Value, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let latin = crate::i18n::latin_digits(&q).to_lowercase();
    let id = crate::data::s(&party["id"]);
    let name = s.books.party_name(&id);
    let p = profile(s, &id);
    let phone = crate::data::s(&p["phone"]);
    let mobile = crate::data::s(&p["mobile"]);
    let email = crate::data::s(&p["email"]);
    let org = crate::data::s(&p["organization"]);
    let city = crate::data::s(&p["city"]);
    [name.as_str(), phone.as_str(), mobile.as_str(), email.as_str(), org.as_str(), city.as_str(), id.as_str()]
        .iter()
        .any(|field| {
            let field = field.to_lowercase();
            field.contains(&q) || (!latin.is_empty() && field.contains(&latin))
        })
}

fn filtered_parties(s: &State) -> Vec<serde_json::Value> {
    let q = s.forms.get("contact-search");
    all_parties(s)
        .iter()
        .filter(|p| {
            let is_cust = p["customer"].as_bool().unwrap_or(false);
            let is_supp = p["supplier"].as_bool().unwrap_or(false);
            let role_ok = match s.contacts.filter {
                ContactFilter::All => true,
                ContactFilter::Customers => is_cust,
                ContactFilter::Suppliers => is_supp,
            };
            role_ok && party_matches(s, p, &q)
        })
        .cloned()
        .collect()
}

fn balance_for(s: &State, id: &str) -> String {
    let party_balances = s.books.report["party_balances"].as_object();
    party_balances
        .and_then(|m| m.get(&format!("customer:{id}")).or_else(|| m.get(&format!("vendor:{id}"))))
        .map(crate::data::s)
        .unwrap_or_else(|| "0".into())
}

fn clear_contact_forms(st: &mut State) {
    for key in [
        "contact-name",
        "contact-first",
        "contact-last",
        "contact-org",
        "contact-phone",
        "contact-mobile",
        "contact-email",
        "contact-website",
        "contact-address",
        "contact-city",
        "contact-postal",
        "contact-national",
        "contact-tax",
        "contact-notes",
    ] {
        st.forms.set(key, "");
    }
}

fn open_new(st: &mut State, supplier: bool) {
    st.contacts.modal_open = true;
    st.contacts.is_supplier = supplier;
    clear_contact_forms(st);
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let list = filtered_parties(s);
    let table = Table::new(
        |s: &State| &s.contacts.table,
        |s: &mut State| &mut s.contacts.table,
        |s: &State| filtered_parties(s).len(),
        |s: &State, i| Cow::Owned(format!("contact-{}", crate::data::s(&filtered_parties(s)[i]["id"]))),
    )
    .name(ui.t("Contacts"))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("org", ui.t("Organization")).flex(1.4).min(110.0))
    .column(TableColumn::new("role", ui.t("Role")).width(110.0).min(80.0))
    .column(TableColumn::new("phone", ui.t("Phone")).width(130.0).min(96.0))
    .column(TableColumn::new("city", ui.t("City")).width(110.0).min(80.0))
    .column(TableColumn::new("balance", ui.t("Balance")).width(150.0).min(110.0).align(Align::End).sortable())
    .cell(|s: &State, i, col| {
        let list = filtered_parties(s);
        let Some(party) = list.get(i) else {
            return Cell::text("");
        };
        let id = crate::data::s(&party["id"]);
        let is_cust = party["customer"].as_bool().unwrap_or(false);
        let is_supp = party["supplier"].as_bool().unwrap_or(false);
        let p = profile(s, &id);
        match col {
            "name" => Cell::text(s.books.party_name(&id)),
            "org" => {
                let org = crate::data::s(&p["organization"]);
                Cell::text(if org.is_empty() { "—" } else { &org }).muted()
            }
            "role" => {
                let role = match (is_cust, is_supp) {
                    (true, true) => s.fmt.t("Both"),
                    (true, false) => s.fmt.t("Customer"),
                    (false, true) => s.fmt.t("Supplier"),
                    (false, false) => s.fmt.t("Contact"),
                };
                Cell::text(role).muted()
            }
            "phone" => {
                let phone = crate::data::s(&p["phone"]);
                let mobile = crate::data::s(&p["mobile"]);
                let shown = if !mobile.is_empty() { mobile } else if !phone.is_empty() { phone } else { "—".into() };
                Cell::text(shown).muted()
            }
            "city" => {
                let city = crate::data::s(&p["city"]);
                Cell::text(if city.is_empty() { "—" } else { &city }).muted()
            }
            _ => Cell::text(s.fmt.money(&balance_for(s, &id))).mono(),
        }
    })
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
    });

    let add_btn = ui.btn(Btn::Primary, "New customer", Some(&icons::PLUS), |st| open_new(st, false));
    let add_supp_btn = ui.btn(Btn::Glass, "Add supplier", Some(&icons::PLUS), |st| open_new(st, true));

    if !s.contacts.open_id.is_empty() {
        mount_profile(s, ui, layer);
    }
    if s.contacts.modal_open {
        mount_create(s, ui, layer);
    }

    let empty_title = if all_parties(s).is_empty() {
        "No contacts"
    } else {
        "Nothing to show"
    };
    let body = col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(filter_bar)
                .child(ui.field(s, "contact-search", &ui.t("Search")).grow(1.0).min_w(0.0)),
        )
        .child(ui.fill_or_empty(
            list.len(),
            empty_title,
            "People and companies you sell to or buy from",
            &icons::CONTACTS,
            table,
        ));

    ui.page(
        "contacts-screen",
        &ui.t("Contacts"),
        &ui.t("People and companies you sell to or buy from"),
        vec![
            add_supp_btn,
            add_btn,
            ui.btn(Btn::Glass, "Contact details", None, |st| {
                if st.contacts.selected.is_empty() {
                    st.toast(Tone::Info, st.fmt.t("Select contact"));
                    return;
                }
                let id = st.contacts.selected.clone();
                open_contact(st, &id);
            }),
        ],
        body,
    )
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
        .child(ui.section(
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
        ))
        .child(ui.section(
            &format!("{prefix}-reach"),
            &ui.t("Contact details"),
            col()
                .gap(12.0)
                .child(ui.form_grid(
                    ui.labeled("Phone", ui.field(s, &phone, "۰۲۱-۸۸۸۸۸۸۸۸")),
                    ui.labeled("Mobile", ui.field(s, &mobile, "")),
                ))
                .child(ui.form_grid(
                    ui.labeled("Email", ui.field(s, &email, "name@example.com")),
                    ui.labeled("Website", ui.field(s, &website, "")),
                )),
        ))
        .child(ui.section(
            &format!("{prefix}-place"),
            &ui.t("Address"),
            col()
                .gap(12.0)
                .child(ui.labeled("Address", ui.field(s, &address, "")))
                .child(ui.form_grid(
                    ui.labeled("City", ui.field(s, &city, "")),
                    ui.labeled("Postal code", ui.field(s, &postal, "")),
                )),
        ))
        .child(ui.section(
            &format!("{prefix}-legal"),
            &ui.t("Identity & registration"),
            col()
                .gap(12.0)
                .child(ui.form_grid(
                    ui.labeled("National / registration ID", ui.field(s, &national, "")),
                    ui.labeled("Tax / economic ID", ui.field(s, &tax, "")),
                ))
                .child(ui.labeled("Contact notes", ui.field(s, &notes, &ui.t("Note (optional)")))),
        ))
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
    ui.dialog_described(
        layer,
        &mut portals,
        "contact",
        &title,
        &ui.t("Save contact details for sales and purchases."),
        true,
        640.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.contacts.modal_open = false),
            save_btn,
        ],
        |st| st.contacts.modal_open = false,
    );
}

fn open_contact(st: &mut State, row_id: &str) {
    let party_id = row_id.strip_prefix("contact-").unwrap_or(row_id).to_string();
    match st.query(
        "contact_detail",
        json!({ "party_id": party_id, "as_of": crate::data::today() }),
    ) {
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
        .child(ui.metric("contact-outstanding", &icons::PAYMENTS, &ui.t("Balance"), outstanding, ui.t("Open customer invoices"), Tone::Warning))
        .child(ui.metric("contact-overdue", &icons::PAYMENTS, &ui.t("Overdue"), overdue, ui.t("Past due"), Tone::Danger))
        .child(ui.metric("contact-net", &icons::SALES, &ui.t("Net sales"), net, format!("{} {invoices}", ui.t("Invoices")), Tone::Success));

    let form = col().gap(14.0).child(metrics).child(contact_form_fields(s, ui, "profile"));
    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "contact-profile",
        &ui.t("Contact details"),
        &ui.t("Contact details, linked documents and performance for this business."),
        true,
        720.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.contacts.open_id.clear()),
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
        |st| st.contacts.open_id.clear(),
    );
}
