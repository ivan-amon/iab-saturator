//! Readout font: Roboto Slab SemiBold (600), embedded (Apache-2.0, see assets/fonts).

use egui::{FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

const ROBOTO_SLAB_600: &[u8] = include_bytes!("../../assets/fonts/RobotoSlab-SemiBold.ttf");
const READOUT_FAMILY: &str = "roboto_slab_600";

/// Registers the readout font. Called once per editor opening.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        READOUT_FAMILY.to_owned(),
        Arc::new(FontData::from_static(ROBOTO_SLAB_600)),
    );
    fonts.families.insert(
        FontFamily::Name(READOUT_FAMILY.into()),
        vec![READOUT_FAMILY.to_owned()],
    );
    ctx.set_fonts(fonts);
}

/// Readout font at `size` px.
pub fn readout(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(READOUT_FAMILY.into()))
}
