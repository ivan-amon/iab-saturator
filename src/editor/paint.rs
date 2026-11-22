//! Textures and drawing, in the `draw_order` of layout.json. Reads parameter values, never
//! writes them.

use super::assets::{self, CURVE_COUNT, OVERSAMPLING_COUNT};
use super::fonts;
use super::geometry::{
    centered_rect, index_from_normalized, knob_angle_deg, sprite_size, text_top_left,
};
use super::layout::{Layout, Readout};
use crate::params::IabSaturatorParams;
use egui::emath::Rot2;
use egui::epaint::{Mesh, Shape};
use egui::{
    Color32, Context, Painter, Pos2, Rect, TextureFilter, TextureHandle, TextureOptions, Vec2, pos2,
};
use nice_plug::prelude::Param;

/// Readout text size as a fraction of its pill height. layout.json gives the pill but not the
/// text size; move it there (`readout.text_size`) when the generator emits it.
const READOUT_TEXT_TO_PILL_HEIGHT: f32 = 0.5;

/// Sprites are drawn at half size (2x assets), so sample them with mipmaps.
const TEXTURE_OPTIONS: TextureOptions = TextureOptions {
    mipmap_mode: Some(TextureFilter::Linear),
    ..TextureOptions::LINEAR
};

const FULL_UV: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

/// A texture and its size at 1x.
pub struct Sprite {
    texture: TextureHandle,
    size: Vec2,
}

/// All GUI textures, uploaded once per editor opening. Per-element vectors follow the order of
/// their layout.json arrays.
pub struct Textures {
    bg: Sprite,
    needle: Sprite,
    /// `(body, pointer)` per `knobs` entry.
    knobs: Vec<(Sprite, Sprite)>,
    curves: Vec<Sprite>,
    curve_leds_on: Vec<Sprite>,
    curve_labels_on: Vec<Sprite>,
    /// Glass per `meters` entry.
    meter_glass: Vec<Sprite>,
    oversampling_on: Vec<Sprite>,
    /// Lit LED per `bottom_bar.toggles` entry.
    toggle_leds_on: Vec<Sprite>,
}

impl Textures {
    /// Decodes every PNG and uploads it. Fails if an asset is missing or does not decode
    /// (the asset tests guarantee neither happens).
    pub fn load(ctx: &Context, layout: &Layout) -> Result<Self, String> {
        let scale = layout.meta.render_scale;
        let load = |name: &str, bytes: &[u8]| -> Result<Sprite, String> {
            let image = assets::decode_png(bytes).map_err(|e| format!("{name}: {e}"))?;
            let size = sprite_size(image.size, scale);
            Ok(Sprite {
                texture: ctx.load_texture(name, image, TEXTURE_OPTIONS),
                size,
            })
        };
        let missing = |what: &str| format!("no asset for {what}");

        let knobs = layout
            .knobs
            .iter()
            .map(|k| {
                let (body, pointer) = assets::knob(&k.id).ok_or_else(|| missing(&k.id))?;
                Ok((load(&k.id, body)?, load(&k.id, pointer)?))
            })
            .collect::<Result<_, String>>()?;
        let meter_glass = layout
            .meters
            .iter()
            .map(|m| {
                load(
                    &m.id,
                    assets::meter_glass(&m.id).ok_or_else(|| missing(&m.id))?,
                )
            })
            .collect::<Result<_, String>>()?;
        let toggle_leds_on = layout
            .bottom_bar
            .toggles
            .iter()
            .map(|t| {
                load(
                    &t.id,
                    assets::toggle_led_on(&t.id).ok_or_else(|| missing(&t.id))?,
                )
            })
            .collect::<Result<_, String>>()?;
        let all = |name: &str, list: &[&[u8]]| -> Result<Vec<Sprite>, String> {
            list.iter().map(|bytes| load(name, bytes)).collect()
        };

        Ok(Self {
            bg: load("bg", assets::BG)?,
            needle: load("needle", assets::NEEDLE)?,
            knobs,
            curves: all("curve", &assets::CURVES)?,
            curve_leds_on: all("curve_led", &assets::CURVE_LEDS_ON)?,
            curve_labels_on: all("curve_label", &assets::CURVE_LABELS_ON)?,
            meter_glass,
            oversampling_on: all("oversampling", &assets::OVERSAMPLING_ON)?,
            toggle_leds_on,
        })
    }
}

/// Draws the whole panel for the current parameter values.
pub fn paint(painter: &Painter, layout: &Layout, textures: &Textures, params: &IabSaturatorParams) {
    let curve = index_from_normalized(params.curve.unmodulated_normalized_value(), CURVE_COUNT);
    let oversampling = index_from_normalized(
        params.oversampling.unmodulated_normalized_value(),
        OVERSAMPLING_COUNT,
    );

    // 1. Static art.
    let panel = layout.meta.panel;
    draw(
        painter,
        &textures.bg,
        Rect::from_min_size(Pos2::ZERO, Vec2::new(panel.w, panel.h)),
    );

    // 2. Knob bodies and the selector at its position.
    for (k, (body, _)) in layout.knobs.iter().zip(&textures.knobs) {
        draw_centred(painter, body, pos2(k.cx, k.cy));
    }
    let sel = &layout.selector;
    if let Some(sprite) = textures.curves.get(curve) {
        draw_centred(painter, sprite, pos2(sel.cx, sel.cy));
    }

    // 3. Knob pointers, rotated.
    for (k, (_, pointer)) in layout.knobs.iter().zip(&textures.knobs) {
        if let Some(param) = float_param(params, &k.param) {
            let angle = knob_angle_deg(
                k.sweep.from,
                k.sweep.to,
                param.unmodulated_normalized_value(),
            );
            draw_rotated(painter, pointer, pos2(k.cx, k.cy), angle);
        }
    }

    // 4. Needles, clipped to the meter face. At rest until the meters have a detector.
    for m in &layout.meters {
        let face = Rect::from_min_size(pos2(m.face.x, m.face.y), Vec2::new(m.face.w, m.face.h));
        let clipped = painter.with_clip_rect(face);
        draw_rotated(
            &clipped,
            &textures.needle,
            pos2(m.pivot.x, m.pivot.y),
            layout.meter_scale.rest_angle,
        );
    }

    // 5. Meter glass, above the needles.
    for (m, glass) in layout.meters.iter().zip(&textures.meter_glass) {
        let h = m.housing;
        draw(
            painter,
            glass,
            Rect::from_min_size(pos2(h.x, h.y), Vec2::new(h.w, h.h)),
        );
    }

    // 6. Lit LEDs, segments and labels.
    for (t, led) in layout
        .bottom_bar
        .toggles
        .iter()
        .zip(&textures.toggle_leds_on)
    {
        if bool_param(params, &t.param).is_some_and(|p| p.value()) {
            draw_centred(painter, led, pos2(t.led.cx, t.led.cy));
        }
    }
    let os = &layout.bottom_bar.oversampling;
    if let (Some(seg), Some(sprite)) = (
        os.segments.get(oversampling),
        textures.oversampling_on.get(oversampling),
    ) {
        draw(
            painter,
            sprite,
            segment_rect(os.bounds.y, os.bounds.h, seg.x, seg.w),
        );
    }
    if let Some(p) = sel.positions.get(curve) {
        if let Some(led) = textures.curve_leds_on.get(curve) {
            draw_centred(painter, led, pos2(p.led.cx, p.led.cy));
        }
        if let Some(label) = textures.curve_labels_on.get(curve) {
            draw_centred(painter, label, pos2(p.label_sprite.cx, p.label_sprite.cy));
        }
    }

    // 7. Readouts: live text, each parameter's own formatter (what the host shows).
    for k in &layout.knobs {
        if let Some(param) = float_param(params, &k.param) {
            readout(painter, layout, &k.readout, param_text(param));
        }
    }
    readout(painter, layout, &sel.readout, param_text(&params.curve));
}

/// The float parameter a knob is bound to.
pub fn float_param<'a>(
    params: &'a IabSaturatorParams,
    id: &str,
) -> Option<&'a nice_plug::prelude::FloatParam> {
    match id {
        "drive" => Some(&params.drive),
        "output" => Some(&params.output),
        "dry_wet" => Some(&params.dry_wet),
        _ => None,
    }
}

/// The bool parameter a toggle is bound to.
pub fn bool_param<'a>(
    params: &'a IabSaturatorParams,
    id: &str,
) -> Option<&'a nice_plug::prelude::BoolParam> {
    match id {
        "auto_gain" => Some(&params.auto_gain),
        _ => None,
    }
}

/// Rectangle of an oversampling segment: its own x and w, the control's y and h.
pub fn segment_rect(y: f32, h: f32, x: f32, w: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), Vec2::new(w, h))
}

fn param_text<P: Param>(param: &P) -> String {
    param.normalized_value_to_string(param.unmodulated_normalized_value(), true)
}

fn draw(painter: &Painter, sprite: &Sprite, rect: Rect) {
    painter.image(sprite.texture.id(), rect, FULL_UV, Color32::WHITE);
}

fn draw_centred(painter: &Painter, sprite: &Sprite, centre: Pos2) {
    draw(
        painter,
        sprite,
        centered_rect(centre.x, centre.y, sprite.size.x, sprite.size.y),
    );
}

/// Draws a sprite centred on `centre`, rotated about it by `angle_deg` (clockwise).
fn draw_rotated(painter: &Painter, sprite: &Sprite, centre: Pos2, angle_deg: f32) {
    let rect = centered_rect(centre.x, centre.y, sprite.size.x, sprite.size.y);
    let mut mesh = Mesh::with_texture(sprite.texture.id());
    mesh.add_rect_with_uv(rect, FULL_UV, Color32::WHITE);
    // Screen y grows down, so a positive angle turns clockwise, as in layout.json.
    mesh.rotate(Rot2::from_angle(angle_deg.to_radians()), centre);
    painter.add(Shape::mesh(mesh));
}

/// Text centred on the pill: horizontally on `cx`, cap height on `cy`.
fn readout(painter: &Painter, layout: &Layout, r: &Readout, text: String) {
    let size = r.h * READOUT_TEXT_TO_PILL_HEIGHT;
    let color = layout.color(&r.text_color);
    let galley = painter.layout_no_wrap(text, fonts::readout(size), color);
    let baseline = galley
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first().map(|g| row.pos.y + g.pos.y))
        .unwrap_or(0.0);
    let cap_height = layout.fonts.cap_height_em * size;
    let pos = text_top_left(r.cx, r.cy, galley.size().x, baseline, cap_height);
    painter.galley(pos, galley, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{FullOutput, RawInput, Ui};

    fn run_frame(ctx: &Context, layout: &Layout, mut draw: impl FnMut(&mut Ui)) -> FullOutput {
        let panel = layout.meta.panel;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(panel.w, panel.h))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| draw(ui));
        // No GPU in tests: drop the texture uploads on purpose.
        out.textures_delta.clear();
        out
    }

    /// A context with the readout font installed (fonts take effect on the next pass).
    fn context(layout: &Layout) -> Context {
        let ctx = Context::default();
        fonts::install(&ctx);
        run_frame(&ctx, layout, |_| {});
        ctx
    }

    #[test]
    fn paints_a_full_frame() {
        let layout = Layout::load().unwrap();
        let ctx = context(&layout);
        let textures = Textures::load(&ctx, &layout).unwrap();
        let params = IabSaturatorParams::default();
        let out = run_frame(&ctx, &layout, |ui| {
            paint(ui.painter(), &layout, &textures, &params)
        });

        // bg, 3 knob bodies, selector, 3 pointers, 2 needles, 2 glasses, oversampling segment,
        // curve LED, curve label, 4 readouts. The auto-gain LED is off by default.
        assert_eq!(out.shapes.len(), 19);
    }

    #[test]
    fn readout_font_has_every_character() {
        let layout = Layout::load().unwrap();
        let ctx = context(&layout);
        let font = fonts::readout(11.0);
        let text = "+\u{2212}0123456789. dB % SOFT CLIP HARD WARM TUBE FOLD RECTIFY BIT CRUSH";
        assert!(ctx.fonts_mut(|f| f.has_glyphs(&font, text)));
    }
}
