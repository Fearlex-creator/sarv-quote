//! Contacts — customers & suppliers with kit Table + Dialog fields.

use std::borrow::Cow;

use rune::{Align, Cx, El, col};
use rune_kit::{Cell, Switch, Table, TableColumn, TableState};
use serde_json::{Value, json};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

#[derive(Default)]
pub struct ContactsState {
    pub modal_open: bool,
    pub is_supplier: bool,
    pub table: TableState,
    pub open_id: String,
    pub selected: String,
    pub generation: String,
    pub detail: Value,
}

fn all_parties(s: &State) -> &[serde_json::Value] {
    s.books.workspace["resources"]["parties"].as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

pub fn party_matches(s: &State, party: &serde_json::Value, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let latin = crate::i18n::latin_digits(&q).to_lowercase();
    let id = crate::data::s(&party["id"]);
    let name = s.books.party_name(&id);
    let phone = crate::data::s(&s.books.workspace["party_profiles"][&id]["phone"]);
    [name.as_str(), phone.as_str(), id.as_str()].iter().any(|field| {
        let field = field.to_lowercase();
        field.contains(&q) || (!latin.is_empty() && field.contains(&latin))
    })
}

fn filtered_parties(s: &State) -> Vec<serde_json::Value> {
    let q = s.forms.get("contact-search");
    all_parties(s)
        .iter()
        .filter(|p| party_matches(s, p, &q))
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

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let table = Table::new(
        |s: &State| &s.contacts.table,
        |s: &mut State| &mut s.contacts.table,
        |s: &State| filtered_parties(s).len(),
        |s: &State, i| Cow::Owned(format!("contact-{}", crate::data::s(&filtered_parties(s)[i]["id"]))),
    )
    .name(ui.t("Contacts"))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("role", ui.t("Role")).width(120.0).min(88.0))
    .column(TableColumn::new("phone", ui.t("Phone")).width(140.0).min(100.0))
    .column(TableColumn::new("balance", ui.t("Balance")).width(160.0).min(110.0).align(Align::End).sortable())
    .cell(|s: &State, i, col| {
        let list = filtered_parties(s);
        let Some(party) = list.get(i) else {
            return Cell::text("");
        };
        let id = crate::data::s(&party["id"]);
        let is_cust = party["customer"].as_bool().unwrap_or(false);
        let is_supp = party["supplier"].as_bool().unwrap_or(false);
        match col {
            "name" => Cell::text(s.books.party_name(&id)),
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
                let phone = crate::data::s(&s.books.workspace["party_profiles"][&id]["phone"]);
                Cell::text(if phone.is_empty() { "—" } else { &phone }).muted()
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

    let add_btn = ui.btn(Btn::Primary, "New customer", Some(&crate::ui::icons::PLUS), |st| {
        st.contacts.modal_open = true;
        st.contacts.is_supplier = false;
        st.forms.set("contact-name", "");
        st.forms.set("contact-phone", "");
        st.forms.set("contact-email", "");
        st.forms.set("contact-notes", "");
    });
    let add_supp_btn = ui.btn(Btn::Glass, "Add supplier", Some(&crate::ui::icons::PLUS), |st| {
        st.contacts.modal_open = true;
        st.contacts.is_supplier = true;
        st.forms.set("contact-name", "");
        st.forms.set("contact-phone", "");
        st.forms.set("contact-email", "");
        st.forms.set("contact-notes", "");
    });

    if !s.contacts.open_id.is_empty() {
        mount_profile(s, ui, layer);
    }
    if s.contacts.modal_open {
        let is_supp = s.contacts.is_supplier;
        let title = if is_supp { ui.t("New supplier") } else { ui.t("New customer") };
        let role_switch = Switch::new(is_supp)
            .label(ui.t("Supplier"))
            .description(ui.t("People and companies you sell to or buy from"))
            .on_toggle(|st: &mut State| st.contacts.is_supplier = !st.contacts.is_supplier)
            .build(&ui.k)
            .key("contact-role-switch");
        let form = col()
            .gap(14.0)
            .child(ui.labeled("Name", ui.field(s, "contact-name", &ui.t("Contact name"))))
            .child(ui.form_grid(
                ui.labeled("Phone", ui.field(s, "contact-phone", "۰۲۱-۸۸۸۸۸۸۸۸")),
                ui.labeled("Email", ui.field(s, "contact-email", "name@example.com")),
            ))
            .child(ui.labeled("Description", ui.field(s, "contact-notes", &ui.t("Note (optional)"))))
            .child(role_switch);
        let save_btn = ui.btn(Btn::Primary, "Save", Some(&crate::ui::icons::CHECK), move |st| {
            let name = st.forms.get("contact-name").trim().to_string();
            if name.is_empty() {
                st.toast(Tone::Danger, st.fmt.t("Enter a contact name."));
                return;
            }
            let phone = st.forms.get("contact-phone").trim().to_string();
            let email = st.forms.get("contact-email").trim().to_string();
            let notes = st.forms.get("contact-notes").trim().to_string();
            let kind = if st.contacts.is_supplier {
                crate::ui::resources::ResourceKind::Vendor
            } else {
                crate::ui::resources::ResourceKind::Customer
            };
            if st.create_party(kind, &name, &phone, &email, &notes).is_ok() {
                st.contacts.modal_open = false;
            }
        });
        ui.dialog_described(
            layer,
            &mut Vec::new(),
            "contact",
            &title,
            &ui.t("Save contact details for sales and purchases."),
            true,
            480.0,
            form,
            vec![
                ui.btn(Btn::Ghost, "Cancel", None, |st| st.contacts.modal_open = false),
                save_btn,
            ],
            |st| st.contacts.modal_open = false,
        );
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
        .child(ui.field(s, "contact-search", &ui.t("Search")))
        .child(ui.fill_or_empty(
            filtered_parties(s).len(),
            empty_title,
            "People and companies you sell to or buy from",
            &crate::ui::icons::CONTACTS,
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

fn open_contact(st: &mut State, row_id: &str) {
    let party_id = row_id.strip_prefix("contact-").unwrap_or(row_id).to_string();
    match st.query(
        "contact_detail",
        json!({ "party_id": party_id, "as_of": crate::data::today() }),
    ) {
        Ok(v) => {
            let profile = &v["profile"];
            st.forms.set("profile-name", &crate::data::s(&profile["display_name"]));
            st.forms.set("profile-phone", &crate::data::s(&profile["phone"]));
            st.forms.set("profile-email", &crate::data::s(&profile["email"]));
            st.forms.set("profile-address", &crate::data::s(&profile["address"]));
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
    let invoices = crate::data::s(&stats["invoices"]);
    let form = col()
        .gap(12.0)
        .child(ui.labeled("Name", ui.field(s, "profile-name", "")))
        .child(ui.form_grid(
            ui.labeled("Phone", ui.field(s, "profile-phone", "")),
            ui.labeled("Email", ui.field(s, "profile-email", "")),
        ))
        .child(ui.labeled("Address", ui.field(s, "profile-address", "")))
        .child(ui.labeled("Contact notes", ui.field(s, "profile-notes", "")))
        .child(ui.muted(format!("{} · {}", ui.t("Open customer invoices"), outstanding)))
        .child(ui.muted(format!("{} {invoices}", ui.t("Invoices"))));
    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "contact-profile",
        &ui.t("Contact details"),
        &ui.t("Contact details, linked documents and performance for this business."),
        true,
        560.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.contacts.open_id.clear()),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                let id = st.contacts.open_id.clone();
                if id.is_empty() {
                    return;
                }
                let profile = json!({
                    "company_id": st.books.company_id,
                    "display_name": st.forms.get("profile-name"),
                    "first_name": "",
                    "last_name": "",
                    "organization": "",
                    "phone": st.forms.get("profile-phone"),
                    "mobile": "",
                    "email": st.forms.get("profile-email"),
                    "address": st.forms.get("profile-address"),
                    "city": "",
                    "postal_code": "",
                    "national_id": "",
                    "tax_id": "",
                    "website": "",
                    "notes": st.forms.get("profile-notes")
                });
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
