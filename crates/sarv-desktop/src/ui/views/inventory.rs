//! Inventory — kit Table + product dialog with unit Select and creatable fields.

use std::borrow::Cow;

use rune::{Align, Cx, El, col};
use rune_kit::{Cell, Table, TableColumn, TableState};

use serde_json::json;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;
use crate::ui::pickers;

#[derive(Default)]
pub struct InventoryState {
    pub modal_open: bool,
    pub transfer_open: bool,
    pub table: TableState,
    pub unit_id: String,
    pub supplier_id: String,
    pub is_service: bool,
    pub transfer_item: String,
    pub transfer_from: String,
    pub transfer_to: String,
    pub selected: String,
    pub profile_open: bool,
    pub stock_label: String,
    pub generation: String,
}

fn items(s: &State) -> &[serde_json::Value] {
    s.books.workspace["resources"]["items"].as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

fn sale_price(s: &State, id: &str) -> String {
    let p = &s.books.workspace["product_profiles"][id]["sale_price"];
    if p.is_null() {
        "0".into()
    } else {
        crate::data::s(p)
    }
}

fn unit_name(s: &State, unit_id: &str) -> String {
    s.books.workspace["resources"]["units"]
        .as_array()
        .and_then(|units| units.iter().find(|u| u["id"].as_str() == Some(unit_id)))
        .map(|u| crate::data::s(&u["name"]))
        .unwrap_or_else(|| "Unit".into())
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let table = Table::new(
        |s: &State| &s.inventory.table,
        |s: &mut State| &mut s.inventory.table,
        |s: &State| items(s).len(),
        |s: &State, i| {
            let id = crate::data::s(&items(s)[i]["id"]);
            Cow::Owned(format!("item-{id}"))
        },
    )
    .name(ui.t("Inventory"))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("code", ui.t("Code")).width(120.0).min(80.0).sortable())
    .column(TableColumn::new("unit", ui.t("Unit")).width(100.0).min(72.0))
    .column(TableColumn::new("price", ui.t("Price")).width(140.0).min(96.0).align(Align::End).sortable())
    .cell(|s: &State, i, col| {
        let item = &items(s)[i];
        let id = crate::data::s(&item["id"]);
        match col {
            "name" => Cell::text(crate::data::s(&item["name"])),
            "code" => {
                let code = crate::data::s(&item["sku"]);
                Cell::text(if code.is_empty() { id } else { code }).muted()
            }
            "unit" => Cell::text(unit_name(s, &crate::data::s(&item["base_unit_id"]))).muted(),
            _ => Cell::text(s.fmt.money(&sale_price(s, &id))).mono(),
        }
    })
    .on_select(|st, id, _| st.inventory.selected = id.strip_prefix("item-").unwrap_or(&id).to_string())
    .on_open(|st, id, _| open_product(st, id.strip_prefix("item-").unwrap_or(id)))
    .zebra(true)
    .build(s, &ui.k)
    .key("inventory-table")
    .grow(1.0)
    .min_h(0.0);

    let add_btn = ui.btn(Btn::Primary, "New product", Some(&icons::PLUS), |st| {
        st.inventory.modal_open = true;
        st.inventory.unit_id = pickers::first_unit(st);
        st.inventory.supplier_id.clear();
        st.inventory.is_service = false;
        st.forms.set("item-name", "");
        st.forms.set("item-sku", "");
        st.forms.set("item-price", "");
        st.forms.set("item-cost", "");
        st.forms.set("item-barcode", "");
        st.forms.set("item-notes", "");
    });

    if s.inventory.modal_open {
        let mut portals = Vec::new();
        let units = pickers::unit_options(s);
        let unit_open = s.selects.get("item-unit").copied().unwrap_or(false);
        let unit_select = pickers::select_field(
            ui,
            &mut portals,
            "item-unit",
            &units,
            if s.inventory.unit_id.is_empty() { None } else { Some(s.inventory.unit_id.as_str()) },
            &ui.t("Select unit"),
            pickers::PICKER_WIDTH,
            unit_open,
            |st, id| st.inventory.unit_id = id.to_string(),
        );
        let suppliers = pickers::party_options(s, false, true)
            .into_iter()
            .map(|o| (o.id, o.label))
            .collect::<Vec<_>>();
        let supp_open = s.selects.get("item-supplier").copied().unwrap_or(false);
        let supplier_select = pickers::select_field(
            ui,
            &mut portals,
            "item-supplier",
            &suppliers,
            if s.inventory.supplier_id.is_empty() {
                None
            } else {
                Some(s.inventory.supplier_id.as_str())
            },
            &ui.t("Select supplier"),
            pickers::PICKER_WIDTH,
            supp_open,
            |st, id| st.inventory.supplier_id = id.to_string(),
        );

        let kind_stock = ui.t("Stock item · Track inventory");
        let kind_svc = ui.t("Service / Non-stock");
        let kind_tabs = [
            (kind_stock.as_str(), !s.inventory.is_service),
            (kind_svc.as_str(), s.inventory.is_service),
        ];
        let kind_bar = ui.tabs("item-kind", &kind_tabs, |st, idx| st.inventory.is_service = idx == 1);

        let form = col()
            .gap(12.0)
            .child(kind_bar)
            .child(ui.labeled("Name", ui.field(s, "item-name", &ui.t("Enter a product name."))))
            .child(ui.form_grid(
                ui.labeled("Code", ui.field(s, "item-sku", "PRD-101")),
                ui.labeled("Unit", unit_select),
            ))
            .child(ui.form_grid(
                ui.labeled("Your cost", ui.field(s, "item-cost", "۰")),
                ui.labeled("Unit price", ui.field(s, "item-price", "۱۵۰٬۰۰۰")),
            ))
            .child(ui.form_grid(
                ui.labeled("Barcode", ui.field(s, "item-barcode", "")),
                ui.labeled("Preferred supplier", supplier_select),
            ))
            .child(ui.labeled("Description", ui.field(s, "item-notes", &ui.t("Note (optional)"))));

        let save_btn = ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), move |st| {
            let name = st.forms.get("item-name").trim().to_string();
            if name.is_empty() {
                st.toast(Tone::Danger, st.fmt.t("Enter a product name."));
                return;
            }
            let sku = st.forms.get("item-sku").trim().to_string();
            let barcode = st.forms.get("item-barcode").trim().to_string();
            let notes = st.forms.get("item-notes").trim().to_string();
            let sale = st.parse_money_input(&st.forms.get("item-price")).to_string();
            let cost_raw = st.forms.get("item-cost");
            let cost = if cost_raw.trim().is_empty() {
                None
            } else {
                Some(st.parse_money_input(&cost_raw).to_string())
            };
            let unit = st.inventory.unit_id.clone();
            let supplier = st.inventory.supplier_id.clone();
            let service = st.inventory.is_service;
            match st.create_item(
                &name,
                &sku,
                &unit,
                &sale,
                cost.as_deref(),
                &barcode,
                &notes,
                &supplier,
                service,
            ) {
                Ok(_) => st.inventory.modal_open = false,
                Err(e) => st.toast_err(e),
            }
        });
        ui.dialog_described(
            layer,
            &mut portals,
            "product",
            &ui.t("New product"),
            &ui.t("Track stock, pricing, and preferred suppliers."),
            true,
            560.0,
            form,
            vec![
                ui.btn(Btn::Ghost, "Cancel", None, |st| st.inventory.modal_open = false),
                save_btn,
            ],
            |st| st.inventory.modal_open = false,
        );
    }

    if s.inventory.transfer_open {
        mount_transfer(s, ui, layer);
    }
    if s.inventory.profile_open {
        mount_product_profile(s, ui, layer);
    }

    ui.page(
        "inventory-screen",
        &ui.t("Inventory"),
        &ui.t("Products and services you sell"),
        vec![
            ui.btn(Btn::Glass, "Transfer stock", Some(&icons::ARROW), |st| {
                st.inventory.transfer_open = true;
                st.inventory.transfer_item.clear();
                st.inventory.transfer_from = pickers::first_warehouse(st);
                st.inventory.transfer_to.clear();
                st.forms.set("transfer-qty", "1");
            }),
            add_btn,
            ui.btn(Btn::Glass, "Product details", None, |st| {
                if st.inventory.selected.is_empty() {
                    st.toast(Tone::Info, st.fmt.t("Select product"));
                    return;
                }
                let id = st.inventory.selected.clone();
                open_product(st, &id);
            }),
        ],
        ui.fill_or_empty(items(s).len(), "No products", "Products and services you sell", &icons::INVENTORY, table),
    )
}

fn mount_transfer(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    use serde_json::json;

    let mut portals = Vec::new();
    let items: Vec<_> = pickers::item_options(s).into_iter().map(|o| (o.id, o.label)).collect();
    let wh = pickers::warehouse_options(s);
    let item_open = s.selects.get("transfer-item").copied().unwrap_or(false);
    let from_open = s.selects.get("transfer-from").copied().unwrap_or(false);
    let to_open = s.selects.get("transfer-to").copied().unwrap_or(false);
    let item = pickers::select_field(
        ui,
        &mut portals,
        "transfer-item",
        &items,
        if s.inventory.transfer_item.is_empty() { None } else { Some(s.inventory.transfer_item.as_str()) },
        &ui.t("Select product"),
        pickers::PICKER_WIDTH,
        item_open,
        |st, id| st.inventory.transfer_item = id.to_string(),
    );
    let from = pickers::select_field(
        ui,
        &mut portals,
        "transfer-from",
        &wh,
        if s.inventory.transfer_from.is_empty() { None } else { Some(s.inventory.transfer_from.as_str()) },
        &ui.t("Select warehouse"),
        pickers::PICKER_WIDTH,
        from_open,
        |st, id| st.inventory.transfer_from = id.to_string(),
    );
    let to = pickers::select_field(
        ui,
        &mut portals,
        "transfer-to",
        &wh,
        if s.inventory.transfer_to.is_empty() { None } else { Some(s.inventory.transfer_to.as_str()) },
        &ui.t("Select warehouse"),
        pickers::PICKER_WIDTH,
        to_open,
        |st, id| st.inventory.transfer_to = id.to_string(),
    );
    let form = col()
        .gap(12.0)
        .child(ui.labeled("Select product", item))
        .child(ui.form_grid(ui.labeled("From", from), ui.labeled("To", to)))
        .child(ui.labeled("Qty", ui.field(s, "transfer-qty", "1")));

    ui.dialog_described(
        layer,
        &mut portals,
        "transfer",
        &ui.t("Transfer stock"),
        &ui.t("Move stock between warehouses at carried cost."),
        true,
        520.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.inventory.transfer_open = false),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.inventory.transfer_item.is_empty()
                    || st.inventory.transfer_from.is_empty()
                    || st.inventory.transfer_to.is_empty()
                {
                    st.toast(Tone::Danger, st.fmt.t("Select product"));
                    return;
                }
                if st.inventory.transfer_from == st.inventory.transfer_to {
                    st.toast(Tone::Danger, st.fmt.t("Select warehouse"));
                    return;
                }
                let qty = crate::i18n::latin_digits(&st.forms.get("transfer-qty")).replace(',', "");
                let unit = pickers::item_unit(st, &st.inventory.transfer_item);
                let mut payload = json!({
                    "item_id": st.inventory.transfer_item,
                    "source_warehouse_id": st.inventory.transfer_from,
                    "target_warehouse_id": st.inventory.transfer_to,
                    "unit_id": unit,
                    "quantity": qty
                });
                match st.query("transfer_preview", payload.clone()) {
                    Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
                    Err(e) => {
                        st.toast_err(e);
                        return;
                    }
                }
                if st.run("transfer_post", payload, Some("Posted")).is_ok() {
                    st.inventory.transfer_open = false;
                }
            }),
        ],
        |st| st.inventory.transfer_open = false,
    );
}

fn open_product(st: &mut State, item_id: &str) {
    match st.query("product_detail", json!({ "item_id": item_id, "as_of": crate::data::today() })) {
        Ok(v) => {
            let profile = &v["profile"];
            st.forms.set("prod-barcode", &crate::data::s(&profile["barcode"]));
            st.forms.set("prod-category", &crate::data::s(&profile["category"]));
            st.forms.set("prod-notes", &crate::data::s(&profile["notes"]));
            let price = crate::data::s(&profile["sale_price"]);
            let shown = if price.is_empty() {
                String::new()
            } else if st.fmt.unit == crate::i18n::Unit::Toman {
                (price.parse::<i64>().unwrap_or(0) / 10).to_string()
            } else {
                price
            };
            st.forms.set("prod-price", &shown);
            let stock = crate::data::s(&v["stats"]["stock_quantity"]);
            st.inventory.stock_label = crate::ui::ops::qty_from_thousandths(&stock);
            st.inventory.generation = crate::data::s(&v["_generation"]);
            st.inventory.selected = item_id.to_string();
            st.inventory.profile_open = true;
        }
        Err(e) => st.toast_err(e),
    }
}

fn mount_product_profile(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let form = col()
        .gap(12.0)
        .child(ui.muted(format!("{} {}", ui.t("Remaining"), s.inventory.stock_label)))
        .child(ui.form_grid(
            ui.labeled("Barcode", ui.field(s, "prod-barcode", "")),
            ui.labeled("Product type", ui.field(s, "prod-category", "")),
        ))
        .child(ui.labeled("Unit price", ui.field(s, "prod-price", "0")))
        .child(ui.labeled("Product notes", ui.field(s, "prod-notes", "")));
    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "product-profile",
        &ui.t("Product details"),
        &ui.t("Product details, stock costs and performance from posted documents."),
        true,
        560.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.inventory.profile_open = false),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                let id = st.inventory.selected.clone();
                let typed = st.forms.get("prod-price");
                let sale_price = if typed.trim().is_empty() {
                    serde_json::Value::Null
                } else {
                    json!(st.parse_money_input(&typed).to_string())
                };
                let profile = json!({
                    "barcode": st.forms.get("prod-barcode"),
                    "category": st.forms.get("prod-category"),
                    "notes": st.forms.get("prod-notes"),
                    "sale_price": sale_price,
                    "minimum_stock": "0"
                });
                if st
                    .run(
                        "product_profile",
                        json!({
                            "item_id": id,
                            "profile": profile,
                            "expected_generation": st.inventory.generation
                        }),
                        Some("Product saved"),
                    )
                    .is_ok()
                {
                    st.inventory.profile_open = false;
                }
            }),
        ],
        |st| st.inventory.profile_open = false,
    );
}
