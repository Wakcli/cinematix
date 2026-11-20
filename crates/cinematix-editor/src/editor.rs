use crate::document::Document;
use crate::frame::Frame;
use crate::menu::Menu;
use cinematix_anim::{Interp, Prop, Value};
use cinematix_timeline::{Playhead, Snap, snap_t};
use cinematix_ui::{Mods, Tokens};
use glam::{Quat, Vec3};

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

/// What a panel button asked the game to do. Only implemented actions have a variant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Request {
    Save,
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
    TimelineSplit {
        start: f32,
        y: f32,
    },
    CurveKey {
        chan: usize,
        key: usize,
        axis: usize,
        start: f64,
        y: f32,
        unit: f64,
    },
    InspectorSplit {
        start: f32,
        x: f32,
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
    /// The bottom dock's share of the stage, and the inspector's share of the width.
    pub tab: crate::frame::TimelineTab,
    pub curve_axis: usize,
    pub expanded: std::collections::HashSet<cinematix_scene::NodeId>,
    pub timeline_frac: f32,
    pub inspector_frac: f32,
    request: Option<Request>,
    history: Vec<Document>,
    redo: Vec<Document>,
    pub(crate) menu: Option<Menu>,
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
            tab: crate::frame::TimelineTab::DopeSheet,
            curve_axis: 0,
            expanded: Default::default(),
            timeline_frac: 0.38,
            inspector_frac: 0.22,
            request: None,
            history: Vec::new(),
            redo: Vec::new(),
            menu: None,
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
        Frame::build(
            &self.doc,
            self.section,
            self.scroll,
            self.size,
            self.timeline_frac,
            self.inspector_frac,
            self.selected_channel,
            &self.expanded,
        )
        .with_window(self.t0, self.t1)
    }

    /// Snapshots the document before an edit, so it can be undone. The redo stack is dropped:
    /// a new edit forks the history.
    pub fn checkpoint(&mut self) {
        const LIMIT: usize = 64;
        if self.history.len() == LIMIT {
            self.history.remove(0);
        }
        self.history.push(self.doc.clone());
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.redo.push(self.doc.clone());
        self.restore(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.history.push(self.doc.clone());
        self.restore(next);
        true
    }

    fn restore(&mut self, doc: Document) {
        self.doc = doc;
        let channels = self.doc.channels.len();
        if self.selected_channel.is_some_and(|ci| ci >= channels) {
            self.selected_channel = None;
        }
        if let Some((ci, ki)) = self.selected_key
            && self.doc.channels.get(ci).is_none_or(|c| ki >= c.keys.len())
        {
            self.selected_key = None;
        }
        if self
            .selected_node
            .is_some_and(|id| !self.doc.scene.nodes.contains_key(&id))
        {
            self.selected_node = None;
        }
        self.seek(self.playhead.t);
    }

    /// A one-shot command the panel buttons asked for (Save), for the game to perform.
    pub fn take_request(&mut self) -> Option<Request> {
        self.request.take()
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
        self.checkpoint();
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

    /// Sets the selected key's outgoing interpolation. The key decides the segment after it, so a
    /// spline key bends toward the next key and a step key holds until it.
    pub fn set_selected_interp(&mut self, interp: Interp) -> bool {
        let Some((ci, ki)) = self.selected_key else {
            return false;
        };
        self.checkpoint();
        match self
            .doc
            .channels
            .get_mut(ci)
            .and_then(|c| c.keys.get_mut(ki))
        {
            Some(key) => {
                key.interp = interp;
                self.status = Some("cinematix.status.interp");
                true
            }
            None => false,
        }
    }

    pub fn delete_key(&mut self) -> bool {
        let Some((ci, ki)) = self.selected_key else {
            return false;
        };
        self.checkpoint();
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
            EditorKey::Escape => {
                if self.menu_escape() {
                    KeyResult::Consumed
                } else {
                    KeyResult::Close
                }
            }
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
        if self.menu_click(pos) {
            return true;
        }
        let f = self.frame();
        for (tool, r) in &f.tools {
            if r.contains(pos.0, pos.1) {
                match tool {
                    crate::frame::Tool::Save => self.request = Some(Request::Save),
                    crate::frame::Tool::Undo => {
                        self.undo();
                    }
                    crate::frame::Tool::Redo => {
                        self.redo();
                    }
                    crate::frame::Tool::Render => {}
                }
                return true;
            }
        }
        if f.split_h.contains(pos.0, pos.1) {
            self.drag = Some(Drag::TimelineSplit {
                start: self.timeline_frac,
                y: pos.1,
            });
            return true;
        }
        if f.split_v.contains(pos.0, pos.1) {
            self.drag = Some(Drag::InspectorSplit {
                start: self.inspector_frac,
                x: pos.0,
            });
            return true;
        }
        for (tab, r) in &f.tabs {
            if r.contains(pos.0, pos.1) {
                match tab {
                    crate::frame::TimelineTab::DopeSheet | crate::frame::TimelineTab::Curves => {
                        self.tab = *tab;
                    }
                    crate::frame::TimelineTab::Clips => {}
                }
                return true;
            }
        }
        for (axis, r) in &f.curve_axes {
            if r.contains(pos.0, pos.1) {
                self.curve_axis = *axis;
                return true;
            }
        }
        if self.tab == crate::frame::TimelineTab::Curves
            && f.curve_plot.contains(pos.0, pos.1)
            && let Some(ci) = self.selected_channel
            && let Some(ch) = self.doc.channel(ci)
        {
            let times: Vec<f64> = ch.keys.iter().map(|k| k.t).collect();
            if let Some(ki) = cinematix_timeline::hit_key(pos.0, &f.axis, &times, 6.0)
                && let Some((lo, hi)) = crate::curve::range(ch, self.curve_axis)
                && let Some(start) = crate::curve::component(ch.keys[ki].v, self.curve_axis)
            {
                let unit = (hi - lo) / f64::from(f.curve_plot.h.max(1.0));
                self.checkpoint();
                self.selected_channel = Some(ci);
                self.selected_key = Some((ci, ki));
                self.drag = Some(Drag::CurveKey {
                    chan: ci,
                    key: ki,
                    axis: self.curve_axis,
                    start,
                    y: pos.1,
                    unit,
                });
            }
            return true;
        }
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
                // A click on the track sets the value from the position, as
                // `AbstractSliderButton` does, and a drag then scrubs from there.
                let (min, max, step) = field.range();
                let track = (r.w - 8.0).max(1.0);
                let frac = f64::from((pos.0 - r.x - 4.0) / track);
                let scrub = cinematix_ui::Scrubber::new(field.get(&self.doc), min, max, step);
                field.set(&mut self.doc, scrub.quantise(scrub.from_frac(frac)));
                self.drag = Some(Drag::Number {
                    field: *field,
                    start: field.get(&self.doc),
                    start_x: pos.0,
                });
                return true;
            }
        }
        for (id, r) in &f.tree_toggle {
            if r.contains(pos.0, pos.1) {
                if !self.expanded.remove(id) {
                    self.expanded.insert(*id);
                }
                return true;
            }
        }
        for (id, _, _, r) in &f.tree {
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
            Drag::TimelineSplit { start, y } => {
                let stage = (self.size.1 - Tokens::default().header_h).max(1.0);
                self.timeline_frac = (start + (y - pos.1) / stage).clamp(0.2, 0.62);
                true
            }
            Drag::InspectorSplit { start, x } => {
                let width = self.size.0.max(1.0);
                self.inspector_frac = (start + (x - pos.0) / width).clamp(0.12, 0.4);
                true
            }
            Drag::CurveKey {
                chan,
                key,
                axis,
                start,
                y,
                unit,
            } => {
                let value = start + f64::from(y - pos.1) * unit;
                match self
                    .doc
                    .channels
                    .get_mut(chan)
                    .and_then(|c| c.keys.get_mut(key))
                {
                    Some(k) => crate::curve::set_component(&mut k.v, axis, value as f32),
                    None => false,
                }
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

    /// The camera node's pose at `t`: the animated value when the channel holds keys, the rest
    /// value otherwise.
    pub fn camera_at(&self, t: f64) -> Option<(Vec3, Quat)> {
        let id = self.doc.scene.camera;
        let node = self.doc.node(id)?;
        let mut position = node.rest.translation;
        let mut rotation = node.rest.rotation;
        if let Some(ci) = self.doc.channel_index(id, Prop::Translation)
            && let Some(Value::Vec3(v)) = self.doc.value_at(ci, t)
        {
            position = Vec3::from_array(v);
        }
        if let Some(ci) = self.doc.channel_index(id, Prop::Rotation)
            && let Some(Value::Quat(q)) = self.doc.value_at(ci, t)
        {
            rotation = Quat::from_array(q).normalize();
        }
        Some((position, rotation))
    }

    /// What the renderer should look through while the editor is open: the camera at the
    /// playhead, as `camera_view` hands it over.
    pub fn preview_view(&self) -> Option<([f64; 3], f32, f32)> {
        let (position, rotation) = self.camera_at(self.playhead.t)?;
        let (yaw, pitch) = view_from_rotation(rotation);
        Some((
            [
                f64::from(position.x),
                f64::from(position.y),
                f64::from(position.z),
            ],
            yaw,
            pitch,
        ))
    }

    /// The camera's field of view at the playhead.
    pub fn preview_fov(&self) -> f32 {
        let id = self.doc.scene.camera;
        if let Some(ci) = self.doc.channel_index(id, Prop::Fov)
            && let Some(Value::F32(v)) = self.doc.value_at(ci, self.playhead.t)
        {
            return v;
        }
        match self.doc.camera_data() {
            Some(c) => c.fov_deg,
            None => 70.0,
        }
    }

    /// Adopts a pose as the camera's pose. Without `force` a keyed camera is left alone:
    /// opening the editor over a film must not move it. With `force` a keyed pose becomes a key
    /// at the playhead, so the reset is itself part of the animation.
    pub fn take_pose(&mut self, position: Vec3, yaw: f32, pitch: f32, force: bool) -> bool {
        if !force && self.doc.camera_keyed() {
            return false;
        }
        self.checkpoint();
        let rotation = rotation_from_view(yaw, pitch);
        if !force && self.doc.camera_keyed() {
            return false;
        }
        if self.doc.camera_keyed() {
            let id = self.doc.scene.camera;
            let t = self.playhead.t;
            let mut ok = true;
            if let Some(ci) = self.doc.channel_index(id, Prop::Translation)
                && !self.doc.channels[ci].keys.is_empty()
            {
                ok &= self
                    .doc
                    .upsert_key(ci, t, Value::Vec3(position.to_array()))
                    .is_some();
            } else {
                ok &= self.doc.set_camera_translation(position);
            }
            if let Some(ci) = self.doc.channel_index(id, Prop::Rotation)
                && !self.doc.channels[ci].keys.is_empty()
            {
                ok &= self
                    .doc
                    .upsert_key(ci, t, Value::Quat(rotation.to_array()))
                    .is_some();
            } else {
                ok &= self.doc.set_camera_rotation(rotation);
            }
            return ok;
        }
        self.doc.set_camera_translation(position) && self.doc.set_camera_rotation(rotation)
    }

    /// Moves the preview camera by `delta`. A translation channel that already holds keys gets a
    /// key at the playhead instead, so a moving camera stays a moving camera.
    pub fn nudge_camera(&mut self, delta: Vec3) -> bool {
        let id = self.doc.scene.camera;
        let t = self.playhead.t;
        match self.doc.channel_index(id, Prop::Translation) {
            Some(ci) if !self.doc.channels[ci].keys.is_empty() => {
                let base = match self.doc.value_at(ci, t) {
                    Some(Value::Vec3(v)) => Vec3::from_array(v),
                    _ => Vec3::ZERO,
                };
                self.doc
                    .upsert_key(ci, t, Value::Vec3((base + delta).to_array()))
                    .is_some()
            }
            _ => self.doc.set_camera_translation(match self.camera_at(t) {
                Some((position, _)) => position + delta,
                None => delta,
            }),
        }
    }

    /// Turns the preview camera by degrees (positive yaw turns right, positive pitch looks
    /// down), pitching no further than straight up or down. A keyed rotation gets a key at the
    /// playhead instead of the rest pose.
    pub fn look(&mut self, dyaw: f32, dpitch: f32) -> bool {
        let id = self.doc.scene.camera;
        let t = self.playhead.t;
        let (_, rotation) = match self.camera_at(t) {
            Some(pose) => pose,
            None => return false,
        };
        let (yaw, pitch) = view_from_rotation(rotation);
        let rotation = rotation_from_view(yaw + dyaw, pitch + dpitch);
        match self.doc.channel_index(id, Prop::Rotation) {
            Some(ci) if !self.doc.channels[ci].keys.is_empty() => self
                .doc
                .upsert_key(ci, t, Value::Quat(rotation.to_array()))
                .is_some(),
            _ => self.doc.set_camera_rotation(rotation),
        }
    }

    /// Sets the camera's field of view: a keyed FOV channel gets a key at the playhead, an
    /// unkeyed one changes the camera's rest data.
    pub fn set_fov(&mut self, fov: f32) -> bool {
        self.checkpoint();
        let id = self.doc.scene.camera;
        let t = self.playhead.t;
        let fov = fov.clamp(1.0, 179.0);
        match self.doc.channel_index(id, Prop::Fov) {
            Some(ci) if !self.doc.channels[ci].keys.is_empty() => {
                self.doc.upsert_key(ci, t, Value::F32(fov)).is_some()
            }
            _ => match self.doc.camera_data_mut() {
                Some(camera) => {
                    camera.fov_deg = fov;
                    true
                }
                None => false,
            },
        }
    }

    /// The camera's pose, for the status reply.
    pub fn camera_status(&self) -> Option<([f32; 3], f32, f32, f32)> {
        let (position, rotation) = self.camera_at(self.playhead.t)?;
        let (yaw, pitch) = view_from_rotation(rotation);
        Some((position.to_array(), yaw, pitch, self.preview_fov()))
    }

    /// Replaces the film after a load or a new document: the playhead and the window follow the
    /// new range, and no selection points into the old channels.
    pub fn adopt_document(&mut self, doc: Document) {
        self.checkpoint();
        self.doc = doc;
        let (r0, r1) = self.doc.scene.meta.range;
        self.playhead.fps = self.doc.scene.meta.fps.round().max(1.0);
        self.seek(self.playhead.t.clamp(r0.min(r1), r0.max(r1)));
        self.fit();
        self.selected_channel = None;
        self.selected_key = None;
        self.selected_node = None;
        self.scroll = 0;
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

/// The camera's orientation from its view angles, in Horizon's convention
/// (`Camera.setRotation`: `rotationYXZ(PI - yaw, -pitch, 0)`).
pub fn rotation_from_view(yaw: f32, pitch: f32) -> Quat {
    Quat::from_euler(
        glam::EulerRot::YXZ,
        std::f32::consts::PI - yaw.to_radians(),
        -pitch.clamp(-89.9, 89.9).to_radians(),
        0.0,
    )
}

/// The view angles back from an orientation: yaw in (-180, 180], pitch in (-90, 90).
pub fn view_from_rotation(q: Quat) -> (f32, f32) {
    let (y, x, _) = q.normalize().to_euler(glam::EulerRot::YXZ);
    let mut yaw = (std::f32::consts::PI - y).to_degrees();
    while yaw <= -180.0 {
        yaw += 360.0;
    }
    while yaw > 180.0 {
        yaw -= 360.0;
    }
    (yaw, (-x.to_degrees()).clamp(-89.9, 89.9))
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

    /// Press on the slider's handle for its current value, so a drag starts where the value is.
    fn press_handle(e: &mut Editor, field: Field) -> cinematix_ui::Rect {
        let f = e.frame();
        let (_, r) = *f.fields.iter().find(|(f, _)| *f == field).unwrap();
        let (min, max, _) = field.range();
        let frac = ((field.get(&e.doc) - min) / (max - min)) as f32;
        let x = r.x + 4.0 + frac * (r.w - 8.0);
        e.press((x, r.y + 5.0), Mods::default());
        r
    }

    #[test]
    fn dragging_a_number_field_changes_the_document_value() {
        let mut e = editor();
        let r = press_handle(&mut e, Field::Fov);
        let before = Field::Fov.get(&e.doc);
        e.drag_to(
            (
                r.x + (r.w - 8.0) * ((before - 10.0) / 160.0) as f32 + 4.0 + 18.0,
                r.y + 5.0,
            ),
            Mods::default(),
        );
        let after = Field::Fov.get(&e.doc);
        assert!((after - (before + 3.0)).abs() < 1e-6, "{before} -> {after}");
    }

    #[test]
    fn shift_makes_a_scrub_coarser_and_ctrl_finer() {
        let mut e = editor();
        let r = press_handle(&mut e, Field::Fov);
        let before = Field::Fov.get(&e.doc);
        let dx = 6.0;
        let at = |r: cinematix_ui::Rect| {
            (
                r.x + 4.0 + ((before - 10.0) / 160.0) as f32 * (r.w - 8.0) + dx,
                r.y + 5.0,
            )
        };
        e.drag_to(at(r), Mods::default());
        let coarse = Field::Fov.get(&e.doc);
        e.release();

        let mut e = editor();
        let r = press_handle(&mut e, Field::Fov);
        e.drag_to(
            at(r),
            Mods {
                shift: true,
                ..Default::default()
            },
        );
        let shifted = Field::Fov.get(&e.doc);
        e.release();

        let mut e = editor();
        let r = press_handle(&mut e, Field::Fov);
        e.drag_to(
            at(r),
            Mods {
                ctrl: true,
                ..Default::default()
            },
        );
        let fine = Field::Fov.get(&e.doc);

        assert!((coarse - (before + 1.0)).abs() < 1e-6, "{coarse}");
        assert!((shifted - (before + 10.0)).abs() < 1e-6, "{shifted}");
        assert!((fine - (before + 0.1)).abs() < 1e-4, "{fine}");
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
    fn view_angles_survive_the_round_trip_through_a_rotation() {
        for yaw in [-179.0, -90.0, -30.0, 0.0, 45.0, 90.0, 179.0] {
            for pitch in [-60.0, -10.0, 0.0, 10.0, 60.0] {
                let q = rotation_from_view(yaw, pitch);
                let (back_yaw, back_pitch) = view_from_rotation(q);
                assert!((back_yaw - yaw).abs() < 0.01, "{yaw} -> {back_yaw}");
                assert!((back_pitch - pitch).abs() < 0.01, "{pitch} -> {back_pitch}");
            }
        }
    }

    #[test]
    fn the_default_camera_looks_north_from_the_origin() {
        let e = editor();
        let (position, rotation) = e.camera_at(0.0).unwrap();
        assert_eq!(position, Vec3::ZERO);
        let (yaw, pitch) = view_from_rotation(rotation);
        assert!((yaw - 180.0).abs() < 0.01, "{yaw}");
        assert!(pitch.abs() < 0.01);
    }

    #[test]
    fn nudging_an_unkeyed_camera_moves_its_rest_pose() {
        let mut e = editor();
        assert!(e.nudge_camera(Vec3::new(1.0, 2.0, 3.0)));
        let (position, _) = e.camera_at(0.0).unwrap();
        assert_eq!(position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(e.doc.key_count(), 0);
    }

    #[test]
    fn nudging_a_keyed_camera_writes_a_key_and_leaves_the_rest_pose_alone() {
        let mut e = editor();
        let ci = e
            .doc
            .channel_index(e.doc.scene.camera, Prop::Translation)
            .unwrap();
        e.seek_frame(24);
        e.add_key();
        let rest = e.doc.node(e.doc.scene.camera).unwrap().rest.translation;
        assert!(e.nudge_camera(Vec3::new(0.0, 5.0, 0.0)));
        assert_eq!(
            e.doc.node(e.doc.scene.camera).unwrap().rest.translation,
            rest
        );
        let value = e.doc.channels[ci].eval(1.0).unwrap();
        match value {
            Value::Vec3(v) => assert!((v[1] - (rest.y + 5.0)).abs() < 1e-5, "{v:?}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn looking_turns_the_camera_and_stops_at_straight_up_and_down() {
        let mut e = editor();
        assert!(e.look(30.0, 10.0));
        let (_, yaw, pitch, _) = e.camera_status().unwrap();
        assert!(
            (yaw - 210.0).abs() < 0.01 || (yaw + 150.0).abs() < 0.01,
            "{yaw}"
        );
        assert!((pitch - 10.0).abs() < 0.01, "{pitch}");
        assert!(e.look(0.0, 200.0));
        let (_, _, pitch, _) = e.camera_status().unwrap();
        assert!(pitch <= 89.91, "{pitch}");
    }

    #[test]
    fn taking_a_pose_is_refused_once_the_camera_is_keyed() {
        let mut e = editor();
        assert!(e.take_pose(Vec3::new(4.0, 80.0, -2.0), 15.0, -5.0, false));
        let (_, yaw, pitch, _) = e.camera_status().unwrap();
        assert!((yaw - 15.0).abs() < 0.01);
        assert!((pitch + 5.0).abs() < 0.01);
        let ci = e
            .doc
            .channel_index(e.doc.scene.camera, Prop::Translation)
            .unwrap();
        e.add_key();
        assert!(e.doc.channel(ci).is_some_and(|c| !c.keys.is_empty()));
        assert!(!e.take_pose(Vec3::ZERO, 0.0, 0.0, false));
        let (position, _, _, _) = e.camera_status().unwrap();
        assert_eq!(position, [4.0, 80.0, -2.0]);
    }

    #[test]
    fn the_preview_fov_follows_the_fov_channel() {
        let mut e = editor();
        let ci = e.doc.channel_index(e.doc.scene.camera, Prop::Fov).unwrap();
        e.selected_channel = Some(ci);
        e.seek_frame(0);
        e.add_key();
        e.doc.channels[ci].keys[0].v = Value::F32(35.0);
        e.seek_frame(48);
        assert!(e.add_key());
        e.doc.channels[ci].keys[1].v = Value::F32(55.0);
        e.seek(1.0);
        assert_eq!(e.doc.channels[ci].keys.len(), 2);
        assert!((e.preview_fov() - 45.0).abs() < 1.0, "{}", e.preview_fov());
    }

    #[test]
    fn new_translation_keys_spline_and_rotation_keys_smooth() {
        let mut e = editor();
        let ti = e
            .doc
            .channel_index(e.doc.scene.camera, Prop::Translation)
            .unwrap();
        let ri = e
            .doc
            .channel_index(e.doc.scene.camera, Prop::Rotation)
            .unwrap();
        e.doc.insert_key(ti, 0.0).unwrap();
        e.doc.insert_key(ri, 0.0).unwrap();
        assert_eq!(e.doc.channels[ti].keys[0].interp, Interp::Spline);
        assert_eq!(e.doc.channels[ri].keys[0].interp, Interp::Smooth);
    }

    #[test]
    fn changing_a_keys_interpolation_changes_what_the_channel_evaluates() {
        let mut e = editor();
        let ci = e
            .doc
            .channel_index(e.doc.scene.camera, Prop::Translation)
            .unwrap();
        e.selected_channel = Some(ci);
        e.seek_frame(0);
        e.add_key();
        e.doc.channels[ci].keys[0].v = Value::Vec3([0.0, 0.0, 0.0]);
        e.seek_frame(24);
        e.add_key();
        e.doc.channels[ci].keys[1].v = Value::Vec3([10.0, 0.0, 0.0]);
        e.selected_key = Some((ci, 0));
        assert!(e.set_selected_interp(Interp::Step));
        let Some(Value::Vec3(step)) = e.doc.channels[ci].eval(0.5) else {
            panic!()
        };
        assert!((step[0] - 0.0).abs() < 1e-5, "{step:?}");
        assert!(e.set_selected_interp(Interp::Spline));
        let Some(Value::Vec3(spline)) = e.doc.channels[ci].eval(0.5) else {
            panic!()
        };
        assert!((spline[0] - 5.0).abs() < 1e-4, "{spline:?}");
    }

    #[test]
    fn clicking_the_track_sets_the_field_from_the_click_position() {
        let mut e = editor();
        let f = e.frame();
        let (field, r) = *f.fields.first().unwrap();
        assert_eq!(field, Field::Fov);
        let mid = r.x + r.w * 0.5;
        e.press((mid, r.y + 5.0), Mods::default());
        e.release();
        let v = Field::Fov.get(&e.doc);
        assert!((v - 90.0).abs() < 3.0, "{v}");
    }

    #[test]
    fn dragging_the_splitter_resizes_the_timeline_and_inspector() {
        let mut e = editor();
        let f = e.frame();
        let before = e.timeline_frac;
        let p = (f.split_h.x + 20.0, f.split_h.y + 2.0);
        assert!(e.press(p, Mods::default()));
        assert!(e.drag_to((p.0, p.1 - 40.0), Mods::default()));
        assert!(e.timeline_frac > before, "{}", e.timeline_frac);
        e.release();

        let f = e.frame();
        let before = e.inspector_frac;
        let p = (f.split_v.x + 2.0, f.split_v.y + 40.0);
        assert!(e.press(p, Mods::default()));
        assert!(e.drag_to((p.0 - 40.0, p.1), Mods::default()));
        assert!(e.inspector_frac > before, "{}", e.inspector_frac);
    }

    #[test]
    fn the_unimplemented_tabs_take_a_click_without_changing_anything() {
        let mut e = editor();
        let f = e.frame();
        let (tab, r) = *f
            .tabs
            .iter()
            .find(|(t, _)| *t == crate::frame::TimelineTab::Curves)
            .unwrap();
        assert_eq!(tab, crate::frame::TimelineTab::Curves);
        assert!(e.press((r.x + 2.0, r.y + 2.0), Mods::default()));
        assert_eq!(e.section, Section::Camera);
    }

    #[test]
    fn keying_can_be_undone_and_redone() {
        let mut e = editor();
        e.seek_frame(12);
        assert!(e.add_key());
        assert_eq!(e.doc.key_count(), 1);
        assert!(e.can_undo());
        assert!(e.undo());
        assert_eq!(e.doc.key_count(), 0);
        assert!(e.can_redo());
        assert!(e.redo());
        assert_eq!(e.doc.key_count(), 1);
    }

    #[test]
    fn deleting_and_interpolation_changes_are_undoable() {
        let mut e = editor();
        e.seek_frame(6);
        assert!(e.add_key());
        let (ci, _) = e.selected_key.unwrap();
        assert!(e.delete_key());
        assert!(e.undo());
        assert_eq!(e.doc.channels[ci].keys.len(), 1);
        e.selected_key = Some((ci, 0));
        assert!(e.set_selected_interp(Interp::Step));
        assert_eq!(e.doc.channels[ci].keys[0].interp, Interp::Step);
        assert!(e.undo());
        assert_eq!(e.doc.channels[ci].keys[0].interp, Interp::Spline);
    }

    #[test]
    fn a_new_edit_drops_the_redo_stack() {
        let mut e = editor();
        e.seek_frame(3);
        assert!(e.add_key());
        assert!(e.undo());
        e.seek_frame(9);
        assert!(e.add_key());
        assert!(!e.can_redo());
    }

    #[test]
    fn the_curve_tab_edits_a_key_vertically() {
        use crate::curve;
        let mut e = editor();
        e.seek_frame(0);
        assert!(e.add_key());
        e.seek_frame(24);
        assert!(e.add_key());
        let (ci, _) = e.selected_key.unwrap();
        let f = e.frame();
        let (_, tab) = *f
            .tabs
            .iter()
            .find(|(t, _)| *t == crate::frame::TimelineTab::Curves)
            .unwrap();
        assert!(e.press((tab.x + 4.0, tab.y + 4.0), Mods::default()));
        assert_eq!(e.tab, crate::frame::TimelineTab::Curves);
        let f = e.frame();
        let x = f.axis.x(0.0);
        let y = f.curve_plot.y + 10.0;
        assert!(e.press((x, y), Mods::default()));
        assert!(e.dragging());
        let before = curve::component(e.doc.channels[ci].keys[0].v, 0).unwrap();
        e.drag_to((x, y + 40.0), Mods::default());
        let after = curve::component(e.doc.channels[ci].keys[0].v, 0).unwrap();
        assert!(after < before, "{before} -> {after}");
        assert!(e.undo());
    }

    #[test]
    fn a_vector_channel_switches_the_edited_axis() {
        use crate::curve;
        let mut e = editor();
        let ci = e
            .doc
            .channel_index(e.doc.scene.camera, Prop::Translation)
            .unwrap();
        e.selected_channel = Some(ci);
        e.seek_frame(0);
        assert!(e.add_key());
        e.seek_frame(24);
        assert!(e.add_key());
        let f = e.frame();
        let (_, tab) = *f
            .tabs
            .iter()
            .find(|(t, _)| *t == crate::frame::TimelineTab::Curves)
            .unwrap();
        e.press((tab.x + 4.0, tab.y + 4.0), Mods::default());
        let f = e.frame();
        let (axis, r) = *f.curve_axes.iter().find(|(a, _)| *a == 1).unwrap();
        assert_eq!(axis, 1);
        assert!(e.press((r.x + 3.0, r.y + 3.0), Mods::default()));
        assert_eq!(e.curve_axis, 1);
        let x = f.axis.x(0.0);
        assert!(e.press((x, f.curve_plot.y + 10.0), Mods::default()));
        let before = curve::component(e.doc.channels[ci].keys[0].v, 1).unwrap();
        e.drag_to((x, f.curve_plot.y + 50.0), Mods::default());
        let after = curve::component(e.doc.channels[ci].keys[0].v, 1).unwrap();
        assert!(after < before, "{before} -> {after}");
        assert_eq!(curve::component(e.doc.channels[ci].keys[0].v, 0), Some(0.0));
    }

    #[test]
    fn the_tree_lists_parents_and_toggles_their_children() {
        let mut e = editor();
        let mut child = crate::document::new_node(3, cinematix_scene::NodeKind::Empty);
        child.parent = Some(cinematix_scene::NodeId(1));
        e.doc.scene.nodes.insert(child.id, child);
        if let Some(camera) = e.doc.scene.nodes.get_mut(&cinematix_scene::NodeId(1)) {
            camera.children.push(cinematix_scene::NodeId(3));
        }
        e.doc.sync_channels();
        e.expanded.insert(cinematix_scene::NodeId(1));
        let f = e.frame();
        assert!(
            f.tree
                .iter()
                .any(|(id, depth, _, _)| *id == cinematix_scene::NodeId(3) && *depth == 1)
        );
        let (id, r) = *f
            .tree_toggle
            .iter()
            .find(|(id, _)| *id == cinematix_scene::NodeId(1))
            .unwrap();
        assert_eq!(id, cinematix_scene::NodeId(1));
        assert!(e.press((r.x + 2.0, r.y + 2.0), Mods::default()));
        assert!(!e.expanded.contains(&cinematix_scene::NodeId(1)));
        let f = e.frame();
        assert!(
            !f.tree
                .iter()
                .any(|(id, _, _, _)| *id == cinematix_scene::NodeId(3))
        );
    }

    #[test]
    fn escape_asks_to_close_and_nothing_else_does() {
        assert_eq!(editor().key(EditorKey::Escape), KeyResult::Close);
        assert_eq!(editor().key(EditorKey::Space), KeyResult::Consumed);
    }
}
