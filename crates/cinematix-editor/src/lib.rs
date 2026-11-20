//! Cinematix's in-game editor: the document, the frame layout and the drawing. The engine-facing
//! glue (the `Screen` variant, input routing and the control socket) lives in `horizon-client`;
//! everything here is testable with `cargo test` and no GPU.

mod document;
mod draw;
mod editor;
mod film;
mod frame;
pub mod icons;

pub use cinematix_anim::{Interp, Prop};
pub use cinematix_scene::{BlockChange, BlockPos, BlockState as FilmBlock};
pub use cinematix_ui::{Mods, Rect};
pub use document::{
    Document, camera_data, default_interp, key_light_data, kind_key, new_node, prop_key, props_of,
    rest_value,
};
pub use draw::draw;
pub use editor::{Button, Editor, EditorKey, Field, KeyResult, Section, fields_for};
pub use film::{EXTENSION, FORMAT_VERSION, Stamp, load, save};
pub use frame::{Frame, GUTTER_W, ROW_H};
pub use icons as glyphs;
