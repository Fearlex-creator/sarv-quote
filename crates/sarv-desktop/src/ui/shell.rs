//! App chrome matching rune-kit's shell example: full close, peek, framed panel, icon rail.

use std::rc::Rc;
use std::time::Duration;

use rune::{Cx, El, Spring, col, icon, row};
use rune_kit::shell::{PaneMotion, SidebarPeek, SidebarToggle};
use rune_kit::{
    Badge, BadgeVariant, Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, SegmentedControl, Sidebar, SidebarRow,
    SidebarSection, SplitView, Toast, ToastRegion, Toolbar,
};

use crate::app::{Page, State, Tone};
use crate::i18n::Lang;
use crate::ui::glass::{Btn, DIALOG_SM, Ui};
use crate::ui::icons;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn nav_row(ui: &Ui, page: Page, selected: bool) -> El {
    SidebarRow::new(ui.t(page.label()))
        .icon(page.icon())
        .height(32.0)
        .selected(selected)
        .on_select(move |st: &mut State, _| st.nav(page))
        .build(&ui.k)
        .key(page.key())
}

fn build_sidebar(s: &State, ui: &Ui, ready: bool) -> El {
    let k = &ui.k;
    let mut sidebar = Sidebar::new()
        .label(ui.t("Navigation"))
        .inset(0.0)
        .header_gap(4.0)
        .header_padding(0.0, 12.0)
        .row_gap(2.0);

    if ready {
        // Search and a new sale first — not the brand tagline or company name.
        sidebar = sidebar
            .header(
                SidebarRow::new(ui.t("Search anything…"))
                    .icon(&icons::SEARCH)
                    .detail("⌘K")
                    .height(32.0)
                    .on_select(|st: &mut State, _| st.open_palette())
                    .build(k)
                    .key("sidebar-search"),
            )
            .header(
                SidebarRow::new(ui.t("New sales invoice"))
                    .icon(&icons::PLUS)
                    .height(32.0)
                    .on_select(|st: &mut State, _| {
                        crate::ui::views::sales::open_sale(st);
                        st.nav(Page::Sales);
                    })
                    .build(k)
                    .key("btn-New order"),
            );

        for (id, title, pages) in Page::GROUPS {
            let rows: Vec<_> = pages
                .iter()
                .copied()
                .filter(|page| page.nav_pages(s.show_pos))
                .map(|page| nav_row(ui, page, s.page == page))
                .collect();
            if rows.is_empty() {
                continue;
            }
            sidebar = sidebar.section(SidebarSection::new(ui.t(title)).key(format!("nav-group-{id}")).rows(rows));
        }
    }

    if s.books.has_company() {
        let name = s.books.company_name();
        let period = s.books.period_name();
        let title = if name.is_empty() { ui.t("Your business") } else { name };
        let sub = if period.is_empty() { String::new() } else { s.fmt.digits(&period) };
        let mut company = SidebarRow::new(title)
            .icon(&icons::BUILDING)
            .height(40.0)
            .on_select(|st: &mut State, _| st.company_switch_open = true);
        if !sub.is_empty() {
            company = company.subtitle(sub);
        }
        sidebar = sidebar.footer(company.build(k).key("org-selector"));
    }

    let lang_label = match s.lang() {
        Lang::Fa => "English",
        Lang::En => "فارسی",
    };
    let theme_label = if s.dark { ui.t("Light") } else { ui.t("Dark") };
    let theme_icon = if s.dark { &icons::SUN } else { &icons::MOON };

    sidebar
        .footer(
            SidebarRow::new(lang_label)
                .icon(&icons::GLOBE)
                .height(32.0)
                .on_select(|st: &mut State, _| {
                    let next = st.lang().other();
                    st.set_lang(next);
                    let _ = st.save_preferences();
                })
                .build(k)
                .key("lang-toggle"),
        )
        .footer(
            SidebarRow::new(theme_label)
                .icon(theme_icon)
                .height(32.0)
                .on_select(|st: &mut State, _| {
                    st.dark = !st.dark;
                    let _ = st.save_preferences();
                })
                .build(k)
                .key("theme-toggle"),
        )
        .build(k)
}

/// When the sidebar is fully closed, keep page icons reachable in the toolbar (Rune shell + rail).
fn page_rail(s: &State, ui: &Ui) -> Vec<El> {
    if s.sidebar.is_open() {
        return Vec::new();
    }
    Page::ALL
        .into_iter()
        .filter(|page| page.nav_pages(s.show_pos))
        .map(|page| {
            let selected = s.page == page;
            IconButton::new(page.icon())
                .variant(if selected {
                    IconButtonVariant::Contrast
                } else {
                    IconButtonVariant::GhostMuted
                })
                .label(ui.t(page.label()))
                .on_press(move |st: &mut State| st.nav(page))
                .build(&ui.k)
                .key(format!("rail-{}", page.key()))
        })
        .collect()
}

fn build_toolbar(s: &State, ui: &Ui) -> Toolbar {
    let k = &ui.k;
    let page_label = ui.t(s.page.label());

    let search = IconButton::new(&icons::SEARCH)
        .variant(IconButtonVariant::GhostMuted)
        .label(ui.t("Search anything…"))
        .shortcut(if cfg!(target_os = "macos") { "cmd+k" } else { "ctrl+k" })
        .on_press(|st: &mut State| st.open_palette())
        .build(k)
        .key("global-search");

    let refresh = IconButton::new(&icons::REFRESH)
        .variant(IconButtonVariant::GhostMuted)
        .label(ui.t("Refresh"))
        .on_press(|st: &mut State| {
            st.refresh();
            if !st.books.error.is_empty() {
                let err = st.books.error.clone();
                st.toast(Tone::Danger, err);
            } else {
                st.toast(Tone::Success, st.fmt.t("Books refreshed"));
            }
        })
        .build(k)
        .key("refresh-books");

    let synced = if !s.books.error.is_empty() {
        Badge::new(ui.t("Books could not be loaded")).variant(BadgeVariant::Subtle).build(k)
    } else {
        Badge::new(ui.t("Saved on this device")).variant(BadgeVariant::Subtle).build(k)
    };

    let mut bar = Toolbar::new().title(page_label);
    for rail in page_rail(s, ui) {
        bar = bar.leading(rail);
    }
    bar = bar.trailing(search).trailing(refresh).trailing(synced);
    let period = s.books.period_name();
    if !period.is_empty() {
        let (start, end) = s.books.range();
        let now = crate::data::today();
        let is_current = !start.is_empty() && !end.is_empty() && now >= start && now <= end;
        if !is_current {
            bar = bar.trailing(ui.badge(Tone::Warning, ui.t("Viewing a past fiscal year")));
        }
        let label = format!("{} {}", ui.t("Fiscal year"), s.fmt.digits(&period));
        let switcher = Button::new(label)
            .variant(ButtonVariant::Outline)
            .control_size(ControlSize::Small)
            .on_press(|st: &mut State| st.period_switch_open = true)
            .build(k)
            .key("fiscal-year-switch");
        bar = bar.trailing(switcher);
    }
    bar
}

/// Full window chrome: Rune shell defaults (framed panel, full close, peek, icon rail).
pub fn app_shell(s: &State, ui: &Ui, cx: &mut Cx<State>, content: El) -> El {
    let ready = s.books.has_company() && s.books.initialized();
    let sidebar = build_sidebar(s, ui, ready || s.books.has_company());
    let main = col()
        .key("main")
        .grow(1.0)
        .h_full()
        .min_w(0.0)
        .min_h(0.0)
        .fill(ui.k.c.bg_base)
        .child(content.grow(1.0).min_h(0.0));

    SplitView::new(main)
        .sidebar(&s.sidebar, |st: &mut State| &mut st.sidebar, sidebar)
        .toolbar(build_toolbar(s, ui))
        .titlebar_inset(cx.titlebar_inset())
        .frame_inset(8.0)
        .toolbar_in_panel()
        .sidebar_padding(18.0, 10.0)
        .sidebar_toggle(
            SidebarToggle::new()
                .size(ControlSize::Large)
                .past_lights(16.0)
                .edge(18.0)
                .gap(4.0)
                .in_header(),
        )
        .pane_motion(
            PaneMotion::new()
                .spring(Spring::new(0.3, 0.0))
                .scale_with_width(480.0, 0.15, 0.3)
                .fade_tail(0.15),
        )
        .sidebar_peek(
            SidebarPeek::new()
                .edge(16.0, ms(40))
                .toggle_delay(ms(80))
                .below(56.0)
                .hold(15.0, ms(100))
                .spring(Spring::new(0.18, 0.0)),
        )
        .reduced_motion(cx.is_reduced_motion())
        .build(&ui.k)
}

pub struct PaletteEntry {
    pub key: String,
    pub group: Option<&'static str>,
    pub icon: &'static rune::Icon,
    pub title: String,
    pub hint: String,
    pub run: Rc<dyn Fn(&mut State)>,
}

fn shortcut_icon(id: &str) -> &'static rune::Icon {
    match id {
        "sc-s" => &icons::SETTINGS,
        _ => &icons::PLUS,
    }
}

pub fn palette_entries(s: &State) -> Vec<PaletteEntry> {
    let q_raw = s.forms.get("palette-query");
    let q = crate::i18n::fold(q_raw.trim());
    let mut entries = Vec::new();

    for (_, title, pages) in Page::GROUPS {
        for page in pages.iter().copied().filter(|page| page.nav_pages(s.show_pos)) {
            let label = s.fmt.t(page.label());
            let hay = crate::i18n::fold(&format!("{} {label}", page.label()));
            if q.is_empty() || hay.contains(&q) {
                entries.push(PaletteEntry {
                    key: format!("palette-{}", page.key()),
                    group: Some(title),
                    icon: page.icon(),
                    title: label,
                    hint: s.fmt.t("Go"),
                    run: Rc::new(move |st| st.nav(page)),
                });
            }
        }
    }

    for (_, sc) in crate::shortcuts::CREATE {
        let translated = s.fmt.t(sc.label);
        let hay = crate::i18n::fold(&format!("{} {translated}", sc.label));
        if q.is_empty() || hay.contains(&q) {
            let run = sc.run;
            entries.push(PaletteEntry {
                key: format!("action-{}", sc.id),
                group: None,
                icon: shortcut_icon(sc.id),
                title: translated,
                hint: "↵".into(),
                run: Rc::new(run),
            });
        }
    }

    type GlobalAction = (&'static str, &'static str, &'static rune::Icon, fn(&mut State));
    let globals: [GlobalAction; 3] = [
        ("action-theme", "Toggle theme", &icons::SUN, |st| {
            st.dark = !st.dark;
            let _ = st.save_preferences();
        }),
        ("action-sidebar", "Collapse sidebar", &icons::SIDEBAR, |st| st.toggle_sidebar()),
        ("action-lang", "Language", &icons::GLOBE, |st| {
            let next = st.lang().other();
            st.set_lang(next);
            let _ = st.save_preferences();
        }),
    ];
    for (key, label, glyph, action) in globals {
        let translated = s.fmt.t(label);
        let hay = crate::i18n::fold(&format!("{label} {translated}"));
        if q.is_empty() || hay.contains(&q) {
            entries.push(PaletteEntry {
                key: key.to_string(),
                group: None,
                icon: glyph,
                title: translated,
                hint: "↵".into(),
                run: Rc::new(action),
            });
        }
    }

    if !q.is_empty() {
        let mut hits = 0usize;
        if let Some(parties) = s.books.workspace["resources"]["parties"].as_array() {
            for party in parties {
                if hits >= 8 {
                    break;
                }
                let name = crate::data::s(&party["name"]);
                let id = crate::data::s(&party["id"]);
                let hay = crate::i18n::fold(&format!("{name} {id}"));
                if hay.contains(&q) {
                    let party_id = id.clone();
                    entries.push(PaletteEntry {
                        key: format!("palette-party-{id}"),
                        group: None,
                        icon: &icons::CONTACTS,
                        title: name,
                        hint: s.fmt.t("Contacts"),
                        run: Rc::new(move |st| {
                            st.nav(Page::Contacts);
                            crate::ui::views::contacts::open_contact(st, &party_id);
                        }),
                    });
                    hits += 1;
                }
            }
        }
        if let Some(products) = s.books.workspace["resources"]["items"].as_array() {
            for item in products {
                if hits >= 12 {
                    break;
                }
                let name = crate::data::s(&item["name"]);
                let id = crate::data::s(&item["id"]);
                let hay = crate::i18n::fold(&format!("{name} {id}"));
                if hay.contains(&q) {
                    let item_id = id.clone();
                    entries.push(PaletteEntry {
                        key: format!("palette-item-{id}"),
                        group: None,
                        icon: &icons::INVENTORY,
                        title: name,
                        hint: s.fmt.t("Inventory"),
                        run: Rc::new(move |st| {
                            st.nav(Page::Inventory);
                            crate::ui::views::inventory::open_product(st, &item_id);
                        }),
                    });
                    hits += 1;
                }
            }
        }
    }

    entries
}

fn palette_item(ui: &Ui, entry: &PaletteEntry, selected: bool) -> El {
    let k = &ui.k;
    let run = Rc::clone(&entry.run);
    let mut r = row()
        .key(entry.key.clone())
        .role("option")
        .label(entry.title.clone())
        .hit()
        .w_full()
        .h(36.0)
        .px(8.0)
        .gap(10.0)
        .center_y()
        .radius(k.radius.md)
        .child(
            row()
                .size(22.0, 22.0)
                .center()
                .radius(k.radius.sm)
                .fill(k.c.layer_02)
                .child(icon(entry.icon, 13.0).color(k.c.icon_base)),
        )
        .child(ui.body(entry.title.clone()).grow(1.0).min_w(0.0))
        .child(ui.muted(entry.hint.clone()))
        .on_press(move |st| {
            run(st);
            st.palette_open = false;
        });
    if selected {
        r = r.fill(k.c.hover);
    }
    k.interactive(r)
}

pub fn mount_palette(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s.palette_open {
        return;
    }
    let entries = palette_entries(s);
    let q = s.forms.get("palette-query");
    let cursor = if entries.is_empty() || q != s.palette_last_query {
        0
    } else {
        s.palette_cursor.min(entries.len() - 1)
    };

    let mut items = col()
        .key("palette-results")
        .w_full()
        .gap(2.0)
        .max_h(340.0)
        .scroll(rune::ScrollSpec { stick_bottom: false });

    let mut last_group: Option<&'static str> = None;
    for (idx, entry) in entries.iter().enumerate() {
        if let Some(group) = entry.group
            && last_group != Some(group)
        {
            items = items.child(ui.muted(ui.t(group)).key(format!("palette-group-{group}")).ml(8.0).mt(8.0));
            last_group = Some(group);
        }
        items = items.child(palette_item(ui, entry, idx == cursor));
    }

    if entries.is_empty() {
        items = items.child(ui.empty(&icons::SEARCH, "No records yet.", ""));
    }

    let body = col()
        .gap(12.0)
        .child(ui.field(s, "palette-query", &ui.t("Type a command…")))
        .child(items);

    ui.dialog(
        layer,
        "palette",
        &ui.t("Command menu"),
        true,
        DIALOG_SM,
        body,
        vec![ui.btn(Btn::Ghost, "Cancel", None, |st| st.palette_open = false)],
        |st| st.palette_open = false,
    );
}

pub fn mount_toasts(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if s.toasts.is_empty() {
        return;
    }
    let mut region = ToastRegion::new();
    for t in &s.toasts {
        let id = t.id;
        let glyph = match t.tone {
            Tone::Success => &icons::CHECK,
            Tone::Warning | Tone::Danger => &icons::WARNING,
            Tone::Info => &icons::INFO,
        };
        let mut toast = Toast::new(format!("toast-{id}"), t.message.clone())
            .lead(icon(glyph, 16.0).color(ui.tone_accent(t.tone)))
            .on_close(move |st: &mut State| st.toasts.retain(|x| x.id != id));
        if let Some(path) = &t.open_path {
            let p_open = path.clone();
            let p_folder = path.clone();
            toast = toast
                .action(&ui.k, &ui.t("Open"), true, move |st: &mut State, _tx| {
                    if let Err(e) = crate::ui::documents::reveal(&p_open, false) {
                        st.toast_err(e);
                    }
                })
                .action(&ui.k, &ui.t("Show in folder"), false, move |st: &mut State, _tx| {
                    if let Err(e) = crate::ui::documents::reveal(&p_folder, true) {
                        st.toast_err(e);
                    }
                });
        }
        region = region.toast(toast.build(&ui.k));
    }
    layer.push(region.build(&ui.k));
}

pub fn mount_company_switcher(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s.company_switch_open {
        return;
    }
    let companies = s.books.companies();
    let mut list = col().w_full().gap(6.0);
    for (id, name) in companies {
        let active = id == s.books.company_id;
        let label = if name.is_empty() { ui.t("Your business") } else { name };
        let display = if active { format!("{label}  ({})", ui.t("Current")) } else { label };
        let target_id = id.clone();
        let row_btn = Button::new(display)
            .variant(if active { ButtonVariant::Submit } else { ButtonVariant::Outline })
            .control_size(ControlSize::Regular)
            .on_press(move |st: &mut State| {
                st.company_switch_open = false;
                if target_id != st.books.company_id {
                    st.select_company(&target_id);
                }
            })
            .build(&ui.k)
            .key(format!("switch-company-{id}"))
            .w_full();
        list = list.child(row_btn);
    }

    let add_btn = Button::new(ui.t("Add company"))
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Regular)
        .on_press(crate::ui::views::setup::start_add_company)
        .build(&ui.k)
        .key("switch-add-company");

    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "company-switch",
        &ui.t("Switch company"),
        &ui.t("Choose a business"),
        true,
        DIALOG_SM,
        list,
        vec![add_btn, ui.btn(Btn::Ghost, "Close", None, |st| st.company_switch_open = false)],
        |st| st.company_switch_open = false,
    );
}

pub fn mount_period_switcher(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s.period_switch_open {
        return;
    }
    let mut periods = s.books.workspace["periods"].as_array().cloned().unwrap_or_default();
    periods.sort_by(|a, b| crate::data::s(&b["start_date"]).cmp(&crate::data::s(&a["start_date"])));

    let mut list = col().w_full().gap(6.0);
    for p in periods {
        let id = crate::data::s(&p["id"]);
        let name = s.fmt.digits(&crate::data::s(&p["name"]));
        let start = s.fmt.date(&crate::data::s(&p["start_date"]));
        let end = s.fmt.date(&crate::data::s(&p["end_date"]));
        let active = id == s.books.period_id;
        let label = format!("{name}  ({start} – {end})");
        let target_id = id.clone();
        let row_btn = Button::new(label)
            .variant(if active { ButtonVariant::Submit } else { ButtonVariant::Outline })
            .control_size(ControlSize::Regular)
            .on_press(move |st: &mut State| {
                st.books.period_id = target_id.clone();
                st.period_switch_open = false;
                st.refresh();
            })
            .build(&ui.k)
            .key(format!("switch-period-{id}"))
            .w_full();
        list = list.child(row_btn);
    }

    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "period-switch",
        &ui.t("Fiscal year"),
        &ui.t("Choose a fiscal year"),
        true,
        DIALOG_SM,
        list,
        vec![ui.btn(Btn::Ghost, "Close", None, |st| st.period_switch_open = false)],
        |st| st.period_switch_open = false,
    );
}

pub fn segmented(
    ui: &Ui,
    id: &str,
    labels: &[String],
    selected: Option<usize>,
    on_pick: impl Fn(&mut State, usize) + Clone + 'static,
) -> El {
    SegmentedControl::new(labels, selected)
        .fit()
        .label(id.to_string())
        .on_pick(move |st: &mut State, i, _| on_pick(st, i))
        .build(&ui.k)
        .key(id.to_string())
}
