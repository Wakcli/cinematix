//! Cinematix's editor screen (`Screen::Cinematix`): the lifecycle, the drawing hook, the input
//! routing and the `/cinematix` and `horizon-ctl cinematix` commands. The editor itself (the
//! document, the frame layout, the drawing) is `cinematix-editor`; this file only connects it to
//! the game's screen, pointer and key dispatch.

use super::{Game, HudBatch, Screen};
use cinematix_editor::{EditorKey, KeyResult, Mods, Section};
use horizon_client::{Result, text::Text};
use serde_json::{Value, json};
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
        self.screen = Screen::Cinematix;
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
        cinematix_editor::draw(&self.cinematix, batch, &self.gui.atlas, self.cursor);
    }

    /// A key while the editor is up: the editor's shortcuts, and nothing reaches the world.
    pub(super) fn cinematix_key(&mut self, key: KeyCode) {
        if key == KeyCode::F8 {
            self.close_cinematix_screen();
            return;
        }
        let Some(editor_key) = editor_key(key) else {
            return;
        };
        let mods = self.cinematix_mods();
        let result = match editor_key {
            EditorKey::Left | EditorKey::Right if mods.shift => {
                let frames = if editor_key == EditorKey::Left { -10 } else { 10 };
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
        if button == winit::event::MouseButton::Left {
            if pressed {
                let mods = self.cinematix_mods();
                self.cinematix.press(self.cursor, mods);
            } else {
                self.cinematix.release();
            }
        }
        true
    }

    /// The pointer moved while the editor is up: an active drag follows it.
    pub(super) fn cinematix_mouse_dragged(&mut self) -> bool {
        if self.screen != Screen::Cinematix {
            return false;
        }
        let mods = self.cinematix_mods();
        self.cinematix.drag_to(self.cursor, mods);
        true
    }

    /// The wheel while the editor is up: it scrolls the channel list, and Ctrl zooms the time
    /// axis around its middle.
    pub(super) fn cinematix_wheel(&mut self, ticks: f32, ctrl: bool) -> bool {
        if self.screen != Screen::Cinematix {
            return false;
        }
        self.cinematix.wheel(ticks, ctrl);
        true
    }

    /// Playback advances once per drawn frame, while the editor is open and playing.
    pub(super) fn cinematix_tick(&mut self, elapsed: Duration) {
        if self.screen != Screen::Cinematix {
            return;
        }
        self.cinematix.advance(elapsed.as_secs_f64().min(0.1));
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
        let reply = match self.cinematix_control(&words) {
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
                _ => "horizon.cinematix.status",
            };
            let frame = reply["frame"].as_i64().unwrap_or_default();
            self.system_message(Text::tr(key, &[&frame]));
        } else {
            let error = reply["error"].as_str().unwrap_or("unknown error");
            self.system_message(Text::tr("horizon.cinematix.error", &[&error]));
        }
        true
    }

    /// `horizon-ctl cinematix ...` and `/cinematix ...`: one command surface for the headless
    /// harness and chat.
    pub(super) fn cinematix_control(&mut self, words: &[&str]) -> Result<Option<Value>> {
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
                    "resolution": [w, h],
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
                    None => return Ok(Some(json!({ "ok": false, "error": "frame needs a number" }))),
                };
                self.cinematix.seek_frame(n);
                Ok(Some(json!({ "ok": true, "frame": self.cinematix.frame_number(), "time": self.cinematix.time() })))
            }
            "seek" => {
                let t = match words.get(1).and_then(|w| w.parse::<f64>().ok()) {
                    Some(t) => t,
                    None => return Ok(Some(json!({ "ok": false, "error": "seek needs seconds" }))),
                };
                self.cinematix.seek(t);
                Ok(Some(json!({ "ok": true, "frame": self.cinematix.frame_number(), "time": self.cinematix.time() })))
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
            "section" => {
                let Some(name) = words.get(1).copied() else {
                    return Ok(Some(json!({ "ok": false, "error": "section needs a name" })));
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
}
