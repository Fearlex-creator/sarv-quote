//! Line icons (24-unit grid, 1.6 stroke) drawn through Rune's SVG `Icon`.

use rune::Icon;

macro_rules! icons {
    ($($name:ident = $body:expr;)*) => {
        $(pub static $name: Icon = Icon {
            name: concat!("sarv-", stringify!($name)),
            view: 24.0,
            body: concat!(
                r#"<g fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">"#,
                $body,
                "</g>"
            ),
        };)*
    };
}

icons! {
    OVERVIEW = r#"<rect x="3.5" y="3.5" width="7" height="7" rx="2"/><rect x="13.5" y="3.5" width="7" height="7" rx="2"/><rect x="3.5" y="13.5" width="7" height="7" rx="2"/><rect x="13.5" y="13.5" width="7" height="7" rx="2"/>"#;
    SALES = r#"<path d="M6 3.5h8.5L19 8v12.5H6z"/><path d="M14 3.5V8.5h5"/><path d="M9 13h6M9 16.5h6"/>"#;
    POS = r#"<rect x="3" y="5" width="18" height="14" rx="2.5"/><path d="M7 5V3.8h10V5M6.5 10h11M8 14.5h3.5"/>"#;
    PURCHASES = r#"<path d="M5 8h14l-1 12.5H6z"/><path d="M9 8V6.5a3 3 0 0 1 6 0V8"/>"#;
    PAYMENTS = r#"<path d="M4 8.5h14.5M15 5l3.5 3.5L15 12"/><path d="M20 15.5H5.5M9 12l-3.5 3.5L9 19"/>"#;
    BANKING = r#"<path d="M3.5 9.5 12 4l8.5 5.5"/><path d="M5.5 10.5v7M10 10.5v7M14 10.5v7M18.5 10.5v7"/><path d="M3.5 20h17"/>"#;
    CONTACTS = r#"<circle cx="9" cy="8" r="3.4"/><path d="M2.8 20c.4-3.6 3-5.6 6.2-5.6s5.8 2 6.2 5.6"/><path d="M16 4.8a3.2 3.2 0 0 1 0 6.2M18.5 14.8c1.6.8 2.5 2.4 2.8 4.8"/>"#;
    INVENTORY = r#"<path d="M12 3 20.5 7.5v9L12 21l-8.5-4.5v-9z"/><path d="M3.8 7.7 12 12l8.2-4.3M12 12v9"/>"#;
    ACCOUNTING = r#"<path d="M12 6.5C10.3 5 7.5 4.5 4 4.8v13c3.5-.3 6.3.2 8 1.7 1.7-1.5 4.5-2 8-1.7v-13c-3.5-.3-6.3.2-8 1.7z"/><path d="M12 6.5v13"/>"#;
    REPORTS = r#"<path d="M4 4v16h16"/><path d="m7.5 14.5 3.5-4 3 2.5 4.5-6"/>"#;
    SHEETS = r#"<rect x="3.5" y="3.5" width="17" height="17" rx="2"/><path d="M3.5 8.5h17M3.5 13.5h17M9 3.5v17M15 3.5v17"/>"#;
    SETTINGS = r#"<circle cx="12" cy="12" r="3"/><path d="M12 3v2.4M12 18.6V21M3 12h2.4M18.6 12H21M5.6 5.6l1.7 1.7M16.7 16.7l1.7 1.7M5.6 18.4l1.7-1.7M16.7 7.3l1.7-1.7"/>"#;
    PLUS = r#"<path d="M12 5v14M5 12h14"/>"#;
    SEARCH = r#"<circle cx="11" cy="11" r="6.5"/><path d="m20 20-4.2-4.2"/>"#;
    CHECK = r#"<path d="m5 12.5 4.5 4.5L19 7.5"/>"#;
    CHEVRON = r#"<path d="m9 5 7 7-7 7"/>"#;
    CHEVRON_DOWN = r#"<path d="m5 9 7 7 7-7"/>"#;
    ARROW = r#"<path d="M5 12h14M13 6l6 6-6 6"/>"#;
    SUN = r#"<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2.2M12 19.3v2.2M2.5 12h2.2M19.3 12h2.2M5.3 5.3l1.6 1.6M17.1 17.1l1.6 1.6M5.3 18.7l1.6-1.6M17.1 6.9l1.6-1.6"/>"#;
    MOON = r#"<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z"/>"#;
    GLOBE = r#"<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.5 2.4 3.6 5.2 3.6 8.5s-1.1 6.1-3.6 8.5c-2.5-2.4-3.6-5.2-3.6-8.5S9.5 5.9 12 3.5z"/>"#;
    SPARKLE = r#"<path d="M12 3.5c.6 4.4 2.1 5.9 6.5 6.5-4.4.6-5.9 2.1-6.5 6.5-.6-4.4-2.1-5.9-6.5-6.5 4.4-.6 5.9-2.1 6.5-6.5z"/><path d="M18.5 16.5c.2 1.5.7 2 2 2.2-1.3.2-1.8.7-2 2.1-.2-1.4-.7-1.9-2-2.1 1.3-.2 1.8-.7 2-2.2z"/>"#;
    BUILDING = r#"<path d="M5 20.5V4.5h9v16"/><path d="M14 9.5h5v11"/><path d="M8.5 8h2M8.5 11.5h2M8.5 15h2M3.5 20.5h17"/>"#;
    CALENDAR = r#"<rect x="3.5" y="5" width="17" height="15.5" rx="3"/><path d="M3.5 10h17M8 3v4M16 3v4"/>"#;
    CHEQUE = r#"<rect x="3" y="6" width="18" height="12" rx="2.5"/><path d="M6.5 10h6M6.5 14h3.5M15 15.5c.8-1.6 1.6-1.6 2.5 0"/>"#;
    WALLET = r#"<path d="M4 7.5A2.5 2.5 0 0 1 6.5 5H18v3.5"/><path d="M4 7.5V17a2.5 2.5 0 0 0 2.5 2.5H20V8.5H6.5A2.5 2.5 0 0 1 4 6"/><circle cx="16.3" cy="14" r="1"/>"#;
    BACKUP = r#"<path d="M7 18a4.5 4.5 0 0 1-.6-8.9 6 6 0 0 1 11.4 1.4A3.8 3.8 0 0 1 17.5 18"/><path d="M12 12v8M9 17l3 3 3-3"/>"#;
    ROBOT = r#"<rect x="4.5" y="8" width="15" height="11" rx="3.5"/><path d="M12 8V4.5M9 13v1.5M15 13v1.5"/>"#;
    SHIELD = r#"<path d="M12 3.5 19 6v5.5c0 4.2-2.8 7.4-7 9-4.2-1.6-7-4.8-7-9V6z"/><path d="m9 12 2.2 2.2L15.5 10"/>"#;
    BELL = r#"<path d="M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 2h-15z"/><path d="M10 20.5a2.2 2.2 0 0 0 4 0"/>"#;
    CLOSE = r#"<path d="m6 6 12 12M18 6 6 18"/>"#;
    SIDEBAR = r#"<rect x="3.5" y="4" width="17" height="16" rx="2.5"/><path d="M9 4v16"/>"#;
    PANEL_CLOSE = r#"<path d="M15 6 9 12l6 6"/><path d="M4 4v16"/>"#;
    PANEL_OPEN = r#"<path d="M9 6l6 6-6 6"/><path d="M20 4v16"/>"#;
    REFRESH = r#"<path d="M4.5 12a7.5 7.5 0 0 1 13.2-4.8L20 9.5"/><path d="M19.5 4.5v5h-5"/><path d="M19.5 12a7.5 7.5 0 0 1-13.2 4.8L4 14.5"/><path d="M4.5 19.5v-5h5"/>"#;
    HELP = r#"<circle cx="12" cy="12" r="8.5"/><path d="M9.5 9.2a2.5 2.5 0 1 1 3.4 2.3c-.8.4-1.4 1-1.4 2"/><path d="M12 17.2v.1"/>"#;
    TRASH = r#"<path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 13h9l1-13M10 11v5M14 11v5"/>"#;
    PRINT = r#"<path d="M7 9V3.5h10V9"/><rect x="3.5" y="9" width="17" height="8" rx="2.5"/><path d="M7 14h10v6.5H7z"/>"#;
    DOWNLOAD = r#"<path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19.5h14"/>"#;
    EDIT = r#"<path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z"/><path d="m13.5 6.5 4 4"/>"#;
    FILTER = r#"<path d="M4 6h16M7 12h10M10 18h4"/>"#;
    INFO = r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.8v.1"/>"#;
    WARNING = r#"<path d="M12 4 21 19.5H3z"/><path d="M12 10v4.5M12 17.2v.1"/>"#;
}
