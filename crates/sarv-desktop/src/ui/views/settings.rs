//! Settings — rune-kit Switch, SegmentedControl, Field, Card rows (cursed-settings idiom).

use rune::{Cx, El, col, row, text};
use rune_kit::{Keybind, Switch};
use serde_json::json;

use crate::app::{State, Tone};
use crate::i18n::{CalendarKind, Lang, Unit};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

#[derive(Default)]
pub struct SettingsState {
    pub tab_idx: usize,
    pub law_notice: String,
    pub business_open: bool,
    pub restore_open: bool,
    pub backups: serde_json::Value,
    pub restore_name: String,
    pub restore_note: String,
    pub generation_note: String,
}

fn setting_row(ui: &Ui, key: &str, label: &str, detail: &str, control: El) -> El {
    let mut left = col().gap(2.0).grow(1.0).min_w(0.0).child(ui.body(label.to_string()));
    if !detail.is_empty() {
        left = left.child(ui.muted(detail.to_string()));
    }
    row()
        .key(key.to_string())
        .w_full()
        .min_h(48.0)
        .px(12.0)
        .py(10.0)
        .gap(16.0)
        .center_y()
        .justify_between()
        .child(left)
        .child(control)
}

fn mod_key() -> &'static str {
    if cfg!(target_os = "macos") { "⌘" } else { "Ctrl" }
}

fn shortcut_row(ui: &Ui, key: &str, label: &str, keys: &[&str]) -> El {
    row()
        .key(key.to_string())
        .w_full()
        .h(44.0)
        .px(12.0)
        .center_y()
        .justify_between()
        .child(ui.body(label.to_string()))
        .child(Keybind::new(keys).build(&ui.k))
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let t0 = ui.t("General");
    let t1 = ui.t("Keyboard shortcuts");
    let tabs = [(t0.as_str(), s.settings.tab_idx == 0), (t1.as_str(), s.settings.tab_idx == 1)];
    let tab_bar = ui.tabs("settings-tabs", &tabs, |st, idx| st.settings.tab_idx = idx);

    let body = if s.settings.tab_idx == 1 {
        col().w_full().gap(16.0).child(tab_bar).child(ui.section(
            "shortcuts",
            &ui.t("Keyboard shortcuts"),
            ui.panel_rows(
                "shortcuts-card",
                vec![
                    shortcut_row(ui, "sc-k", &ui.t("Search"), &[mod_key(), "K"]),
                    shortcut_row(ui, "sc-b", &ui.t("Collapse sidebar"), &[mod_key(), "B"]),
                    shortcut_row(ui, "sc-l", &ui.t("Toggle language"), &[mod_key(), "L"]),
                    shortcut_row(ui, "sc-d", &ui.t("Toggle theme"), &[mod_key(), "D"]),
                ],
            ),
        ))
    } else {
        let theme = Switch::new(s.dark)
            .label(ui.t("Theme"))
            .hide_label()
            .on_toggle(|st: &mut State| st.dark = !st.dark)
            .build(&ui.k)
            .key("settings-theme-switch");

        let lang_fa = ui.t("Persian");
        let lang_items = [(lang_fa.as_str(), s.lang() == Lang::Fa), ("English", s.lang() == Lang::En)];
        let lang_ctrl = ui.tabs("settings-lang", &lang_items, |st, idx| {
            st.set_lang(if idx == 0 { Lang::Fa } else { Lang::En });
        });

        let digits = Switch::new(s.fmt.persian_digits)
            .label(ui.t("Display digits"))
            .hide_label()
            .on_toggle(|st: &mut State| st.fmt.persian_digits = !st.fmt.persian_digits)
            .build(&ui.k)
            .key("settings-digits-switch");

        let unit_toman = ui.t("toman");
        let unit_rial = ui.t("Rial");
        let unit_items = [
            (unit_toman.as_str(), s.fmt.unit == Unit::Toman),
            (unit_rial.as_str(), s.fmt.unit == Unit::Rial),
        ];
        let unit_ctrl = ui.tabs("settings-unit", &unit_items, |st, idx| {
            st.fmt.unit = if idx == 0 { Unit::Toman } else { Unit::Rial };
        });

        let cal_jalali = ui.t("Jalali");
        let cal_gregorian = ui.t("Gregorian");
        let cal_items = [
            (cal_jalali.as_str(), s.fmt.calendar == CalendarKind::Jalali),
            (cal_gregorian.as_str(), s.fmt.calendar == CalendarKind::Gregorian),
        ];
        let cal_ctrl = ui.tabs("settings-cal", &cal_items, |st, idx| {
            st.fmt.calendar = if idx == 0 { CalendarKind::Jalali } else { CalendarKind::Gregorian };
        });

        let appearance = ui.section(
            "appearance",
            &ui.t("Appearance and language"),
            ui.panel_rows(
                "appearance-card",
                vec![
                    setting_row(ui, "settings-theme", &ui.t("Theme"), &ui.t("Dark or light window"), theme),
                    setting_row(ui, "lang", &ui.t("Interface language"), "", lang_ctrl),
                    setting_row(ui, "digits", &ui.t("Display digits"), &ui.t("Persian or Latin numerals"), digits),
                    setting_row(ui, "unit", &ui.t("Money unit"), "", unit_ctrl),
                    setting_row(ui, "cal", &ui.t("Calendar"), "", cal_ctrl),
                ],
            ),
        );

        let pos_switch = Switch::new(s.show_pos)
            .label(ui.t("Point of sale"))
            .hide_label()
            .on_toggle(|st: &mut State| st.show_pos = !st.show_pos)
            .build(&ui.k)
            .key("settings-pos-switch");
        let features = ui.section(
            "features",
            &ui.t("Features"),
            ui.panel_rows(
                "features-card",
                vec![setting_row(
                    ui,
                    "settings-pos",
                    &ui.t("Point of sale"),
                    &ui.t("Show the counter in the sidebar for walk-in sales."),
                    pos_switch,
                )],
            ),
        );

        let company = s.books.company_name();
        let company_label = if company.is_empty() { ui.t("Your business") } else { company };
        let business = ui.section(
            "business",
            &ui.t("Your business"),
            ui.kit_card()
                .title(company_label, Some(&icons::BUILDING))
                .description(ui.t("Saved on this device"))
                .child(text(s.books.period_name()).font(ui.k.ty.ui).color(ui.k.c.text_muted))
                .build(&ui.k)
                .key("company"),
        );

        let mut column = col().w_full().gap(24.0).child(tab_bar).child(appearance).child(features).child(business);
        if !s.settings.law_notice.is_empty() {
            column = column.child(ui.muted(s.settings.law_notice.clone()).key("law-notice"));
        }
        let companies = s.books.companies();
        if companies.len() > 1 {
            let mut switcher = row().w_full().gap(8.0).key("company-switch");
            for (id, name) in companies {
                let current = id == s.books.company_id;
                let label = name.clone();
                switcher = switcher.child(ui.button(
                    if current { Btn::Primary } else { Btn::Glass },
                    &format!("company-{id}"),
                    &label,
                    None,
                    move |st| st.select_company(&id),
                ));
            }
            column = column.child(switcher);
        }
        column
    };

    if s.settings.business_open {
        mount_business(s, ui, layer);
    }
    if s.settings.restore_open {
        mount_restore(s, ui, layer);
    }

    ui.page(
        "settings-screen",
        &ui.t("Settings"),
        &ui.t("Appearance, language, and business defaults"),
        vec![
            ui.btn(Btn::Glass, "Your business", None, open_business),
            ui.btn(Btn::Glass, "Restore backup", None, open_restore),
            ui.btn(Btn::Glass, "Backup", Some(&icons::DOWNLOAD), |st| match st.run("backup_create", json!({}), None) {
                Ok(v) => st.toast(Tone::Success, crate::data::s(&v["filename"])),
                Err(e) => st.toast_err(e),
            }),
            ui.btn(Btn::Glass, "Law data", None, |st| match st.query("law_data", json!({})) {
                Ok(v) => {
                    let notice = crate::data::s(&v["notice_en"]);
                    st.settings.law_notice = if notice.is_empty() {
                        crate::data::s(&v["notice_fa"])
                    } else {
                        notice
                    };
                }
                Err(e) => st.toast_err(e),
            }),
            ui.btn(Btn::Primary, "Save preferences", Some(&icons::CHECK), |st| {
            let appearance = if st.dark { "Dark" } else { "Light" };
            let digits = if st.fmt.persian_digits { "Persian" } else { "Latin" };
            let currency = if st.fmt.unit == Unit::Rial { "Rial" } else { "Toman" };
            let date_display = if st.fmt.calendar == CalendarKind::Jalali { "Jalali" } else { "Iso" };
            let mut settings = st.books.workspace["settings"].clone();
            if settings.is_null() {
                settings = json!({});
            }
            settings["appearance"] = json!(appearance);
            settings["display_digits"] = json!(digits);
            settings["default_currency"] = json!(currency);
            settings["date_display"] = json!(date_display);
            settings["text_direction"] = json!(if st.fmt.lang == Lang::En { "Ltr" } else { "Rtl" });
            settings["show_pos"] = json!(st.show_pos);
            for (k, d) in [
                ("next_number", "1"),
                ("default_validity_days", "14"),
                ("default_discount_rial", "0"),
                ("tax_basis_points", "0"),
            ] {
                if settings[k].as_str().is_none() {
                    settings[k] = json!(settings[k].as_u64().map(|n| n.to_string()).unwrap_or_else(|| d.into()));
                }
            }
            if st.run("settings", settings, Some("Preferences saved")).is_ok() {
                st.apply_settings();
            }
        })],
        body.scroll(rune::ScrollSpec { stick_bottom: false }).min_h(0.0),
    )
}

fn profile_field(st: &State, key: &str) -> String {
    crate::data::s(&st.books.workspace["business_profile"][key])
}

pub fn open_business(st: &mut State) {
    st.forms.set("biz-phone", &profile_field(st, "phone"));
    st.forms.set("biz-address", &profile_field(st, "address"));
    st.forms.set("biz-tax", &profile_field(st, "tax_id"));
    st.forms.set("biz-prefix", &profile_field(st, "invoice_prefix"));
    st.forms.set("biz-footer", &profile_field(st, "footer"));
    st.forms.set("biz-bank", &profile_field(st, "bank_details"));
    st.settings.business_open = true;
}

fn mount_business(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let form = col()
        .gap(12.0)
        .child(ui.form_grid(
            ui.labeled("Phone", ui.field(s, "biz-phone", "")),
            ui.labeled("Tax / economic ID", ui.field(s, "biz-tax", "")),
        ))
        .child(ui.labeled("Invoice prefix", ui.field(s, "biz-prefix", "INV")))
        .child(ui.labeled("Address", ui.field(s, "biz-address", "")))
        .child(ui.labeled("Footer", ui.field(s, "biz-footer", "")))
        .child(ui.labeled("Bank details", ui.field(s, "biz-bank", "")));
    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "business-profile",
        &ui.t("Your business"),
        &ui.t("Local-first accounting"),
        true,
        560.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.settings.business_open = false),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                let profile = json!({
                    "phone": st.forms.get("biz-phone"),
                    "address": st.forms.get("biz-address"),
                    "registration_id": profile_field(st, "registration_id"),
                    "tax_id": st.forms.get("biz-tax"),
                    "invoice_prefix": st.forms.get("biz-prefix"),
                    "terms": profile_field(st, "terms"),
                    "footer": st.forms.get("biz-footer"),
                    "bank_details": st.forms.get("biz-bank")
                });
                match st.run("business_profile", json!({ "profile": profile }), Some("Saved")) {
                    Ok(_) => st.settings.business_open = false,
                    Err(e) => st.toast_err(e),
                }
            }),
        ],
        |st| st.settings.business_open = false,
    );
}

fn open_restore(st: &mut State) {
    match st.query("backup_list", json!({})) {
        Ok(v) => {
            st.settings.backups = v;
            st.settings.restore_name.clear();
            st.settings.restore_note.clear();
            st.settings.restore_open = true;
        }
        Err(e) => st.toast_err(e),
    }
}

fn mount_restore(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let rows = s.settings.backups["backups"].as_array().cloned().unwrap_or_default();
    let mut form = col().gap(8.0);
    if rows.is_empty() {
        form = form.child(ui.empty(&icons::DOWNLOAD, "No backups yet. Create one before your first closing.", "Backup"));
    }
    for row in rows.iter().take(6) {
        let name = crate::data::s(&row["filename"]);
        let key = format!("backup-{name}");
        form = form.child(ui.button(Btn::Glass, &key, &name, None, {
            let name = name.clone();
            move |st| {
                match st.query("backup_restore_preview", json!({ "filename": name })) {
                    Ok(v) => {
                        let companies = v["backup"]["companies"].as_array().map(|a| a.len()).unwrap_or(0);
                        let journals = crate::data::s(&v["backup"]["journals"]);
                        st.settings.restore_name = name.clone();
                        st.settings.restore_note = format!("{companies} · {journals}");
                        st.settings.generation_note = crate::data::s(&v["_generation"]);
                    }
                    Err(e) => st.toast_err(e),
                }
            }
        }));
    }
    if !s.settings.restore_note.is_empty() {
        form = form.child(ui.muted(s.settings.restore_note.clone()));
    }
    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "restore-backup",
        &ui.t("Restore backup"),
        &ui.t("Restore books from backup?"),
        true,
        560.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.settings.restore_open = false),
            ui.button(Btn::Danger, "restore-confirm", &ui.t("Restore backup"), None, |st| {
                if st.settings.restore_name.is_empty() {
                    st.toast(Tone::Info, st.fmt.t("No backups yet. Create one before your first closing."));
                    return;
                }
                let payload = json!({
                    "filename": st.settings.restore_name,
                    "confirmed": true,
                    "expected_generation": st.settings.generation_note
                });
                match st.run("backup_restore", payload, Some("Saved")) {
                    Ok(_) => st.settings.restore_open = false,
                    Err(e) => st.toast_err(e),
                }
            }),
        ],
        |st| st.settings.restore_open = false,
    );
}
