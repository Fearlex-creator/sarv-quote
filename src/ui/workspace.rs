//! Shared native workspace presentation. Fonts are embedded in the executable
//! and registered in Fontconfig's application set, never installed globally.
use super::components::*;
use adw::prelude::*;
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{CString, c_void},
    path::PathBuf,
};

pub(super) const FONT_LICENSE: &str = include_str!("../../assets/fonts/OFL.txt");
const FONTS: &[(&str, &[u8])] = &[
    (
        "Vazirmatn-Regular.ttf",
        include_bytes!("../../assets/fonts/Vazirmatn-Regular.ttf"),
    ),
    (
        "Vazirmatn-Medium.ttf",
        include_bytes!("../../assets/fonts/Vazirmatn-Medium.ttf"),
    ),
    (
        "Vazirmatn-SemiBold.ttf",
        include_bytes!("../../assets/fonts/Vazirmatn-SemiBold.ttf"),
    ),
    (
        "Vazirmatn-Bold.ttf",
        include_bytes!("../../assets/fonts/Vazirmatn-Bold.ttf"),
    ),
];

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent() -> *mut c_void;
    fn FcConfigAppFontAddFile(config: *mut c_void, file: *const u8) -> i32;
}

/// Keep this guard alive for the complete application run. Each process owns
/// only its fresh private temporary directory, which is removed on shutdown.
pub(super) struct FontGuard {
    directory: PathBuf,
}
impl Drop for FontGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

pub(super) fn install_fonts() -> Result<FontGuard> {
    use std::os::unix::fs::DirBuilderExt;
    let directory = std::env::temp_dir().join(format!("sarv-fonts-{}", sarv_quote::domain::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .context("ساخت پوشهٔ موقت قلم فارسی به پایان نرسید")?;
    let fonts = FontGuard { directory };
    // FcConfigGetCurrent initializes the process Fontconfig configuration if
    // needed. The pointer stays owned by Fontconfig; we neither free nor mutate
    // unrelated system font settings. Only validated embedded TTF bytes enter.
    let config = unsafe { FcConfigGetCurrent() };
    ensure!(!config.is_null(), "پیکربندی قلم فارسی در دسترس نیست");
    for &(name, bytes) in FONTS {
        sarv_quote::store::write_new(&fonts.directory.join(name), bytes)?;
    }
    sarv_quote::store::write_new(&fonts.directory.join("OFL.txt"), FONT_LICENSE.as_bytes())?;
    for &(name, _) in FONTS {
        let path = CString::new(fonts.directory.join(name).as_os_str().as_encoded_bytes())?;
        // The CString is NUL-terminated and remains alive for the call. The
        // private file remains available through FontGuard for lazy font loads.
        ensure!(
            unsafe { FcConfigAppFontAddFile(config, path.as_ptr().cast()) } != 0,
            "ثبت قلم فارسی به پایان نرسید: {name}"
        );
    }
    Ok(fonts)
}

pub(super) struct NavigationRow {
    pub(super) button: gtk::ToggleButton,
    pub(super) count: gtk::Label,
}
impl NavigationRow {
    pub(super) fn set_count(&self, value: Option<usize>) {
        self.count.set_visible(value.is_some());
        if let Some(value) = value {
            self.count.set_text(&sarv_quote::domain::fa(value));
        }
    }
}

/// GTK owns selected, focus, pointer and disabled states. Familiar symbolic
/// icons identify sections; text remains the complete accessible name.
pub(super) fn navigation_row(caption: &str, icon_name: &str, shortcut: &str) -> NavigationRow {
    let button = gtk::ToggleButton::new();
    button.add_css_class("flat");
    button.add_css_class("nav");
    button.add_css_class("sarv-navigation-row");
    button.set_tooltip_text(Some(&format!("{caption} · {shortcut}")));
    accessible_name(&button, caption);
    button.update_property(&[gtk::accessible::Property::KeyShortcuts(shortcut)]);
    let content = row();
    content.set_spacing(8);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(18);
    icon.add_css_class("navigation-icon");
    content.append(&icon);
    let title = label(caption);
    title.add_css_class("navigation-label");
    title.set_selectable(false);
    title.set_wrap(false);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_hexpand(true);
    content.append(&title);
    let count = label("");
    count.add_css_class("navigation-count");
    count.add_css_class("caption");
    count.set_selectable(false);
    count.set_wrap(false);
    count.set_visible(false);
    content.append(&count);
    let hint = label(shortcut);
    hint.add_css_class("navigation-shortcut");
    hint.add_css_class("caption");
    hint.set_direction(gtk::TextDirection::Ltr);
    hint.set_selectable(false);
    hint.set_wrap(false);
    content.append(&hint);
    button.set_child(Some(&content));
    NavigationRow { button, count }
}

pub(super) fn navigation_group(title: &str) -> gtk::Label {
    let caption = label(title);
    caption.set_selectable(false);
    caption.add_css_class("navigation-group");
    caption.add_css_class("caption-heading");
    caption
}

pub(super) fn sidebar_header(business: &str) -> gtk::Box {
    let header = column();
    header.set_spacing(4);
    header.add_css_class("workspace-brand");
    let title = label("سرو");
    title.set_selectable(false);
    title.add_css_class("title-3");
    header.append(&title);
    let business = if business.trim().is_empty() {
        "پیشنهاد قیمت · فضای کار محلی"
    } else {
        business
    };
    let description = label(business);
    description.add_css_class("caption");
    description.add_css_class("dimmed");
    description.set_selectable(false);
    description.set_wrap(false);
    description.set_max_width_chars(25);
    description.set_ellipsize(gtk::pango::EllipsizeMode::End);
    description.set_tooltip_text(Some(business));
    header.append(&description);
    header
}

pub(super) fn page_header(title: &str, subtitle: Option<&str>) -> gtk::Box {
    let header = column();
    header.set_spacing(4);
    header.add_css_class("workspace-page-header");
    header.append(&page_title(title));
    if let Some(subtitle) = subtitle {
        let description = label(subtitle);
        description.set_selectable(false);
        description.add_css_class("dimmed");
        description.add_css_class("caption");
        header.append(&description);
    }
    header
}
