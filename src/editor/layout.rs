//! The GUI layout table from CLAUDE.md, in logical px (origin: top-left of the 600×360 window).

use nih_plug_egui::egui::{pos2, Color32, Pos2};

/// Readout font size (em), logical px.
pub const READOUT_EM_PX: f32 = 12.0;

const INK: Color32 = Color32::from_rgb(0x29, 0x28, 0x24);
const CREAM: Color32 = Color32::from_rgb(0xd5, 0xcb, 0xb1);

/// A control: interactive circle, sprite and value readout.
#[derive(Debug, Clone, Copy)]
pub struct ControlLayout {
    /// Parameter ID it controls.
    pub param_id: &'static str,
    pub center: Pos2,
    /// Diameter of the interactive circle.
    pub diameter: f32,
    /// Sprite key in `sprites.json`.
    pub sprite: &'static str,
    pub readout: Pos2,
    pub readout_color: Color32,
}

pub const DRIVE: ControlLayout = ControlLayout {
    param_id: "drive",
    center: pos2(380.0, 176.0),
    diameter: 112.0,
    sprite: "drive",
    readout: pos2(380.0, 287.0),
    readout_color: INK,
};

pub const DRY_WET: ControlLayout = ControlLayout {
    param_id: "dry_wet",
    center: pos2(240.0, 218.0),
    diameter: 52.0,
    sprite: "knob_small",
    readout: pos2(240.0, 287.0),
    readout_color: INK,
};

pub const OUTPUT: ControlLayout = ControlLayout {
    param_id: "output",
    center: pos2(520.0, 218.0),
    diameter: 52.0,
    sprite: "knob_small",
    readout: pos2(520.0, 287.0),
    readout_color: INK,
};

pub const CURVE: ControlLayout = ControlLayout {
    param_id: "curve",
    center: pos2(88.0, 240.0),
    diameter: 40.0,
    sprite: "curve",
    readout: pos2(88.0, 305.0),
    readout_color: CREAM,
};

/// All controls, in a fixed order shared with the editor's views.
pub const CONTROLS: [ControlLayout; 4] = [DRIVE, DRY_WET, OUTPUT, CURVE];

/// A decorative screw.
#[derive(Debug, Clone, Copy)]
pub struct ScrewLayout {
    pub center: Pos2,
    /// From the layout table; screws are not interactive, so only the tests read it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub diameter: f32,
    pub sprite: &'static str,
}

pub const SCREWS: [ScrewLayout; 4] = [
    ScrewLayout {
        center: pos2(20.0, 20.0),
        diameter: 9.0,
        sprite: "screw_tl",
    },
    ScrewLayout {
        center: pos2(580.0, 20.0),
        diameter: 9.0,
        sprite: "screw_tr",
    },
    ScrewLayout {
        center: pos2(20.0, 340.0),
        diameter: 9.0,
        sprite: "screw_bl",
    },
    ScrewLayout {
        center: pos2(580.0, 340.0),
        diameter: 9.0,
        sprite: "screw_br",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::sprites::SpriteManifest;

    #[test]
    fn controls_use_sprites_meant_for_them() {
        let m = SpriteManifest::embedded();
        for c in CONTROLS {
            let sheet = m.sheet(c.sprite);
            assert!(
                sheet.used_by.iter().any(|id| id == c.param_id),
                "{} is not listed in {}.used_by",
                c.param_id,
                c.sprite
            );
        }
    }

    #[test]
    fn sprites_cover_their_element() {
        let m = SpriteManifest::embedded();
        let elements = CONTROLS
            .iter()
            .map(|c| (c.sprite, c.diameter))
            .chain(SCREWS.iter().map(|s| (s.sprite, s.diameter)));
        for (sprite, diameter) in elements {
            let size = m.sheet(sprite).display_size(m.scale);
            assert!(size.x >= diameter && size.y >= diameter, "{sprite}");
        }
    }

    #[test]
    fn every_element_is_inside_the_window() {
        let (w, h) = crate::editor::WINDOW_SIZE;
        let window =
            nih_plug_egui::egui::Rect::from_min_size(Pos2::ZERO, (w as f32, h as f32).into());
        for p in CONTROLS
            .iter()
            .flat_map(|c| [c.center, c.readout])
            .chain(SCREWS.iter().map(|s| s.center))
        {
            assert!(window.contains(p), "{p:?}");
        }
    }
}
