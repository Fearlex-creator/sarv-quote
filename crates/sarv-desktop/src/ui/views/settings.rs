//! Settings — rune-kit Switch, SegmentedControl, Field, Card rows (cursed-settings idiom).

use rune::{Cx, El, col, icon, row, text};
use rune_kit::{Keybind, Switch};
use serde_json::json;

use crate::app::{State, Tone};
use crate::i18n::{CalendarKind, Lang, Unit};
use crate::security::{self, VerifyOutcome};
use crate::ui::glass::{Btn, DIALOG_LG, DIALOG_MD, DIALOG_SM, Ui};
use crate::ui::icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockDialogMode {
    Enable,
    Change,
    Disable,
}

#[derive(Default)]
pub struct SettingsState {
    pub tab_idx: usize,
    pub law_notice: String,
    pub law_open: bool,
    pub law_data: serde_json::Value,
    pub business_open: bool,
    pub restore_open: bool,
    pub backups: serde_json::Value,
    pub restore_name: String,
    pub restore_note: String,
    pub generation_note: String,
    pub business_snapshot: String,
    pub lock_dialog: Option<LockDialogMode>,
    pub lock_error: String,
    pub lock_revealed: bool,
    pub sql_import_open: bool,
    pub sql_import_preview: Option<serde_json::Value>,
    pub sql_import_key: String,
    pub sql_import_posting: bool,
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
    let t1 = ui.t("Business & Backups");
    let t2 = ui.t("Security");
    let t3 = ui.t("Shortcuts & About");
    let tabs = [
        (t0.as_str(), s.settings.tab_idx == 0),
        (t1.as_str(), s.settings.tab_idx == 1),
        (t2.as_str(), s.settings.tab_idx == 2),
        (t3.as_str(), s.settings.tab_idx == 3),
    ];
    let tab_bar = ui.tabs("settings-tabs", &tabs, |st, idx| st.settings.tab_idx = idx);

    let body = match s.settings.tab_idx {
        1 => {
            let company = s.books.company_name();
            let company_label = if company.is_empty() { ui.t("Your business") } else { company };
            let business = ui.section(
                "business",
                &ui.t("Your business"),
                ui.kit_card()
                    .title(company_label, Some(&icons::BUILDING))
                    .description(ui.t("Saved on this device"))
                    .child(
                        row()
                            .w_full()
                            .gap(12.0)
                            .center_y()
                            .justify_between()
                            .child(text(s.fmt.digits(&s.books.period_name())).font(ui.k.ty.ui).color(ui.k.c.text_muted))
                            .child(ui.btn(Btn::Glass, "Your business", None, open_business)),
                    )
                    .build(&ui.k)
                    .key("company"),
            );

            let companies = s.books.companies();
            let mut company_rows: Vec<El> = companies
                .into_iter()
                .map(|(id, name)| {
                    let current = id == s.books.company_id;
                    let label = if name.is_empty() { ui.t("Your business") } else { name };
                    let control = if current {
                        ui.badge(Tone::Info, ui.t("Current"))
                    } else {
                        let target = id.clone();
                        ui.button(Btn::Glass, &format!("company-{id}"), &ui.t("Open"), None, move |st| {
                            st.select_company(&target);
                        })
                    };
                    setting_row(ui, &format!("company-row-{id}"), &label, "", control)
                })
                .collect();
            company_rows.push(setting_row(
                ui,
                "company-row-add",
                &ui.t("Add company"),
                "",
                ui.btn(
                    Btn::Glass,
                    "Add company",
                    Some(&icons::PLUS),
                    crate::ui::views::setup::start_add_company,
                ),
            ));
            let companies_section = ui.section("companies", &ui.t("Companies"), ui.panel_rows("company-switch", company_rows));

            let backup_controls = row()
                .gap(8.0)
                .center_y()
                .child(ui.btn(Btn::Glass, "Backup", Some(&icons::DOWNLOAD), |st| {
                    if let Ok(v) = st.run("backup_create", json!({}), None) {
                        let name = crate::data::s(&v["filename"]);
                        if !name.is_empty() {
                            st.toast_file(st.engine.dir.join(name));
                        }
                    }
                }))
                .child(ui.btn(Btn::Glass, "Restore backup", None, open_restore));

            let exports = s.engine.dir.join("exports");
            let exports_btn = exports.clone();
            let exports_detail = format!(
                "{} · {}",
                crate::i18n::isolate(&exports.display().to_string()),
                ui.t("Exports are saved here and are not included in backups.")
            );
            let backups_and_folders = ui.section(
                "backups-folders",
                &ui.t("Backups & folders"),
                ui.panel_rows(
                    "backups-folders-card",
                    vec![
                        setting_row(ui, "settings-backups", &ui.t("Backups"), "", backup_controls),
                        setting_row(
                            ui,
                            "settings-sql-import",
                            &ui.t("Import from SQL Server"),
                            &ui.t("Migrate contacts and products from Holoo, Sepidar, Hesabfa (.sql or .csv)"),
                            ui.btn(Btn::Glass, "Import from SQL Server", Some(&icons::DOWNLOAD), open_sql_import),
                        ),
                        setting_row(
                            ui,
                            "settings-books-folder",
                            &ui.t("Books folder"),
                            "",
                            ui.muted(crate::i18n::isolate(&s.engine.dir.display().to_string())),
                        ),
                        setting_row(
                            ui,
                            "settings-exports-folder",
                            &ui.t("Exports folder"),
                            &exports_detail,
                            ui.btn(Btn::Glass, "Show in folder", None, move |st| {
                                if let Err(e) = std::fs::create_dir_all(&exports_btn) {
                                    st.toast(Tone::Danger, e.to_string());
                                    return;
                                }
                                if let Err(e) = crate::ui::documents::reveal(&exports_btn, false) {
                                    st.toast_err(e);
                                }
                            }),
                        ),
                    ],
                ),
            );

            let vat_registered = s.books.workspace["settings"]["vat_registered"].as_bool().unwrap_or(false);
            let vat_bp: u32 = s.books.workspace["settings"]["tax_basis_points"]
                .as_u64()
                .or_else(|| crate::data::s(&s.books.workspace["settings"]["tax_basis_points"]).parse().ok())
                .unwrap_or(0) as u32;
            let vat_pct_display = if vat_bp.is_multiple_of(100) {
                (vat_bp / 100).to_string()
            } else {
                format!("{}.{:02}", vat_bp / 100, vat_bp % 100)
            };
            let vat_controls = row()
                .gap(8.0)
                .center_y()
                .child(ui.field(s, "vat-rate-pct", &vat_pct_display).w(88.0))
                .child(ui.btn(Btn::Glass, "Save VAT", Some(&icons::CHECK), |st| {
                    let raw = st.forms.get("vat-rate-pct");
                    let raw = if raw.trim().is_empty() { "0" } else { raw.trim() };
                    let norm = crate::i18n::latin_digits(raw).replace(',', "");
                    let parsed: f64 = match norm.parse() {
                        Ok(v) if (0.0..=100.0).contains(&v) => v,
                        _ => {
                            st.toast(Tone::Warning, st.fmt.t("VAT rate must be between 0 and 100."));
                            return;
                        }
                    };
                    let bp = (parsed * 100.0).round() as u32;
                    st.books.workspace["settings"]["tax_basis_points"] = json!(bp.to_string());
                    if st.save_preferences() {
                        st.toast(Tone::Success, st.fmt.t("VAT settings saved."));
                    }
                }));
            let vat_section = ui.section(
                "vat-settings",
                &ui.t("Value Added Tax (VAT)"),
                ui.panel_rows(
                    "vat-settings-card",
                    vec![
                        setting_row(
                            ui,
                            "settings-vat-registered",
                            &ui.t("VAT registered"),
                            &ui.t("Charge and track output/input VAT using your configured rate"),
                            Switch::new(vat_registered)
                                .on_toggle(|st: &mut State| {
                                    let cur = st.books.workspace["settings"]["vat_registered"].as_bool().unwrap_or(false);
                                    st.books.workspace["settings"]["vat_registered"] = json!(!cur);
                                    let _ = st.save_preferences();
                                })
                                .build(&ui.k)
                                .key("settings-vat-toggle"),
                        ),
                        setting_row(
                            ui,
                            "settings-vat-rate",
                            &ui.t("Default VAT rate (%)"),
                            &ui.t("User-entered rate (0 to 100). Law notices are never applied automatically."),
                            vat_controls,
                        ),
                    ],
                ),
            );

            col()
                .w_full()
                .gap(24.0)
                .child(tab_bar)
                .child(business)
                .child(vat_section)
                .child(companies_section)
                .child(backups_and_folders)
        }
        2 => {
            let lock_enabled = s.app_lock_enabled();
            let lock_controls = if lock_enabled {
                row()
                    .gap(8.0)
                    .center_y()
                    .child(ui.badge(Tone::Success, ui.t("Enabled")))
                    .child(ui.btn(Btn::Glass, "Lock now", Some(&icons::LOCK), |st| {
                        st.lock_app();
                    }))
                    .child(ui.btn(Btn::Glass, "Change password", None, |st| {
                        open_lock_dialog(st, LockDialogMode::Change);
                    }))
                    .child(ui.button(Btn::Ghost, "lock-disable", &ui.t("Disable"), None, |st| {
                        open_lock_dialog(st, LockDialogMode::Disable);
                    }))
            } else {
                row().gap(8.0).center_y().child(ui.badge(Tone::Info, ui.t("Off"))).child(ui.btn(
                    Btn::Primary,
                    "Enable app lock",
                    Some(&icons::LOCK),
                    |st| {
                        open_lock_dialog(st, LockDialogMode::Enable);
                    },
                ))
            };
            let security = ui.section(
                "security",
                &ui.t("Security"),
                ui.panel_rows(
                    "security-card",
                    vec![setting_row(
                        ui,
                        "settings-app-lock",
                        &ui.t("App lock"),
                        &ui.t("Require a password and encrypt your books and documents on disk"),
                        lock_controls,
                    )],
                ),
            );
            col().w_full().gap(24.0).child(tab_bar).child(security)
        }
        3 => {
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
            let shortcuts = ui.section("shortcuts", &ui.t("Keyboard shortcuts"), ui.panel_rows("shortcuts-card", rows));

            let version_text = crate::i18n::isolate(&format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("SARV_GIT_SHA")));
            let logs_path = crate::logfile::logs_dir().unwrap_or_else(|| s.engine.dir.parent().unwrap_or(&s.engine.dir).join("logs"));
            let logs_detail = crate::i18n::isolate(&logs_path.display().to_string());
            let logs_btn_path = logs_path.clone();
            let about = ui.section(
                "about",
                &ui.t("About"),
                ui.panel_rows(
                    "about-card",
                    vec![
                        setting_row(ui, "about-version", &ui.t("Version"), "", ui.muted(version_text)),
                        setting_row(
                            ui,
                            "about-logs",
                            &ui.t("Logs folder"),
                            &logs_detail,
                            ui.btn(Btn::Ghost, "Show in folder", None, move |st| {
                                if let Err(e) = std::fs::create_dir_all(&logs_btn_path) {
                                    st.toast(Tone::Danger, e.to_string());
                                    return;
                                }
                                if let Err(e) = crate::ui::documents::reveal(&logs_btn_path.join("sarv.log"), true) {
                                    st.toast_err(e);
                                }
                            }),
                        ),
                        setting_row(
                            ui,
                            "about-law",
                            &ui.t("Law data"),
                            "",
                            ui.btn(Btn::Glass, "Law data", None, |st| match st.query("law_data", json!({})) {
                                Ok(v) => {
                                    let (primary, fallback) = if st.lang() == Lang::Fa {
                                        (crate::data::s(&v["notice_fa"]), crate::data::s(&v["notice_en"]))
                                    } else {
                                        (crate::data::s(&v["notice_en"]), crate::data::s(&v["notice_fa"]))
                                    };
                                    st.settings.law_notice = if primary.is_empty() { fallback } else { primary };
                                    st.settings.law_data = v;
                                    st.settings.law_open = true;
                                }
                                Err(e) => st.toast_err(e),
                            }),
                        ),
                    ],
                ),
            );

            let mut column = col().w_full().gap(24.0).child(tab_bar).child(shortcuts).child(about);
            if !s.settings.law_notice.is_empty() {
                column = column.child(ui.muted(s.settings.law_notice.clone()).key("law-notice"));
            }
            column
        }
        _ => {
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

            col().w_full().gap(24.0).child(tab_bar).child(appearance).child(features)
        }
    };

    if s.settings.business_open {
        mount_business(s, ui, layer);
    }
    if s.settings.restore_open {
        mount_restore(s, ui, layer);
    }
    if s.settings.law_open {
        mount_law_data(s, ui, layer);
    }
    if let Some(mode) = s.settings.lock_dialog {
        mount_lock_dialog(s, ui, layer, mode);
    }
    if s.settings.sql_import_open {
        mount_sql_import(s, ui, layer);
    }

    ui.page(
        "settings-screen",
        &ui.t("Settings"),
        &ui.t("Appearance, language, and business defaults"),
        vec![],
        body.pb(16.0).scroll(rune::ScrollSpec { stick_bottom: false }).min_h(0.0),
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
        .child(ui.hinted(
            "Bank details",
            ui.field(s, "biz-bank", ""),
            crate::ui::views::contacts::sheba_hint(ui, &s.forms.get("biz-bank")),
        ));
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

pub(crate) fn preview_summary_fmt(fmt: &crate::i18n::Fmt, v: &serde_json::Value) -> String {
    let companies = v["backup"]["companies"].as_array().map(|a| a.len()).unwrap_or(0);
    let journals = crate::data::s(&v["backup"]["journals"]);
    format!(
        "{}: {} · {}: {}",
        fmt.t("Companies"),
        fmt.number(companies),
        fmt.t("Journals"),
        fmt.digits(&journals)
    )
}

pub(crate) fn preview_summary(v: &serde_json::Value) -> String {
    preview_summary_fmt(&crate::i18n::Fmt::default(), v)
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
        let isolated_name = crate::i18n::isolate(&name);
        let kind = backup_kind_label(ui, &crate::data::s(&row["kind"]));
        let label = if kind.is_empty() {
            isolated_name
        } else {
            format!("{isolated_name} · {kind}")
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
            st.settings.restore_note = preview_summary_fmt(&st.fmt, &v);
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
                    st.toast(Tone::Info, st.fmt.t("Select a backup first."));
                    return;
                }
                let payload = json!({
                    "filename": st.settings.restore_name,
                    "confirmed": true,
                    "expected_generation": st.settings.generation_note
                });
                if st.run("backup_restore", payload, Some("Books restored from backup")).is_ok() {
                    st.settings.restore_open = false;
                    st.sheets.loaded = false;
                    st.pos = Default::default();
                    st.sales.selected.clear();
                    st.purchases.selected.clear();
                    st.accounting.selected.clear();
                    st.contacts.selected.clear();
                    st.inventory.selected.clear();
                    st.reports.selected_account_id.clear();
                    st.books.period_id.clear();
                    st.refresh();
                }
            }),
        ],
        |st| st.settings.restore_open = false,
    );
}

fn format_law_from_date(ui: &Ui, from_year: i64) -> String {
    let Ok(year) = i32::try_from(from_year) else {
        return String::new();
    };
    if let Ok(jd) = sarv_quote::calendar::JalaliDate::new(year, 1, 1) {
        let iso = sarv_quote::calendar::to_gregorian(jd).format("%Y-%m-%d").to_string();
        return ui.f.date(&iso);
    }
    ui.f.digits(&from_year.to_string())
}

fn mount_law_data(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let v = &s.settings.law_data;
    let notice = {
        let (primary, fallback) = if s.lang() == Lang::Fa {
            (crate::data::s(&v["notice_fa"]), crate::data::s(&v["notice_en"]))
        } else {
            (crate::data::s(&v["notice_en"]), crate::data::s(&v["notice_fa"]))
        };
        if primary.is_empty() { fallback } else { primary }
    };
    let entries = v["entries"].as_array().cloned().unwrap_or_default();
    let header = row()
        .w_full()
        .gap(12.0)
        .px(12.0)
        .py(8.0)
        .child(ui.muted(ui.t("Name")).grow(1.2).min_w(0.0))
        .child(ui.muted(ui.t("Value")).grow(1.4).min_w(0.0))
        .child(ui.muted(ui.t("Effective from")).w(110.0))
        .child(ui.muted(ui.t("Status")).w(110.0));
    let mut rows_col = col().w_full().gap(4.0).child(header);
    let rows: Vec<El> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let title = if s.lang() == Lang::Fa {
                let fa = crate::data::s(&entry["title_fa"]);
                if fa.is_empty() { crate::data::s(&entry["title_en"]) } else { fa }
            } else {
                crate::data::s(&entry["title_en"])
            };
            let key = crate::data::s(&entry["key"]);
            let val = ui.f.digits(&crate::data::s(&entry["value"]));
            let from_date = entry["from_year"].as_i64().map(|y| format_law_from_date(ui, y)).unwrap_or_default();
            let reviewed = entry["reviewed"].as_bool() == Some(true);
            let verify = entry["verify"].as_bool() == Some(true);
            let badge = if reviewed && !verify {
                ui.badge(Tone::Success, ui.t("Reviewed"))
            } else {
                ui.badge(Tone::Warning, ui.t("Not reviewed"))
            };
            row()
                .key(format!("law-entry-{idx}"))
                .w_full()
                .gap(12.0)
                .px(12.0)
                .py(8.0)
                .center_y()
                .child(
                    col()
                        .gap(2.0)
                        .grow(1.2)
                        .min_w(0.0)
                        .child(ui.body(title))
                        .child(ui.muted(crate::i18n::isolate(&key))),
                )
                .child(ui.body(val).grow(1.4).min_w(0.0))
                .child(ui.muted(from_date).w(110.0))
                .child(row().w(110.0).child(badge))
        })
        .collect();
    if !rows.is_empty() {
        rows_col = rows_col.child(ui.panel_rows("law-entries", rows));
    }
    let body = col()
        .w_full()
        .gap(12.0)
        .child(ui.body(notice).key("law-dialog-notice"))
        .child(rows_col.scroll(rune::ScrollSpec { stick_bottom: false }).max_h(320.0))
        .child(
            ui.muted(ui.t("Shown for reference. Sarv does not use these values in any calculation."))
                .key("law-footer"),
        );
    ui.dialog(
        layer,
        "law-data",
        &ui.t("Law data"),
        true,
        DIALOG_LG,
        body,
        vec![ui.btn(Btn::Primary, "Close", None, |st| st.settings.law_open = false)],
        |st| st.settings.law_open = false,
    );
}

fn open_lock_dialog(st: &mut State, mode: LockDialogMode) {
    st.settings.lock_dialog = Some(mode);
    st.settings.lock_error.clear();
    st.settings.lock_revealed = false;
    st.forms.clear(&["sec-cur-pass", "sec-new-pass", "sec-confirm-pass"]);
}

fn close_lock_dialog(st: &mut State) {
    st.settings.lock_dialog = None;
    st.settings.lock_error.clear();
    st.settings.lock_revealed = false;
    st.forms.clear(&["sec-cur-pass", "sec-new-pass", "sec-confirm-pass"]);
}

fn submit_lock_dialog(st: &mut State, mode: LockDialogMode) {
    let cur = st.forms.get("sec-cur-pass");
    let new = st.forms.get("sec-new-pass");
    let confirm = st.forms.get("sec-confirm-pass");
    let now = security::now_unix();

    match mode {
        LockDialogMode::Enable => {
            if let Err(e) = security::validate_password(&new) {
                st.settings.lock_error = st.fmt.t(&e);
                return;
            }
            if new != confirm {
                st.settings.lock_error = st.fmt.t("Passwords do not match.");
                return;
            }
            match security::set_password(&st.engine.dir, &new, security::UI_ITERATIONS) {
                Ok(dek) => {
                    st.active_dek = Some(zeroize::Zeroizing::new(dek));
                    st.refresh_lock_status();
                    close_lock_dialog(st);
                    st.toast(Tone::Success, st.fmt.t("App lock enabled"));
                }
                Err(e) => st.settings.lock_error = e,
            }
        }
        LockDialogMode::Change => {
            if let Err(e) = security::validate_password(&new) {
                st.settings.lock_error = st.fmt.t(&e);
                return;
            }
            if new != confirm {
                st.settings.lock_error = st.fmt.t("Passwords do not match.");
                return;
            }
            match security::change_password(&st.engine.dir, &cur, &new, now, security::UI_ITERATIONS) {
                Ok(VerifyOutcome::Unlocked(dek)) => {
                    st.active_dek = Some(zeroize::Zeroizing::new(dek));
                    close_lock_dialog(st);
                    st.toast(Tone::Success, st.fmt.t("Password updated"));
                }
                Ok(outcome) => {
                    st.forms.clear(&["sec-cur-pass"]);
                    st.settings.lock_error = crate::ui::views::lock::format_outcome_error(st, outcome);
                }
                Err(e) => st.settings.lock_error = e,
            }
        }
        LockDialogMode::Disable => match security::disable(&st.engine.dir, &cur, now) {
            Ok(VerifyOutcome::Unlocked(_)) => {
                st.active_dek = None;
                st.refresh_lock_status();
                close_lock_dialog(st);
                st.toast(Tone::Success, st.fmt.t("App lock disabled"));
            }
            Ok(outcome) => {
                st.forms.clear(&["sec-cur-pass"]);
                st.settings.lock_error = crate::ui::views::lock::format_outcome_error(st, outcome);
            }
            Err(e) => st.settings.lock_error = e,
        },
    }
}

fn mount_lock_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>, mode: LockDialogMode) {
    let (title_key, desc_key, action_key) = match mode {
        LockDialogMode::Enable => ("Enable app lock", "Set a password to protect your books when Sarv opens.", "Enable"),
        LockDialogMode::Change => ("Change password", "Enter your current password and choose a new one.", "Save"),
        LockDialogMode::Disable => ("Disable app lock", "Enter your current password to remove the app lock.", "Disable"),
    };

    let revealed = s.settings.lock_revealed;
    let mut form = col().w_full().gap(12.0);
    if matches!(mode, LockDialogMode::Change | LockDialogMode::Disable) {
        form = form.child(ui.labeled(
            "Current password",
            ui.secret_field(s, "sec-cur-pass", &ui.t("Current password"), revealed),
        ));
    }
    if matches!(mode, LockDialogMode::Enable | LockDialogMode::Change) {
        form = form
            .child(ui.labeled(
                "New password",
                ui.secret_field(s, "sec-new-pass", &ui.t("At least 4 characters"), revealed),
            ))
            .child(ui.labeled(
                "Confirm password",
                ui.secret_field(s, "sec-confirm-pass", &ui.t("Repeat password"), revealed),
            ));
    }
    let toggle_label = if revealed { "Hide" } else { "Show" };
    form = form.child(row().w_full().justify_end().child(ui.btn(Btn::Ghost, toggle_label, None, |st| {
        st.settings.lock_revealed = !st.settings.lock_revealed;
    })));

    if !s.settings.lock_error.is_empty() {
        let sc = &ui.k.c.danger;
        form = form.child(
            row()
                .key("settings-lock-error")
                .w_full()
                .gap(8.0)
                .px(12.0)
                .py(10.0)
                .center_y()
                .radius(ui.k.radius.md)
                .fill(sc.bg)
                .border(1.0, sc.border)
                .child(icon(&icons::WARNING, 16.0).color(sc.icon))
                .child(
                    text(s.settings.lock_error.clone())
                        .font(ui.k.ty.caption())
                        .color(sc.fg)
                        .grow(1.0)
                        .min_w(0.0),
                ),
        );
    }

    let action_kind = if mode == LockDialogMode::Disable {
        Btn::Danger
    } else {
        Btn::Primary
    };
    let mut portals = Vec::new();
    ui.form_dialog(
        layer,
        &mut portals,
        "app-lock",
        &ui.t(title_key),
        &ui.t(desc_key),
        true,
        DIALOG_SM,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_lock_dialog),
            ui.btn(action_kind, action_key, Some(&icons::CHECK), move |st| {
                submit_lock_dialog(st, mode);
            }),
        ],
        close_lock_dialog,
    );
}

pub fn open_sql_import(st: &mut State) {
    st.forms.clear(&["sql-import-path"]);
    st.settings.sql_import_preview = None;
    st.settings.sql_import_key = st.mint_key("sql-import");
    st.settings.sql_import_posting = false;
    st.settings.sql_import_open = true;
}

fn close_sql_import(st: &mut State) {
    st.settings.sql_import_open = false;
    st.settings.sql_import_preview = None;
    st.settings.sql_import_posting = false;
    st.forms.clear(&["sql-import-path"]);
}

fn preview_sql_import(st: &mut State) {
    let path = st.forms.get("sql-import-path").trim().to_string();
    if path.is_empty() {
        st.toast(Tone::Warning, st.fmt.t("Enter a .sql or .csv file path first."));
        return;
    }
    let unit = match st.fmt.unit {
        Unit::Toman => "Toman",
        Unit::Rial => "Rial",
    };
    match st.query("sql_import_preview", json!({ "path": path, "unit": unit })) {
        Ok(v) => st.settings.sql_import_preview = Some(v),
        Err(e) => {
            st.settings.sql_import_preview = None;
            st.toast_err(e);
        }
    }
}

fn post_sql_import(st: &mut State) {
    if st.settings.sql_import_posting {
        return;
    }
    let path = st.forms.get("sql-import-path").trim().to_string();
    if path.is_empty() {
        st.toast(Tone::Warning, st.fmt.t("Enter a .sql or .csv file path first."));
        return;
    }
    let Some(preview) = st.settings.sql_import_preview.clone() else {
        preview_sql_import(st);
        return;
    };
    let unit = match st.fmt.unit {
        Unit::Toman => "Toman",
        Unit::Rial => "Rial",
    };
    let generation = crate::data::s(&preview["_generation"]);
    let mut payload = json!({
        "path": path,
        "unit": unit,
        "request_key": st.settings.sql_import_key,
    });
    if !generation.is_empty() {
        payload["expected_generation"] = json!(generation);
    }
    st.settings.sql_import_posting = true;
    match st.run("sql_import_post", payload, Some("Books imported from SQL Server")) {
        Ok(_) => close_sql_import(st),
        Err(_) => st.settings.sql_import_posting = false,
    }
}

fn mount_sql_import(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let mut body = col()
        .w_full()
        .gap(12.0)
        .child(ui.labeled(
            "SQL or CSV file path",
            ui.field(s, "sql-import-path", &ui.t("Path to .sql script or .csv export")),
        ))
        .child(ui.muted(
            ui.t("Have a SQL Server .bak file? Restore it in SSMS and run Tasks -> Generate Scripts (Data only) to save a .sql file."),
        ));

    if let Some(preview) = &s.settings.sql_import_preview {
        let format_name = crate::data::s(&preview["detected_format"]);
        let contacts = preview["contacts_count"].as_u64().unwrap_or(0);
        let items = preview["items_count"].as_u64().unwrap_or(0);
        let skipped = preview["existing_contacts_skipped"].as_u64().unwrap_or(0) + preview["existing_items_skipped"].as_u64().unwrap_or(0);
        let summary = format!(
            "{}: {} · {}: {} · {}: {} · {}: {}",
            ui.t("Format"),
            crate::i18n::isolate(&format_name),
            ui.t("Contacts"),
            s.fmt.digits(&contacts.to_string()),
            ui.t("Products"),
            s.fmt.digits(&items.to_string()),
            ui.t("Skipped"),
            s.fmt.digits(&skipped.to_string()),
        );
        let mut preview_box = col().w_full().gap(6.0).child(ui.body(summary).key("sql-import-summary"));
        if let Some(warns) = preview["warnings"].as_array() {
            for w in warns.iter().take(3) {
                if let Some(msg) = w.as_str() {
                    preview_box = preview_box.child(ui.muted(msg.to_string()));
                }
            }
        }
        body = body.child(ui.kit_card().child(preview_box).build(&ui.k).key("sql-import-preview-card"));
    }

    let mut actions = vec![
        ui.btn(Btn::Ghost, "Cancel", None, close_sql_import),
        ui.btn(Btn::Glass, "Preview", None, preview_sql_import),
    ];
    if s.settings.sql_import_preview.is_some() {
        actions.push(ui.btn(Btn::Primary, "Import", Some(&icons::CHECK), post_sql_import));
    }

    let mut portals = Vec::new();
    ui.form_dialog(
        layer,
        &mut portals,
        "sql-import",
        &ui.t("Import from SQL Server"),
        &ui.t("Preview and import contacts and products from a SQL Server export"),
        true,
        DIALOG_MD,
        body,
        actions,
        close_sql_import,
    );
}
