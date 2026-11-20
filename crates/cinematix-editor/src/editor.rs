use crate::document::Document;
use crate::frame::Frame;
use cinematix_timeline::{Playhead, Snap, snap_t};
use cinematix_ui::Mods;

pub const MIN_SPAN: f64 = 1.0 / 120.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Section {
    Camera,
    Actors,
    Models,
    Lights,
    World,
    Film,
    Render,
    Project,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::Camera,
        Section::Actors,
        Section::Models,
        Section::Lights,
        Section::World,
        Section::Film,
        Section::Render,
        Section::Project,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Section::Camera => "cinematix.section.camera",
            Section::Actors => "cinematix.section.actors",
            Section::Models => "cinematix.section.models",
            Section::Lights => "cinematix.section.lights",
            Section::World => "cinematix.section.world",
            Section::Film => "cinematix.section.film",
            Section::Render => "cinematix.section.render",
            Section::Project => "cinematix.section.project",
        }
    }

    /// The section's ctl name: the last part of its translation key.
    pub fn name(self) -> &'static str {
        match self {
            Section::Camera => "camera",
            Section::Actors => "actors",
            Section::Models => "models",
            Section::Lights => "lights",
            Section::World => "world",
            Section::Film => "film",
            Section::Render => "render",
            Section::Project => "project",
        }
    }

    pub fn short_key(self) -> &'static str {
        match self {
            Section::Camera => "cinematix.section.camera.short",
            Section::Actors => "cinematix.section.actors.short",
            Section::Models => "cinematix.section.models.short",
            Section::Lights => "cinematix.section.lights.short",
            Section::World => "cinematix.section.world.short",
            Section::Film => "cinematix.section.film.short",
            Section::Render => "cinematix.section.render.short",
            Section::Project => "cinematix.section.project.short",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    PlayPause,
    StepPrev,
    StepNext,
    AddKey,
    DeleteKey,
    ZoomIn,
    ZoomOut,
    Fit,
}

impl Button {
    pub const ALL: [Button; 8] = [
        Button::PlayPause,
        Button::StepPrev,
        Button::StepNext,
        Button::AddKey,
        Button::DeleteKey,
        Button::ZoomIn,
        Button::ZoomOut,
        Button::Fit,
    ];

    pub fn width(self) -> f32 {
        match self {
            Button::PlayPause => 28.0,
            Button::ZoomIn | Button::ZoomOut | Button::Fit => 22.0,
            _ => 24.0,
        }
    }

    /// The face of the button: a symbol, not a word, so it needs no translation.
    pub fn face(self, playing: bool) -> &'static str {
        match self {
            Button::PlayPause if playing => "II",
            Button::PlayPause => ">",
            Button::StepPrev => "<",
            Button::StepNext => ">",
            Button::AddKey => "K",
            Button::DeleteKey => "X",
            Button::ZoomIn => "+",
            Button::ZoomOut => "-",
            Button::Fit => "F",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Fov,
    Near,
    Far,
    Focus,
    Aperture,
    Fps,
}

pub fn fields_for(section: Section) -> &'static [Field] {
    match section {
        Section::Camera => &[
            Field::Fov,
            Field::Near,
            Field::Far,
            Field::Focus,
            Field::Aperture,
        ],
        Section::Render => &[Field::Fps],
        _ => &[],
    }
}

impl Field {
    pub fn range(self) -> (f64, f64, f64) {
        match self {
            Field::Fov => (10.0, 170.0, 1.0),
            Field::Near => (0.01, 1.0, 0.01),
            Field::Far => (16.0, 2048.0, 16.0),
            Field::Focus => (0.0, 256.0, 0.1),
            Field::Aperture => (1.0, 22.0, 0.1),
            Field::Fps => (1.0, 240.0, 1.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Field::Fov => "cinematix.field.fov",
            Field::Near => "cinematix.field.near",
            Field::Far => "cinematix.field.far",
            Field::Focus => "cinematix.field.focus",
            Field::Aperture => "cinematix.field.aperture",
            Field::Fps => "cinematix.field.fps",
        }
    }

    pub fn get(self, doc: &Document) -> f64 {
        match self {
            Field::Fps => doc.scene.meta.fps as f64,
            _ => match doc.camera_data() {
                Some(c) => match self {
                    Field::Fov => c.fov_deg as f64,
                    Field::Near => c.near as f64,
                    Field::Far => c.far as f64,
                    Field::Focus => c.focus_distance as f64,
                    Field::Aperture => c.aperture as f64,
                    Field::Fps => doc.scene.meta.fps as f64,
                },
                None => 0.0,
            },
        }
    }

    pub fn set(self, doc: &mut Document, v: f64) {
        let (min, max, _) = self.range();
        let v = v.clamp(min, max);
        match self {
            Field::Fps => doc.scene.meta.fps = v as f32,
            _ => {
                if let Some(c) = doc.camera_data_mut() {
                    match self {
                        Field::Fov => c.fov_deg = v as f32,
                        Field::Near => c.near = v as f32,
                        Field::Far => c.far = v as f32,
                        Field::Focus => c.focus_distance = v as f32,
                        Field::Aperture => c.aperture = v as f32,
                        Field::Fps => {}
                    }
                }
            }
        }
    }

    pub fn text(self, doc: &Document) -> String {
        let v = self.get(doc);
        match self {
            Field::Near => format!("{v:.2}"),
            Field::Focus | Field::Aperture => format!("{v:.1}"),
            _ => format!("{v:.0}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorKey {
    Space,
    Keyframe,
    Delete,
    Left,
    Right,
    Home,
    End,
    ZoomIn,
    ZoomOut,
    Fit,
    Escape,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyResult {
    Ignored,
    Consumed,
    Close,
}

#[derive(Clone, Copy, Debug)]
enum Drag {
    Playhead,
    Key {
        chan: usize,
        key: usize,
        grab_dt: f64,
    },
    Number {
        field: Field,
        start: f64,
        start_x: f32,
    },
}

pub struct Editor {
    pub doc: Document,
    pub section: Section,
    pub playhead: Playhead,
    pub playing: bool,
    pub looping: bool,
    pub scroll: usize,
    pub selected_channel: Option<usize>,
    pub selected_key: Option<(usize, usize)>,
    pub selected_node: Option<cinematix_scene::NodeId>,
    pub status: Option<&'static str>,
    t0: f64,
    t1: f64,
    size: (f32, f32),
    drag: Option<Drag>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        let doc = Document::new();
        let (r0, r1) = doc.scene.meta.range;
        let fps = doc.scene.meta.fps;
        let (t0, t1) = window_for(r0, r1);
        Self {
            playhead: Playhead::new(fps),
            doc,
            section: Section::Camera,
            playing: false,
            looping: true,
            scroll: 0,
            selected_channel: None,
            selected_key: None,
            selected_node: None,
            status: None,
            t0,
            t1,
            size: (0.0, 0.0),
            drag: None,
        }
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        self.size = (w.max(1.0), h.max(1.0));
    }

    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    pub fn window(&self) -> (f64, f64) {
        (self.t0, self.t1)
    }

    pub fn frame(&self) -> Frame {
        Frame::build(&self.doc, self.section, self.scroll, self.size).with_window(self.t0, self.t1)
    }

    pub fn frame_number(&self) -> i64 {
        self.playhead.frame()
    }

    pub fn time(&self) -> f64 {
        self.playhead.t
    }

    pub fn fps(&self) -> f32 {
        self.doc.scene.meta.fps.max(1.0)
    }

    pub fn range(&self) -> (f64, f64) {
        self.doc.scene.meta.range
    }

    fn max_scroll(&self) -> usize {
        let visible = self.frame().visible_rows.max(1);
        self.doc.channels.len().saturating_sub(visible)
    }

    pub fn set_window(&mut self, t0: f64, t1: f64) {
        let span = (t1 - t0).max(MIN_SPAN);
        self.t0 = t0;
        self.t1 = t0 + span;
    }

    pub fn zoom(&mut self, factor: f64) {
        let span = ((self.t1 - self.t0) / factor.clamp(0.05, 20.0))
            .max(MIN_SPAN)
            .min(86_400.0);
        let mid = 0.5 * (self.t0 + self.t1);
        self.set_window(mid - span * 0.5, mid + span * 0.5);
    }

    pub fn fit(&mut self) {
        let (r0, r1) = self.range();
        let (t0, t1) = window_for(r0, r1);
        self.set_window(t0, t1);
    }

    pub fn seek(&mut self, t: f64) {
        let (r0, r1) = self.range();
        self.playhead.t = t.clamp(r0.min(r1), r0.max(r1));
    }

    pub fn seek_frame(&mut self, frame: i64) {
        let fps = f64::from(self.fps());
        self.seek(frame as f64 / fps);
    }

    pub fn scrub_to(&mut self, t: f64, snap: bool) {
        let t = if snap {
            let fps = self.fps();
            (t * f64::from(fps)).round() / f64::from(fps)
        } else {
            t
        };
        self.seek(t);
    }

    pub fn step(&mut self, frames: i64) {
        self.playing = false;
        let fps = f64::from(self.fps());
        self.seek(self.playhead.t + frames as f64 / fps);
    }

    pub fn advance(&mut self, dt: f64) {
        if !self.playing || dt <= 0.0 {
            return;
        }
        let (r0, r1) = self.range();
        let mut t = self.playhead.t + dt;
        if t >= r1 {
            if self.looping && r1 > r0 {
                let span = r1 - r0;
                t = r0 + (t - r0) % span;
            } else {
                t = r1;
                self.playing = false;
            }
        }
        self.playhead.t = t;
    }

    fn active_channel(&self) -> Option<usize> {
        self.selected_channel
            .or_else(|| self.doc.channels.iter().position(|c| !c.keys.is_empty()))
            .or(Some(0))
            .filter(|i| *i < self.doc.channels.len())
    }

    pub fn add_key(&mut self) -> bool {
        let Some(ci) = self.active_channel() else {
            return false;
        };
        let t = self.playhead.t;
        match self.doc.insert_key(ci, t) {
            Some(ki) => {
                self.selected_channel = Some(ci);
                self.selected_key = Some((ci, ki));
                self.status = Some("cinematix.status.key_added");
                true
            }
            None => false,
        }
    }

    pub fn delete_key(&mut self) -> bool {
        let Some((ci, ki)) = self.selected_key else {
            return false;
        };
        if !self.doc.delete_key(ci, ki) {
            return false;
        }
        self.selected_key = self
            .doc
            .channels
            .get(ci)
            .filter(|c| !c.keys.is_empty())
            .map(|_| (ci, ki.min(self.doc.channels[ci].keys.len() - 1)));
        self.status = Some("cinematix.status.key_deleted");
        true
    }

    pub fn key(&mut self, k: EditorKey) -> KeyResult {
        match k {
            EditorKey::Space => {
                self.playing = !self.playing;
                KeyResult::Consumed
            }
            EditorKey::Keyframe => {
                self.add_key();
                KeyResult::Consumed
            }
            EditorKey::Delete => {
                self.delete_key();
                KeyResult::Consumed
            }
            EditorKey::Left => {
                self.step(-1);
                KeyResult::Consumed
            }
            EditorKey::Right => {
                self.step(1);
                KeyResult::Consumed
            }
            EditorKey::Home => {
                self.seek_frame(0);
                KeyResult::Consumed
            }
            EditorKey::End => {
                let end = self.range().1;
                self.seek(end);
                KeyResult::Consumed
            }
            EditorKey::ZoomIn => {
                self.zoom(1.25);
                KeyResult::Consumed
            }
            EditorKey::ZoomOut => {
                self.zoom(0.8);
                KeyResult::Consumed
            }
            EditorKey::Fit => {
                self.fit();
                KeyResult::Consumed
            }
            EditorKey::Escape => KeyResult::Close,
        }
    }

    pub fn wheel(&mut self, ticks: f32, ctrl: bool) -> bool {
        if ticks == 0.0 {
            return false;
        }
        if ctrl {
            self.zoom(if ticks > 0.0 { 1.25 } else { 0.8 });
        } else {
            let step = if ticks > 0.0 { 3 } else { -3 };
            let next = (self.scroll as i64 + i64::from(step)).clamp(0, self.max_scroll() as i64);
            self.scroll = next as usize;
        }
        true
    }

    /// A pointer press. Returns true when the editor took it: everything inside the screen is the
    /// editor's while it is open, so a click can never reach the world's block breaking.
    pub fn press(&mut self, pos: (f32, f32), mods: Mods) -> bool {
        let f = self.frame();
        for (s, r) in &f.rail_items {
            if r.contains(pos.0, pos.1) {
                self.section = *s;
                self.scroll = 0;
                return true;
            }
        }
        for (b, r) in &f.buttons {
            if r.contains(pos.0, pos.1) {
                self.press_button(*b);
                return true;
            }
        }
        for (field, r) in &f.fields {
            if r.contains(pos.0, pos.1) {
                self.drag = Some(Drag::Number {
                    field: *field,
                    start: field.get(&self.doc),
                    start_x: pos.0,
                });
                return true;
            }
        }
        for (id, r) in &f.list {
            if r.contains(pos.0, pos.1) {
                self.selected_node = Some(*id);
                return true;
            }
        }
        if f.ruler.contains(pos.0, pos.1) {
            self.scrub_to(f.axis.t(pos.0), !mods.alt);
            self.playing = false;
            self.drag = Some(Drag::Playhead);
            return true;
        }
        for row in &f.rows {
            if row.label.contains(pos.0, pos.1) {
                self.selected_channel = Some(row.chan);
                self.selected_key = None;
                return true;
            }
            if row.keys.contains(pos.0, pos.1) {
                self.selected_channel = Some(row.chan);
                self.selected_key = None;
                if let Some(ch) = self.doc.channels.get(row.chan) {
                    let times: Vec<f64> = ch.keys.iter().map(|k| k.t).collect();
                    if let Some(ki) = cinematix_timeline::hit_key(pos.0, &f.axis, &times, 6.0) {
                        let grab = f.axis.t(pos.0) - ch.keys[ki].t;
                        self.selected_key = Some((row.chan, ki));
                        self.drag = Some(Drag::Key {
                            chan: row.chan,
                            key: ki,
                            grab_dt: grab,
                        });
                    }
                }
                return true;
            }
        }
        true
    }

    fn press_button(&mut self, b: Button) {
        match b {
            Button::PlayPause => self.playing = !self.playing,
            Button::StepPrev => self.step(-1),
            Button::StepNext => self.step(1),
            Button::AddKey => {
                self.add_key();
            }
            Button::DeleteKey => {
                self.delete_key();
            }
            Button::ZoomIn => self.zoom(1.25),
            Button::ZoomOut => self.zoom(0.8),
            Button::Fit => self.fit(),
        }
    }

    pub fn drag_to(&mut self, pos: (f32, f32), mods: Mods) -> bool {
        let Some(drag) = self.drag else { return false };
        let f = self.frame();
        match drag {
            Drag::Playhead => {
                self.scrub_to(f.axis.t(pos.0), !mods.alt);
                true
            }
            Drag::Key { chan, key, grab_dt } => {
                let Some(ch) = self.doc.channels.get(chan) else {
                    return false;
                };
                let times: Vec<f64> = ch.keys.iter().map(|k| k.t).collect();
                let raw = (f.axis.t(pos.0) - grab_dt).max(0.0);
                let tol = 6.0 / f.axis.px_per_sec().max(1e-9);
                let t = if mods.alt {
                    raw
                } else {
                    snap_t(
                        raw,
                        self.fps(),
                        &times,
                        tol,
                        Snap {
                            frames: true,
                            keys: true,
                            markers: false,
                        },
                    )
                };
                if let Some(new) = self.doc.move_key(chan, key, t) {
                    self.selected_key = Some((chan, new));
                    if let Some(Drag::Key { key, .. }) = self.drag.as_mut() {
                        *key = new;
                    }
                }
                true
            }
            Drag::Number {
                field,
                start,
                start_x,
            } => {
                let (min, max, step) = field.range();
                let scrub = cinematix_ui::Scrubber::new(start, min, max, step);
                field.set(&mut self.doc, scrub.drag(pos.0 - start_x, mods));
                true
            }
        }
    }

    pub fn release(&mut self) -> bool {
        self.drag.take().is_some()
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Whether this field is the one under an active pointer drag, so it draws held.
    pub fn dragging_field(&self, field: Field) -> bool {
        matches!(self.drag, Some(Drag::Number { field: dragged, .. }) if dragged == field)
    }

    /// The channels that hold at least one key, for the film summary.
    pub fn keyed_channels(&self) -> usize {
        self.doc
            .channels
            .iter()
            .filter(|c| !c.keys.is_empty())
            .count()
    }
}

fn window_for(r0: f64, r1: f64) -> (f64, f64) {
    let (a, b) = if r1 > r0 { (r0, r1) } else { (r0, r0 + 1.0) };
    let pad = ((b - a) * 0.02).max(MIN_SPAN);
    (a - pad, b + pad)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        let mut e = Editor::new();
        e.resize(1280.0, 720.0);
        e
    }

    #[test]
    fn a_press_on_a_rail_button_switches_section() {
        let mut e = editor();
        let f = e.frame();
        let (s, r) = *f.rail_items.last().unwrap();
        assert!(e.press((r.x + 2.0, r.y + 2.0), Mods::default()));
        assert_eq!(e.section, s);
        assert_eq!(e.scroll, 0);
    }

    #[test]
    fn pressing_the_ruler_moves_the_playhead_and_dragging_follows() {
        let mut e = editor();
        let f = e.frame();
        let x = f.ruler.x + f.ruler.w * 0.5;
        assert!(e.press((x, f.ruler.y + 4.0), Mods::default()));
        assert!(e.dragging());
        let mid = e.range();
        assert!(
            (e.time() - (mid.0 + mid.1) * 0.5).abs() < 0.25,
            "{}",
            e.time()
        );
        e.drag_to(
            (f.ruler.x + f.ruler.w * 0.75, f.ruler.y + 4.0),
            Mods::default(),
        );
        assert!(e.time() > (mid.0 + mid.1) * 0.5);
        assert!(e.release());
        assert!(!e.dragging());
    }

    #[test]
    fn a_press_anywhere_in_the_editor_is_taken() {
        let mut e = editor();
        let f = e.frame();
        assert!(e.press((f.viewport.x + 10.0, f.viewport.y + 10.0), Mods::default()));
    }

    #[test]
    fn dragging_a_number_field_changes_the_document_value() {
        let mut e = editor();
        let f = e.frame();
        let (field, r) = *f.fields.first().unwrap();
        assert_eq!(field, Field::Fov);
        let before = Field::Fov.get(&e.doc);
        e.press((r.x + 20.0, r.y + 4.0), Mods::default());
        e.drag_to((r.x + 38.0, r.y + 4.0), Mods::default());
        let after = Field::Fov.get(&e.doc);
        assert!((after - (before + 3.0)).abs() < 1e-6, "{before} -> {after}");
        assert_eq!(after.round() as i32, 73);
    }

    #[test]
    fn shift_makes_a_scrub_coarser_and_ctrl_finer() {
        let mut e = editor();
        let f = e.frame();
        let (_, r) = *f.fields.first().unwrap();
        e.press((r.x + 10.0, r.y + 3.0), Mods::default());
        e.drag_to((r.x + 16.0, r.y + 3.0), Mods::default());
        let coarse = Field::Fov.get(&e.doc);
        e.release();

        let mut e = editor();
        let f = e.frame();
        let (_, r) = *f.fields.first().unwrap();
        e.press((r.x + 10.0, r.y + 3.0), Mods::default());
        e.drag_to(
            (r.x + 16.0, r.y + 3.0),
            Mods {
                shift: true,
                ..Default::default()
            },
        );
        let shifted = Field::Fov.get(&e.doc);
        e.release();

        let mut e = editor();
        let f = e.frame();
        let (_, r) = *f.fields.first().unwrap();
        e.press((r.x + 10.0, r.y + 3.0), Mods::default());
        e.drag_to(
            (r.x + 16.0, r.y + 3.0),
            Mods {
                ctrl: true,
                ..Default::default()
            },
        );
        let fine = Field::Fov.get(&e.doc);

        assert!((coarse - 71.0).abs() < 1e-6, "{coarse}");
        assert!((shifted - 80.0).abs() < 1e-6, "{shifted}");
        assert!((fine - 70.1).abs() < 1e-4, "{fine}");
    }

    #[test]
    fn playing_advances_and_loops_inside_the_range() {
        let mut e = editor();
        e.playing = true;
        e.advance(1.0);
        assert!((e.time() - 1.0).abs() < 1e-9);
        e.advance(100.0);
        let (r0, r1) = e.range();
        assert!(e.time() >= r0 && e.time() <= r1);
        assert!(e.playing);
    }

    #[test]
    fn stepping_stops_playback_and_moves_one_frame() {
        let mut e = editor();
        e.playing = true;
        e.step(1);
        assert!(!e.playing);
        assert_eq!(e.frame_number(), 1);
        e.step(-1);
        assert_eq!(e.frame_number(), 0);
    }

    #[test]
    fn keying_needs_a_channel_and_lands_at_the_playhead() {
        let mut e = editor();
        e.seek_frame(12);
        assert!(e.add_key());
        let (ci, ki) = e.selected_key.unwrap();
        let k = e.doc.channels[ci].keys[ki];
        assert!((k.t - 0.5).abs() < 1e-9);
        assert!(e.delete_key());
        assert!(e.doc.channels[ci].keys.is_empty());
    }

    #[test]
    fn clicking_a_key_selects_it_and_dragging_moves_it_in_time() {
        let mut e = editor();
        e.seek_frame(24);
        e.add_key();
        let (ci, _) = e.selected_key.unwrap();
        let f = e.frame();
        let row = f
            .rows
            .iter()
            .find(|r| r.chan == ci)
            .expect("the keyed row is visible");
        let x = f.axis.x(1.0);
        assert!(e.press((x, row.keys.y + 4.0), Mods::default()));
        assert!(e.selected_key.is_some());
        e.drag_to((f.axis.x(3.0), row.keys.y + 4.0), Mods::default());
        let key = e.doc.channels[ci].keys[0];
        assert!((key.t - 3.0).abs() < 0.05, "{}", key.t);
    }

    #[test]
    fn zoom_keeps_the_middle_of_the_window() {
        let mut e = editor();
        let (a, b) = e.window();
        let mid = (a + b) * 0.5;
        e.zoom(2.0);
        let (c, d) = e.window();
        assert!(((c + d) * 0.5 - mid).abs() < 1e-9);
        assert!(((d - c) - (b - a) * 0.5).abs() < 1e-9);
    }

    #[test]
    fn zoom_never_passes_the_minimum_span() {
        let mut e = editor();
        for _ in 0..200 {
            e.zoom(2.0);
        }
        let (a, b) = e.window();
        assert!(b - a >= MIN_SPAN - 1e-12);
    }

    #[test]
    fn escape_asks_to_close_and_nothing_else_does() {
        assert_eq!(editor().key(EditorKey::Escape), KeyResult::Close);
        assert_eq!(editor().key(EditorKey::Space), KeyResult::Consumed);
    }
}
