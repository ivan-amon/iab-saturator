//! Offscreen preview of the editor (test-only, ignored by default).
//!
//! Runs the editor's own `paint()` in a headless egui context, tessellates the output and
//! rasterizes the triangles in software, the way egui's OpenGL painter does (premultiplied
//! alpha, blending in gamma space). Writes PNGs for visual review:
//!
//! ```text
//! IAB_PREVIEW_DIR=<dir> cargo test --lib render_gui_preview -- --ignored
//! ```
//! (default directory: `target/gui_preview`).

use super::assets::DecodedAssets;
use super::fonts;
use super::paint::{paint, ControlView, Gfx};
use super::WINDOW_SIZE;
use crate::{CurveType, IabSaturatorParams};
use nih_plug::prelude::Param;
use nih_plug_egui::egui::{
    self, epaint::Primitive, pos2, vec2, ClippedPrimitive, Color32, Context, ImageData, LayerId,
    Rect, TextureFilter, TextureId, TexturesDelta, ViewportId, ViewportInfo,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

struct Texture {
    width: usize,
    height: usize,
    /// Premultiplied, gamma-space (egui's texel format).
    pixels: Vec<Color32>,
    linear: bool,
}

#[derive(Default)]
struct TextureStore(HashMap<TextureId, Texture>);

impl TextureStore {
    fn apply(&mut self, delta: &TexturesDelta) {
        for (id, d) in &delta.set {
            let (width, height, pixels): (usize, usize, Vec<Color32>) = match &d.image {
                ImageData::Color(c) => (c.size[0], c.size[1], c.pixels.clone()),
                ImageData::Font(f) => (f.size[0], f.size[1], f.srgba_pixels(None).collect()),
            };
            let linear = d.options.magnification == TextureFilter::Linear;
            match d.pos {
                None => {
                    self.0.insert(
                        *id,
                        Texture {
                            width,
                            height,
                            pixels,
                            linear,
                        },
                    );
                }
                Some([x0, y0]) => {
                    let t = self
                        .0
                        .get_mut(id)
                        .expect("partial update of a known texture");
                    for y in 0..height {
                        for x in 0..width {
                            t.pixels[(y0 + y) * t.width + x0 + x] = pixels[y * width + x];
                        }
                    }
                }
            }
        }
    }
}

fn texel(t: &Texture, x: i64, y: i64) -> [f32; 4] {
    let x = x.clamp(0, t.width as i64 - 1) as usize;
    let y = y.clamp(0, t.height as i64 - 1) as usize;
    t.pixels[y * t.width + x]
        .to_array()
        .map(|c| c as f32 / 255.0)
}

fn sample(t: &Texture, u: f32, v: f32) -> [f32; 4] {
    let (x, y) = (u * t.width as f32 - 0.5, v * t.height as f32 - 0.5);
    if !t.linear {
        return texel(t, x.round() as i64, y.round() as i64);
    }
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let [a, b, c, d] = [
        texel(t, x0, y0),
        texel(t, x0 + 1, y0),
        texel(t, x0, y0 + 1),
        texel(t, x0 + 1, y0 + 1),
    ];
    std::array::from_fn(|i| {
        let top = a[i] + (b[i] - a[i]) * fx;
        let bottom = c[i] + (d[i] - c[i]) * fx;
        top + (bottom - top) * fy
    })
}

/// Edge function: > 0 when `p` is on the inner side of edge a→b (for area > 0 triangles).
fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

/// Tie-break for pixel centers exactly on an edge, so triangles sharing an edge (the quads'
/// diagonals) cover each pixel once. Reversing the edge flips the result.
fn owns_edge(a: [f32; 2], b: [f32; 2]) -> bool {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    dy < 0.0 || (dy == 0.0 && dx > 0.0)
}

fn rasterize(
    primitives: &[ClippedPrimitive],
    ppp: f32,
    textures: &TextureStore,
    (width, height): (usize, usize),
) -> Vec<[f32; 4]> {
    let mut fb = vec![[0.0, 0.0, 0.0, 1.0]; width * height];
    for prim in primitives {
        let Primitive::Mesh(mesh) = &prim.primitive else {
            continue;
        };
        let tex = &textures.0[&mesh.texture_id];
        let clip = prim.clip_rect;
        let (cx0, cy0) = (
            (clip.min.x * ppp).max(0.0) as usize,
            (clip.min.y * ppp).max(0.0) as usize,
        );
        let cx1 = ((clip.max.x * ppp).ceil() as usize).min(width);
        let cy1 = ((clip.max.y * ppp).ceil() as usize).min(height);

        for tri in mesh.indices.as_chunks::<3>().0 {
            let mut v = tri.map(|i| &mesh.vertices[i as usize]);
            let mut p = v.map(|v| [v.pos.x * ppp, v.pos.y * ppp]);
            let mut area = edge(p[0], p[1], p[2]);
            if area == 0.0 {
                continue;
            }
            if area < 0.0 {
                v.swap(1, 2);
                p.swap(1, 2);
                area = -area;
            }
            let min_x = p
                .iter()
                .map(|q| q[0])
                .fold(f32::MAX, f32::min)
                .floor()
                .max(cx0 as f32) as usize;
            let min_y = p
                .iter()
                .map(|q| q[1])
                .fold(f32::MAX, f32::min)
                .floor()
                .max(cy0 as f32) as usize;
            let max_x = (p.iter().map(|q| q[0]).fold(f32::MIN, f32::max).ceil() as usize).min(cx1);
            let max_y = (p.iter().map(|q| q[1]).fold(f32::MIN, f32::max).ceil() as usize).min(cy1);
            let edges = [(p[1], p[2]), (p[2], p[0]), (p[0], p[1])];

            for y in min_y..max_y {
                for x in min_x..max_x {
                    let c = [x as f32 + 0.5, y as f32 + 0.5];
                    let w = edges.map(|(a, b)| edge(a, b, c));
                    let inside = w
                        .iter()
                        .zip(edges)
                        .all(|(&w, (a, b))| w > 0.0 || (w == 0.0 && owns_edge(a, b)));
                    if !inside {
                        continue;
                    }
                    let [l0, l1, l2] = w.map(|w| w / area);
                    let u = l0 * v[0].uv.x + l1 * v[1].uv.x + l2 * v[2].uv.x;
                    let t = l0 * v[0].uv.y + l1 * v[1].uv.y + l2 * v[2].uv.y;
                    let col = [0, 1, 2].map(|i| v[i].color.to_array().map(|c| c as f32 / 255.0));
                    let tint: [f32; 4] =
                        std::array::from_fn(|i| l0 * col[0][i] + l1 * col[1][i] + l2 * col[2][i]);
                    let s = sample(tex, u, t);
                    let src: [f32; 4] = std::array::from_fn(|i| s[i] * tint[i]);
                    let dst = &mut fb[y * width + x];
                    for i in 0..4 {
                        dst[i] = src[i] + dst[i] * (1.0 - src[3]);
                    }
                }
            }
        }
    }
    fb
}

fn write_png(path: &Path, fb: &[[f32; 4]], (width, height): (usize, usize)) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let bytes: Vec<u8> = fb
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect();
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&bytes)
        .unwrap();
}

fn raw_input(ppp: f32) -> egui::RawInput {
    let mut input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            pos2(0.0, 0.0),
            vec2(WINDOW_SIZE.0 as f32, WINDOW_SIZE.1 as f32),
        )),
        max_texture_side: Some(16384),
        ..Default::default()
    };
    input.viewports.insert(
        ViewportId::ROOT,
        ViewportInfo {
            native_pixels_per_point: Some(ppp),
            ..Default::default()
        },
    );
    input
}

/// Render "H" through egui at `font` and return its ink height in px (dark-on-black threshold).
fn rendered_cap_height(font: egui::FontId) -> usize {
    let ctx = Context::default();
    ctx.set_fonts(fonts::font_definitions());
    let mut store = TextureStore::default();
    let mut out = None;
    // Two passes: fonts set before the first pass become active in it; keep the last output.
    for _ in 0..2 {
        let o = ctx.run(raw_input(1.0), |ctx| {
            ctx.layer_painter(LayerId::background()).text(
                pos2(50.0, 50.0),
                egui::Align2::LEFT_TOP,
                "H",
                font.clone(),
                Color32::WHITE,
            );
        });
        store.apply(&o.textures_delta);
        out = Some(o);
    }
    let out = out.unwrap();
    let size = (WINDOW_SIZE.0 as usize, WINDOW_SIZE.1 as usize);
    let fb = rasterize(&ctx.tessellate(out.shapes, 1.0), 1.0, &store, size);
    let rows: Vec<usize> = (0..size.1)
        .filter(|&y| (0..size.0).any(|x| fb[y * size.0 + x][0] > 0.5))
        .collect();
    rows.last().unwrap() - rows.first().unwrap() + 1
}

#[test]
fn readout_text_is_sized_by_em() {
    use ab_glyph::{Font, FontRef};
    let em = 100.0; // large, so pixel rounding is negligible
    let font = FontRef::try_from_slice(fonts::BARLOW_CONDENSED_MEDIUM).unwrap();
    // ab_glyph scales by line height; convert so that one em = `em` px.
    let px_scale = em * font.height_unscaled() / font.units_per_em().unwrap();
    let outline = font
        .outline_glyph(font.glyph_id('H').with_scale(px_scale))
        .unwrap();
    let expected = outline.px_bounds().height();
    let measured = rendered_cap_height(fonts::readout_font(em)) as f32;
    assert!(
        (measured - expected).abs() <= 2.0,
        "cap height {measured} px, expected {expected:.1} px for a {em} px em"
    );
}

#[test]
#[ignore = "writes preview PNGs; run explicitly"]
fn render_gui_preview() {
    let dir = std::env::var_os("IAB_PREVIEW_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gui_preview"));
    std::fs::create_dir_all(&dir).unwrap();

    let p = IabSaturatorParams::default();
    let views = |drive: f32, dry_wet: f32, output: f32, curve: CurveType| {
        [
            ControlView::of(&p.drive, p.drive.preview_normalized(drive)),
            ControlView::of(&p.dry_wet, p.dry_wet.preview_normalized(dry_wet)),
            ControlView::of(&p.output, p.output.preview_normalized(output)),
            ControlView::of(&p.curve, p.curve.preview_normalized(curve)),
        ]
    };
    let states = [
        ("defaults", 2.0, views(0.0, 100.0, 0.0, CurveType::Tanh)),
        (
            "mockup_values",
            2.0,
            views(15.0, 75.0, -6.0, CurveType::Curve3),
        ),
        (
            "opposite_ends",
            2.0,
            views(36.0, 0.0, -36.0, CurveType::Curve4),
        ),
        (
            "mockup_values_1x",
            1.0,
            views(15.0, 75.0, -6.0, CurveType::Curve3),
        ),
    ];

    let assets = DecodedAssets::decode();
    let ctx = Context::default();
    ctx.set_fonts(fonts::font_definitions());
    let mut store = TextureStore::default();
    let mut gfx = None;
    let out = ctx.run(raw_input(2.0), |ctx| gfx = Some(Gfx::upload(ctx, &assets)));
    store.apply(&out.textures_delta);
    let gfx = gfx.unwrap();

    for (name, ppp, views) in states {
        let out = ctx.run(raw_input(ppp), |ctx| {
            paint(&ctx.layer_painter(LayerId::background()), &gfx, &views);
        });
        store.apply(&out.textures_delta);
        let primitives = ctx.tessellate(out.shapes, out.pixels_per_point);
        let size = (
            (WINDOW_SIZE.0 as f32 * ppp) as usize,
            (WINDOW_SIZE.1 as f32 * ppp) as usize,
        );
        let fb = rasterize(&primitives, out.pixels_per_point, &store, size);
        let path = dir.join(format!("{name}.png"));
        write_png(&path, &fb, size);
        println!("wrote {}", path.display());
    }
}
