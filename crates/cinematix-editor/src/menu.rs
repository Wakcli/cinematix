//! The timeline's context menu: a target-free item list that acts on the current selection,
//! opened by a right click on a channel row, a key, or the lanes. Items whose feature is not
//! implemented are drawn dimmed and take no click, matching the editor's other inert controls.

use crate::editor::Editor;
use cinematix_anim::{Ease, Interp};
use cinematix_ui::Rect;

pub const ROW_H: f32 = 14.0;
pub const WIDTH: f32 = 156.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    AddKey,
    DeleteKey,
    CopyKeys,
    PasteKeys,
    DuplicateKeys,
    Interp(Interp),
    Ease(Ease),
    Fit,
}

pub struct Item {
    pub label: &'static str,
    pub action: Option<Action>,
    pub enabled: bool,
    pub checked: bool,
    pub header: bool,
}

impl Item {
    fn command(label: &'static str, action: Action, enabled: bool, checked: bool) -> Self {
        Self {
            label,
            action: Some(action),
            enabled,
            checked,
            header: false,
        }
    }

    fn header(label: &'static str) -> Self {
        Self {
            label,
            action: None,
            enabled: false,
            checked: false,
            header: true,
        }
    }

    fn inert(label: &'static str) -> Self {
        Self {
            label,
            action: None,
            enabled: false,
            checked: false,
            header: false,
        }
    }
}

pub struct Menu {
    pub at: (f32, f32),
    pub items: Vec<Item>,
}

impl Menu {
    pub fn height(&self) -> f32 {
        self.items.len() as f32 * ROW_H + 4.0
    }

    pub fn rect(&self, size: (f32, f32)) -> Rect {
        let w = WIDTH;
        let h = self.height();
        let x = self.at.0.min(size.0 - w - 2.0).max(2.0);
        let y = (self.at.1 - 4.0).min(size.1 - h - 2.0).max(2.0);
        Rect::new(x, y, w, h)
    }

    pub fn row(&self, r: Rect, i: usize) -> Rect {
        Rect::new(r.x + 1.0, r.y + 2.0 + i as f32 * ROW_H, r.w - 2.0, ROW_H)
    }
}

fn channel_items(ed: &Editor) -> Vec<Item> {
    let has_key = ed
        .selected_key
        .and_then(|(ci, ki)| ed.doc.channel(ci).map(|c| ki < c.keys.len()))
        .unwrap_or(false);
    let current = ed.selected_key.and_then(|(ci, ki)| {
        ed.doc
            .channel(ci)
            .and_then(|c| c.keys.get(ki))
            .map(|k| k.interp)
    });
    let mut items = vec![
        Item::command("cinematix.action.add_key", Action::AddKey, true, false),
        Item::command(
            "cinematix.action.delete_key",
            Action::DeleteKey,
            has_key,
            false,
        ),
        Item::command("cinematix.menu.copy_keys", Action::CopyKeys, has_key, false),
        Item::command(
            "cinematix.menu.paste",
            Action::PasteKeys,
            ed.can_paste_keys(),
            false,
        ),
        Item::command(
            "cinematix.menu.duplicate_keys",
            Action::DuplicateKeys,
            has_key,
            false,
        ),
        Item::header("cinematix.menu.interpolation"),
    ];
    for (interp, key) in [
        (Interp::Step, "cinematix.interp.step"),
        (Interp::Linear, "cinematix.interp.linear"),
        (Interp::Smooth, "cinematix.interp.smooth"),
        (Interp::Spline, "cinematix.interp.spline"),
        (
            Interp::Bezier {
                out: [0.42, 0.0],
                in_: [0.58, 1.0],
            },
            "cinematix.interp.bezier",
        ),
    ] {
        items.push(Item::command(
            key,
            Action::Interp(interp),
            has_key,
            current == Some(interp),
        ));
    }
    let current_ease = ed
        .selected_key
        .and_then(|(ci, ki)| ed.doc.channel(ci)?.keys.get(ki))
        .map(|key| key.ease);
    items.push(Item::header("cinematix.menu.timing"));
    for (ease, label) in [
        (Ease::None, "cinematix.ease.none"),
        (Ease::In, "cinematix.ease.in"),
        (Ease::Out, "cinematix.ease.out"),
        (Ease::InOut, "cinematix.ease.in_out"),
    ] {
        items.push(Item::command(
            label,
            Action::Ease(ease),
            has_key,
            current_ease == Some(ease),
        ));
    }
    items
}

fn lane_items(ed: &Editor) -> Vec<Item> {
    vec![
        Item::command("cinematix.action.add_key", Action::AddKey, true, false),
        Item::command("cinematix.action.fit", Action::Fit, true, false),
        Item::inert("cinematix.tab.curves"),
        Item::command(
            "cinematix.menu.paste",
            Action::PasteKeys,
            ed.can_paste_keys(),
            false,
        ),
    ]
}

impl Editor {
    /// A right click inside the timeline: open the channel menu for a row or key, the lane menu
    /// otherwise. Returns whether the timeline took it (the camera look must not start).
    pub fn right_press(&mut self, pos: (f32, f32)) -> bool {
        if self.menu.is_some() {
            self.menu = None;
            return true;
        }
        let f = self.frame();
        if let Some(row) = f
            .rows
            .iter()
            .find(|r| r.label.contains(pos.0, pos.1) || r.keys.contains(pos.0, pos.1))
        {
            self.selected_channel = Some(row.chan);
            let mut hit = None;
            if row.keys.contains(pos.0, pos.1)
                && let Some(ch) = self.doc.channel(row.chan)
            {
                let times: Vec<f64> = ch.keys.iter().map(|k| k.t).collect();
                if let Some(ki) = cinematix_timeline::hit_key(pos.0, &f.axis, &times, 6.0) {
                    hit = Some((row.chan, ki));
                }
            }
            if !hit.is_some_and(|(ci, ki)| self.key_selected(ci, ki)) {
                self.select_only_key(hit);
            }
            self.menu = Some(Menu {
                at: pos,
                items: channel_items(self),
            });
            return true;
        }
        if f.ruler.contains(pos.0, pos.1)
            || f.lanes.contains(pos.0, pos.1)
            || f.timeline.contains(pos.0, pos.1)
        {
            self.menu = Some(Menu {
                at: pos,
                items: lane_items(self),
            });
            return true;
        }
        false
    }

    /// A left click while the menu is open: activate the row under it, or close. Returns whether
    /// the menu consumed the click.
    pub fn menu_click(&mut self, pos: (f32, f32)) -> bool {
        let Some(menu) = self.menu.take() else {
            return false;
        };
        let r = menu.rect(self.size());
        if !r.contains(pos.0, pos.1) {
            return true;
        }
        for (i, item) in menu.items.iter().enumerate() {
            let row = menu.row(r, i);
            if row.contains(pos.0, pos.1) {
                if item.enabled
                    && let Some(action) = item.action
                {
                    self.act(action);
                }
                return true;
            }
        }
        true
    }

    pub fn menu_open(&self) -> bool {
        self.menu.is_some()
    }

    /// Escape closes the menu first and only then the editor.
    pub fn menu_escape(&mut self) -> bool {
        self.menu.take().is_some()
    }

    fn act(&mut self, action: Action) {
        match action {
            Action::AddKey => {
                self.add_key();
            }
            Action::DeleteKey => {
                self.delete_key();
            }
            Action::Interp(interp) => {
                self.set_selected_interp(interp);
            }
            Action::CopyKeys => {
                self.copy_keys();
            }
            Action::PasteKeys => {
                self.paste_keys();
            }
            Action::DuplicateKeys => {
                self.duplicate_keys();
            }
            Action::Ease(ease) => {
                self.set_selected_ease(ease);
            }
            Action::Fit => self.fit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{EditorKey, KeyResult};
    use cinematix_ui::Mods;

    fn editor() -> Editor {
        let mut e = Editor::new();
        e.resize(1280.0, 720.0);
        e
    }

    #[test]
    fn right_clicking_a_channel_row_opens_the_menu() {
        let mut e = editor();
        let f = e.frame();
        let row = &f.rows[0];
        assert!(e.right_press((row.label.x + 5.0, row.label.y + 5.0)));
        let menu = e.menu.as_ref().expect("a menu");
        assert_eq!(menu.items[0].label, "cinematix.action.add_key");
        assert!(
            menu.items
                .iter()
                .any(|i| i.label == "cinematix.menu.interpolation")
        );
    }

    #[test]
    fn the_delete_item_removes_the_selected_key() {
        let mut e = editor();
        e.seek_frame(24);
        assert!(e.add_key());
        let (ci, _) = e.selected_key.unwrap();
        assert_eq!(e.doc.channels[ci].keys.len(), 1);
        let f = e.frame();
        let row = f.rows.iter().find(|r| r.chan == ci).unwrap();
        let key_x = f.axis.x(1.0);
        assert!(e.right_press((key_x, row.keys.y + 5.0)));
        assert_eq!(e.selected_key, Some((ci, 0)));
        let menu = e.menu.as_ref().unwrap();
        let delete = menu
            .items
            .iter()
            .position(|i| i.label == "cinematix.action.delete_key")
            .unwrap();
        assert!(menu.items[delete].enabled);
        let r = menu.rect(e.size());
        let at = menu.row(r, delete);
        assert!(e.press((at.x + 5.0, at.y + 5.0), Mods::default()));
        assert!(e.doc.channels[ci].keys.is_empty());
        assert!(!e.menu_open());
    }

    #[test]
    fn escape_closes_the_menu_before_the_editor() {
        let mut e = editor();
        let f = e.frame();
        e.right_press((f.ruler.x + 20.0, f.ruler.y + 4.0));
        assert!(e.menu_open());
        assert_eq!(e.key(EditorKey::Escape), KeyResult::Consumed);
        assert!(!e.menu_open());
        assert_eq!(e.key(EditorKey::Escape), KeyResult::Close);
    }

    #[test]
    fn paste_without_a_clipboard_takes_a_click_and_changes_nothing() {
        let mut e = editor();
        e.seek_frame(12);
        assert!(e.add_key());
        let keys = e.doc.key_count();
        let f = e.frame();
        let row = &f.rows[0];
        e.right_press((row.label.x + 5.0, row.label.y + 5.0));
        let menu = e.menu.as_ref().unwrap();
        let paste = menu
            .items
            .iter()
            .position(|i| i.label == "cinematix.menu.paste")
            .unwrap();
        assert!(!menu.items[paste].enabled);
        let at = menu.row(menu.rect(e.size()), paste);
        assert!(e.press((at.x + 5.0, at.y + 5.0), Mods::default()));
        assert_eq!(e.doc.key_count(), keys);
    }
}
