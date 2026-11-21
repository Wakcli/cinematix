//! BBS mod's GUI icon atlas, copied under its MIT licence so Cinematix's editor speaks the same
//! visual language as the film studio it is modelled on. The sheet is a 16x16 grid of 16px
//! glyphs; `docs/cinematix/ATTRIBUTION.md` and the licence beside the file record the source.

/// The sheet's name in the HUD atlas.
pub const SPRITE: &str = "horizon:gui/cinematix/icons";
/// One glyph's size in the sheet.
pub const GRID: u32 = 16;
const PNG: &[u8] =
    include_bytes!("../../horizon-client/assets/horizon/textures/gui/cinematix/icons.png");

/// The sheet as the HUD atlas takes an image.
pub fn image() -> std::iter::Once<(String, u32, u32, Vec<u8>)> {
    let (width, height, rgba) = crate::hud::decode_bytes(PNG).expect("BBS icons are PNG");
    std::iter::once((SPRITE.to_owned(), width, height, rgba))
}
