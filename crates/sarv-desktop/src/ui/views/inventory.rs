//! Inventory — brands, categories, search, and a full product profile suite (not a plain list).

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, SortDir, Table, TableColumn, TableState};
use serde_json::{Value, json};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, DIALOG_MD, Ui};
use crate::ui::icons;
use crate::ui::pickers;
use crate::ui::rows::{FilterCache, Row, SortKey, fold_text, reapply_sort, sort_rows};

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
    pub transfer_key: String,
    pub transfer_posting: bool,
    pub selected: String,
    pub profile_open: bool,
    pub stock_label: String,
    pub generation: String,
    pub brand_filter: String,
    pub category_filter: String,
    pub detail: Value,
    pub new_snapshot: String,
    pub profile_snapshot: String,
    pub transfer_snapshot: String,
    pub rows: Vec<Row>,
    pub search_blobs: HashMap<String, String>,
    pub filter_cache: FilterCache,
    pub rows_gen: u64,
    pub rows_stamp: u64,
}

fn new_product_snapshot(st: &State) -> String {
    format!(
        "{}\nunit={}\nsupplier={}\nservice={}",
        st.forms.snapshot_prefix("item-"),
        st.inventory.unit_id,
        st.inventory.supplier_id,
        st.inventory.is_service
    )
}

fn product_profile_snapshot(st: &State) -> String {
    format!("{}\nsupplier={}", st.forms.snapshot_prefix("prod-"), st.inventory.supplier_id)
}

fn transfer_snapshot(st: &State) -> String {
    format!(
        "{}\nitem={}\nfrom={}\nto={}",
        st.forms.snapshot_prefix("transfer-"),
        st.inventory.transfer_item,
        st.inventory.transfer_from,
        st.inventory.transfer_to
    )
}

pub fn open_new_product(st: &mut State) {
    st.forms.clear_prefix("item-");
    st.forms.set("item-min-stock", "0");
    st.inventory.unit_id = pickers::first_unit(st);
    st.inventory.supplier_id.clear();
    st.inventory.is_service = false;
    st.inventory.new_snapshot = new_product_snapshot(st);
    st.inventory.modal_open = true;
}

pub fn close_new_or_confirm(st: &mut State) {
    if new_product_snapshot(st) == st.inventory.new_snapshot {
        st.inventory.modal_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.inventory.modal_open = false);
}

pub fn open_transfer(st: &mut State) {
    st.forms.clear_prefix("transfer-");
    st.forms.set("transfer-qty", "1");
    st.inventory.transfer_item.clear();
    st.inventory.transfer_from = pickers::first_warehouse(st);
    st.inventory.transfer_to.clear();
    st.inventory.transfer_key = st.mint_key("transfer");
    st.inventory.transfer_posting = false;
    st.inventory.transfer_snapshot = transfer_snapshot(st);
    st.inventory.transfer_open = true;
}

pub fn close_transfer_or_confirm(st: &mut State) {
    if transfer_snapshot(st) == st.inventory.transfer_snapshot {
        st.inventory.transfer_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.inventory.transfer_open = false);
}

pub fn close_profile_or_confirm(st: &mut State) {
    if product_profile_snapshot(st) == st.inventory.profile_snapshot {
        st.inventory.profile_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.inventory.profile_open = false);
}

fn items(s: &State) -> &[serde_json::Value] {
    s.books.workspace["resources"]["items"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

fn product_profile<'a>(s: &'a State, id: &str) -> &'a Value {
    &s.books.workspace["product_profiles"][id]
}

fn sale_price(s: &State, id: &str) -> String {
    let p = &product_profile(s, id)["sale_price"];
    if p.is_null() { "0".into() } else { crate::data::s(p) }
}

fn unit_name(s: &State, unit_id: &str) -> String {
    s.books.workspace["resources"]["units"]
        .as_array()
        .and_then(|units| units.iter().find(|u| u["id"].as_str() == Some(unit_id)))
        .map(|u| crate::data::s(&u["name"]))
        .unwrap_or_else(|| s.fmt.t("Unit"))
}

fn catalog_values(s: &State, field: &str) -> Vec<String> {
    let mut set = BTreeSet::new();
    for item in items(s) {
        let id = crate::data::s(&item["id"]);
        let value = crate::data::s(&product_profile(s, &id)[field]);
        if !value.trim().is_empty() {
            set.insert(value);
        }
    }
    set.into_iter().collect()
}

fn money_field_from_rial(st: &State, rials: &str) -> String {
    if rials.is_empty() {
        return String::new();
    }
    match rials.trim().parse::<i64>() {
        Ok(v) => st.fmt.money_input(v),
        Err(_) => String::new(),
    }
}

fn col_index(col: &str) -> Option<usize> {
    match col {
        "name" => Some(0),
        "brand" => Some(1),
        "category" => Some(2),
        "code" => Some(3),
        "unit" => Some(4),
        "price" => Some(5),
        _ => None,
    }
}

fn build_rows(s: &State) -> (Vec<Row>, HashMap<String, String>) {
    let mut rows = Vec::new();
    let mut blobs = HashMap::new();
    for item in items(s) {
        let id = crate::data::s(&item["id"]);
        let profile = product_profile(s, &id);
        let brand_raw = crate::data::s(&profile["brand"]);
        let category_raw = crate::data::s(&profile["category"]);
        if !s.inventory.brand_filter.is_empty() && brand_raw != s.inventory.brand_filter {
            continue;
        }
        if !s.inventory.category_filter.is_empty() && category_raw != s.inventory.category_filter {
            continue;
        }
        let row_id = format!("item-{id}");
        let name = crate::data::s(&item["name"]);
        let brand = if brand_raw.is_empty() { "—".into() } else { brand_raw.clone() };
        let category = if category_raw.is_empty() {
            "—".into()
        } else {
            category_raw.clone()
        };
        let sku = crate::data::s(&item["sku"]);
        let code = if sku.is_empty() { id.clone() } else { sku.clone() };
        let unit = unit_name(s, &crate::data::s(&item["base_unit_id"]));
        let price_raw = sale_price(s, &id);
        let barcode = crate::data::s(&profile["barcode"]);
        let blob = fold_text(&format!("{name}\n{sku}\n{barcode}\n{brand_raw}\n{category_raw}\n{id}"));
        blobs.insert(row_id.clone(), blob);
        rows.push(Row {
            id: row_id,
            cells: vec![
                name.clone(),
                brand.clone(),
                category.clone(),
                crate::i18n::isolate(&code),
                unit.clone(),
                s.fmt.money(&price_raw),
            ],
            keys: vec![
                SortKey::text(&name),
                SortKey::text(&brand),
                SortKey::text(&category),
                SortKey::text(&code),
                SortKey::text(&unit),
                SortKey::num_str(&price_raw),
            ],
        });
    }
    (rows, blobs)
}

pub fn ensure(st: &mut State) {
    if st.inventory.rows_stamp == st.stamp {
        return;
    }
    let (rows, blobs) = build_rows(st);
    st.inventory.rows = rows;
    st.inventory.search_blobs = blobs;
    st.inventory.rows_stamp = st.stamp;
    st.inventory.rows_gen += 1;
    reapply_sort(&st.inventory.table, &mut st.inventory.rows, col_index);
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.inventory.table.set_sort(col, dir);
    if let Some(idx) = col_index(col) {
        sort_rows(&mut st.inventory.rows, idx, dir);
        st.inventory.rows_gen += 1;
    }
}

fn visible_indices(s: &State) -> std::cell::Ref<'_, Vec<usize>> {
    let q = s.forms.get("inventory-search");
    s.inventory
        .filter_cache
        .visible(&s.inventory.rows, s.inventory.rows_gen, &q, |row, q_folded| {
            let blob = s.inventory.search_blobs.get(&row.id).map(String::as_str).unwrap_or("");
            blob.contains(q_folded)
        })
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let visible_count = visible_indices(s).len();
    let table = Table::new(
        |s: &State| &s.inventory.table,
        |s: &mut State| &mut s.inventory.table,
        |s: &State| visible_indices(s).len(),
        |s: &State, i| {
            let vis = visible_indices(s);
            Cow::Owned(
                vis.get(i)
                    .and_then(|&idx| s.inventory.rows.get(idx))
                    .map(|r| r.id.clone())
                    .unwrap_or_default(),
            )
        },
    )
    .name(ui.t("Inventory"))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("brand", ui.t("Brand")).width(120.0).min(88.0).sortable())
    .column(TableColumn::new("category", ui.t("Category")).width(120.0).min(88.0).sortable())
    .column(TableColumn::new("code", ui.t("Code")).width(110.0).min(72.0).sortable())
    .column(TableColumn::new("unit", ui.t("Unit")).width(90.0).min(64.0).sortable())
    .column(
        TableColumn::new("price", ui.t("Price"))
            .width(130.0)
            .min(96.0)
            .align(Align::End)
            .sortable(),
    )
    .cell(|s: &State, i, col| {
        let vis = visible_indices(s);
        let Some(row) = vis.get(i).and_then(|&idx| s.inventory.rows.get(idx)) else {
            return Cell::text("");
        };
        match col {
            "name" => Cell::text(&row.cells[0]),
            "brand" => Cell::text(&row.cells[1]).muted(),
            "category" => Cell::text(&row.cells[2]).muted(),
            "code" => Cell::text(&row.cells[3]).muted(),
            "unit" => Cell::text(&row.cells[4]).muted(),
            _ => Cell::text(&row.cells[5]).mono(),
        }
    })
    .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
    .on_select(|st, id, _| st.inventory.selected = id.strip_prefix("item-").unwrap_or(id).to_string())
    .on_open(|st, id, _| open_product(st, id.strip_prefix("item-").unwrap_or(id)))
    .zebra(true)
    .build(s, &ui.k)
    .key("inventory-table")
    .grow(1.0)
    .min_h(0.0);

    let brands = catalog_values(s, "brand");
    let categories = catalog_values(s, "category");
    let mut portals = Vec::new();

    let mut brand_opts = vec![("".to_string(), ui.t("All brands"))];
    brand_opts.extend(brands.iter().cloned().map(|b| (b.clone(), b)));
    let brand_open = s.selects.get("inventory-brand").copied().unwrap_or(false);
    let brand_select = pickers::select_field(
        ui,
        &mut portals,
        "inventory-brand",
        &brand_opts,
        Some(s.inventory.brand_filter.as_str()),
        &ui.t("Brand"),
        pickers::PICKER_WIDTH,
        brand_open,
        |st, id| {
            st.inventory.brand_filter = id.to_string();
            st.inventory.rows_stamp = 0;
            ensure(st);
        },
    );

    let mut category_opts = vec![("".to_string(), ui.t("All categories"))];
    category_opts.extend(categories.iter().cloned().map(|c| (c.clone(), c)));
    let category_open = s.selects.get("inventory-category").copied().unwrap_or(false);
    let category_select = pickers::select_field(
        ui,
        &mut portals,
        "inventory-category",
        &category_opts,
        Some(s.inventory.category_filter.as_str()),
        &ui.t("Category"),
        pickers::PICKER_WIDTH,
        category_open,
        |st, id| {
            st.inventory.category_filter = id.to_string();
            st.inventory.rows_stamp = 0;
            ensure(st);
        },
    );

    let add_btn = ui.btn(Btn::Primary, "New product", Some(&icons::PLUS), open_new_product);

    if s.inventory.modal_open {
        mount_create(s, ui, layer);
    }
    if s.inventory.transfer_open {
        mount_transfer(s, ui, layer);
    }
    if s.inventory.profile_open {
        mount_product_profile(s, ui, layer);
    }

    let all_items = items(s);
    let total_items = all_items.len();
    let inv_kpis = row()
        .w_full()
        .gap(12.0)
        .child(ui.metric(
            "inv-kpi-total",
            &icons::INVENTORY,
            &ui.t("Products"),
            ui.f.digits(&total_items.to_string()),
            ui.t("Products and services you sell"),
            Tone::Info,
        ))
        .child(ui.metric(
            "inv-kpi-categories",
            &icons::SPARKLE,
            &ui.t("Category"),
            ui.f.digits(&categories.len().to_string()),
            format!("{} {}", ui.f.digits(&brands.len().to_string()), ui.t("Brand")),
            Tone::Success,
        ));

    let mut filters = row()
        .w_full()
        .gap(10.0)
        .center_y()
        .child(ui.field(s, "inventory-search", &ui.t("Search products")).grow(1.0).min_w(0.0))
        .child(brand_select)
        .child(category_select);
    if !s.inventory.selected.is_empty() {
        filters = filters.child(ui.btn(Btn::Glass, "Product details", None, |st| {
            let id = st.inventory.selected.clone();
            open_product(st, &id);
        }));
    }

    let body = col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(inv_kpis)
        .child(filters)
        .child(ui.fill_or_empty(
            visible_count,
            if total_items == 0 { "No products" } else { "Nothing to show" },
            "Products and services you sell",
            &icons::INVENTORY,
            table,
        ));

    layer.append(&mut portals);

    ui.page(
        "inventory-screen",
        &ui.t("Inventory"),
        &ui.t("Organize products by brand and category"),
        vec![ui.btn(Btn::Glass, "Transfer stock", Some(&icons::ARROW), open_transfer), add_btn],
        body,
    )
}

fn mount_create(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let mut portals = Vec::new();
    let units = pickers::unit_options(s);
    let unit_open = s.selects.get("item-unit").copied().unwrap_or(false);
    let unit_select = pickers::select_field(
        ui,
        &mut portals,
        "item-unit",
        &units,
        if s.inventory.unit_id.is_empty() {
            None
        } else {
            Some(s.inventory.unit_id.as_str())
        },
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
        .gap(16.0)
        .child(kind_bar)
        .child(
            ui.section(
                "item-basics",
                &ui.t("Product"),
                col()
                    .gap(12.0)
                    .child(ui.labeled("Name", ui.field(s, "item-name", &ui.t("Enter a product name."))))
                    .child(ui.form_grid(
                        ui.labeled("Code", ui.field(s, "item-sku", "PRD-101")),
                        ui.labeled("Unit", unit_select),
                    ))
                    .child(ui.form_grid(
                        ui.labeled("Brand", ui.field(s, "item-brand", &ui.t("Brand"))),
                        ui.labeled("Category", ui.field(s, "item-category", &ui.t("Category"))),
                    )),
            ),
        )
        .child(
            ui.section(
                "item-pricing",
                &ui.t("Pricing"),
                col()
                    .gap(12.0)
                    .child(ui.form_grid(
                        ui.labeled("Your cost", ui.money_field(s, "item-cost")),
                        ui.labeled("Unit price", ui.money_field(s, "item-price")),
                    ))
                    .child(ui.form_grid(
                        ui.labeled("Barcode", ui.field(s, "item-barcode", "")),
                        ui.labeled("Preferred supplier", supplier_select),
                    ))
                    .child(ui.labeled("Minimum stock · base units", ui.field(s, "item-min-stock", "0"))),
            ),
        )
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
        let brand = st.forms.get("item-brand").trim().to_string();
        let category = st.forms.get("item-category").trim().to_string();
        let sale = match st.parse_money_input(&st.forms.get("item-price")) {
            Ok(v) => v.to_string(),
            Err(crate::i18n::MoneyError::Empty) => "0".into(),
            Err(_) => {
                st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
                return;
            }
        };
        let cost_raw = st.forms.get("item-cost");
        let cost = if cost_raw.trim().is_empty() {
            None
        } else {
            match st.parse_money_input(&cost_raw) {
                Ok(v) => Some(v.to_string()),
                Err(_) => {
                    st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
                    return;
                }
            }
        };
        let unit = st.inventory.unit_id.clone();
        let supplier = st.inventory.supplier_id.clone();
        let service = st.inventory.is_service;
        let min_raw = crate::i18n::latin_digits(&st.forms.get("item-min-stock")).replace(',', "");
        let min_stock = if min_raw.trim().is_empty() { "0".into() } else { min_raw };
        if st
            .create_item(
                &name,
                &sku,
                &unit,
                &sale,
                cost.as_deref(),
                &barcode,
                &notes,
                &supplier,
                &brand,
                &category,
                &min_stock,
                service,
            )
            .is_ok()
        {
            st.inventory.modal_open = false;
        }
    });
    ui.form_dialog(
        layer,
        &mut portals,
        "product",
        &ui.t("New product"),
        &ui.t("Track stock, pricing, brands, and preferred suppliers."),
        true,
        DIALOG_MD,
        form,
        vec![ui.btn(Btn::Ghost, "Cancel", None, close_new_or_confirm), save_btn],
        close_new_or_confirm,
    );
}

fn mount_transfer(s: &State, ui: &Ui, layer: &mut Vec<El>) {
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
        if s.inventory.transfer_item.is_empty() {
            None
        } else {
            Some(s.inventory.transfer_item.as_str())
        },
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
        if s.inventory.transfer_from.is_empty() {
            None
        } else {
            Some(s.inventory.transfer_from.as_str())
        },
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
        if s.inventory.transfer_to.is_empty() {
            None
        } else {
            Some(s.inventory.transfer_to.as_str())
        },
        &ui.t("Select warehouse"),
        pickers::PICKER_WIDTH,
        to_open,
        |st, id| st.inventory.transfer_to = id.to_string(),
    );
    let form = col()
        .gap(12.0)
        .child(ui.labeled("Select product", item))
        .child(ui.form_grid(ui.labeled("From", from), ui.labeled("To", to)))
        .child(ui.form_grid(
            ui.labeled("Qty", ui.field(s, "transfer-qty", "1")),
            ui.date_field(s, "transfer-date", "Date"),
        ));

    ui.form_dialog(
        layer,
        &mut portals,
        "transfer",
        &ui.t("Transfer stock"),
        &ui.t("Move stock between warehouses at carried cost."),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_transfer_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.inventory.transfer_posting || !st.inventory.transfer_open {
                    return;
                }
                st.inventory.transfer_posting = true;
                if st.inventory.transfer_item.is_empty() || st.inventory.transfer_from.is_empty() || st.inventory.transfer_to.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Choose the product and both warehouses."));
                    st.inventory.transfer_posting = false;
                    return;
                }
                if st.inventory.transfer_from == st.inventory.transfer_to {
                    st.toast(Tone::Danger, st.fmt.t("Choose two different warehouses."));
                    st.inventory.transfer_posting = false;
                    return;
                }
                if st.inventory.transfer_key.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Reopen this form and try again."));
                    st.inventory.transfer_posting = false;
                    return;
                }
                let Ok(date) = st.form_date("transfer-date") else {
                    st.inventory.transfer_posting = false;
                    return;
                };
                let key = st.inventory.transfer_key.clone();
                let qty = crate::i18n::latin_digits(&st.forms.get("transfer-qty")).replace(',', "");
                let unit = pickers::item_unit(st, &st.inventory.transfer_item);
                let mut payload = json!({
                    "item_id": st.inventory.transfer_item,
                    "source_warehouse_id": st.inventory.transfer_from,
                    "target_warehouse_id": st.inventory.transfer_to,
                    "unit_id": unit,
                    "quantity": qty,
                    "date": date,
                    "source_id": key,
                    "request_key": key
                });
                match st.query("transfer_preview", payload.clone()) {
                    Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
                    Err(e) => {
                        st.toast_err(e);
                        st.inventory.transfer_posting = false;
                        return;
                    }
                }
                if st.run("transfer_post", payload, Some("Posted")).is_ok() {
                    st.inventory.transfer_open = false;
                    st.inventory.transfer_key.clear();
                }
                st.inventory.transfer_posting = false;
            }),
        ],
        close_transfer_or_confirm,
    );
}

pub(crate) fn open_product(st: &mut State, item_id: &str) {
    match st.query("product_detail", json!({ "item_id": item_id, "as_of": crate::data::today() })) {
        Ok(v) => {
            let profile = &v["profile"];
            st.forms.set("prod-barcode", &crate::data::s(&profile["barcode"]));
            st.forms.set("prod-brand", &crate::data::s(&profile["brand"]));
            st.forms.set("prod-category", &crate::data::s(&profile["category"]));
            st.forms.set("prod-notes", &crate::data::s(&profile["notes"]));
            st.forms
                .set("prod-price", &money_field_from_rial(st, &crate::data::s(&profile["sale_price"])));
            st.forms
                .set("prod-cost", &money_field_from_rial(st, &crate::data::s(&profile["purchase_price"])));
            let min_th = crate::data::s(&profile["minimum_stock_thousandths"]);
            st.forms.set("prod-min-stock", &crate::ui::ops::qty_from_thousandths(&min_th));
            st.inventory.supplier_id = crate::data::s(&profile["preferred_supplier_id"]);
            let stock = crate::data::s(&v["stats"]["stock_quantity"]);
            st.inventory.stock_label = crate::ui::ops::qty_from_thousandths(&stock);
            st.inventory.generation = crate::data::s(&v["_generation"]);
            st.inventory.detail = v;
            st.inventory.selected = item_id.to_string();
            st.inventory.profile_snapshot = product_profile_snapshot(st);
            st.inventory.profile_open = true;
        }
        Err(e) => st.toast_err(e),
    }
}

fn mount_product_profile(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let stats = &s.inventory.detail["stats"];
    let stock_value = ui.f.money(&crate::data::s(&stats["stock_value"]));
    let net_sales = ui.f.money(&crate::data::s(&stats["net_sales"]));
    let low = stats["low_stock"].as_bool().unwrap_or(false);
    let stock_sub = if low { ui.t("Low stock") } else { ui.t("On hand") };
    let metrics = row()
        .w_full()
        .gap(10.0)
        .child(ui.metric(
            "prod-stock",
            &icons::INVENTORY,
            &ui.t("Remaining"),
            s.fmt.digits(&s.inventory.stock_label),
            stock_sub,
            if low { Tone::Warning } else { Tone::Info },
        ))
        .child(ui.metric(
            "prod-value",
            &icons::ACCOUNTING,
            &ui.t("Stock value"),
            stock_value,
            ui.t("Carried cost"),
            Tone::Info,
        ))
        .child(ui.metric(
            "prod-sales",
            &icons::SALES,
            &ui.t("Net sales"),
            net_sales,
            ui.t("Posted invoices"),
            Tone::Success,
        ));

    let mut portals = Vec::new();
    let suppliers = pickers::party_options(s, false, true)
        .into_iter()
        .map(|o| (o.id, o.label))
        .collect::<Vec<_>>();
    let supp_open = s.selects.get("prod-supplier").copied().unwrap_or(false);
    let supplier_select = pickers::select_field(
        ui,
        &mut portals,
        "prod-supplier",
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

    let form = col()
        .gap(16.0)
        .child(metrics)
        .child(
            ui.section(
                "prod-catalog",
                &ui.t("Catalog"),
                col()
                    .gap(12.0)
                    .child(ui.form_grid(
                        ui.labeled("Brand", ui.field(s, "prod-brand", "")),
                        ui.labeled("Category", ui.field(s, "prod-category", "")),
                    ))
                    .child(ui.form_grid(
                        ui.labeled("Barcode", ui.field(s, "prod-barcode", "")),
                        ui.labeled("Preferred supplier", supplier_select),
                    )),
            ),
        )
        .child(
            ui.section(
                "prod-commerce",
                &ui.t("Pricing"),
                col()
                    .gap(12.0)
                    .child(ui.form_grid(
                        ui.labeled("Your cost", ui.money_field(s, "prod-cost")),
                        ui.labeled("Unit price", ui.money_field(s, "prod-price")),
                    ))
                    .child(ui.labeled("Minimum stock · base units", ui.field(s, "prod-min-stock", "0")))
                    .child(ui.labeled("Product notes", ui.field(s, "prod-notes", ""))),
            ),
        );

    ui.form_dialog(
        layer,
        &mut portals,
        "product-profile",
        &ui.t("Product details"),
        &ui.t("Product details, stock costs and performance from posted documents."),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_profile_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                let id = st.inventory.selected.clone();
                let sale_typed = st.forms.get("prod-price");
                let sale_price = if sale_typed.trim().is_empty() {
                    serde_json::Value::Null
                } else {
                    match st.parse_money_input(&sale_typed) {
                        Ok(v) => json!(v.to_string()),
                        Err(_) => {
                            st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
                            return;
                        }
                    }
                };
                let cost_typed = st.forms.get("prod-cost");
                let purchase_price = if cost_typed.trim().is_empty() {
                    serde_json::Value::Null
                } else {
                    match st.parse_money_input(&cost_typed) {
                        Ok(v) => json!(v.to_string()),
                        Err(_) => {
                            st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
                            return;
                        }
                    }
                };
                let min_raw = crate::i18n::latin_digits(&st.forms.get("prod-min-stock")).replace(',', "");
                let profile = json!({
                    "barcode": st.forms.get("prod-barcode"),
                    "brand": st.forms.get("prod-brand"),
                    "category": st.forms.get("prod-category"),
                    "notes": st.forms.get("prod-notes"),
                    "sale_price": sale_price,
                    "purchase_price": purchase_price,
                    "minimum_stock": if min_raw.trim().is_empty() { "0" } else { &min_raw },
                    "preferred_supplier_id": st.inventory.supplier_id
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
        close_profile_or_confirm,
    );
}
