//! Sarv kit: Rune's neutral theme with Sarv teal accent and Vazirmatn for Persian UI.

use std::sync::OnceLock;

use rune::tok::{Family, TextStyle};
use rune_kit::Kit;

const VAZIR: Family = Family::Named("Vazirmatn");

fn vazir(style: TextStyle) -> TextStyle {
    style.family(VAZIR)
}

/// Sarv teal (ADR-0001). Same hue the product has always used.
pub fn sarv_accent(dark: bool) -> rune::Color {
    if dark {
        rune::oklch(0.72, 0.10, 186.0)
    } else {
        rune::oklch(0.511, 0.096, 186.4)
    }
}

fn build_sarv_kit(dark: bool) -> Kit {
    let mut k = Kit::neutral(dark);
    let accent = sarv_accent(dark);
    k.c.bg_accent = accent;
    k.c.text_accent = accent;
    k.c.icon_accent = accent;
    k.c.border_focus = accent;
    k.c.border_active = accent;
    k.ty.ui = vazir(k.ty.ui);
    k.ty.ui_medium = vazir(k.ty.ui_medium);
    k.ty.ui_compact = vazir(k.ty.ui_compact);
    k.ty.small = vazir(k.ty.small);
    k.ty.small_medium = vazir(k.ty.small_medium);
    k.ty.body = vazir(k.ty.body);
    k.ty.body_medium = vazir(k.ty.body_medium);
    k.ty.text_small = vazir(k.ty.text_small);
    k.ty.text_small_medium = vazir(k.ty.text_small_medium);
    k.ty.text_xsmall = vazir(k.ty.text_xsmall);
    k.ty.text_xsmall_medium = vazir(k.ty.text_xsmall_medium);
    k
}

/// Theme (`Kit::neutral`) with Sarv teal accent and Vazirmatn type for bilingual UI.
pub fn sarv_kit(dark: bool) -> Kit {
    static LIGHT: OnceLock<Kit> = OnceLock::new();
    static DARK: OnceLock<Kit> = OnceLock::new();
    if dark {
        DARK.get_or_init(|| build_sarv_kit(true)).clone()
    } else {
        LIGHT.get_or_init(|| build_sarv_kit(false)).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sarv_kit_uses_sarv_teal_accent() {
        assert_eq!(sarv_kit(false).c.bg_accent, sarv_accent(false));
        assert_eq!(sarv_kit(true).c.bg_accent, sarv_accent(true));
        assert_ne!(sarv_accent(false), rune::tok::accent(0, false));
    }
}
