//! Entity Combobox and Select helpers. Pick an existing record or create one.

use rune::{El, row, text};
use rune_kit::{
    Combobox, ComboboxOption, ComboboxState, ControlSize, Placement, Select, SelectAppearance,
};

use crate::app::State;
use crate::ui::glass::{Btn, Ui};

pub const CREATE_PREFIX: &str = "__create__:";

/// Party / product / unit pickers.
pub const PICKER_WIDTH: f32 = 280.0;
/// Chart-of-accounts and other account selects (`code · name`).
pub const ACCOUNT_PICKER_WIDTH: f32 = 420.0;

pub fn is_create(id: &str) -> bool {
    id.starts_with(CREATE_PREFIX)
}

pub fn create_text(id: &str) -> Option<&str> {
    id.strip_prefix(CREATE_PREFIX)
}

/// Options for a searchable picker; when `creatable`, append a “Add …” row from the live query.
pub fn with_create_option(mut options: Vec<ComboboxOption>, query: &str, add_label: &str) -> Vec<ComboboxOption> {
    let q = query.trim();
    if q.is_empty() {
        return options;
    }
    let exists = options.iter().any(|o| o.label.eq_ignore_ascii_case(q));
    if !exists {
        options.insert(
            0,
            ComboboxOption::new(format!("{CREATE_PREFIX}{q}"), format!("{add_label} “{q}”")),
        );
    }
    options
}

pub fn party_options(s: &State, customers: bool, suppliers: bool) -> Vec<ComboboxOption> {
    s.books
        .workspace["resources"]["parties"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| {
            (customers && p["customer"].as_bool().unwrap_or(false))
                || (suppliers && p["supplier"].as_bool().unwrap_or(false))
        })
        .map(|p| {
            let id = crate::data::s(&p["id"]);
            let name = s.books.party_name(&id);
            ComboboxOption::new(id, if name.is_empty() { crate::data::s(&p["name"]) } else { name })
        })
        .collect()
}

pub fn item_options(s: &State) -> Vec<ComboboxOption> {
    s.books
        .workspace["resources"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|i| {
            let id = crate::data::s(&i["id"]);
            let name = crate::data::s(&i["name"]);
            let sku = crate::data::s(&i["sku"]);
            let mut o = ComboboxOption::new(id, name);
            o.detail = sku;
            o
        })
        .collect()
}

pub fn unit_options(s: &State) -> Vec<(String, String)> {
    s.books
        .workspace["resources"]["units"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|u| (crate::data::s(&u["id"]), crate::data::s(&u["name"])))
        .collect()
}

pub fn warehouse_options(s: &State) -> Vec<(String, String)> {
    s.books
        .workspace["resources"]["warehouses"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|w| (crate::data::s(&w["id"]), crate::data::s(&w["name"])))
        .collect()
}

pub fn account_options(s: &State) -> Vec<(String, String)> {
    s.books
        .workspace["accounts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| {
            a["postable"].as_bool().unwrap_or(false)
                && !a["archived"].as_bool().unwrap_or(false)
                && !a["control"].as_bool().unwrap_or(false)
        })
        .map(|a| {
            let code = crate::data::s(&a["code"]);
            let name = crate::data::s(&a["name"]);
            (crate::data::s(&a["id"]), format!("{code} · {name}"))
        })
        .collect()
}

pub fn first_warehouse(s: &State) -> String {
    warehouse_options(s).into_iter().next().map(|(id, _)| id).unwrap_or_default()
}

pub fn first_unit(s: &State) -> String {
    unit_options(s).into_iter().next().map(|(id, _)| id).unwrap_or_default()
}

pub fn item_unit(s: &State, item_id: &str) -> String {
    s.books
        .workspace["resources"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|i| i["id"].as_str() == Some(item_id))
        .map(|i| crate::data::s(&i["base_unit_id"]))
        .unwrap_or_else(|| first_unit(s))
}

pub fn item_sale_price(s: &State, item_id: &str) -> String {
    let p = &s.books.workspace["product_profiles"][item_id]["sale_price"];
    if p.is_null() {
        String::new()
    } else {
        crate::data::s(p)
    }
}

/// Searchable entity picker (kit Combobox). `on_choose` receives the option id (or `__create__:…`).
pub fn entity_combo(
    ui: &Ui,
    layer: &mut Vec<El>,
    key: &str,
    state: &ComboboxState,
    access: impl for<'a> Fn(&'a mut State) -> &'a mut ComboboxState + 'static,
    options: &[ComboboxOption],
    current: Option<&str>,
    placeholder: &str,
    width: f32,
    on_choose: impl Fn(&mut State, &str) + 'static,
) -> El {
    // The kit names the anchor `{key}-trigger`. Replacing that key sends the menu to the wrong place.
    let field = Combobox::new(key, state, options, access)
        .current(current)
        .placeholder(placeholder.to_string())
        .search_placeholder(ui.t("Search"))
        .empty_message(ui.t("No records yet."))
        .label(placeholder.to_string())
        .width(width)
        .placement(Placement::BottomStart)
        .on_choose(move |st, id, _| on_choose(st, id))
        .build(&ui.k, layer);
    row().key(key.to_string()).child(field)
}

/// Field-styled Select over `(id, label)` pairs. Open state lives in `State::selects[key]`.
pub fn select_field(
    ui: &Ui,
    layer: &mut Vec<El>,
    key: &str,
    options: &[(String, String)],
    current: Option<&str>,
    placeholder: &str,
    width: f32,
    open: bool,
    on_choose: impl Fn(&mut State, &str) + Clone + 'static,
) -> El {
    let labels: Vec<String> = options.iter().map(|(_, l)| l.clone()).collect();
    let ids: Vec<String> = options.iter().map(|(id, _)| id.clone()).collect();
    let selected_label = current.and_then(|c| options.iter().find(|(id, _)| id == c).map(|(_, l)| l.as_str()));
    let open_key = key.to_string();
    let close_key = key.to_string();
    let choose_key = key.to_string();
    // `{key}-trigger` is the anchor. A wrapper keeps the test key without moving the menu.
    let field = Select::new(key, &labels)
        .current(selected_label)
        .placeholder(placeholder.to_string())
        .label(placeholder.to_string())
        .appearance(SelectAppearance::Field)
        .width(width)
        .placement(Placement::BottomStart)
        .open(open)
        .on_toggle(move |st: &mut State| {
            let on = st.selects.get(&open_key).copied().unwrap_or(false);
            st.selects.insert(open_key.clone(), !on);
        })
        .on_close(move |st: &mut State| {
            st.selects.insert(close_key.clone(), false);
        })
        .on_choose(move |st: &mut State, label, _| {
            if let Some(i) = labels.iter().position(|l| l == label) {
                on_choose(st, &ids[i]);
            }
            st.selects.insert(choose_key.clone(), false);
        })
        .build(&ui.k, layer);
    row().key(key.to_string()).child(field)
}

/// Compact “New …” button beside a picker.
pub fn add_new_btn(ui: &Ui, label: &str, on_press: impl Fn(&mut State) + 'static) -> El {
    ui.btn(Btn::Ghost, label, None, on_press)
}

pub fn picker_row(picker: El, add: El) -> El {
    row().w_full().gap(8.0).center_y().child(picker).child(add)
}

pub fn selected_label(options: &[ComboboxOption], id: Option<&str>) -> String {
    id.and_then(|c| options.iter().find(|o| o.id == c).map(|o| o.label.clone()))
        .unwrap_or_default()
}

pub fn muted_hint(ui: &Ui, s: impl Into<String>) -> El {
    text(s.into()).font(ui.k.ty.text_xsmall).color(ui.k.c.text_muted)
}

pub fn control_size() -> ControlSize {
    ControlSize::Large
}
