//! Drawing the panel from parameter values. Shared by the editor and the preview test, so the
//! preview shows exactly what the editor draws.

use super::assets::{DecodedAssets, RgbaImage};
use super::fonts;
use super::layout::{CONTROLS, READOUT_EM_PX, SCREWS};
use super::sprites::{frame_index, SpriteSheet};
use nih_plug::prelude::Param;
use nih_plug_egui::egui::{
    pos2, Align2, Color32, ColorImage, Context, FontId, Painter, Pos2, Rect, TextureHandle,
    TextureOptions,
};
use std::collections::BTreeMap;

/// A sprite sheet uploaded to the GPU.
pub struct Sprite {
    pub texture: TextureHandle,
    pub sheet: SpriteSheet,
}

impl Sprite {
    /// Draw frame `k` centered on `center`, at the sheet's logical size.
    fn draw(&self, painter: &Painter, center: Pos2, k: u32, scale: f32) {
        let rect = Rect::from_center_size(center, self.sheet.display_size(scale));
        painter.image(
            self.texture.id(),
            rect,
            self.sheet.uv_rect(k),
            Color32::WHITE,
        );
    }
}

/// Every texture and font the editor draws with. Created once per editor opening.
pub struct Gfx {
    /// Asset px per logical px.
    scale: f32,
    panel: TextureHandle,
    panel_size: (u32, u32),
    /// In `layout::CONTROLS` order.
    controls: [Sprite; 4],
    /// In `layout::SCREWS` order.
    screws: [Sprite; 4],
    readout_font: FontId,
}

fn upload(ctx: &Context, name: &str, image: &RgbaImage) -> TextureHandle {
    let size = [image.width as usize, image.height as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, &image.pixels);
    ctx.load_texture(name, color, TextureOptions::LINEAR)
}

impl Gfx {
    /// Upload the decoded images. Must run inside an egui pass, after the backend has reported
    /// its maximum texture size.
    pub fn upload(ctx: &Context, assets: &DecodedAssets) -> Self {
        // Each sheet is uploaded once, even when shared (DRY/WET and OUTPUT).
        let mut textures: BTreeMap<&str, TextureHandle> = BTreeMap::new();
        let mut sprite = |key: &'static str| Sprite {
            texture: textures
                .entry(key)
                .or_insert_with(|| upload(ctx, key, &assets.sheets[key]))
                .clone(),
            sheet: assets.manifest.sheet(key).clone(),
        };
        Self {
            scale: assets.manifest.scale,
            panel: upload(ctx, "panel", &assets.panel),
            panel_size: (assets.panel.width, assets.panel.height),
            controls: CONTROLS.map(|c| sprite(c.sprite)),
            screws: SCREWS.map(|s| sprite(s.sprite)),
            readout_font: fonts::readout_font(READOUT_EM_PX),
        }
    }

    /// Largest texture side this set needs (must fit the GPU's limit).
    pub fn largest_side(assets: &DecodedAssets) -> u32 {
        assets
            .sheets
            .values()
            .chain(std::iter::once(&assets.panel))
            .map(|i| i.width.max(i.height))
            .max()
            .unwrap_or(0)
    }
}

/// What one control shows: its normalized value and its formatted readout.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlView {
    pub normalized: f32,
    pub readout: String,
}

impl ControlView {
    /// The view of `param` at `normalized`, formatted by the parameter's own formatter (the
    /// same text the host shows).
    pub fn of<P: Param>(param: &P, normalized: f32) -> Self {
        Self {
            normalized,
            readout: param.normalized_value_to_string(normalized, true),
        }
    }

    pub fn current<P: Param>(param: &P) -> Self {
        Self::of(param, param.unmodulated_normalized_value())
    }
}

/// Paint the whole editor: panel, screws, knobs, readouts. `views` is in `layout::CONTROLS`
/// order.
pub fn paint(painter: &Painter, gfx: &Gfx, views: &[ControlView; 4]) {
    let panel_rect = Rect::from_min_max(
        Pos2::ZERO,
        pos2(gfx.panel_size.0 as f32, gfx.panel_size.1 as f32) / gfx.scale,
    );
    let full_uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(gfx.panel.id(), panel_rect, full_uv, Color32::WHITE);

    for (screw, layout) in gfx.screws.iter().zip(SCREWS) {
        screw.draw(painter, layout.center, 0, gfx.scale);
    }

    for ((sprite, layout), view) in gfx.controls.iter().zip(CONTROLS).zip(views) {
        let k = frame_index(view.normalized, sprite.sheet.frame_count);
        sprite.draw(painter, layout.center, k, gfx.scale);
    }

    for (layout, view) in CONTROLS.iter().zip(views) {
        painter.text(
            layout.readout,
            Align2::CENTER_CENTER,
            &view.readout,
            gfx.readout_font.clone(),
            layout.readout_color,
        );
    }
}
