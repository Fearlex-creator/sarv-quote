//! App chrome matching rune-kit's shell example: full close, peek, framed panel, icon rail.

use std::time::Duration;

use rune::{Cx, El, Spring, col, icon, row};
use rune_kit::shell::{PaneMotion, SidebarPeek, SidebarToggle};
use rune_kit::{
    Badge, BadgeVariant, ControlSize, IconButton, IconButtonVariant, SegmentedControl, Sidebar, SidebarRow,
    SidebarSection, SplitView, Toast, ToastRegion, Toolbar,
};

use crate::app::{Page, State, Tone};
use crate::i18n::Lang;
use crate::ui::glass::{Btn, Ui};
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
                        st.sales.start_new();
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
            .on_select(|st: &mut State, _| st.nav(Page::Settings));
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
                })
                .build(k)
                .key("lang-toggle"),
        )
        .footer(
            SidebarRow::new(theme_label)
                .icon(theme_icon)
                .height(32.0)
                .on_select(|st: &mut State, _| st.dark = !st.dark)
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
            st.toast(Tone::Success, st.fmt.t("Books refreshed"));
        })
        .build(k)
        .key("refresh-books");

    let synced = Badge::new(ui.t("Saved on this device")).variant(BadgeVariant::Subtle).build(k);

    let mut bar = Toolbar::new().title(page_label);
    for rail in page_rail(s, ui) {
        bar = bar.leading(rail);
    }
    bar = bar.trailing(search).trailing(refresh).trailing(synced);
    let period = s.books.period_name();
    if !period.is_empty() {
        let label = format!("{} {}", ui.t("Fiscal year"), s.fmt.digits(&period));
        bar = bar.trailing(Badge::new(label).variant(BadgeVariant::Neutral).build(k));
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

fn palette_item(ui: &Ui, key: &str, glyph: &'static rune::Icon, title: String, hint: String, on_press: impl Fn(&mut State) + 'static) -> El {
    let k = &ui.k;
    k.interactive(
        row()
            .key(key.to_string())
            .role("option")
            .label(title.clone())
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
                    .child(icon(glyph, 13.0).color(k.c.icon_base)),
            )
            .child(ui.body(title).grow(1.0).min_w(0.0))
            .child(ui.muted(hint))
            .on_press(on_press),
    )
}

pub fn mount_palette(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s.palette_open {
        return;
    }
    let query = s.forms.get("palette-query").to_lowercase();
    let query_latin = crate::i18n::latin_digits(&query).to_lowercase();

    let mut items = col()
        .key("palette-results")
        .w_full()
        .gap(2.0)
        .max_h(340.0)
        .scroll(rune::ScrollSpec { stick_bottom: false });
    let mut count = 0usize;

    for (id, title, pages) in Page::GROUPS {
        let mut group = Vec::new();
        for page in pages.iter().copied().filter(|page| page.nav_pages(s.show_pos)) {
            let label = ui.t(page.label());
            let hay = format!("{} {}", page.label(), label).to_lowercase();
            if query.is_empty() || hay.contains(&query) || hay.contains(&query_latin) {
                group.push(palette_item(
                    ui,
                    &format!("palette-{}", page.key()),
                    page.icon(),
                    label,
                    ui.t("Go"),
                    move |st| st.nav(page),
                ));
            }
        }
        if group.is_empty() {
            continue;
        }
        count += group.len();
        items = items.child(
            ui.muted(ui.t(title))
                .key(format!("palette-group-{id}"))
                .ml(8.0)
                .mt(8.0),
        );
        for row in group {
            items = items.child(row);
        }
    }

    let actions: [(&str, &str, &'static rune::Icon, fn(&mut State)); 4] = [
        ("action-new-order", "New sales invoice", &icons::PLUS, |st| {
            st.sales.start_new();
            st.nav(Page::Sales);
        }),
        ("action-theme", "Toggle theme", &icons::SUN, |st| st.dark = !st.dark),
        ("action-sidebar", "Collapse sidebar", &icons::SIDEBAR, |st| st.toggle_sidebar()),
        ("action-lang", "Language", &icons::GLOBE, |st| {
            let next = st.lang().other();
            st.set_lang(next);
        }),
    ];
    for (key, label, glyph, action) in actions {
        let translated = ui.t(label);
        let hay = format!("{label} {translated}").to_lowercase();
        if query.is_empty() || hay.contains(&query) || hay.contains(&query_latin) {
            items = items.child(palette_item(ui, key, glyph, translated, "↵".into(), action));
            count += 1;
        }
    }

    if !query.is_empty() {
        let mut hits = 0usize;
        if let Some(parties) = s.books.workspace["resources"]["parties"].as_array() {
            for party in parties {
                if hits >= 8 {
                    break;
                }
                let name = crate::data::s(&party["name"]);
                if name.to_lowercase().contains(&query) {
                    let id = crate::data::s(&party["id"]);
                    items = items.child(palette_item(
                        ui,
                        &format!("palette-party-{id}"),
                        &icons::CONTACTS,
                        name,
                        ui.t("Contacts"),
                        move |st| {
                            st.nav(Page::Contacts);
                            st.palette_open = false;
                        },
                    ));
                    hits += 1;
                    count += 1;
                }
            }
        }
        if let Some(products) = s.books.workspace["resources"]["items"].as_array() {
            for item in products {
                if hits >= 12 {
                    break;
                }
                let name = crate::data::s(&item["name"]);
                if name.to_lowercase().contains(&query) {
                    let id = crate::data::s(&item["id"]);
                    items = items.child(palette_item(
                        ui,
                        &format!("palette-item-{id}"),
                        &icons::INVENTORY,
                        name,
                        ui.t("Inventory"),
                        move |st| {
                            st.nav(Page::Inventory);
                            st.palette_open = false;
                        },
                    ));
                    hits += 1;
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
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
        480.0,
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
        let toast = Toast::new(format!("toast-{id}"), t.message.clone())
            .lead(icon(&icons::INFO, 16.0).color(ui.tone_accent(t.tone)))
            .on_close(move |st: &mut State| st.toasts.retain(|x| x.id != id))
            .build(&ui.k);
        region = region.toast(toast);
    }
    layer.push(region.build(&ui.k));
}

pub fn segmented(ui: &Ui, id: &str, labels: &[String], selected: Option<usize>, on_pick: impl Fn(&mut State, usize) + Clone + 'static) -> El {
    SegmentedControl::new(labels, selected)
        .fit()
        .label(id.to_string())
        .on_pick(move |st: &mut State, i, _| on_pick(st, i))
        .build(&ui.k)
        .key(id.to_string())
}
