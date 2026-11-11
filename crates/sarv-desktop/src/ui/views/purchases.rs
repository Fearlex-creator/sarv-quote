//! Purchases — list + order editor with supplier/product Combobox (pick or create).

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, ComboboxState, Switch, Table, TableColumn, TableState};
use serde_json::json;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;
use crate::ui::pickers;
use crate::ui::resources::ResourceKind;

#[derive(Clone, Debug)]
pub struct PurchaseLine {
    pub id: String,
    pub item_id: String,
    pub qty: String,
    pub price: String,
}

impl PurchaseLine {
    fn blank() -> Self {
        Self {
            id: format!("pline-{}", chrono::Utc::now().timestamp_millis()),
            item_id: String::new(),
            qty: "1".into(),
            price: String::new(),
        }
    }
}

#[derive(Default)]
pub struct PurchasesState {
    pub tab_idx: usize,
    pub table: TableState,
    pub modal_open: bool,
    pub supplier_id: String,
    pub supplier_combo: ComboboxState,
    pub lines: Vec<PurchaseLine>,
    pub line_combos: Vec<ComboboxState>,
    pub creating_item_for_line: Option<usize>,
    pub post_key: String,
    pub posting: bool,
    /// When set, the order is received and billed in the same step.
    pub receive_and_bill: bool,
    pub selected: String,
}

impl PurchasesState {
    pub fn start_new(&mut self) {
        self.modal_open = true;
        self.supplier_id.clear();
        self.supplier_combo = ComboboxState::default();
        self.lines = vec![PurchaseLine::blank()];
        self.line_combos = vec![ComboboxState::default()];
        self.creating_item_for_line = None;
        let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        self.post_key = format!("po-{stamp}");
        self.posting = false;
        self.receive_and_bill = true;
    }

    fn ensure_line_combos(&mut self) {
        while self.line_combos.len() < self.lines.len() {
            self.line_combos.push(ComboboxState::default());
        }
        self.line_combos.truncate(self.lines.len());
    }
}

fn record_kind(record: &serde_json::Value) -> &str {
    let cmd = &record["command"];
    if cmd.get("Order").is_some() {
        "Order"
    } else if cmd.get("Receive").is_some() {
        "Receive"
    } else if cmd.get("Bill").is_some() {
        "Bill"
    } else if cmd.get("Return").is_some() {
        "Return"
    } else {
        record["kind"].as_str().unwrap_or("")
    }
}

fn first_nonempty(a: String, b: String) -> String {
    if a.is_empty() { b } else { a }
}

fn filtered(s: &State) -> Vec<serde_json::Value> {
    let kinds = ["Order", "Receive", "Bill", "Return"];
    let kind = kinds.get(s.purchases.tab_idx).copied().unwrap_or("Order");
    s.books.list("purchases").into_iter().filter(|r| record_kind(r) == kind).collect()
}

fn post_purchase(st: &mut State) {
    if st.purchases.posting || !st.purchases.modal_open {
        return;
    }
    st.purchases.posting = true;
    for (i, line) in st.purchases.lines.iter_mut().enumerate() {
        let q = st.forms.get(&format!("buy-qty-{i}"));
        let p = st.forms.get(&format!("buy-price-{i}"));
        if !q.trim().is_empty() {
            line.qty = q;
        }
        if !p.trim().is_empty() {
            line.price = p;
        }
    }
    let filled: Vec<_> = st.purchases.lines.iter().filter(|l| !l.item_id.is_empty()).cloned().collect();
    if filled.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Add at least one product."));
        st.purchases.posting = false;
        return;
    }
    if st.purchases.supplier_id.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Select supplier"));
        st.purchases.posting = false;
        return;
    }
    let wh = pickers::first_warehouse(st);
    let date = crate::data::today();
    let lines: Vec<_> = filled
        .iter()
        .map(|l| {
            json!({
                "id": l.id,
                "item_id": l.item_id,
                "warehouse_id": wh,
                "unit_id": pickers::item_unit(st, &l.item_id),
                "quantity": crate::i18n::latin_digits(&l.qty).replace(',', ""),
                "unit_price": st.parse_money_input(&l.price).to_string(),
                "line_discount": "0"
            })
        })
        .collect();
    let key = if st.purchases.post_key.is_empty() {
        format!("po-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
    } else {
        st.purchases.post_key.clone()
    };
    let reference = format!("PO-{}", &key[key.len().saturating_sub(6)..]);
    let mut payload = json!({
        "supplier_id": st.purchases.supplier_id,
        "reference": reference,
        "date": date,
        "discount": "0",
        "reason": "Purchase order",
        "source_id": key,
        "request_key": key,
        "lines": lines
    });
    let review = match st.query("purchase_preview", payload.clone()) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e);
            st.purchases.posting = false;
            return;
        }
    };
    if !review["_generation"].is_null() {
        payload["expected_generation"] = review["_generation"].clone();
    }
    let posted = match st.run("purchase_post", payload, None) {
        Ok(v) => v,
        Err(_) => {
            // `run` already toasted the engine error.
            st.purchases.posting = false;
            return;
        }
    };
    if st.purchases.receive_and_bill {
        let order_id = crate::data::s(&posted["record_id"]);
        if order_id.is_empty() {
            st.toast(Tone::Danger, st.fmt.t("No received purchases"));
            st.purchases.posting = false;
            return;
        }
        let receipt = match crate::ui::ops::post_operation(
            st,
            &order_id,
            "receive",
            &format!("R-{reference}"),
            "Receive goods",
            &format!("{key}-recv"),
        ) {
            Ok(v) => v,
            Err(e) => {
                if st.toasts.last().map(|t| t.message.as_str()) != Some(e.as_str()) {
                    st.toast_err(e);
                }
                st.purchases.posting = false;
                return;
            }
        };
        let receipt_id = crate::data::s(&receipt["record_id"]);
        let bill_against = if receipt_id.is_empty() { order_id } else { receipt_id };
        if let Err(e) = crate::ui::ops::post_operation(
            st,
            &bill_against,
            "bill",
            &format!("B-{reference}"),
            "Bill received goods",
            &format!("{key}-bill"),
        ) {
            if st.toasts.last().map(|t| t.message.as_str()) != Some(e.as_str()) {
                st.toast_err(e);
            }
            st.purchases.posting = false;
            return;
        }
    }
    st.purchases.modal_open = false;
    st.purchases.posting = false;
    st.toast(Tone::Success, st.fmt.t("Posted"));
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let tabs_labels = [ui.t("Orders"), ui.t("Receipts"), ui.t("Bills"), ui.t("Returns")];
    let kinds = ["Order", "Receive", "Bill", "Return"];
    let tabs: Vec<(&str, bool)> = tabs_labels
        .iter()
        .enumerate()
        .map(|(i, l)| (l.as_str(), s.purchases.tab_idx == i))
        .collect();
    let tab_bar = ui.tabs("purchases-tabs", &tabs, |st, idx| {
        st.purchases.tab_idx = idx;
        st.purchases.table = TableState::new();
    });

    let kind = kinds.get(s.purchases.tab_idx).copied().unwrap_or("Order");

    let table = Table::new(
        |s: &State| &s.purchases.table,
        |s: &mut State| &mut s.purchases.table,
        |s: &State| filtered(s).len(),
        |s: &State, i| Cow::Owned(format!("purchase-{}", crate::data::s(&filtered(s)[i]["id"]))),
    )
    .name(ui.t("Purchases"))
    .column(TableColumn::new("ref", ui.t("Reference")).width(140.0).min(96.0))
    .column(TableColumn::new("vendor", ui.t("Vendor")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0))
    .column(TableColumn::new("amount", ui.t("Amount")).width(160.0).min(100.0).align(Align::End).sortable())
    .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0))
    .cell(|s: &State, i, col| {
        let list = filtered(s);
        let Some(record) = list.get(i) else {
            return Cell::text("");
        };
        let id = crate::data::s(&record["id"]);
        let cmd = &record["command"];
        let inner = cmd
            .get("Order")
            .or_else(|| cmd.get("Receive"))
            .or_else(|| cmd.get("Bill"))
            .or_else(|| cmd.get("Return"))
            .unwrap_or(cmd);
        match col {
            "ref" => {
                let ref_no = first_nonempty(crate::data::s(&inner["reference"]), crate::data::s(&inner["return_reference"]));
                Cell::text(if ref_no.is_empty() {
                    format!("#{}", &id[..id.len().min(8)])
                } else {
                    ref_no
                })
            }
            "vendor" => {
                let supplier_id = crate::data::s(&inner["supplier_id"]);
                let supplier = if supplier_id.is_empty() {
                    crate::data::s(&record["data"]["supplier"]["name"])
                } else {
                    s.books.party_name(&supplier_id)
                };
                Cell::text(supplier)
            }
            "date" => {
                let date_raw = inner
                    .get("context")
                    .and_then(|c| c.get("effective_date"))
                    .cloned()
                    .unwrap_or_else(|| record["date"].clone());
                Cell::text(s.fmt.date(&crate::data::s(&date_raw))).muted()
            }
            "amount" => {
                let amount = ["Order", "Receive", "Bill", "Return"]
                    .iter()
                    .map(|kind| crate::data::s(&record["data"][kind]["total"]))
                    .find(|value| !value.is_empty())
                    .unwrap_or_else(|| "0".into());
                Cell::text(s.fmt.money(&amount)).mono()
            }
            _ => Cell::text(s.fmt.t("Posted")).muted(),
        }
    })
    .on_select(|st, id, _| st.purchases.selected = id.strip_prefix("purchase-").unwrap_or(&id).to_string())
    .zebra(true)
    .build(s, &ui.k)
    .key("purchases-table")
    .grow(1.0)
    .min_h(0.0);

    let hint = match kind {
        "Order" => "Goods you ordered from suppliers.",
        "Receive" => "Goods that have arrived.",
        "Bill" => "Supplier bills waiting to be paid.",
        _ => "Goods sent back to suppliers.",
    };

    if s.purchases.modal_open {
        mount_purchase_dialog(s, ui, layer);
    }

    let body = col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(tab_bar)
        .child(ui.muted(ui.t(hint)))
        .child(ui.fill_or_empty(filtered(s).len(), "No records", hint, &icons::PURCHASES, table));

    ui.page(
        "purchases-screen",
        &ui.t("Purchases"),
        &ui.t("Orders, receipts, and bills"),
        vec![
            ui.btn(Btn::Primary, "New purchase order", Some(&icons::PLUS), |st| {
                st.purchases.start_new();
            }),
            ui.btn(Btn::Glass, "Return", None, return_selected),
        ],
        body,
    )
}

fn mount_purchase_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let mut portals = Vec::new();
    let mut supplier_opts = pickers::party_options(s, false, true);
    let q = s.purchases.supplier_combo.search().text.clone();
    supplier_opts = pickers::with_create_option(supplier_opts, &q, &ui.t("New supplier"));

    let supplier_picker = pickers::entity_combo(
        ui,
        &mut portals,
        "buy-supplier",
        &s.purchases.supplier_combo,
        |st| &mut st.purchases.supplier_combo,
        &supplier_opts,
        if s.purchases.supplier_id.is_empty() {
            None
        } else {
            Some(s.purchases.supplier_id.as_str())
        },
        &ui.t("Select supplier"),
        pickers::PICKER_WIDTH,
        |st, id| {
            if let Some(name) = pickers::create_text(id) {
                match st.create_party(ResourceKind::Vendor, name, "", "", "") {
                    Ok(sid) => st.purchases.supplier_id = sid,
                    Err(e) => st.toast_err(e),
                }
            } else {
                st.purchases.supplier_id = id.to_string();
            }
        },
    );
    let add_supplier = pickers::add_new_btn(ui, "Add supplier", |st| {
        let name = st.purchases.supplier_combo.search().text.trim().to_string();
        if name.is_empty() {
            st.toast(Tone::Info, st.fmt.t("Select supplier"));
            return;
        }
        match st.create_party(ResourceKind::Vendor, &name, "", "", "") {
            Ok(id) => st.purchases.supplier_id = id,
            Err(e) => st.toast_err(e),
        }
    });

    let header = row()
        .w_full()
        .gap(8.0)
        .child(ui.muted(ui.t("Select product")).grow(1.0))
        .child(ui.muted(ui.t("Qty")).w(90.0))
        .child(ui.muted(ui.t("Unit price")).w(140.0));
    let mut lines_col = col().key("buy-lines").w_full().gap(8.0).child(header);
    for (i, line) in s.purchases.lines.iter().enumerate() {
        let mut item_opts = pickers::item_options(s);
        let line_q = s.purchases.line_combos.get(i).map(|c| c.search().text.clone()).unwrap_or_default();
        item_opts = pickers::with_create_option(item_opts, &line_q, &ui.t("New product"));
        let line_id = line.id.clone();
        let item_picker = pickers::entity_combo(
            ui,
            &mut portals,
            &format!("buy-item-{i}"),
            s.purchases.line_combos.get(i).unwrap_or(&ComboboxState::default()),
            move |st| {
                st.purchases.ensure_line_combos();
                &mut st.purchases.line_combos[i]
            },
            &item_opts,
            if line.item_id.is_empty() { None } else { Some(line.item_id.as_str()) },
            &ui.t("Select product"),
            pickers::PICKER_WIDTH,
            move |st, id| {
                st.purchases.ensure_line_combos();
                if let Some(name) = pickers::create_text(id) {
                    st.purchases.creating_item_for_line = Some(i);
                    st.forms.set("quick-item-name", name);
                    st.forms.set("quick-item-price", "");
                    st.forms.set("quick-item-cost", "");
                    return;
                }
                if let Some(l) = st.purchases.lines.iter_mut().find(|l| l.id == line_id) {
                    l.item_id = id.to_string();
                }
                if st.purchases.lines.last().map(|l| !l.item_id.is_empty()).unwrap_or(false) {
                    st.purchases.lines.push(PurchaseLine::blank());
                    st.purchases.line_combos.push(ComboboxState::default());
                }
            },
        );
        lines_col = lines_col.child(
            row()
                .key(format!("buy-line-{i}"))
                .w_full()
                .gap(8.0)
                .center_y()
                .child(item_picker.grow(1.0).min_w(0.0))
                .child(ui.field(s, &format!("buy-qty-{i}"), "1").w(90.0))
                .child(ui.field(s, &format!("buy-price-{i}"), "۰").w(140.0)),
        );
    }

    let form = col()
        .gap(16.0)
        .child(ui.labeled("Vendor", pickers::picker_row(supplier_picker, add_supplier)))
        .child(ui.section("buy-lines-sec", &ui.t("Products"), lines_col))
        .child(
            Switch::new(s.purchases.receive_and_bill)
                .label(ui.t("Receive & Bill"))
                .on_toggle(|st: &mut State| st.purchases.receive_and_bill = !st.purchases.receive_and_bill)
                .build(&ui.k)
                .key("buy-receive-bill"),
        );

    ui.dialog_described(
        layer,
        &mut portals,
        "purchase",
        &ui.t("New purchase order"),
        &ui.t("Pick a supplier and products, or type to add new ones."),
        true,
        760.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.purchases.modal_open = false),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), post_purchase),
        ],
        |st| st.purchases.modal_open = false,
    );

    if s.purchases.creating_item_for_line.is_some() {
        let quick = col()
            .gap(12.0)
            .child(ui.labeled("Name", ui.field(s, "quick-item-name", &ui.t("Enter a product name."))))
            .child(ui.form_grid(
                ui.labeled("Your cost", ui.field(s, "quick-item-cost", "۰")),
                ui.labeled("Unit price", ui.field(s, "quick-item-price", "۰")),
            ));
        ui.dialog_described(
            layer,
            &mut Vec::new(),
            "buy-quick-product",
            &ui.t("New product"),
            &ui.t("Add it to your inventory and this purchase at the same time."),
            true,
            440.0,
            quick,
            vec![
                ui.btn(Btn::Ghost, "Cancel", None, |st| st.purchases.creating_item_for_line = None),
                ui.btn(Btn::Primary, "Add product", Some(&icons::CHECK), |st| {
                    let Some(idx) = st.purchases.creating_item_for_line else { return };
                    let name = st.forms.get("quick-item-name").trim().to_string();
                    if name.is_empty() {
                        st.toast(Tone::Danger, st.fmt.t("Enter a product name."));
                        return;
                    }
                    let cost = st.parse_money_input(&st.forms.get("quick-item-cost")).to_string();
                    let sale = st.parse_money_input(&st.forms.get("quick-item-price")).to_string();
                    match st.create_item(&name, "", "", &sale, Some(&cost), "", "", "", false) {
                        Ok(id) => {
                            if let Some(l) = st.purchases.lines.get_mut(idx) {
                                l.item_id = id;
                                l.price = st.forms.get("quick-item-cost");
                            }
                            st.purchases.creating_item_for_line = None;
                            if st.purchases.lines.last().map(|l| !l.item_id.is_empty()).unwrap_or(false) {
                                st.purchases.lines.push(PurchaseLine::blank());
                                st.purchases.line_combos.push(ComboboxState::default());
                            }
                        }
                        Err(e) => st.toast_err(e),
                    }
                }),
            ],
            |st| st.purchases.creating_item_for_line = None,
        );
    }
}

fn return_selected(st: &mut State) {
    let Some(record) = st.books.list("purchases").into_iter().find(|r| crate::data::s(&r["id"]) == st.purchases.selected) else {
        st.toast(Tone::Info, st.fmt.t("No records"));
        return;
    };
    let returnable = record["command"].get("Receive").is_some() || record["command"].get("Bill").is_some();
    if !returnable {
        st.toast(Tone::Info, st.fmt.t("Goods that have arrived."));
        return;
    }
    let id = st.purchases.selected.clone();
    let reference = format!("PRET-{}", &id[..id.len().min(6)]);
    let key = format!("preturn-{id}");
    match crate::ui::ops::post_operation(st, &id, "purchase_return", &reference, "Return goods to the supplier", &key) {
        Ok(_) => st.toast(Tone::Success, st.fmt.t("Goods operation posted")),
        Err(e) => {
            if st.toasts.last().map(|t| t.message.as_str()) != Some(e.as_str()) {
                st.toast_err(e);
            }
        }
    }
}
