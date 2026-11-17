//! Embedded GUI images, decoded on the GUI thread when the editor opens.

use super::sprites::SpriteManifest;
use std::collections::BTreeMap;

pub const PANEL_PNG: &[u8] = include_bytes!("../../assets/gui/panel.png");

/// Sprite sheets referenced by `sprites.json`, by file name.
const SHEET_FILES: &[(&str, &[u8])] = &[
    ("drive.png", include_bytes!("../../assets/gui/drive.png")),
    (
        "knob_small.png",
        include_bytes!("../../assets/gui/knob_small.png"),
    ),
    ("curve.png", include_bytes!("../../assets/gui/curve.png")),
    (
        "screw_tl.png",
        include_bytes!("../../assets/gui/screw_tl.png"),
    ),
    (
        "screw_tr.png",
        include_bytes!("../../assets/gui/screw_tr.png"),
    ),
    (
        "screw_bl.png",
        include_bytes!("../../assets/gui/screw_bl.png"),
    ),
    (
        "screw_br.png",
        include_bytes!("../../assets/gui/screw_br.png"),
    ),
];

/// 8-bit RGBA image, straight (non-premultiplied) alpha, row-major.
#[derive(Debug, Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    #[cfg(test)]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = 4 * (y as usize * self.width as usize + x as usize);
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }
}

/// Decode a PNG of any 8-bit-or-less colour type into RGBA8.
pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage, png::DecodingError> {
    let mut decoder = png::Decoder::new(bytes);
    // Palette → RGB(A), 16-bit → 8-bit, low bit depths → 8-bit.
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    buf.truncate(info.buffer_size());

    let pixels = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[l, a]| [l, l, l, a])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        // `normalize_to_color8` expands palettes, so this cannot happen.
        png::ColorType::Indexed => unreachable!("palette PNGs are expanded by the decoder"),
    };
    Ok(RgbaImage {
        width: info.width,
        height: info.height,
        pixels,
    })
}

/// Embedded bytes of a sprite sheet named in `sprites.json`.
pub fn sheet_png(file: &str) -> Option<&'static [u8]> {
    SHEET_FILES
        .iter()
        .find(|(name, _)| *name == file)
        .map(|(_, bytes)| *bytes)
}

/// Everything the editor draws, decoded to RGBA. Built once per editor opening; the CPU copies
/// are dropped after the textures are uploaded.
pub struct DecodedAssets {
    pub manifest: SpriteManifest,
    pub panel: RgbaImage,
    /// Sprite sheets by sprite key (`"drive"`, `"screw_tl"`, …).
    pub sheets: BTreeMap<String, RgbaImage>,
}

impl DecodedAssets {
    /// Decode all embedded images. The embedded files are validated by the tests, so a failure
    /// here is a build defect.
    pub fn decode() -> Self {
        let manifest = SpriteManifest::embedded();
        let panel = decode_png(PANEL_PNG).expect("embedded panel.png decodes (checked by tests)");
        let sheets = manifest
            .sprites
            .iter()
            .map(|(key, sheet)| {
                let bytes = sheet_png(&sheet.file)
                    .unwrap_or_else(|| panic!("{} is not embedded", sheet.file));
                let image = decode_png(bytes)
                    .unwrap_or_else(|e| panic!("embedded {} decodes: {e}", sheet.file));
                (key.clone(), image)
            })
            .collect();
        Self {
            manifest,
            panel,
            sheets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::WINDOW_SIZE;

    #[test]
    fn panel_is_the_window_at_asset_scale() {
        let manifest = SpriteManifest::embedded();
        let panel = decode_png(PANEL_PNG).unwrap();
        let scale = manifest.scale as u32;
        assert_eq!(
            (panel.width, panel.height),
            (WINDOW_SIZE.0 * scale, WINDOW_SIZE.1 * scale)
        );
        // Opaque: nothing must show through the faceplate.
        assert!(panel.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }

    #[test]
    fn every_sheet_is_embedded_and_matches_its_grid() {
        let assets = DecodedAssets::decode();
        for (key, sheet) in &assets.manifest.sprites {
            let image = &assets.sheets[key];
            assert_eq!(
                (image.width, image.height),
                (
                    sheet.frame_width * sheet.columns,
                    sheet.frame_height * sheet.rows
                ),
                "{key}: image size vs frame grid"
            );
        }
        assert_eq!(assets.sheets.len(), SHEET_FILES.len());
    }
}
