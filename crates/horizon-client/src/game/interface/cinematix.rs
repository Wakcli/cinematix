//! Cinematix's editor screen (`Screen::Cinematix`): the lifecycle, the drawing hook, the input
//! routing and the `/cinematix` and `horizon-ctl cinematix` commands. The editor itself (the
//! document, the frame layout, the drawing) is `cinematix-editor`; this file only connects it to
//! the game's screen, pointer and key dispatch.

use super::{BendDrag, Game, HudBatch, Screen};
use cinematix_editor::{EditorKey, Interp, KeyResult, Mods, Prop, Section};
use glam::{Mat4, Quat, Vec2, Vec3};
use horizon_client::gizmos::{Gizmos, Line};
use horizon_client::{Result, text::Text};
use serde_json::{Value, json};
use std::sync::mpsc::Sender;
use std::time::Duration;
use winit::keyboard::KeyCode;

pub const HELP: &[&str] = &[
    "cinematix open                 open the Cinematix editor (Screen::Cinematix)",
    "cinematix close                close the Cinematix editor back to Playing",
    "cinematix status               report whether the editor is open and where its playhead is",
    "cinematix play|pause           start or stop playback of the open film",
    "cinematix frame N              move the playhead to frame N",
    "cinematix seek SECONDS         move the playhead to a time in seconds",
    "cinematix key                  insert a key on the selected channel at the playhead",
    "cinematix delkey               delete the selected key",
    "cinematix fit                  fit the film's range into the timeline window",
    "cinematix section NAME         switch the inspector (camera, actors, models, lights, world,",
    "                               film, render, project)",
    "cinematix preview film|free    view through the film camera or a detached editor camera",
    "cinematix preview free X Y Z YAW PITCH   place the detached view",
    "cinematix camera player        set the film camera to the player's eye and view angles",
    "cinematix camera pose X Y Z YAW PITCH [FOV]   place the film camera numerically",
    "cinematix interp step|linear|smooth|spline   set the selected key's interpolation",
    "cinematix channel position|rotation|fov|focus|aperture   select the camera channel to key",
    "cinematix world set X Y Z BLOCK [AT]     draw a block there from AT seconds (no AT: always)",
    "cinematix world remove X Y Z [AT]        draw air there",
    "cinematix world restore X Y Z [AT]       stop overriding that position",
    "cinematix world clear|list|status        drop every op, list them, or report the active set",
    "cinematix save [PATH]          write the film as .cmx (default: <game dir>/cinematix/film.cmx)",
    "cinematix load PATH            replace the film with a .cmx",
    "cinematix new                  start an empty film",
];

/// What `Game::open_cinematix_screen` does over `screen` (`blocked`: an overlay, the title
/// screen, no window or no player, the world loading): `None` opens it, `Some` answers without
/// changing anything (true: already open).
pub(super) fn cinematix_screen_opens(screen: Screen, blocked: bool) -> Option<bool> {
    if screen == Screen::Cinematix {
        return Some(true);
    }
    if screen != Screen::Playing || blocked {
        return Some(false);
    }
    None
}

/// Where `cinematix save` writes without a path: the game's own folder, never the repository.
fn film_path() -> std::path::PathBuf {
    horizon_client::logging::player_dir()
        .join("cinematix")
        .join("film.cmx")
}

fn editor_key(key: KeyCode) -> Option<EditorKey> {
    Some(match key {
        KeyCode::Space => EditorKey::Space,
        KeyCode::KeyK => EditorKey::Keyframe,
        KeyCode::Delete | KeyCode::Backspace => EditorKey::Delete,
        KeyCode::ArrowLeft => EditorKey::Left,
        KeyCode::ArrowRight => EditorKey::Right,
        KeyCode::Home => EditorKey::Home,
        KeyCode::End => EditorKey::End,
        KeyCode::Equal | KeyCode::NumpadAdd => EditorKey::ZoomIn,
        KeyCode::Minus | KeyCode::NumpadSubtract => EditorKey::ZoomOut,
        KeyCode::KeyF => EditorKey::Fit,
        KeyCode::Escape => EditorKey::Escape,
        _ => return None,
    })
}

fn section_named(name: &str) -> Option<Section> {
    Section::ALL.into_iter().find(|s| s.name() == name)
}

impl Game<'_> {
    /// Whether the editor is up.
    pub(super) fn cinematix_open(&self) -> bool {
        self.screen == Screen::Cinematix
    }

    /// Opens the Cinematix editor (`Screen::Cinematix`) over the world. The mouse is freed and
    /// the world keeps running, as with Studio; Escape closes back to `Screen::Playing`.
    pub(super) fn open_cinematix_screen(&mut self) -> bool {
        let blocked = self.title.is_some()
            || self.renderer.is_none()
            || self.lan_acting.is_some()
            || self.player.is_none()
            || self.screen_over_playing()
            || self.remote_loading();
        if let Some(open) = cinematix_screen_opens(self.screen, blocked) {
            return open;
        }
        let pose = (self.view.eye, self.yaw, self.pitch);
        self.screen = Screen::Cinematix;
        let position = Vec3::new(pose.0[0] as f32, pose.0[1] as f32, pose.0[2] as f32);
        self.cinematix.take_pose(position, pose.1, pose.2, false);
        self.cinematix_look = None;
        self.cinematix_bend = None;
        let _ = self.sync_cinematix_world();
        self.keys.clear();
        self.left = false;
        self.right = false;
        self.mining = None;
        self.capture(false);
        true
    }

    /// Closes the editor, if open, returning to `Screen::Playing`. The document stays, so the
    /// film survives closing and reopening the screen.
    pub(super) fn close_cinematix_screen(&mut self) {
        if self.screen == Screen::Cinematix {
            self.cinematix.playing = false;
            self.cinematix_look = None;
            self.cinematix_bend = None;
            self.clear_cinematix_world();
            self.screen = Screen::Playing;
            self.capture(true);
        }
    }

    fn cinematix_mods(&self) -> Mods {
        Mods {
            shift: self.modifiers.shift_key(),
            ctrl: self.modifiers.control_key(),
            alt: self.modifiers.alt_key(),
        }
    }

    /// The editor's frame: the whole window, drawn over the world.
    pub(super) fn draw_cinematix(&mut self, batch: &mut HudBatch) {
        if self.screen != Screen::Cinematix {
            return;
        }
        self.cinematix.resize(batch.width, batch.height);
        self.draw_cinematix_path(batch);
        cinematix_editor::draw(&self.cinematix, batch, &self.gui.atlas, self.cursor);
    }

    /// A key while the editor is up: the editor's shortcuts, and nothing reaches the world.
    pub(super) fn cinematix_key(&mut self, key: KeyCode) {
        if self.cinematix_export.is_some() {
            return;
        }
        if key == KeyCode::F8 {
            self.close_cinematix_screen();
            return;
        }
        let Some(editor_key) = editor_key(key) else {
            return;
        };
        let mods = self.cinematix_mods();
        if mods.ctrl && key == KeyCode::KeyZ {
            if mods.shift {
                self.cinematix.redo();
            } else {
                self.cinematix.undo();
            }
            return;
        }
        if mods.ctrl && key == KeyCode::KeyY {
            self.cinematix.redo();
            return;
        }
        if mods.shift && key == KeyCode::KeyF {
            self.cinematix_bend = None;
            self.cinematix.frame_bend_segment();
            return;
        }
        let result = match editor_key {
            EditorKey::Left | EditorKey::Right if mods.shift => {
                let frames = if editor_key == EditorKey::Left {
                    -10
                } else {
                    10
                };
                self.cinematix.step(frames);
                KeyResult::Consumed
            }
            other => self.cinematix.key(other),
        };
        if result == KeyResult::Close {
            self.close_cinematix_screen();
        }
    }

    /// A mouse button while the editor is open: the editor takes every press and release, so a
    /// click can never reach the world behind it. False only while another screen is open.
    pub(super) fn cinematix_mouse_input(
        &mut self,
        pressed: bool,
        button: winit::event::MouseButton,
    ) -> bool {
        if self.screen != Screen::Cinematix {
            return false;
        }
        if self.cinematix_export.is_some() {
            return true;
        }
        if button == winit::event::MouseButton::Right {
            if pressed && self.cinematix.right_press(self.cursor) {
                return true;
            }
            self.cinematix_look = pressed.then_some(self.cursor);
            return true;
        }
        if button == winit::event::MouseButton::Left {
            if pressed {
                let mods = self.cinematix_mods();
                self.cinematix_bend = self.pick_cinematix_bend();
                if self.cinematix_bend.is_none() {
                    self.cinematix.press(self.cursor, mods);
                } else {
                    self.cinematix.playing = false;
                }
            } else {
                self.cinematix_bend = None;
                self.cinematix.release(self.cursor);
            }
        }
        true
    }

    fn bend_handles(&self, projection: PathProjection) -> impl Iterator<Item = ProjectedBend> + '_ {
        let ed = &self.cinematix;
        let frame = ed.frame();
        ed.camera_bend_handles()
            .filter_map(move |(segment, midpoint)| {
                if !ed.preview_free {
                    return None;
                }
                let local = projection.local(midpoint.to_array().map(f64::from));
                let pixel = projection.project(local)?;
                if !frame.viewport.contains(pixel.x, pixel.y)
                    || frame.inspector.contains(pixel.x, pixel.y)
                {
                    return None;
                }
                Some(ProjectedBend {
                    segment,
                    midpoint,
                    pixel,
                    units_per_pixel: -local.z / projection.focal,
                })
            })
    }

    fn pick_cinematix_bend(&self) -> Option<BendDrag> {
        let ed = &self.cinematix;
        if !ed.can_pick_bend(self.cursor) {
            return None;
        }
        let projection = PathProjection::new(ed)?;
        let handle = closest_bend(self.bend_handles(projection), Vec2::from(self.cursor))?;
        let inverse = projection.view.transpose();
        Some(BendDrag {
            segment: handle.segment,
            cursor: self.cursor,
            midpoint: handle.midpoint,
            right: inverse.transform_vector3(Vec3::X),
            up: inverse.transform_vector3(Vec3::Y),
            units_per_pixel: handle.units_per_pixel,
            moved: false,
        })
    }

    fn draw_cinematix_path(&self, batch: &mut HudBatch) {
        let Some(projection) = PathProjection::new(&self.cinematix) else {
            return;
        };
        let mut g = Gizmos::default();
        self.cinematix_camera_path(&mut g);
        let atlas = &self.gui.atlas;
        let start = batch.vertices.len();
        for line in g.on_top {
            let Some((a, b)) = projection.line(line.start, line.end) else {
                continue;
            };
            let delta = b - a;
            let len = delta.length();
            if len < 0.01 || !len.is_finite() {
                continue;
            }
            let normal = Vec2::new(-delta.y, delta.x) * (line.width * 0.5 / len);
            batch.fill_quad(
                atlas,
                [a - normal, b - normal, b + normal, a + normal].map(|p| p.to_array()),
                hud_colour(line.color),
            );
        }
        let handles = self.bend_handles(projection).collect::<Vec<_>>();
        let hovered = if let Some(drag) = &self.cinematix_bend {
            Some(drag.segment)
        } else if self.cinematix.can_pick_bend(self.cursor) {
            closest_bend(handles.iter().copied(), Vec2::from(self.cursor)).map(|h| h.segment)
        } else {
            None
        };
        // Draw the hovered or dragged handle last when markers overlap.
        for handle in handles
            .iter()
            .filter(|h| Some(h.segment) != hovered)
            .chain(handles.iter().filter(|h| Some(h.segment) == hovered))
        {
            let pixel = handle.pixel;
            let highlighted = Some(handle.segment) == hovered;
            let diamond = |r: f32| {
                [
                    pixel - Vec2::X * r,
                    pixel - Vec2::Y * r,
                    pixel + Vec2::X * r,
                    pixel + Vec2::Y * r,
                ]
                .map(|p| p.to_array())
            };
            batch.fill_quad(atlas, diamond(8.0), 0xFF18_1818);
            batch.fill_quad(
                atlas,
                diamond(6.0),
                if highlighted {
                    0xFFFF_FFFF
                } else {
                    hud_colour(PATH_FULL)
                },
            );
        }
        let r = self.cinematix.frame().viewport;
        batch.clip_since(start, r.x, r.y, r.x + r.w, r.y + r.h);
    }

    /// The pointer moved while the editor is up: an active drag follows it.
    pub(super) fn cinematix_mouse_dragged(&mut self) -> bool {
        if self.screen != Screen::Cinematix {
            return false;
        }
        if self.cinematix_export.is_some() {
            return true;
        }
        if let Some(drag) = &mut self.cinematix_bend {
            let dx = self.cursor.0 - drag.cursor.0;
            let dy = self.cursor.1 - drag.cursor.1;
            let midpoint = drag.midpoint + (drag.right * dx - drag.up * dy) * drag.units_per_pixel;
            let segment = drag.segment;
            if !drag.moved && dx.hypot(dy) < 2.0 {
                return true;
            }
            if !drag.moved {
                self.cinematix.checkpoint();
                drag.moved = true;
            }
            self.cinematix.bend_camera_segment(segment, midpoint);
            return true;
        }
        if let Some(last) = self.cinematix_look {
            let (dx, dy) = (self.cursor.0 - last.0, self.cursor.1 - last.1);
            self.cinematix_look = Some(self.cursor);
            let sensitivity = if self.has_control_down() { 0.04 } else { 0.15 };
            if (dx != 0.0 || dy != 0.0) && !self.cinematix.preview_free {
                self.cinematix.toggle_free_view();
            }
            if self.cinematix.preview_free {
                self.cinematix.free_look(dx * sensitivity, dy * sensitivity);
            }
        }
        let mods = self.cinematix_mods();
        self.cinematix.drag_to(self.cursor, mods);
        true
    }

    /// Wheel scrolls channels, Ctrl zooms the time axis, and Shift pans it.
    pub(super) fn cinematix_wheel(&mut self, ticks: f32) -> bool {
        if self.screen != Screen::Cinematix {
            return false;
        }
        if self.cinematix_export.is_none() {
            let mods = self.cinematix_mods();
            self.cinematix.wheel(ticks, mods);
        }
        true
    }

    /// Playback advances once per drawn frame, while the editor is open and playing.
    pub(super) fn cinematix_tick(&mut self, elapsed: Duration) {
        if self.screen != Screen::Cinematix || self.cinematix_export.is_some() {
            return;
        }
        match self.cinematix.take_request() {
            Some(cinematix_editor::Request::Save) => {
                let path = film_path();
                let stamp = cinematix_editor::Stamp {
                    app_version: env!("CARGO_PKG_VERSION"),
                    saved_unix: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or_default(),
                };
                match cinematix_editor::save(&path, &self.cinematix.doc, stamp) {
                    Ok(()) => self.system_message(Text::tr(
                        "horizon.cinematix.saved",
                        &[&path.display().to_string()],
                    )),
                    Err(error) => {
                        self.system_message(Text::tr("horizon.cinematix.render_error", &[&error]))
                    }
                }
            }
            Some(cinematix_editor::Request::Render) => {
                let fps = self.cinematix.fps().round().max(1.0) as u32;
                let (r0, r1) = self.cinematix.range();
                let start = (r0.max(0.0) * f64::from(fps)).round() as u32;
                let end = ((r1.max(r0 + 0.001) * f64::from(fps)).round() as u32).max(start + 1);
                let (mut width, mut height) = self.cinematix.doc.scene.meta.resolution;
                width = (width.max(16) / 2 * 2).min(8192);
                height = (height.max(16) / 2 * 2).min(8192);
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or_default();
                let path = film_path().with_file_name(format!("render-{stamp}.mp4"));
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match self.start_cinematix_render(
                    start,
                    end,
                    fps,
                    width,
                    height,
                    &path.display().to_string(),
                    false,
                    None,
                ) {
                    Ok(_) => {}
                    Err(error) => {
                        self.system_message(Text::tr("horizon.cinematix.render_error", &[&error]))
                    }
                }
            }
            None => {}
        }
        let dt = elapsed.as_secs_f32().min(0.1);
        self.cinematix.advance(f64::from(dt));
        let _ = self.sync_cinematix_world();
        if dt <= 0.0 {
            return;
        }
        let held = |key| f32::from(u8::from(self.keys.raw(&key)));
        let forward = held(KeyCode::KeyW) - held(KeyCode::KeyS);
        let strafe = held(KeyCode::KeyD) - held(KeyCode::KeyA);
        let vertical = held(KeyCode::KeyE) - held(KeyCode::KeyQ);
        if forward == 0.0 && strafe == 0.0 && vertical == 0.0 {
            return;
        }
        let Some(rotation) = self.cinematix.preview_rotation() else {
            return;
        };
        let dir = rotation * Vec3::new(strafe, 0.0, -forward) + Vec3::Y * vertical;
        if dir.length_squared() < 1e-6 {
            return;
        }
        // `mapped_key` already records the bound movement keys in `self.keys`, releases included,
        // so this only reads: a tap cannot leave the camera drifting.
        const SPEED: f32 = 8.0;
        let delta = dir.normalize() * SPEED * dt;
        if !self.cinematix.preview_free {
            self.cinematix.toggle_free_view();
        }
        self.cinematix.free_nudge(delta);
    }

    /// The film camera's path and the playback camera marker, drawn while the editor is open.
    /// Only the current segment and its immediate neighbours are drawn, as Flashback does: the
    /// whole film's path would be noise, and the current segment is what a keyframe edit changes.
    fn cinematix_camera_path(&self, g: &mut Gizmos) {
        if self.screen != Screen::Cinematix || self.cinematix_export.is_some() {
            return;
        }
        let ed = &self.cinematix;
        let times = ed.camera_path_times();
        if times.is_empty() {
            return;
        }
        let t = ed.time();
        let active = ed.bend_segment();
        let last = times.iter().rposition(|&k| k <= t + 1e-9);
        let next = times.iter().position(|&k| k > t + 1e-9);
        let selected = ed.selected_camera_key_time();
        let mut nearby = Vec::new();
        if let Some(li) = last {
            if li > 0 {
                nearby.push(li - 1);
            }
            nearby.push(li);
        }
        if let Some(ni) = next {
            nearby.push(ni);
            if ni + 1 < times.len() {
                nearby.push(ni + 1);
            }
        }
        nearby.sort_unstable();
        nearby.dedup();

        if ed.preview_free {
            // From the detached view the whole path is readable; the segment under the playhead
            // stays bright and the rest fades.
            for i in 1..times.len() {
                let current = active == Some((times[i - 1], times[i]));
                camera_path_segment(
                    g,
                    ed,
                    times[i - 1],
                    times[i],
                    if current { PATH_FULL } else { PATH_FADE },
                );
            }
            for (i, &at) in times.iter().enumerate() {
                let Some((position, rotation)) = ed.camera_at(at) else {
                    continue;
                };
                let colour = if selected.is_some_and(|s| (s - at).abs() < 1e-9) {
                    PATH_SELECTED
                } else if nearby.contains(&i) {
                    PATH_KEY
                } else {
                    PATH_FADE
                };
                if nearby.contains(&i) {
                    camera_frustum(g, position, rotation, ed.fov_at(at), colour);
                }
                key_cross(g, position, colour);
            }
            if let Some((position, rotation)) = ed.camera_at(t) {
                camera_frustum(g, position, rotation, ed.fov_at(t), PATH_CURRENT);
            }
        } else {
            for &i in &nearby {
                let Some((position, rotation)) = ed.camera_at(times[i]) else {
                    continue;
                };
                let colour = if selected.is_some_and(|s| (s - times[i]).abs() < 1e-9) {
                    PATH_SELECTED
                } else if Some(i) == last || Some(i) == next {
                    PATH_KEY
                } else {
                    PATH_FADE
                };
                camera_frustum(g, position, rotation, ed.fov_at(times[i]), colour);
                key_cross(g, position, colour);
            }
            // Film view: the whole path stays faintly visible, and the segment under the
            // playhead is bright. Otherwise the line vanishes whenever that segment points
            // away from the camera.
            for i in 1..times.len() {
                let current = active == Some((times[i - 1], times[i]));
                camera_path_segment(
                    g,
                    ed,
                    times[i - 1],
                    times[i],
                    if current { PATH_FULL } else { PATH_FADE },
                );
            }
        }
    }

    /// `/cinematix ...`: the open film's transport from chat.
    pub(super) fn cinematix_command(&mut self, command: &str) -> bool {
        let Some(rest) = command.strip_prefix("cinematix") else {
            return false;
        };
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            return false;
        }
        let words: Vec<&str> = rest.split_whitespace().collect();
        if words.is_empty() {
            self.system_message(Text::translate("horizon.cinematix.usage", &[]));
            return true;
        }
        let reply = match self.cinematix_control(&words, None) {
            Ok(Some(value)) => value,
            Ok(None) => json!({ "ok": false, "error": "unknown cinematix subcommand" }),
            Err(error) => json!({ "ok": false, "error": error.to_string() }),
        };
        if reply["ok"].as_bool() == Some(true) {
            let key = match words[0] {
                "open" => "horizon.cinematix.opened",
                "close" => "horizon.cinematix.closed",
                "play" => "horizon.cinematix.playing",
                "pause" => "horizon.cinematix.paused",
                "key" => "horizon.cinematix.key_added",
                "delkey" => "horizon.cinematix.key_deleted",
                "frame" | "seek" => "horizon.cinematix.seeked",
                "save" => "horizon.cinematix.saved",
                "load" => "horizon.cinematix.loaded",
                "new" => "horizon.cinematix.new",
                "interp" => "horizon.cinematix.interp",
                "channel" => "horizon.cinematix.channel",
                "world" => "horizon.cinematix.world_changed",
                "env" => "horizon.cinematix.environment",
                "render" => "horizon.cinematix.render_started",
                "camera" if words.get(1) == Some(&"pose") => "horizon.cinematix.pose",
                "camera"
                    if matches!(words.get(1), Some(&"orbit" | &"dolly" | &"rail" | &"shake")) =>
                {
                    "horizon.cinematix.camera"
                }
                _ => "horizon.cinematix.status",
            };
            if words[0] == "render" {
                self.system_message(Text::tr(
                    key,
                    &[
                        &reply["frames"].as_u64().unwrap_or_default(),
                        &reply["path"].as_str().unwrap_or_default(),
                    ],
                ));
                return true;
            }
            let detail = match (words[0], key) {
                ("save" | "load", _) => reply["path"].as_str().unwrap_or_default().to_owned(),
                ("env", _) => reply["detail"].as_str().unwrap_or_default().to_owned(),
                _ => reply["frame"].as_i64().unwrap_or_default().to_string(),
            };
            self.system_message(Text::tr(key, &[&detail]));
        } else {
            let error = reply["error"].as_str().unwrap_or("unknown error");
            self.system_message(Text::tr("horizon.cinematix.error", &[&error]));
        }
        true
    }

    /// `horizon-ctl cinematix ...` and `/cinematix ...`: one command surface for the headless
    /// harness and chat.
    pub(super) fn cinematix_control(
        &mut self,
        words: &[&str],
        reply: Option<&Sender<String>>,
    ) -> Result<Option<Value>> {
        let Some(sub) = words.first().copied() else {
            return Ok(Some(json!({ "ok": false, "error": "missing subcommand" })));
        };
        match sub {
            "open" => {
                let open = self.open_cinematix_screen();
                Ok(Some(json!({
                    "ok": true, "open": open,
                    "screen": if self.cinematix_open() { "cinematix" } else { self.screen.id() },
                })))
            }
            "close" => {
                let was_open = self.cinematix_open();
                self.close_cinematix_screen();
                Ok(Some(json!({
                    "ok": true, "was_open": was_open,
                    "screen": if self.cinematix_open() { "cinematix" } else { self.screen.id() },
                })))
            }
            "status" => {
                let ed = &self.cinematix;
                let (r0, r1) = ed.range();
                let (w, h) = ed.doc.scene.meta.resolution;
                Ok(Some(json!({
                    "ok": true,
                    "open": self.cinematix_open(),
                    "screen": self.screen.id(),
                    "frame": ed.frame_number(),
                    "time": ed.time(),
                    "fps": ed.fps(),
                    "playing": ed.playing,
                    "looping": ed.looping,
                    "section": ed.section.name(),
                    "channels": ed.doc.channels.len(),
                    "keys": ed.doc.key_count(),
                    "nodes": ed.doc.scene.nodes.len(),
                    "range": [r0, r1],
                    "render": self.cinematix_export.as_ref().map(|r| r.progress()),
                    "resolution": [w, h],
                    "camera": ed.camera_status().map(|(position, yaw, pitch, fov)| json!({
                        "position": position, "yaw": yaw, "pitch": pitch, "fov": fov,
                    })),
                })))
            }
            "play" => {
                self.cinematix.playing = true;
                Ok(Some(json!({ "ok": true, "playing": true })))
            }
            "pause" => {
                self.cinematix.playing = false;
                Ok(Some(json!({ "ok": true, "playing": false })))
            }
            "frame" => {
                let n = match words.get(1).and_then(|w| w.parse::<i64>().ok()) {
                    Some(n) => n,
                    None => {
                        return Ok(Some(
                            json!({ "ok": false, "error": "frame needs a number" }),
                        ));
                    }
                };
                self.cinematix.seek_frame(n);
                let _ = self.sync_cinematix_world();
                Ok(Some(
                    json!({ "ok": true, "frame": self.cinematix.frame_number(), "time": self.cinematix.time() }),
                ))
            }
            "seek" => {
                let t = match words.get(1).and_then(|w| w.parse::<f64>().ok()) {
                    Some(t) => t,
                    None => return Ok(Some(json!({ "ok": false, "error": "seek needs seconds" }))),
                };
                self.cinematix.seek(t);
                let _ = self.sync_cinematix_world();
                Ok(Some(
                    json!({ "ok": true, "frame": self.cinematix.frame_number(), "time": self.cinematix.time() }),
                ))
            }
            "key" => {
                let added = self.cinematix.add_key();
                let keys = self.cinematix.doc.key_count();
                let frame = self.cinematix.frame_number();
                Ok(Some(if added {
                    json!({ "ok": true, "frame": frame, "keys": keys })
                } else {
                    json!({ "ok": false, "error": "no channel to key" })
                }))
            }
            "delkey" => {
                let deleted = self.cinematix.delete_key();
                let keys = self.cinematix.doc.key_count();
                Ok(Some(if deleted {
                    json!({ "ok": true, "keys": keys })
                } else {
                    json!({ "ok": false, "error": "no key selected" })
                }))
            }
            "fit" => {
                self.cinematix.fit();
                let (t0, t1) = self.cinematix.window();
                Ok(Some(json!({ "ok": true, "window": [t0, t1] })))
            }
            "save" => {
                let path = words
                    .get(1)
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(film_path);
                let stamp = cinematix_editor::Stamp {
                    app_version: env!("CARGO_PKG_VERSION"),
                    saved_unix: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or_default(),
                };
                match cinematix_editor::save(&path, &self.cinematix.doc, stamp) {
                    Ok(()) => Ok(Some(json!({
                        "ok": true, "path": path.display().to_string(),
                        "channels": self.cinematix.doc.channels.len(),
                        "keys": self.cinematix.doc.key_count(),
                    }))),
                    Err(error) => Ok(Some(json!({ "ok": false, "error": error }))),
                }
            }
            "load" => {
                let Some(path) = words.get(1).map(std::path::PathBuf::from) else {
                    return Ok(Some(json!({ "ok": false, "error": "load needs a path" })));
                };
                match cinematix_editor::load(&path) {
                    Ok(doc) => {
                        let keys = doc.key_count();
                        let channels = doc.channels.len();
                        self.cinematix.adopt_document(doc);
                        Ok(Some(json!({
                            "ok": true, "path": path.display().to_string(),
                            "channels": channels, "keys": keys,
                        })))
                    }
                    Err(error) => Ok(Some(json!({ "ok": false, "error": error }))),
                }
            }
            "render" => {
                if words.get(1) == Some(&"cancel") {
                    let running = self.cinematix_export.is_some();
                    self.end_cinematix_render(Err("cancelled".into()));
                    return Ok(Some(json!({ "ok": true, "cancelled": running })));
                }
                let numbers: Option<Vec<u32>> = words
                    .get(1..6)
                    .unwrap_or_default()
                    .iter()
                    .map(|w| w.parse().ok())
                    .collect();
                let Some(v) = numbers.filter(|v| v.len() == 5) else {
                    return Ok(Some(json!({
                        "ok": false,
                        "error": "render needs START END FPS WIDTH HEIGHT PATH",
                    })));
                };
                let Some(path) = words.get(6) else {
                    return Ok(Some(json!({
                        "ok": false,
                        "error": "render needs START END FPS WIDTH HEIGHT PATH",
                    })));
                };
                let resume = words.get(7) == Some(&"--resume");
                return self
                    .start_cinematix_render(v[0], v[1], v[2], v[3], v[4], path, resume, reply);
            }
            "new" => {
                self.cinematix
                    .adopt_document(cinematix_editor::Document::new());
                if self.cinematix_open() {
                    let eye = self.view.eye;
                    let position = Vec3::new(eye[0] as f32, eye[1] as f32, eye[2] as f32);
                    self.cinematix
                        .take_pose(position, self.yaw, self.pitch, false);
                }
                Ok(Some(
                    json!({ "ok": true, "channels": self.cinematix.doc.channels.len() }),
                ))
            }
            "camera" => match words.get(1).copied() {
                Some("player") => {
                    let eye = self.view.eye;
                    let position = Vec3::new(eye[0] as f32, eye[1] as f32, eye[2] as f32);
                    let took = self
                        .cinematix
                        .take_pose(position, self.yaw, self.pitch, true);
                    Ok(Some(json!({
                        "ok": true, "took": took,
                        "camera": self.cinematix.camera_status().map(|(p, yaw, pitch, fov)| json!({
                            "position": p, "yaw": yaw, "pitch": pitch, "fov": fov,
                        })),
                    })))
                }
                Some("pose") => {
                    let values: Option<Vec<f32>> =
                        words[2..].iter().map(|w| w.parse::<f32>().ok()).collect();
                    let Some(v) = values.filter(|v| v.len() == 5 || v.len() == 6) else {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "camera pose needs X Y Z YAW PITCH and optionally FOV",
                        })));
                    };
                    let position = Vec3::new(v[0], v[1], v[2]);
                    self.cinematix.take_pose(position, v[3], v[4], true);
                    if let Some(fov) = v.get(5) {
                        self.cinematix.set_fov(*fov);
                    }
                    Ok(Some(json!({
                        "ok": true, "frame": self.cinematix.frame_number(),
                        "camera": self.cinematix.camera_status().map(|(p, yaw, pitch, fov)| json!({
                            "position": p, "yaw": yaw, "pitch": pitch, "fov": fov,
                        })),
                    })))
                }
                Some("orbit") => {
                    let values: Option<Vec<f32>> =
                        words[2..].iter().map(|w| w.parse::<f32>().ok()).collect();
                    let Some(v) = values.filter(|v| v.len() == 5 || v.len() == 6) else {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "camera orbit needs CX CY CZ RADIUS FRAMES [HEIGHT]",
                        })));
                    };
                    let center = Vec3::new(v[0], v[1], v[2]);
                    let frames = v[4].round().max(1.0) as u32;
                    let ok = self
                        .cinematix
                        .orbit(center, v[3], frames, v.get(5).copied());
                    Ok(Some(json!({
                        "ok": ok, "frame": self.cinematix.frame_number(),
                        "keys": self.cinematix.doc.key_count(),
                    })))
                }
                Some("dolly") => {
                    let values: Option<Vec<f32>> =
                        words[2..].iter().map(|w| w.parse::<f32>().ok()).collect();
                    let Some(v) = values.filter(|v| v.len() == 3 || v.len() == 4) else {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "camera dolly needs DX DY DZ [FRAMES]",
                        })));
                    };
                    let delta = Vec3::new(v[0], v[1], v[2]);
                    let frames = v.get(3).map_or(24, |f| f.round().max(1.0) as u32);
                    let ok = self.cinematix.dolly(delta, frames);
                    Ok(Some(json!({
                        "ok": ok, "frame": self.cinematix.frame_number(),
                        "keys": self.cinematix.doc.key_count(),
                    })))
                }
                Some("rail") => {
                    let frames = words.get(2).and_then(|w| w.parse::<u32>().ok());
                    let values: Option<Vec<f32>> =
                        words[3..].iter().map(|w| w.parse::<f32>().ok()).collect();
                    let Some((frames, v)) = frames.zip(values) else {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "camera rail needs FRAMES then X Y Z for every point",
                        })));
                    };
                    if v.len() < 6 || v.len() % 3 != 0 {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "camera rail needs at least two X Y Z points",
                        })));
                    }
                    let points: Vec<Vec3> = v
                        .chunks_exact(3)
                        .map(|p| Vec3::new(p[0], p[1], p[2]))
                        .collect();
                    let ok = self.cinematix.rail(&points, frames);
                    Ok(Some(json!({
                        "ok": ok, "frame": self.cinematix.frame_number(),
                        "keys": self.cinematix.doc.key_count(),
                    })))
                }
                Some("shake") => {
                    let values: Option<Vec<f32>> =
                        words[2..].iter().map(|w| w.parse::<f32>().ok()).collect();
                    let Some(v) = values.filter(|v| v.len() == 2) else {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "camera shake needs AMPLITUDE_DEGREES FREQUENCY_HZ",
                        })));
                    };
                    let fx = &mut self.cinematix.doc.scene.meta.camera_fx;
                    fx.shake_amplitude = v[0].clamp(0.0, 10.0);
                    fx.shake_frequency = v[1].clamp(0.0, 10.0);
                    self.cinematix.revision = self.cinematix.revision.wrapping_add(1);
                    Ok(Some(json!({
                        "ok": true,
                        "shake": [fx.shake_amplitude, fx.shake_frequency],
                    })))
                }
                _ => Ok(Some(json!({
                    "ok": false,
                    "error": "camera needs player, pose, orbit, dolly, rail or shake",
                }))),
            },
            "meta" => {
                let meta = &mut self.cinematix.doc.scene.meta;
                match words.get(1).copied() {
                    None | Some("status") => {}
                    Some("range") => {
                        let values: Option<Vec<f64>> = words
                            .get(2..4)
                            .unwrap_or_default()
                            .iter()
                            .map(|w| w.parse().ok())
                            .collect();
                        let Some(v) = values.filter(|v| {
                            v.len() == 2 && v[0].is_finite() && v[1].is_finite() && v[1] > v[0]
                        }) else {
                            return Ok(Some(json!({
                                "ok": false,
                                "error": "meta range needs START END with END after START",
                            })));
                        };
                        meta.range = (v[0], v[1]);
                        meta.open_ended = false;
                    }
                    Some("fps") => {
                        let Some(v) = words.get(2).and_then(|w| w.parse::<f32>().ok()) else {
                            return Ok(Some(json!({
                                "ok": false, "error": "meta fps needs a number",
                            })));
                        };
                        meta.fps = v.clamp(1.0, 240.0);
                    }
                    Some("size") => {
                        let values: Option<Vec<u32>> = words
                            .get(2..4)
                            .unwrap_or_default()
                            .iter()
                            .map(|w| w.parse().ok())
                            .collect();
                        let Some(v) = values.filter(|v| v.len() == 2) else {
                            return Ok(Some(json!({
                                "ok": false, "error": "meta size needs WIDTH HEIGHT",
                            })));
                        };
                        if !(16..=8192).contains(&v[0])
                            || !(16..=8192).contains(&v[1])
                            || v[0] % 2 != 0
                            || v[1] % 2 != 0
                        {
                            return Ok(Some(json!({
                                "ok": false,
                                "error": "meta size must be even, 16 to 8192",
                            })));
                        }
                        meta.resolution = (v[0], v[1]);
                    }
                    Some(other) => {
                        return Ok(Some(json!({
                            "ok": false, "error": format!("unknown meta field {other}"),
                        })));
                    }
                }
                Ok(Some(json!({
                    "ok": true, "range": meta.range, "open_ended": meta.open_ended, "fps": meta.fps,
                    "size": meta.resolution, "name": meta.name, "author": meta.author,
                })))
            }
            "undo" => {
                let ok = self.cinematix.undo();
                self.cinematix_world_time = None;
                Ok(Some(json!({
                    "ok": ok, "keys": self.cinematix.doc.key_count(),
                    "can_undo": self.cinematix.can_undo(),
                    "can_redo": self.cinematix.can_redo(),
                })))
            }
            "redo" => {
                let ok = self.cinematix.redo();
                self.cinematix_world_time = None;
                Ok(Some(json!({
                    "ok": ok, "keys": self.cinematix.doc.key_count(),
                    "can_undo": self.cinematix.can_undo(),
                    "can_redo": self.cinematix.can_redo(),
                })))
            }
            "preview" => {
                let ed = &mut self.cinematix;
                match words.get(1).copied() {
                    None | Some("status") => {}
                    Some("film") => ed.preview_free = false,
                    Some("free") => {
                        if !ed.preview_free {
                            ed.toggle_free_view();
                        }
                        let values: Option<Vec<f32>> = words
                            .get(2..7)
                            .unwrap_or_default()
                            .iter()
                            .map(|w| w.parse().ok())
                            .collect();
                        if let Some(v) = values.filter(|v| v.len() == 5) {
                            ed.free_pos = Vec3::new(v[0], v[1], v[2]);
                            ed.free_yaw = v[3];
                            ed.free_pitch = v[4].clamp(-89.9, 89.9);
                        }
                    }
                    Some(other) => {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": format!("unknown preview mode {other}"),
                        })));
                    }
                }
                Ok(Some(json!({
                    "ok": true,
                    "free": ed.preview_free,
                    "position": [ed.free_pos.x, ed.free_pos.y, ed.free_pos.z],
                    "yaw": ed.free_yaw,
                    "pitch": ed.free_pitch,
                })))
            }
            "env" => {
                let env = &mut self.cinematix.doc.scene.meta.environment;
                let num = |i: usize, lo: f32, hi: f32| -> Result<f32> {
                    words
                        .get(i)
                        .ok_or_else(|| "env needs a value".into())
                        .and_then(|w| {
                            w.parse::<f32>()
                                .map_err(|_| format!("{w}: not a number").into())
                        })
                        .map(|v| v.clamp(lo, hi))
                };
                let detail = match words.get(1).copied() {
                    None | Some("status") => {
                        return Ok(Some(json!({
                            "ok": true, "time": env.time, "rain": env.rain,
                            "thunder": env.thunder, "fog_start": env.fog_start,
                            "fog_end": env.fog_end, "sky_color": env.sky_color,
                            "fog_color": env.fog_color,
                        })));
                    }
                    Some("clear") => {
                        *env = Default::default();
                        "cleared".to_owned()
                    }
                    Some("time") => {
                        let v = num(2, 0.0, 24_000.0)?;
                        env.time = Some(v);
                        format!("time {v:.0}")
                    }
                    Some("rain") => {
                        let v = num(2, 0.0, 1.0)?;
                        env.rain = Some(v);
                        format!("rain {v:.2}")
                    }
                    Some("thunder") => {
                        let v = num(2, 0.0, 1.0)?;
                        env.thunder = Some(v);
                        format!("thunder {v:.2}")
                    }
                    Some("fog") => {
                        let start = num(2, 0.0, 2048.0)?;
                        let end = num(3, 0.0, 2048.0)?;
                        env.fog_start = Some(start);
                        env.fog_end = Some(end);
                        format!("fog {start:.0}..{end:.0}")
                    }
                    Some("sky") => {
                        let c = [num(2, 0.0, 1.0)?, num(3, 0.0, 1.0)?, num(4, 0.0, 1.0)?];
                        env.sky_color = Some(c);
                        format!("sky {:.2} {:.2} {:.2}", c[0], c[1], c[2])
                    }
                    Some("fogcolor") => {
                        let c = [num(2, 0.0, 1.0)?, num(3, 0.0, 1.0)?, num(4, 0.0, 1.0)?];
                        env.fog_color = Some(c);
                        format!("fogcolor {:.2} {:.2} {:.2}", c[0], c[1], c[2])
                    }
                    Some(other) => {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": format!("unknown env field {other}"),
                        })));
                    }
                };
                Ok(Some(json!({ "ok": true, "detail": detail })))
            }
            "world" => return self.cinematix_world_control(&words[1..]),
            "channel" => {
                let prop = match words.get(1).copied() {
                    Some("position" | "translation") => Prop::Translation,
                    Some("rotation") => Prop::Rotation,
                    Some("fov") => Prop::Fov,
                    Some("focus") => Prop::Focus,
                    Some("aperture") => Prop::Aperture,
                    _ => {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "channel needs position, rotation, fov, focus or aperture",
                        })));
                    }
                };
                match self
                    .cinematix
                    .doc
                    .channel_index(self.cinematix.doc.scene.camera, prop)
                {
                    Some(ci) => {
                        self.cinematix.selected_channel = Some(ci);
                        self.cinematix.selected_key = None;
                        Ok(Some(json!({ "ok": true, "channel": ci })))
                    }
                    None => Ok(Some(json!({ "ok": false, "error": "no such channel" }))),
                }
            }
            "interp" => {
                let interp = match words.get(1).copied() {
                    Some("step") => Interp::Step,
                    Some("linear") => Interp::Linear,
                    Some("smooth") => Interp::Smooth,
                    Some("spline") => Interp::Spline,
                    Some("bezier") => Interp::Bezier {
                        out: [0.42, 0.0],
                        in_: [0.58, 1.0],
                    },
                    _ => {
                        return Ok(Some(json!({
                            "ok": false,
                            "error": "interp needs step, linear, smooth or spline",
                        })));
                    }
                };
                let set = self.cinematix.set_selected_interp(interp);
                Ok(Some(if set {
                    json!({ "ok": true, "interp": words[1] })
                } else {
                    json!({ "ok": false, "error": "no key selected" })
                }))
            }
            "section" => {
                let Some(name) = words.get(1).copied() else {
                    return Ok(Some(
                        json!({ "ok": false, "error": "section needs a name" }),
                    ));
                };
                match section_named(name) {
                    Some(section) => {
                        self.cinematix.section = section;
                        self.cinematix.scroll = 0;
                        Ok(Some(json!({ "ok": true, "section": section.name() })))
                    }
                    None => Ok(Some(json!({ "ok": false, "error": "unknown section" }))),
                }
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_editor_section_has_a_ctl_name() {
        for s in Section::ALL {
            assert_eq!(section_named(s.name()), Some(s), "{}", s.name());
        }
    }

    #[test]
    fn the_gate_opens_only_over_playing_and_answers_when_already_open() {
        assert_eq!(cinematix_screen_opens(Screen::Playing, false), None);
        assert_eq!(cinematix_screen_opens(Screen::Playing, true), Some(false));
        assert_eq!(cinematix_screen_opens(Screen::Chat, false), Some(false));
        assert_eq!(cinematix_screen_opens(Screen::Cinematix, false), Some(true));
        assert_eq!(cinematix_screen_opens(Screen::Cinematix, true), Some(true));
    }

    #[test]
    fn the_shortcut_keys_map_to_the_editor() {
        assert_eq!(editor_key(KeyCode::Space), Some(EditorKey::Space));
        assert_eq!(editor_key(KeyCode::KeyK), Some(EditorKey::Keyframe));
        assert_eq!(editor_key(KeyCode::Escape), Some(EditorKey::Escape));
        assert_eq!(editor_key(KeyCode::KeyA), None);
    }

    #[test]
    fn overlay_clips_at_the_near_plane_and_rejects_points_behind_it() {
        let projection = PathProjection {
            eye: glam::DVec3::ZERO,
            view: Mat4::IDENTITY,
            center: Vec2::new(300.0, 200.0),
            focal: 200.0,
        };
        assert_eq!(
            projection.project(Vec3::new(0.0, 0.0, -1.0)),
            Some(projection.center)
        );
        assert!(projection.project(Vec3::Z).is_none());
        assert!(projection.line([0.0, 0.0, 1.0], [0.0, 0.0, 2.0]).is_none());
        let (a, b) = projection.line([0.0, 0.0, -1.0], [0.01, 0.0, 0.0]).unwrap();
        assert_eq!(a, projection.center);
        assert!(b.x > a.x && b.y == a.y);
        assert_eq!(hud_colour(PATH_FULL), 0xFF00_D7FF);
    }

    #[test]
    fn a_camera_frustum_draws_four_edges_and_a_far_rectangle_on_top() {
        let mut g = Gizmos::default();
        camera_frustum(&mut g, Vec3::ZERO, Quat::IDENTITY, 70.0, PATH_KEY);
        assert_eq!(g.on_top.len(), 8);
        assert!(
            g.on_top
                .iter()
                .all(|l| l.color == PATH_KEY && l.width > 0.0)
        );
        assert!(g.lines.is_empty());
    }

    #[test]
    fn a_key_cross_has_three_coloured_axes() {
        let mut g = Gizmos::default();
        key_cross(&mut g, Vec3::ZERO, PATH_SELECTED);
        assert_eq!(g.on_top.len(), 3);
        assert!(g.on_top.iter().all(|l| l.color == PATH_SELECTED));
    }
}

const BEND_HIT_RADIUS: f32 = 12.0;
const PATH_NEAR: f32 = 0.05;

#[derive(Clone, Copy)]
struct ProjectedBend {
    segment: (f64, f64),
    midpoint: Vec3,
    pixel: Vec2,
    units_per_pixel: f32,
}

fn closest_bend(
    handles: impl Iterator<Item = ProjectedBend>,
    cursor: Vec2,
) -> Option<ProjectedBend> {
    handles
        .map(|handle| (handle.pixel.distance_squared(cursor), handle))
        .filter(|(distance, _)| *distance <= BEND_HIT_RADIUS * BEND_HIT_RADIUS)
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, handle)| handle)
}

/// The world renders to the whole window; the editor clips its overlay to the stage.
#[derive(Clone, Copy)]
struct PathProjection {
    eye: glam::DVec3,
    view: Mat4,
    center: Vec2,
    focal: f32,
}

impl PathProjection {
    fn new(ed: &cinematix_editor::Editor) -> Option<Self> {
        let (eye, yaw, pitch) = ed.preview_view()?;
        let (w, h) = ed.frame().size;
        let focal = h * 0.5 / (ed.preview_fov().clamp(1.0, 179.0).to_radians() * 0.5).tan();
        (focal.is_finite() && focal > 0.0).then_some(Self {
            eye: glam::DVec3::from_array(eye),
            view: horizon_client::camera::view_matrix(yaw, pitch),
            center: Vec2::new(w, h) * 0.5,
            focal,
        })
    }

    fn local(&self, point: [f64; 3]) -> Vec3 {
        self.view
            .transform_vector3((glam::DVec3::from_array(point) - self.eye).as_vec3())
    }

    fn project(&self, p: Vec3) -> Option<Vec2> {
        if !p.is_finite() || p.z > -PATH_NEAR {
            return None;
        }
        let pixel = self.center + Vec2::new(p.x, -p.y) * (self.focal / -p.z);
        pixel.is_finite().then_some(pixel)
    }

    fn line(&self, a: [f64; 3], b: [f64; 3]) -> Option<(Vec2, Vec2)> {
        let (mut a, mut b) = (self.local(a), self.local(b));
        if !a.is_finite() || !b.is_finite() {
            return None;
        }
        if a.z > -PATH_NEAR {
            std::mem::swap(&mut a, &mut b);
        }
        if a.z > -PATH_NEAR {
            return None;
        }
        if b.z > -PATH_NEAR {
            b = a.lerp(b, (-PATH_NEAR - a.z) / (b.z - a.z));
            b.z = -PATH_NEAR;
        }
        Some((self.project(a)?, self.project(b)?))
    }
}

fn hud_colour(argb: u32) -> u32 {
    (argb & 0xFF00_FF00) | ((argb & 0x00FF_0000) >> 16) | ((argb & 0xFF) << 16)
}

const PATH_FULL: u32 = 0xFFFF_D700;
const PATH_FADE: u32 = 0x80FF_D700;
const PATH_KEY: u32 = 0xFFFF_FF20;
const PATH_SELECTED: u32 = 0xFFFF_2020;
const PATH_CURRENT: u32 = 0xFF20_FF20;

fn path_line(g: &mut Gizmos, a: Vec3, b: Vec3, colour: u32, width: f32) {
    g.on_top.push(Line {
        start: [f64::from(a.x), f64::from(a.y), f64::from(a.z)],
        end: [f64::from(b.x), f64::from(b.y), f64::from(b.z)],
        color: colour,
        width,
    });
}

fn camera_path_segment(
    g: &mut Gizmos,
    ed: &cinematix_editor::Editor,
    from: f64,
    to: f64,
    colour: u32,
) {
    let span = to - from;
    if span <= 0.0 {
        return;
    }
    let steps = ((span * 20.0).ceil() as usize).clamp(2, 400);
    let mut previous = None;
    for i in 0..=steps {
        let t = from + span * i as f64 / steps as f64;
        let Some((position, _)) = ed.camera_at(t) else {
            continue;
        };
        if let Some(last) = previous {
            path_line(g, last, position, colour, 2.0);
        }
        previous = Some(position);
    }
}

/// A wireframe camera frustum: the apex at the pose, four edges to the far plane and its
/// rectangle. The local forward is `-Z`, as `camera_at` returns it.
fn camera_frustum(g: &mut Gizmos, position: Vec3, rotation: Quat, fov: f32, colour: u32) {
    let half = (fov.to_radians() * 0.5).tan().clamp(0.2, 1.2);
    let d = 0.55;
    let h = d * half;
    let w = h * (16.0 / 9.0);
    let local = [
        Vec3::new(-w, h, -d),
        Vec3::new(w, h, -d),
        Vec3::new(w, -h, -d),
        Vec3::new(-w, -h, -d),
    ];
    let world = local.map(|c| position + rotation * c);
    for corner in world {
        path_line(g, position, corner, colour, 1.5);
    }
    for i in 0..4 {
        path_line(g, world[i], world[(i + 1) % 4], colour, 1.5);
    }
}

fn key_cross(g: &mut Gizmos, position: Vec3, colour: u32) {
    let r = 0.18;
    path_line(
        g,
        position - Vec3::X * r,
        position + Vec3::X * r,
        colour,
        2.0,
    );
    path_line(
        g,
        position - Vec3::Y * r,
        position + Vec3::Y * r,
        colour,
        2.0,
    );
    path_line(
        g,
        position - Vec3::Z * r,
        position + Vec3::Z * r,
        colour,
        2.0,
    );
}
