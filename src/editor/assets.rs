//! GUI images, embedded in the plugin binary and decoded when the editor opens.
//! File names follow the `assets` block of layout.json.

use egui::ColorImage;

macro_rules! gui_png {
    ($file:literal) => {
        include_bytes!(concat!("../../assets/gui/", $file))
    };
}

/// Number of CURVE positions (layout.json `selector.positions`, the `CurveType` enum).
pub const CURVE_COUNT: usize = 7;
/// Number of oversampling segments (layout.json `bottom_bar.oversampling.segments`).
pub const OVERSAMPLING_COUNT: usize = 4;

pub const BG: &[u8] = gui_png!("bg.png");
pub const NEEDLE: &[u8] = gui_png!("needle.png");

/// Full selector knob at each position angle.
pub const CURVES: [&[u8]; CURVE_COUNT] = [
    gui_png!("curve_0.png"),
    gui_png!("curve_1.png"),
    gui_png!("curve_2.png"),
    gui_png!("curve_3.png"),
    gui_png!("curve_4.png"),
    gui_png!("curve_5.png"),
    gui_png!("curve_6.png"),
];

/// Lit LED of each selector position.
pub const CURVE_LEDS_ON: [&[u8]; CURVE_COUNT] = [
    gui_png!("curve_led_0_on.png"),
    gui_png!("curve_led_1_on.png"),
    gui_png!("curve_led_2_on.png"),
    gui_png!("curve_led_3_on.png"),
    gui_png!("curve_led_4_on.png"),
    gui_png!("curve_led_5_on.png"),
    gui_png!("curve_led_6_on.png"),
];

/// Lit (red) label of each selector position.
pub const CURVE_LABELS_ON: [&[u8]; CURVE_COUNT] = [
    gui_png!("curve_label_0_on.png"),
    gui_png!("curve_label_1_on.png"),
    gui_png!("curve_label_2_on.png"),
    gui_png!("curve_label_3_on.png"),
    gui_png!("curve_label_4_on.png"),
    gui_png!("curve_label_5_on.png"),
    gui_png!("curve_label_6_on.png"),
];

/// Pressed and lit oversampling segment, text included.
pub const OVERSAMPLING_ON: [&[u8]; OVERSAMPLING_COUNT] = [
    gui_png!("os_0_on.png"),
    gui_png!("os_1_on.png"),
    gui_png!("os_2_on.png"),
    gui_png!("os_3_on.png"),
];

/// `(body, pointer)` of the knob with layout.json id `id`.
pub fn knob(id: &str) -> Option<(&'static [u8], &'static [u8])> {
    match id {
        "drive" => Some((
            gui_png!("knob_drive_body.png"),
            gui_png!("knob_drive_pointer.png"),
        )),
        "mix" => Some((
            gui_png!("knob_mix_body.png"),
            gui_png!("knob_mix_pointer.png"),
        )),
        "output" => Some((
            gui_png!("knob_output_body.png"),
            gui_png!("knob_output_pointer.png"),
        )),
        _ => None,
    }
}

/// Glass glare of the meter with layout.json id `id`.
pub fn meter_glass(id: &str) -> Option<&'static [u8]> {
    match id {
        "in" => Some(gui_png!("meter_glass_in.png")),
        "out" => Some(gui_png!("meter_glass_out.png")),
        _ => None,
    }
}

/// Lit LED of the toggle with layout.json id `id`.
pub fn toggle_led_on(id: &str) -> Option<&'static [u8]> {
    match id {
        "auto_gain" => Some(gui_png!("led_auto_gain_on.png")),
        _ => None,
    }
}

/// Decodes an 8-bit RGBA PNG with straight (unpremultiplied) alpha, as rendered by Blender.
pub fn decode_png(bytes: &[u8]) -> Result<ColorImage, png::DecodingError> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // Expand palettes / low bit depths and strip 16-bit to 8-bit RGBA-compatible output.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or(png::DecodingError::LimitsExceeded)?;
    let mut buffer = vec![0; size];
    let info = reader.next_frame(&mut buffer)?;
    let pixels = &buffer[..info.buffer_size()];
    let size = [info.width as usize, info.height as usize];

    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[g, a]| [g, g, g, a])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        // EXPAND turns indexed images into RGB(A), so this is unreachable.
        png::ColorType::Indexed => return Err(png::DecodingError::LimitsExceeded),
    };
    Ok(ColorImage::from_rgba_unmultiplied(size, &rgba))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::layout::{Layout, Size};

    fn size_of(bytes: &[u8]) -> [usize; 2] {
        decode_png(bytes).expect("PNG must decode").size
    }

    /// Asserts the PNG is the layout box at render scale.
    fn assert_box(name: &str, bytes: &[u8], w: f32, h: f32, scale: f32) {
        let expected = [(w * scale).round() as usize, (h * scale).round() as usize];
        assert_eq!(size_of(bytes), expected, "{name}");
    }

    #[test]
    fn decodes_straight_alpha() {
        let image = decode_png(BG).unwrap();
        assert_eq!(image.pixels.len(), image.size[0] * image.size[1]);
        // A fully transparent pixel stays transparent after premultiplying.
        let (_, pointer) = knob("drive").unwrap();
        let corner = decode_png(pointer).unwrap().pixels[0];
        assert_eq!(corner.a(), 0);
    }

    #[test]
    fn sizes_match_the_layout() {
        let l = Layout::load().unwrap();
        let s = l.meta.render_scale;
        let Size { w, h } = l.meta.panel;
        assert_box("bg", BG, w, h, s);

        for k in &l.knobs {
            let (body, pointer) = knob(&k.id).unwrap_or_else(|| panic!("no assets for {}", k.id));
            assert_box(&k.id, body, k.sprite.w, k.sprite.h, s);
            assert_box(&k.id, pointer, k.sprite.w, k.sprite.h, s);
        }

        let sel = &l.selector;
        assert_eq!(sel.positions.len(), CURVE_COUNT);
        for (i, p) in sel.positions.iter().enumerate() {
            assert_box("curve", CURVES[i], sel.sprite.w, sel.sprite.h, s);
            let led = sel.position_leds.sprite;
            assert_box("curve led", CURVE_LEDS_ON[i], led.w, led.h, s);
            assert_box(
                "curve label",
                CURVE_LABELS_ON[i],
                p.label_sprite.w,
                p.label_sprite.h,
                s,
            );
        }

        for m in &l.meters {
            let glass = meter_glass(&m.id).unwrap_or_else(|| panic!("no glass for {}", m.id));
            assert_box(&m.id, glass, m.housing.w, m.housing.h, s);
        }

        let os = &l.bottom_bar.oversampling;
        assert_eq!(os.segments.len(), OVERSAMPLING_COUNT);
        for (i, seg) in os.segments.iter().enumerate() {
            assert_box("os", OVERSAMPLING_ON[i], seg.w, os.bounds.h, s);
        }

        for t in &l.bottom_bar.toggles {
            let led = toggle_led_on(&t.id).unwrap_or_else(|| panic!("no LED for {}", t.id));
            let size = l.bottom_bar.led.sprite;
            assert_box(&t.id, led, size.w, size.h, s);
        }
    }

    #[test]
    fn needle_is_square_and_even() {
        // Rotates about its centre (the pivot), so it must be square.
        let [w, h] = size_of(NEEDLE);
        assert_eq!(w, h);
        assert_eq!(w % 2, 0);
    }
}
