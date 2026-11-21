//! Glyphs from BBS mod's 16x16 icon sheet (`cinematix_icons::SPRITE`), by sheet cell. The
//! coordinates mirror `mchorse.bbs_mod.ui.utils.icons.Icons` so a reader can check a glyph
//! against the reference.

use horizon_client_render::cinematix_icons::{GRID, SPRITE};
use horizon_client_render::hud::{HudAtlas, HudBatch};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    pub x: u16,
    pub y: u16,
}

impl Icon {
    const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    /// Draws the glyph in a `size`-pixel square with its top-left at `(x, y)`.
    pub fn draw(
        self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        x: f32,
        y: f32,
        size: f32,
        tint: u32,
    ) {
        let Some(sheet) = atlas.sprites.get(SPRITE) else {
            return;
        };
        let n = sheet.w as f32;
        let du = (sheet.u1 - sheet.u0) * GRID as f32 / n;
        let dv = (sheet.v1 - sheet.v0) * GRID as f32 / n;
        let u0 = sheet.u0 + (sheet.u1 - sheet.u0) * self.x as f32 / n;
        let v0 = sheet.v0 + (sheet.v1 - sheet.v0) * self.y as f32 / n;
        batch.quad(
            [[x, y], [x + size, y], [x + size, y + size], [x, y + size]],
            [[u0, v0], [u0 + du, v0], [u0 + du, v0 + dv], [u0, v0 + dv]],
            tint,
        );
    }
}

pub const CAMERA: Icon = Icon::new(96, 112);
pub const USER: Icon = Icon::new(64, 128);
pub const SHAPES: Icon = Icon::new(64, 144);
pub const LIGHT: Icon = Icon::new(144, 112);
pub const BRICKS: Icon = Icon::new(32, 96);
pub const FILM: Icon = Icon::new(0, 96);
pub const VIDEO_CAMERA: Icon = Icon::new(160, 128);
pub const SETTINGS: Icon = Icon::new(160, 96);
pub const PLAY: Icon = Icon::new(48, 32);
pub const PAUSE: Icon = Icon::new(64, 32);
pub const FRAME_NEXT: Icon = Icon::new(80, 64);
pub const FRAME_PREV: Icon = Icon::new(96, 64);
pub const ADD: Icon = Icon::new(64, 0);
pub const REMOVE: Icon = Icon::new(96, 0);
pub const MAXIMIZE: Icon = Icon::new(80, 32);
pub const CURVES: Icon = Icon::new(192, 64);
pub const LIST: Icon = Icon::new(144, 96);
pub const UNDO: Icon = Icon::new(224, 48);
pub const REDO: Icon = Icon::new(208, 48);
pub const SAVE: Icon = Icon::new(48, 0);
