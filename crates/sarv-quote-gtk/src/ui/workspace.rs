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

pub(super) fn sidebar_header(business: &str) -> gtk::Box {
    let header = column();
    header.set_spacing(SPACE_1);
    header.add_css_class("workspace-brand");
    let title = heading("سرو");
    title.set_selectable(false);
    title.add_css_class("title-3");
    title.add_css_class("sarv-label-heading");
    header.append(&title);
    let business = if business.trim().is_empty() {
        "حسابداری"
    } else {
        business
    };
    let description = label(business);
    description.add_css_class("caption");
    description.add_css_class("dimmed");
    description.add_css_class("sarv-label-muted");
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
    header.set_spacing(SPACE_1);
    header.add_css_class("workspace-page-header");
    header.append(&page_title(title));
    if let Some(subtitle) = subtitle {
        let description = label(subtitle);
        description.set_selectable(false);
        description.add_css_class("dimmed");
        description.add_css_class("caption");
        description.add_css_class("sarv-label-muted");
        header.append(&description);
    }
    header
}
