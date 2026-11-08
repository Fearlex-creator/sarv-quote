//! Bundled OFL fonts for printable PDFs, confined to one render.
//!
//! A fresh Pango/Fontconfig map confines the bundled fonts to this PDF operation.
//! The private files stay alive through lazy shaping and PDF font embedding.
use crate::store::write_new;
use anyhow::{Result, ensure};
use std::path::PathBuf;

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigCreate() -> *mut std::ffi::c_void;
    fn FcConfigParseAndLoad(config: *mut std::ffi::c_void, file: *const u8, complain: i32) -> i32;
    fn FcConfigAppFontAddFile(config: *mut std::ffi::c_void, file: *const u8) -> i32;
    fn FcConfigDestroy(config: *mut std::ffi::c_void);
}
#[link(name = "pangoft2-1.0")]
unsafe extern "C" {
    fn pango_fc_font_map_set_config(map: *mut std::ffi::c_void, config: *mut std::ffi::c_void);
}

/// A Fontconfig-backed Pango map on every platform. On Windows the default
/// PangoCairo map is the Win32 one, which cannot take private font files.
pub fn font_map() -> pango::FontMap {
    if cfg!(windows) {
        pangocairo::FontMap::for_font_type(cairo::FontType::FontTypeFt)
            .unwrap_or_else(pangocairo::FontMap::new)
    } else {
        pangocairo::FontMap::new()
    }
}

pub struct PrivateFonts(PathBuf);
impl Drop for PrivateFonts {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Registers the bundled Vazirmatn fonts on `map`, which must come from [`font_map`].
pub fn private(map: &pango::FontMap, scope: &str) -> Result<PrivateFonts> {
    use pango::glib::translate::ToGlibPtr;
    use std::ffi::CString;
    let dir = std::env::temp_dir().join(format!("sarv-{scope}-pdf-fonts-{}", crate::domain::id()));
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&dir)?;
    let guard = PrivateFonts(dir);
    let fonts: &[(&str, &[u8])] = &[
        (
            "Vazirmatn-Regular.ttf",
            include_bytes!("../assets/fonts/Vazirmatn-Regular.ttf"),
        ),
        (
            "Vazirmatn-Bold.ttf",
            include_bytes!("../assets/fonts/Vazirmatn-Bold.ttf"),
        ),
    ];
    for &(name, bytes) in fonts {
        write_new(&guard.0.join(name), bytes)?;
    }
    write_new(
        &guard.0.join("OFL.txt"),
        include_bytes!("../assets/fonts/OFL.txt"),
    )?;
    // Fontconfig owns the config; Pango takes its own reference when assigned.
    let config = unsafe { FcConfigCreate() };
    ensure!(!config.is_null(), "پیکربندی قلم PDF در دسترس نیست");
    let result = (|| -> Result<()> {
        // Windows installs ship no system fonts.conf; the bundled fonts are
        // registered explicitly below, so a missing default config is fine there.
        let loaded = unsafe { FcConfigParseAndLoad(config, std::ptr::null(), i32::from(!cfg!(windows))) };
        ensure!(cfg!(windows) || loaded != 0, "خواندن پیکربندی قلم PDF به پایان نرسید");
        for &(name, _) in fonts {
            let path = CString::new(guard.0.join(name).to_string_lossy().as_bytes())?;
            ensure!(
                unsafe { FcConfigAppFontAddFile(config, path.as_ptr().cast()) } != 0,
                "ثبت قلم PDF به پایان نرسید"
            );
        }
        unsafe {
            let pointer: *mut pango::ffi::PangoFontMap = map.to_glib_none().0;
            pango_fc_font_map_set_config(pointer.cast(), config);
        }
        Ok(())
    })();
    unsafe {
        FcConfigDestroy(config);
    }
    result?;
    Ok(guard)
}
