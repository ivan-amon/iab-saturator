//! Barlow Condensed Medium (SIL OFL 1.1, see `assets/fonts/OFL.txt`), embedded and registered in
//! egui as the only font family.

use nih_plug_egui::egui::{FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const BARLOW_CONDENSED_MEDIUM: &[u8] =
    include_bytes!("../../assets/fonts/BarlowCondensed-Medium.ttf");

const FONT_NAME: &str = "BarlowCondensed-Medium";

/// egui font definitions with Barlow Condensed Medium as the only font, for every family.
pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert(
        FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(BARLOW_CONDENSED_MEDIUM)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.insert(family, vec![FONT_NAME.to_owned()]);
    }
    fonts
}

/// The egui font for text whose em size is `em_px` logical px (the CSS / design-tool meaning of
/// "12 px"). egui's `FontId::size` already is the em size: epaint rescales ab_glyph's
/// line-height-based scale by `units_per_em / (ascent − descent)`. Checked end to end by
/// `preview_test::readout_text_is_sized_by_em`.
pub fn readout_font(em_px: f32) -> FontId {
    FontId::proportional(em_px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_contain_the_font() {
        let defs = font_definitions();
        assert!(defs.font_data.contains_key(FONT_NAME));
        assert_eq!(defs.families[&FontFamily::Proportional], vec![FONT_NAME]);
    }
}
