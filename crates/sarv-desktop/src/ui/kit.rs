//! Sarv kit: Rune's neutral theme + Vazirmatn for Persian UI.

use rune::tok::{Family, TextStyle};
use rune_kit::Kit;

const VAZIR: Family = Family::Named("Vazirmatn");

fn vazir(style: TextStyle) -> TextStyle {
    style.family(VAZIR)
}

/// Default rune-kit theme (`Kit::neutral`) with Vazirmatn type for bilingual UI.
pub fn sarv_kit(dark: bool) -> Kit {
    let mut k = Kit::neutral(dark);
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
