//! Settings — rune-kit Switch, SegmentedControl, Field, Card rows (cursed-settings idiom).

use rune::{Cx, El, col, row, text};
use rune_kit::{Keybind, Switch};
use serde_json::json;

use crate::app::{State, Tone};
use crate::i18n::{CalendarKind, Lang, Unit};
use crate::ui::glass::{Btn, DIALOG_MD, DIALOG_SM, Ui};
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
    pub business_snapshot: String,
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
        let global: &[(&str, &str, &[&str])] = &[
            ("sc-k", "Search", &[mod_key(), "K"]),
            ("sc-b", "Collapse sidebar", &[mod_key(), "B"]),
            ("sc-l", "Toggle language", &[mod_key(), "L"]),
            ("sc-d", "Toggle theme", &[mod_key(), "D"]),
            ("sc-f1", "Keyboard shortcuts", &["F1"]),
            ("sc-f9", "Toggle sidebar", &["F9"]),
        ];
        let mut rows: Vec<El> = global
            .iter()
            .map(|(id, label, keys)| shortcut_row(ui, id, &ui.t(label), keys))
            .collect();
        for (_, sc) in crate::shortcuts::CREATE {
            let mut keys = vec![mod_key()];
            keys.extend_from_slice(sc.keys);
            rows.push(shortcut_row(ui, sc.id, &ui.t(sc.label), &keys));
        }
        col().w_full().gap(16.0).child(tab_bar).child(ui.section(
            "shortcuts",
            &ui.t("Keyboard shortcuts"),
            ui.panel_rows("shortcuts-card", rows),
        ))
    } else {
        let theme = Switch::new(s.dark)
            .label(ui.t("Theme"))
            .hide_label()
            .on_toggle(|st: &mut State| {
                st.dark = !st.dark;
                let _ = st.save_preferences();
            })
            .build(&ui.k)
            .key("settings-theme-switch");

        let lang_fa = ui.t("Persian");
        let lang_items = [(lang_fa.as_str(), s.lang() == Lang::Fa), ("English", s.lang() == Lang::En)];
        let lang_ctrl = ui.tabs("settings-lang", &lang_items, |st, idx| {
            st.set_lang(if idx == 0 { Lang::Fa } else { Lang::En });
            let _ = st.save_preferences();
        });

        let digits = Switch::new(s.fmt.persian_digits)
            .label(ui.t("Display digits"))
            .hide_label()
            .on_toggle(|st: &mut State| {
                st.fmt.persian_digits = !st.fmt.persian_digits;
                let _ = st.save_preferences();
            })
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
            let _ = st.save_preferences();
        });

        let cal_jalali = ui.t("Jalali");
        let cal_gregorian = ui.t("Gregorian");
        let cal_items = [
            (cal_jalali.as_str(), s.fmt.calendar == CalendarKind::Jalali),
            (cal_gregorian.as_str(), s.fmt.calendar == CalendarKind::Gregorian),
        ];
        let cal_ctrl = ui.tabs("settings-cal", &cal_items, |st, idx| {
            st.fmt.calendar = if idx == 0 { CalendarKind::Jalali } else { CalendarKind::Gregorian };
            let _ = st.save_preferences();
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
            .on_toggle(|st: &mut State| {
                st.show_pos = !st.show_pos;
                let _ = st.save_preferences();
            })
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
                .child(
                    row()
                        .w_full()
                        .gap(12.0)
                        .center_y()
                        .justify_between()
                        .child(ui.body(ui.t("Books folder")))
                        .child(ui.muted(crate::i18n::isolate(&s.engine.dir.display().to_string()))),
                )
                .build(&ui.k)
                .key("company"),
        );

        let mut column = col()
            .w_full()
            .gap(24.0)
            .child(tab_bar)
            .child(appearance)
            .child(features)
            .child(business);
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
            ui.btn(Btn::Glass, "Backup", Some(&icons::DOWNLOAD), |st| {
                if let Ok(v) = st.run("backup_create", json!({}), None) {
                    st.toast(Tone::Success, crate::data::s(&v["filename"]));
                }
            }),
            ui.btn(Btn::Glass, "Law data", None, |st| match st.query("law_data", json!({})) {
                Ok(v) => {
                    let (primary, fallback) = if st.lang() == Lang::Fa {
                        (crate::data::s(&v["notice_fa"]), crate::data::s(&v["notice_en"]))
                    } else {
                        (crate::data::s(&v["notice_en"]), crate::data::s(&v["notice_fa"]))
                    };
                    st.settings.law_notice = if primary.is_empty() { fallback } else { primary };
                }
                Err(e) => st.toast_err(e),
            }),
            ui.btn(Btn::Primary, "Save preferences", Some(&icons::CHECK), |st| {
                if st.save_preferences() {
                    st.toast(Tone::Success, st.fmt.t("Preferences saved"));
                }
            }),
        ],
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
    st.settings.business_snapshot = st.forms.snapshot_prefix("biz-");
    st.settings.business_open = true;
}

pub fn close_business_or_confirm(st: &mut State) {
    if st.forms.snapshot_prefix("biz-") == st.settings.business_snapshot {
        st.settings.business_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.settings.business_open = false);
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
    ui.form_dialog(
        layer,
        &mut portals,
        "business-profile",
        &ui.t("Your business"),
        &ui.t("Local-first accounting"),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_business_or_confirm),
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
                if st.run("business_profile", json!({ "profile": profile }), Some("Saved")).is_ok() {
                    st.settings.business_open = false;
                }
            }),
        ],
        close_business_or_confirm,
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

pub(crate) fn backup_kind_label(ui: &Ui, kind: &str) -> String {
    match kind {
        "manual" => ui.t("Manual backup"),
        "before-restore" => ui.t("Before restore"),
        "before-upgrade" => ui.t("Before upgrade"),
        other => other.to_string(),
    }
}

pub(crate) fn preview_summary(v: &serde_json::Value) -> String {
    let companies = v["backup"]["companies"].as_array().map(|a| a.len()).unwrap_or(0);
    let journals = crate::data::s(&v["backup"]["journals"]);
    format!("{companies} · {journals}")
}

pub(crate) fn backup_list(
    ui: &Ui,
    backups: &serde_json::Value,
    selected: &str,
    on_pick: impl Fn(&mut State, String) + Clone + 'static,
) -> El {
    let rows = backups["backups"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return ui.empty(&icons::DOWNLOAD, "No backups yet. Create one before your first closing.", "Backup");
    }
    let mut list = col().w_full().gap(6.0);
    for row in rows {
        let name = crate::data::s(&row["filename"]);
        let kind = backup_kind_label(ui, &crate::data::s(&row["kind"]));
        let label = if kind.is_empty() {
            name.clone()
        } else {
            format!("{name} · {kind}")
        };
        let key = format!("backup-{name}");
        let btn_kind = if selected == name { Btn::Primary } else { Btn::Glass };
        let on_pick = on_pick.clone();
        let picked = name.clone();
        list = list.child(ui.button(btn_kind, &key, &label, None, move |st| on_pick(st, picked.clone())));
    }
    list.scroll(rune::ScrollSpec { stick_bottom: false }).w_full().max_h(240.0)
}

fn mount_restore(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let mut form = col().gap(8.0);
    form = form.child(backup_list(ui, &s.settings.backups, &s.settings.restore_name, |st, name| match st
        .query("backup_restore_preview", json!({ "filename": name }))
    {
        Ok(v) => {
            st.settings.restore_name = name;
            st.settings.restore_note = preview_summary(&v);
            st.settings.generation_note = crate::data::s(&v["_generation"]);
        }
        Err(e) => st.toast_err(e),
    }));
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
        DIALOG_SM,
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
                if st.run("backup_restore", payload, Some("Saved")).is_ok() {
                    st.settings.restore_open = false;
                    st.sheets.loaded = false;
                    st.pos = Default::default();
                    st.sales.selected.clear();
                    st.purchases.selected.clear();
                    st.accounting.selected.clear();
                    st.books.period_id.clear();
                    st.refresh();
                }
            }),
        ],
        |st| st.settings.restore_open = false,
    );
}
