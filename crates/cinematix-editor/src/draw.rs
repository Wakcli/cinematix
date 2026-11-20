use crate::document::{kind_key, prop_key};
use crate::editor::{Button, Editor, Field, Section};
use crate::frame::{Frame, GUTTER_W, InspectorTab, ROW_H, Row};
use crate::glyphs::{self, Icon};
use cinematix_scene::NodeKind;
use cinematix_ui::Rect;
use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};
use horizon_client_render::text::Text;
use horizon_client_render::ui;
use horizon_client_render::widgets::{self, Kind};

const HEADER_BG: u32 = rgba(13, 13, 13, 255);
const RAIL_BG: u32 = rgba(17, 17, 17, 255);
const PANEL_BG: u32 = rgba(22, 22, 22, 245);
const ROW_ALT: u32 = rgba(27, 27, 27, 220);
const LINE: u32 = rgba(46, 46, 46, 255);
const TEXT: u32 = rgba(230, 230, 230, 255);
const DIM: u32 = rgba(154, 154, 154, 255);
const FAINT: u32 = rgba(106, 106, 106, 255);
const ACCENT: u32 = rgba(92, 156, 255, 255);
const KEY: u32 = rgba(245, 246, 250, 255);
const HOVER: u32 = rgba(43, 43, 43, 255);
const ACTIVE: u32 = rgba(56, 56, 56, 255);
const BOX_BG: u32 = rgba(27, 31, 36, 255);
const BOX_BORDER: u32 = rgba(11, 12, 14, 255);
const BOX_FILL: u32 = rgba(35, 49, 66, 255);
const VALUE: u32 = rgba(240, 246, 252, 255);
const CHEVRON: u32 = rgba(88, 166, 255, 255);
const OUTLINE: u32 = rgba(87, 96, 106, 255);
const ZERO_LINE: u32 = rgba(72, 79, 88, 255);
const KEY_ANIM: u32 = rgba(38, 79, 120, 255);
const AXIS_X: u32 = rgba(218, 54, 51, 255);
const AXIS_Y: u32 = rgba(46, 160, 67, 255);
const AXIS_Z: u32 = rgba(56, 139, 253, 255);

fn fill(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, colour: u32) {
    if r.w <= 0.0 || r.h <= 0.0 {
        return;
    }
    batch.fill(atlas, r.x, r.y, r.w, r.h, colour);
}

fn line(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    width: f32,
    colour: u32,
) {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.01 {
        return;
    }
    let (nx, ny) = (-dy / len * width * 0.5, dx / len * width * 0.5);
    batch.fill_quad(
        atlas,
        [
            [x0 + nx, y0 + ny],
            [x1 + nx, y1 + ny],
            [x1 - nx, y1 - ny],
            [x0 - nx, y0 - ny],
        ],
        colour,
    );
}

fn text(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, t: &Text, colour: u32) {
    ui::text_scaled(batch, atlas, t, x, y, 1.0, colour);
}

fn tr(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, key: &str, colour: u32) {
    text(batch, atlas, x, y, &Text::translate(key, &[]), colour);
}

fn right_text(batch: &mut HudBatch, atlas: &HudAtlas, right: f32, y: f32, t: &Text, colour: u32) {
    let w = HudBatch::text_width(atlas, &t.plain());
    text(batch, atlas, right - w, y, t, colour);
}

fn register(label: &Text, r: Rect) {
    widgets::record_as(&label.plain(), [r.x, r.y, r.w, r.h], true, Kind::Button);
}

fn box_frame(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    fill(batch, atlas, r, BOX_BG);
    batch.fill(atlas, r.x, r.y, r.w, 1.0, BOX_BORDER);
    batch.fill(atlas, r.x, r.y + r.h - 1.0, r.w, 1.0, BOX_BORDER);
    batch.fill(atlas, r.x, r.y, 1.0, r.h, BOX_BORDER);
    batch.fill(atlas, r.x + r.w - 1.0, r.y, 1.0, r.h, BOX_BORDER);
}

fn chevron(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, cy: f32, filled: bool) {
    let colour = if filled { CHEVRON } else { OUTLINE };
    batch.fill_quad(
        atlas,
        [[x - 4.0, cy], [x, cy - 4.0], [x + 4.0, cy], [x, cy + 4.0]],
        colour,
    );
}

fn bar(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, frac: f32, bipolar: bool) {
    let inner = Rect::new(
        r.x + 1.0,
        r.y + 1.0,
        (r.w - 2.0).max(0.0),
        (r.h - 2.0).max(0.0),
    );
    if inner.w <= 0.0 {
        return;
    }
    let frac = frac.clamp(0.0, 1.0);
    if bipolar {
        let centre = inner.x + inner.w * 0.5;
        let x = inner.x + inner.w * frac;
        if x >= centre {
            fill(
                batch,
                atlas,
                Rect::new(centre, inner.y, x - centre, inner.h),
                BOX_FILL,
            );
        } else {
            fill(
                batch,
                atlas,
                Rect::new(x, inner.y, centre - x, inner.h),
                BOX_FILL,
            );
        }
        batch.fill(atlas, centre, inner.y, 1.0, inner.h, ZERO_LINE);
    } else {
        fill(
            batch,
            atlas,
            Rect::new(inner.x, inner.y, inner.w * frac, inner.h),
            BOX_FILL,
        );
    }
}

fn left_label(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, t: &Text, colour: u32) {
    text(batch, atlas, r.x + 8.0, r.y + (r.h - 8.0) * 0.5, t, colour);
}

fn value_label(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, value: &str, colour: u32) {
    right_text(
        batch,
        atlas,
        r.x + r.w - 14.0,
        r.y + (r.h - 8.0) * 0.5,
        &Text::literal(value),
        colour,
    );
}

/// A flat BBS-style icon button: a hover/active wash, the glyph centred, registered with the
/// control socket so `horizon-ctl ui` and the narrator still see it.
fn icon_button(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    r: Rect,
    icon: Icon,
    label: &Text,
    cursor: (f32, f32),
    active: bool,
) {
    let hover = r.contains(cursor.0, cursor.1);
    if active {
        fill(batch, atlas, r, ACTIVE);
    } else if hover {
        fill(batch, atlas, r, HOVER);
    }
    let size = 16.0;
    icon.draw(
        batch,
        atlas,
        r.x + (r.w - size) * 0.5,
        r.y + (r.h - size) * 0.5,
        size,
        TEXT,
    );
    register(label, r);
}

pub fn draw(ed: &Editor, batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32)) {
    let f = ed.frame();
    let tool = draw_header(ed, &f, batch, atlas, cursor);
    let hovered = draw_rail(ed, &f, batch, atlas, cursor);
    if !f.inspector_floating {
        draw_inspector(ed, &f, batch, atlas, cursor);
    }
    draw_timeline(ed, &f, batch, atlas, cursor);
    grip(batch, atlas, f.split_h, true);
    if !f.inspector_floating {
        grip(batch, atlas, f.split_v, false);
    }
    if f.inspector_floating {
        draw_inspector(ed, &f, batch, atlas, cursor);
    }
    if ed.dragging_panel() && cursor.0 >= f.size.0 - 48.0 {
        let r = f.inspector_dock;
        fill(batch, atlas, r, rgba(92, 156, 255, 34));
        batch.fill(atlas, r.x, r.y, 1.0, r.h, ACCENT);
        batch.fill(atlas, r.x + r.w - 1.0, r.y, 1.0, r.h, ACCENT);
    }
    if let Some(section) = hovered {
        ui::tooltip(batch, atlas, &[Text::translate(section.key(), &[])], cursor);
    } else if let Some(tool) = tool {
        ui::tooltip(batch, atlas, &[Text::translate(tool.label(), &[])], cursor);
    }
    if let Some(menu) = &ed.menu {
        draw_menu(batch, atlas, menu, cursor);
    }
}

fn draw_menu(batch: &mut HudBatch, atlas: &HudAtlas, menu: &crate::menu::Menu, cursor: (f32, f32)) {
    let size = (batch.width, batch.height);
    let r = menu.rect(size);
    fill(batch, atlas, r, rgba(16, 16, 16, 250));
    batch.fill(atlas, r.x, r.y, r.w, 1.0, LINE);
    batch.fill(atlas, r.x, r.y + r.h - 1.0, r.w, 1.0, LINE);
    batch.fill(atlas, r.x, r.y, 1.0, r.h, LINE);
    batch.fill(atlas, r.x + r.w - 1.0, r.y, 1.0, r.h, LINE);
    for (i, item) in menu.items.iter().enumerate() {
        let row = menu.row(r, i);
        let hover = !item.header && item.enabled && row.contains(cursor.0, cursor.1);
        if hover {
            fill(batch, atlas, row, HOVER);
        }
        let colour = if item.header || !item.enabled {
            FAINT
        } else {
            TEXT
        };
        if item.checked {
            batch.fill(atlas, row.x + 2.0, row.y + 4.0, 2.0, row.h - 8.0, ACCENT);
        }
        text(
            batch,
            atlas,
            row.x + 9.0,
            row.y + 3.0,
            &Text::translate(item.label, &[]),
            colour,
        );
    }
}

fn draw_header(
    ed: &Editor,
    f: &Frame,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cursor: (f32, f32),
) -> Option<crate::frame::Tool> {
    fill(batch, atlas, f.header, HEADER_BG);
    batch.fill(
        atlas,
        f.header.x,
        f.header.y + f.header.h - 1.0,
        f.header.w,
        1.0,
        LINE,
    );
    text(
        batch,
        atlas,
        f.header.x + 8.0,
        f.header.y + 8.0,
        &Text::translate("cinematix.screen.title", &[]),
        TEXT,
    );

    let mut hovered = None;
    for (tool, r) in &f.tools {
        let hover = r.contains(cursor.0, cursor.1);
        let enabled = match tool {
            crate::frame::Tool::Save => true,
            crate::frame::Tool::Undo => ed.can_undo(),
            crate::frame::Tool::Redo => ed.can_redo(),
            crate::frame::Tool::Render => true,
        };
        if enabled {
            if hover {
                fill(batch, atlas, *r, HOVER);
            }
            tool_icon(*tool).draw(batch, atlas, r.x + 2.0, r.y + 2.0, 16.0, TEXT);
            register(&Text::translate(tool.label(), &[]), *r);
        } else {
            tool_icon(*tool).draw(batch, atlas, r.x + 2.0, r.y + 2.0, 16.0, FAINT);
        }
        if hover {
            hovered = Some(*tool);
        }
    }

    let fps = ed.fps();
    let frame = Text::tr("cinematix.header.frame", &[&ed.frame_number()]);
    let time = Text::tr("cinematix.header.time", &[&format!("{:.2}", ed.time())]);
    let rate = Text::tr("cinematix.header.fps", &[&format!("{fps:.0}")]);
    let mut right = f.header.x + f.header.w - 8.0;
    for t in [rate, time, frame] {
        let w = HudBatch::text_width(atlas, &t.plain());
        let x = right - w;
        text(batch, atlas, x, f.header.y + 8.0, &t, DIM);
        right = x - 14.0;
    }

    if let Some(status) = ed.status {
        let w = HudBatch::text_width(atlas, &Text::translate(status, &[]).plain());
        let x = f.header.x + (f.header.w - w) * 0.5;
        if x > right {
            tr(batch, atlas, x, f.header.y + 8.0, status, ACCENT);
        }
    }
    hovered
}

fn tool_icon(tool: crate::frame::Tool) -> Icon {
    use crate::frame::Tool;
    match tool {
        Tool::Save => glyphs::SAVE,
        Tool::Undo => glyphs::UNDO,
        Tool::Redo => glyphs::REDO,
        Tool::Render => glyphs::VIDEO_CAMERA,
    }
}

fn tab_icon(tab: crate::frame::TimelineTab) -> Icon {
    use crate::frame::TimelineTab;
    match tab {
        TimelineTab::DopeSheet => glyphs::LIST,
        TimelineTab::Curves => glyphs::CURVES,
        TimelineTab::Clips => glyphs::FILM,
    }
}

fn section_icon(section: Section) -> Icon {
    match section {
        Section::Camera => glyphs::CAMERA,
        Section::Actors => glyphs::USER,
        Section::Models => glyphs::SHAPES,
        Section::Lights => glyphs::LIGHT,
        Section::World => glyphs::BRICKS,
        Section::Film => glyphs::FILM,
        Section::Render => glyphs::VIDEO_CAMERA,
        Section::Project => glyphs::SETTINGS,
    }
}

fn draw_rail(
    ed: &Editor,
    f: &Frame,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cursor: (f32, f32),
) -> Option<Section> {
    // The rail column continues to the window's bottom even though the dock's timeline starts
    // to its right, so the corner below the icons is not a hole in the layout.
    let full = Rect::new(f.rail.x, f.rail.y, f.rail.w, (f.size.1 - f.rail.y).max(0.0));
    fill(batch, atlas, full, RAIL_BG);
    batch.fill(atlas, full.x + full.w - 1.0, full.y, 1.0, full.h, LINE);
    let mut hovered = None;
    for (section, r) in &f.rail_items {
        let label = Text::translate(section.key(), &[]);
        icon_button(
            batch,
            atlas,
            *r,
            section_icon(*section),
            &label,
            cursor,
            ed.section == *section,
        );
        if r.contains(cursor.0, cursor.1) {
            hovered = Some(*section);
        }
    }
    hovered
}

fn grip(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, horizontal: bool) {
    if horizontal {
        batch.fill(atlas, r.x, r.y + 1.0, r.w, 1.0, LINE);
        let cx = r.x + r.w * 0.5;
        for i in -1..=1 {
            batch.fill(atlas, cx + i as f32 * 5.0 - 1.0, r.y + 1.0, 2.0, 2.0, FAINT);
        }
    } else {
        batch.fill(atlas, r.x + 1.0, r.y, 1.0, r.h, LINE);
        let cy = r.y + r.h * 0.5;
        for i in -1..=1 {
            batch.fill(atlas, r.x + 1.0, cy + i as f32 * 5.0 - 1.0, 2.0, 2.0, FAINT);
        }
    }
}

fn draw_inspector(
    ed: &Editor,
    f: &Frame,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cursor: (f32, f32),
) {
    if f.inspector_floating {
        fill(
            batch,
            atlas,
            Rect::new(
                f.inspector.x + 3.0,
                f.inspector.y + 3.0,
                f.inspector.w,
                f.inspector.h,
            ),
            rgba(0, 0, 0, 110),
        );
    }
    fill(batch, atlas, f.inspector, PANEL_BG);
    batch.fill(
        atlas,
        f.inspector.x,
        f.inspector.y,
        1.0,
        f.inspector.h,
        LINE,
    );
    batch.fill(
        atlas,
        f.inspector.x + f.inspector.w - 1.0,
        f.inspector.y,
        1.0,
        f.inspector.h,
        LINE,
    );
    batch.fill(
        atlas,
        f.inspector.x,
        f.inspector.y + f.inspector.h - 1.0,
        f.inspector.w,
        1.0,
        LINE,
    );
    fill(batch, atlas, f.inspector_header, HEADER_BG);
    batch.fill(
        atlas,
        f.inspector.x,
        f.inspector_header.y + f.inspector_header.h - 1.0,
        f.inspector.w,
        1.0,
        LINE,
    );

    for (tab, r) in &f.inspector_tabs {
        let active = *tab == ed.inspector_tab;
        let hover = !active && r.contains(cursor.0, cursor.1);
        if active {
            fill(batch, atlas, *r, ACTIVE);
            batch.fill(atlas, r.x, r.y + r.h - 2.0, r.w, 2.0, ACCENT);
        } else if hover {
            fill(batch, atlas, *r, HOVER);
        }
        let label = Text::translate(tab.label(), &[]);
        text(
            batch,
            atlas,
            r.x + 6.0,
            r.y + 4.0,
            &label,
            if active { TEXT } else { DIM },
        );
        register(&label, *r);
    }
    if let Some(r) = f.inspector_redock {
        let hover = r.contains(cursor.0, cursor.1);
        if hover {
            fill(batch, atlas, r, HOVER);
        }
        text(
            batch,
            atlas,
            r.x + 4.0,
            r.y + 4.0,
            &Text::literal("[]"),
            DIM,
        );
        register(&Text::translate("cinematix.panel.dock", &[]), r);
    }

    if ed.inspector_tab == InspectorTab::Properties {
        let x = f.inspector.x + 6.0;
        let y = f.inspector.y + 26.0;
        text(
            batch,
            atlas,
            x,
            y,
            &Text::translate(ed.section.key(), &[]),
            ACCENT,
        );
        batch.fill(atlas, x, y + 13.0, f.inspector.w - 12.0, 1.0, LINE);
        for (field, r) in &f.fields {
            draw_field(ed, *field, *r, batch, atlas, cursor);
        }
        draw_vector_rows(ed, f, batch, atlas);
        draw_render_controls(ed, f, batch, atlas);
        for (kind, r) in &f.color_rows {
            draw_color_row(ed, *kind, *r, batch, atlas);
        }
    } else {
        for (id, _depth, has_children, r) in &f.tree {
            if ed.selected_node == Some(*id) {
                fill(batch, atlas, *r, ROW_ALT);
            }
            let node = ed.doc.node(*id);
            let expanded = ed.expanded.contains(id);
            if *has_children {
                let marker = if expanded { "v" } else { ">" };
                text(
                    batch,
                    atlas,
                    r.x + 2.0,
                    r.y + 4.0,
                    &Text::literal(marker),
                    DIM,
                );
            }
            let icon = node.map(|n| match &n.kind {
                cinematix_scene::NodeKind::Camera(_) => glyphs::CAMERA,
                cinematix_scene::NodeKind::Light(_) => glyphs::LIGHT,
                cinematix_scene::NodeKind::Actor(_) => glyphs::USER,
                cinematix_scene::NodeKind::Mesh(_) => glyphs::SHAPES,
                cinematix_scene::NodeKind::Empty => glyphs::SETTINGS,
            });
            if let Some(icon) = icon {
                icon.draw(batch, atlas, r.x + 12.0, r.y + 2.0, 14.0, DIM);
            }
            let name = match node {
                Some(n) if !n.name.is_empty() => Text::literal(n.name.clone()),
                Some(n) => Text::translate(kind_key(&n.kind), &[]),
                None => Text::literal(String::new()),
            };
            text(batch, atlas, r.x + 30.0, r.y + 4.0, &name, TEXT);
            register(&name, *r);
        }
    }

    draw_summary(ed, f, batch, atlas);
}

fn draw_field(
    ed: &Editor,
    field: Field,
    r: Rect,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cursor: (f32, f32),
) {
    let (min, max, _) = field.range();
    let v = field.get(&ed.doc).clamp(min, max);
    let frac = if (max - min).abs() < f64::EPSILON {
        0.0
    } else {
        ((v - min) / (max - min)) as f32
    };
    let label = Text::translate(field.label(), &[]);
    let value = format!("{}{}", field.text(&ed.doc), field.unit());
    let hover = r.contains(cursor.0, cursor.1);
    let held = ed.dragging_field(field);
    if field == Field::Time {
        draw_time_field(batch, atlas, r, &label, &value, frac, hover, held);
    } else {
        box_frame(batch, atlas, r);
        bar(batch, atlas, r, frac, false);
        left_label(
            batch,
            atlas,
            r,
            &label,
            if hover || held { VALUE } else { DIM },
        );
        value_label(batch, atlas, r, &value, VALUE);
        chevron(batch, atlas, r.x + r.w - 8.0, r.y + r.h * 0.5, true);
    }
    register(&label, r);
}

fn draw_time_field(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    r: Rect,
    label: &Text,
    value: &str,
    frac: f32,
    hover: bool,
    held: bool,
) {
    box_frame(batch, atlas, r);
    let track = Rect::new(r.x + 2.0, r.y + 9.0, (r.w - 4.0).max(1.0), 10.0);
    const STOPS: [[f32; 3]; 5] = [
        [0.086, 0.106, 0.149],
        [0.361, 0.255, 0.157],
        [0.169, 0.298, 0.435],
        [0.369, 0.227, 0.161],
        [0.086, 0.106, 0.149],
    ];
    let steps = 64;
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let seg = (t * 4.0).min(3.999);
        let a = seg.floor() as usize;
        let u = seg - a as f32;
        let c =
            std::array::from_fn::<f32, 3, _>(|k| STOPS[a][k] + (STOPS[a + 1][k] - STOPS[a][k]) * u);
        batch.fill(
            atlas,
            track.x + track.w * i as f32 / steps as f32,
            track.y,
            track.w / steps as f32 + 1.0,
            track.h,
            rgba(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                255,
            ),
        );
    }
    let x = track.x + track.w * frac.clamp(0.0, 1.0);
    batch.fill(atlas, x - 1.0, track.y - 2.0, 2.0, track.h + 4.0, VALUE);
    text(
        batch,
        atlas,
        r.x + 8.0,
        r.y + 1.0,
        label,
        if hover || held { VALUE } else { DIM },
    );
    right_text(
        batch,
        atlas,
        r.x + r.w - 8.0,
        r.y + 1.0,
        &Text::literal(value),
        VALUE,
    );
    chevron(batch, atlas, r.x + r.w - 8.0, r.y + 5.0, false);
}

fn draw_vector_rows(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas) {
    for (row, label, axes) in &f.vectors {
        if let Some(lr) = label {
            tr(batch, atlas, lr.x, lr.y + 1.0, row.label(), DIM);
        }
        for (axis, r) in axes {
            box_frame(batch, atlas, *r);
            if !axis.is_position() {
                let (min, max, _) = axis.range();
                let v = ed.axis_value(*axis).clamp(min, max);
                let frac = ((v - min) / (max - min)) as f32;
                bar(batch, atlas, *r, frac, true);
            }
            let edge = match axis {
                crate::frame::Axis::PositionX => AXIS_X,
                crate::frame::Axis::PositionY => AXIS_Y,
                crate::frame::Axis::PositionZ => AXIS_Z,
                crate::frame::Axis::Yaw | crate::frame::Axis::Pitch => CHEVRON,
            };
            batch.fill(atlas, r.x + 1.0, r.y + 1.0, 2.0, r.h - 2.0, edge);
            text(
                batch,
                atlas,
                r.x + 8.0,
                r.y + (r.h - 8.0) * 0.5,
                &Text::literal(axis.label()),
                OUTLINE,
            );
            let value = format!("{:.2}", ed.axis_value(*axis));
            right_text(
                batch,
                atlas,
                r.x + r.w - 4.0,
                r.y + (r.h - 8.0) * 0.5,
                &Text::literal(value),
                VALUE,
            );
            register(&Text::translate(axis.tag(), &[]), *r);
        }
    }
}

fn centered_text(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, t: &Text, colour: u32) {
    let w = HudBatch::text_width(atlas, &t.plain());
    text(
        batch,
        atlas,
        r.x + (r.w - w) * 0.5,
        r.y + (r.h - 8.0) * 0.5,
        t,
        colour,
    );
}

fn draw_render_controls(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas) {
    if let Some((_, first)) = f.fps_presets.first() {
        tr(
            batch,
            atlas,
            first.x - 26.0,
            first.y + 7.0,
            "cinematix.field.fps",
            DIM,
        );
        let current = ed.fps().round() as u32;
        for (fps, r) in &f.fps_presets {
            box_frame(batch, atlas, *r);
            let active = *fps == current;
            if active {
                fill(
                    batch,
                    atlas,
                    Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0),
                    BOX_FILL,
                );
                batch.fill(atlas, r.x, r.y, r.w, 1.0, CHEVRON);
                batch.fill(atlas, r.x, r.y + r.h - 1.0, r.w, 1.0, CHEVRON);
                batch.fill(atlas, r.x, r.y, 1.0, r.h, CHEVRON);
                batch.fill(atlas, r.x + r.w - 1.0, r.y, 1.0, r.h, CHEVRON);
            }
            centered_text(
                batch,
                atlas,
                *r,
                &Text::literal(fps.to_string()),
                if active { VALUE } else { DIM },
            );
            register(&Text::literal(fps.to_string()), *r);
        }
    }
    if let Some((w, h, link)) = f.size_pair {
        for (tag, r, value) in [
            ("W", w, ed.doc.scene.meta.resolution.0),
            ("H", h, ed.doc.scene.meta.resolution.1),
        ] {
            box_frame(batch, atlas, r);
            text(
                batch,
                atlas,
                r.x + 8.0,
                r.y + (r.h - 8.0) * 0.5,
                &Text::literal(tag),
                OUTLINE,
            );
            right_text(
                batch,
                atlas,
                r.x + r.w - 6.0,
                r.y + (r.h - 8.0) * 0.5,
                &Text::literal(value.to_string()),
                VALUE,
            );
            register(&Text::literal(tag), r);
        }
        box_frame(batch, atlas, link);
        if ed.resolution_linked {
            batch.fill(atlas, link.x + 3.0, link.y + 5.0, 4.0, 4.0, CHEVRON);
            batch.fill(atlas, link.x + 9.0, link.y + 7.0, 4.0, 4.0, CHEVRON);
        } else {
            batch.fill(atlas, link.x + 4.0, link.y + 6.0, 4.0, 4.0, OUTLINE);
            batch.fill(atlas, link.x + 8.0, link.y + 6.0, 4.0, 4.0, OUTLINE);
        }
        register(&Text::translate("cinematix.render.link", &[]), link);
    }
}

fn draw_color_row(
    ed: &Editor,
    kind: crate::frame::ColorKind,
    r: Rect,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
) {
    let default = match kind {
        crate::frame::ColorKind::Sky => [0.471, 0.655, 1.0],
        crate::frame::ColorKind::Fog => [0.753, 0.847, 1.0],
    };
    let colour = match kind {
        crate::frame::ColorKind::Sky => ed.doc.scene.meta.environment.sky_color,
        crate::frame::ColorKind::Fog => ed.doc.scene.meta.environment.fog_color,
    }
    .unwrap_or(default);
    box_frame(batch, atlas, r);
    left_label(batch, atlas, r, &Text::translate(kind.label(), &[]), DIM);
    let swatch = Rect::new(r.x + r.w - 86.0, r.y + 4.0, 16.0, 16.0);
    fill(
        batch,
        atlas,
        swatch,
        rgba(
            (colour[0] * 255.0) as u8,
            (colour[1] * 255.0) as u8,
            (colour[2] * 255.0) as u8,
            255,
        ),
    );
    batch.fill(atlas, swatch.x, swatch.y, swatch.w, 1.0, BOX_BORDER);
    batch.fill(
        atlas,
        swatch.x,
        swatch.y + swatch.h - 1.0,
        swatch.w,
        1.0,
        BOX_BORDER,
    );
    batch.fill(atlas, swatch.x, swatch.y, 1.0, swatch.h, BOX_BORDER);
    batch.fill(
        atlas,
        swatch.x + swatch.w - 1.0,
        swatch.y,
        1.0,
        swatch.h,
        BOX_BORDER,
    );
    let hex = format!(
        "#{:02X}{:02X}{:02X}",
        (colour[0] * 255.0) as u8,
        (colour[1] * 255.0) as u8,
        (colour[2] * 255.0) as u8
    );
    right_text(
        batch,
        atlas,
        r.x + r.w - 22.0,
        r.y + (r.h - 8.0) * 0.5,
        &Text::literal(hex),
        VALUE,
    );
    chevron(batch, atlas, r.x + r.w - 8.0, r.y + r.h * 0.5, false);
    register(&Text::translate(kind.label(), &[]), r);
}

fn draw_summary(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas) {
    // The camera's vector rows already show its pose, and at a short window the footer would
    // cover the Rotation row; the footer is for the sections that have room for it.
    if ed.section == Section::Camera && !f.vectors.is_empty() {
        return;
    }
    let x = f.inspector.x + 6.0;
    let bottom = f.inspector.y + f.inspector.h - 8.0;
    let line_h = 12.0;
    let mut rows: Vec<Text> = Vec::new();
    match ed.section {
        Section::Camera => {
            if let Some(t) = ed.doc.camera_transform() {
                rows.push(Text::tr(
                    "cinematix.summary.position",
                    &[
                        &format!("{:.2}", t.translation.x),
                        &format!("{:.2}", t.translation.y),
                        &format!("{:.2}", t.translation.z),
                    ],
                ));
            }
            rows.push(Text::tr(
                "cinematix.summary.channels",
                &[&ed.doc.channels.len()],
            ));
        }
        Section::Actors => rows.push(Text::tr(
            "cinematix.summary.nodes",
            &[&ed.doc.nodes_of(|k| matches!(k, NodeKind::Actor(_))).len()],
        )),
        Section::Models => rows.push(Text::tr(
            "cinematix.summary.nodes",
            &[&ed.doc.nodes_of(|k| matches!(k, NodeKind::Mesh(_))).len()],
        )),
        Section::Lights => rows.push(Text::tr(
            "cinematix.summary.nodes",
            &[&ed.doc.nodes_of(|k| matches!(k, NodeKind::Light(_))).len()],
        )),
        Section::World => {
            rows.push(Text::tr(
                "cinematix.summary.ops",
                &[&ed.doc.scene.world.len()],
            ));
            let mut chunks: Vec<(i32, i32)> = ed.doc.scene.world.chunk_keys().collect();
            chunks.sort();
            chunks.dedup();
            rows.push(Text::tr("cinematix.summary.chunks", &[&chunks.len()]));
            let env = &ed.doc.scene.meta.environment;
            let n = [
                env.time,
                env.rain,
                env.thunder,
                env.fog_start,
                env.fog_end,
                env.sky_color.map(|_| 0.0),
                env.fog_color.map(|_| 0.0),
            ]
            .iter()
            .filter(|v| v.is_some())
            .count();
            rows.push(Text::tr("cinematix.summary.overrides", &[&n]));
        }
        Section::Film => {
            rows.push(Text::tr(
                "cinematix.summary.channels",
                &[&ed.doc.channels.len()],
            ));
            rows.push(Text::tr("cinematix.summary.keyed", &[&ed.keyed_channels()]));
            rows.push(Text::tr("cinematix.summary.keys", &[&ed.doc.key_count()]));
            let (r0, r1) = ed.range();
            rows.push(Text::tr(
                "cinematix.summary.range",
                &[&format!("{r0:.2}"), &format!("{r1:.2}")],
            ));
        }
        Section::Render => {
            let (w, h) = ed.doc.scene.meta.resolution;
            rows.push(Text::tr("cinematix.summary.resolution", &[&w, &h]));
            rows.push(Text::tr("cinematix.summary.keys", &[&ed.doc.key_count()]));
        }
        Section::Project => {
            rows.push(Text::tr(
                "cinematix.summary.nodes",
                &[&ed.doc.scene.nodes.len()],
            ));
            rows.push(Text::tr(
                "cinematix.summary.assets",
                &[&ed.doc.scene.assets.len()],
            ));
            rows.push(Text::tr(
                "cinematix.summary.channels",
                &[&ed.doc.channels.len()],
            ));
        }
    }
    let mut y = bottom - rows.len() as f32 * line_h;
    for t in rows {
        if y < f.inspector.y + 8.0 {
            break;
        }
        text(batch, atlas, x, y, &t, DIM);
        y += line_h;
    }
}

fn draw_timeline(
    ed: &Editor,
    f: &Frame,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cursor: (f32, f32),
) {
    fill(batch, atlas, f.timeline, PANEL_BG);
    batch.fill(atlas, f.timeline.x, f.timeline.y, f.timeline.w, 1.0, LINE);

    for (tab, r) in &f.tabs {
        let active = *tab == ed.tab;
        let implemented = true;
        let hover = implemented && !active && r.contains(cursor.0, cursor.1);
        if active {
            fill(batch, atlas, *r, ACTIVE);
            batch.fill(atlas, r.x, r.y + r.h - 2.0, r.w, 2.0, ACCENT);
        } else if hover {
            fill(batch, atlas, *r, HOVER);
        }
        let colour = if active {
            TEXT
        } else if implemented {
            DIM
        } else {
            FAINT
        };
        tab_icon(*tab).draw(batch, atlas, r.x + 3.0, r.y + 2.0, 16.0, colour);
        text(
            batch,
            atlas,
            r.x + 22.0,
            r.y + 6.0,
            &Text::translate(tab.label(), &[]),
            colour,
        );
        if implemented {
            register(&Text::translate(tab.label(), &[]), *r);
        }
    }
    if ed.tab == crate::frame::TimelineTab::Curves {
        for (axis, r) in &f.curve_axes {
            let active = *axis == ed.curve_axis;
            if active {
                fill(batch, atlas, *r, ACTIVE);
            }
            let names = ["X", "Y", "Z", "W"];
            text(
                batch,
                atlas,
                r.x + 5.0,
                r.y + 5.0,
                &Text::literal(names[*axis]),
                if active { TEXT } else { DIM },
            );
            register(&Text::literal(names[*axis]), *r);
        }
    }
    if f.free_check.w == 0.0 {
        let count = Text::tr("cinematix.summary.channels", &[&ed.doc.channels.len()]);
        right_text(
            batch,
            atlas,
            f.timeline.x + f.timeline.w - 6.0,
            f.timeline.y + 6.0,
            &count,
            FAINT,
        );
    }

    if f.loop_check.w > 0.0 {
        let check = Rect::new(f.loop_check.x, f.loop_check.y, 16.0, 16.0);
        box_frame(batch, atlas, check);
        if ed.looping {
            fill(
                batch,
                atlas,
                Rect::new(check.x + 4.0, check.y + 4.0, 8.0, 8.0),
                CHEVRON,
            );
        }
        tr(
            batch,
            atlas,
            check.x + 22.0,
            check.y + 4.0,
            "cinematix.transport.loop",
            if f.loop_check.contains(cursor.0, cursor.1) {
                VALUE
            } else {
                DIM
            },
        );
        register(
            &Text::translate("cinematix.transport.loop", &[]),
            f.loop_check,
        );
    }
    if f.free_check.w > 0.0 {
        let check = Rect::new(f.free_check.x, f.free_check.y, 16.0, 16.0);
        box_frame(batch, atlas, check);
        if ed.preview_free {
            fill(
                batch,
                atlas,
                Rect::new(check.x + 4.0, check.y + 4.0, 8.0, 8.0),
                CHEVRON,
            );
        }
        tr(
            batch,
            atlas,
            check.x + 22.0,
            check.y + 4.0,
            "cinematix.transport.free_view",
            if f.free_check.contains(cursor.0, cursor.1) {
                VALUE
            } else {
                DIM
            },
        );
        register(
            &Text::translate("cinematix.transport.free_view", &[]),
            f.free_check,
        );
    }
    draw_ruler(ed, f, batch, atlas);
    match ed.tab {
        crate::frame::TimelineTab::Curves => draw_curve(ed, f, batch, atlas),
        crate::frame::TimelineTab::Clips => draw_clips(ed, f, batch, atlas, cursor),
        crate::frame::TimelineTab::DopeSheet => draw_rows(ed, f, batch, atlas),
    }
    draw_playhead(ed, f, batch, atlas);
    draw_transport(ed, f, batch, atlas, cursor);
}

fn draw_ruler(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas) {
    let (t0, t1) = f.axis.visible();
    let pps = f.axis.px_per_sec();
    let steps = [
        1.0 / 24.0,
        1.0 / 12.0,
        0.25,
        0.5,
        1.0,
        2.0,
        5.0,
        10.0,
        30.0,
        60.0,
    ];
    let step = steps
        .iter()
        .copied()
        .find(|s| s * pps >= 48.0)
        .unwrap_or(60.0);
    let fps = f64::from(ed.fps());
    let mut t = (t0 / step).floor() * step;
    let mut drawn = 0;
    while t <= t1 && drawn < 512 {
        let x = f.axis.x(t);
        if x >= f.ruler.x {
            batch.fill(atlas, x, f.ruler.y, 1.0, 5.0, FAINT);
            let frame = (t * fps).round() as i64;
            let label = Text::literal(format!("{frame}"));
            let w = HudBatch::text_width(atlas, &label.plain());
            if x + 2.0 + w <= f.ruler.x + f.ruler.w {
                text(batch, atlas, x + 2.0, f.ruler.y + 6.0, &label, FAINT);
            }
        }
        t += step;
        drawn += 1;
    }
    batch.fill(
        atlas,
        f.ruler.x,
        f.ruler.y + f.ruler.h - 1.0,
        f.ruler.w,
        1.0,
        LINE,
    );
}

fn draw_clips(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32)) {
    let hover = f.clip_add.contains(cursor.0, cursor.1);
    fill(batch, atlas, f.clip_add, if hover { HOVER } else { ACTIVE });
    text(
        batch,
        atlas,
        f.clip_add.x + 6.0,
        f.clip_add.y + 4.0,
        &Text::translate("cinematix.clip.add", &[]),
        TEXT,
    );
    register(&Text::translate("cinematix.clip.add", &[]), f.clip_add);
    let count = Text::tr("cinematix.clip.count", &[&ed.doc.scene.meta.clips.len()]);
    right_text(
        batch,
        atlas,
        f.lanes.x + f.lanes.w - 8.0,
        f.lanes.y + 6.0,
        &count,
        FAINT,
    );
    batch.fill(atlas, f.lanes.x, f.lanes.y + ROW_H, f.lanes.w, 1.0, LINE);
    for (i, r) in &f.clips {
        let Some(clip) = ed.doc.scene.meta.clips.get(*i) else {
            continue;
        };
        if ed.selected_clip == Some(*i) {
            fill(batch, atlas, *r, ROW_ALT);
            batch.fill(atlas, r.x, r.y, 2.0, r.h, ACCENT);
        }
        let name = if clip.name.is_empty() {
            Text::tr("cinematix.clip.unnamed", &[])
        } else {
            Text::literal(clip.name.clone())
        };
        text(batch, atlas, r.x + 6.0, r.y + 4.0, &name, TEXT);
        let range = Text::tr(
            "cinematix.clip.range",
            &[&format!("{:.2}", clip.start), &format!("{:.2}", clip.end)],
        );
        right_text(batch, atlas, r.x + r.w - 24.0, r.y + 4.0, &range, DIM);
        if let Some((_, del)) = f.clip_delete.iter().find(|(j, _)| j == i) {
            let hover = del.contains(cursor.0, cursor.1);
            if hover {
                fill(batch, atlas, *del, HOVER);
            }
            text(
                batch,
                atlas,
                del.x + 5.0,
                del.y + 2.0,
                &Text::literal("x"),
                DIM,
            );
            register(&Text::translate("cinematix.clip.delete", &[]), *del);
        }
        register(&name, *r);
    }
    if let Some(track) = f.clip_track {
        box_frame(batch, atlas, track);
        if f.clip_handles.len() == 2 {
            let a = f.clip_handles[0].1.x + 2.0;
            let b = f.clip_handles[1].1.x + 2.0;
            if b > a {
                fill(
                    batch,
                    atlas,
                    Rect::new(a, track.y + 1.0, b - a, track.h - 2.0),
                    KEY_ANIM,
                );
            }
            for (_, r) in &f.clip_handles {
                fill(batch, atlas, *r, OUTLINE);
                batch.fill(atlas, r.x, r.y, 1.0, r.h, BOX_BORDER);
                batch.fill(atlas, r.x + r.w - 1.0, r.y, 1.0, r.h, BOX_BORDER);
            }
        }
    }
    if ed.doc.scene.meta.clips.is_empty() {
        centered(batch, atlas, f, "cinematix.clip.empty");
    }
}

fn draw_curve(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas) {
    let Some(ci) = ed.selected_channel else {
        centered(batch, atlas, f, "cinematix.curve.select");
        return;
    };
    let Some(ch) = ed.doc.channel(ci) else {
        centered(batch, atlas, f, "cinematix.curve.select");
        return;
    };
    let Some((lo, hi)) = crate::curve::range(ch, ed.curve_axis) else {
        centered(batch, atlas, f, "cinematix.curve.no_keys");
        return;
    };
    let plot = f.curve_plot;
    let value_y = |v: f64| plot.y + plot.h * (1.0 - ((v - lo) / (hi - lo).max(1e-9)) as f32);
    for i in 0..=4 {
        let v = lo + (hi - lo) * f64::from(i) / 4.0;
        let y = value_y(v);
        batch.fill(atlas, plot.x, y, plot.w, 1.0, rgba(34, 34, 34, 255));
        let label = Text::literal(format!("{v:.2}"));
        right_text(batch, atlas, plot.x - 4.0, y - 4.0, &label, FAINT);
    }
    let mut prev: Option<(f32, f32)> = None;
    let mut x = plot.x;
    while x <= plot.x + plot.w {
        let t = f.axis.t(x);
        if let Some(value) = ch.eval(t)
            && let Some(v) = crate::curve::component(value, ed.curve_axis)
        {
            let y = value_y(v);
            if let Some((_, py)) = prev {
                let (top, bottom) = if py <= y { (py, y) } else { (y, py) };
                batch.fill(atlas, x - 1.0, top, 2.0, (bottom - top).max(1.0), ACCENT);
            }
            prev = Some((x, y));
        }
        x += 2.0;
    }
    for (ki, key) in ch.keys.iter().enumerate() {
        let x = f.axis.x(key.t);
        if x < plot.x - 2.0 || x > plot.x + plot.w + 2.0 {
            continue;
        }
        let Some(v) = crate::curve::component(key.v, ed.curve_axis) else {
            continue;
        };
        let y = value_y(v);
        let selected = ed.selected_key == Some((ci, ki));
        let r = if selected { 5.0 } else { 4.0 };
        batch.fill_quad(
            atlas,
            [[x, y - r], [x + r, y], [x, y + r], [x - r, y]],
            if selected { KEY } else { ACCENT },
        );
        if selected
            && let cinematix_anim::Interp::Bezier { out, in_ } = key.interp
            && let Some(next) = ch.keys.get(ki + 1)
        {
            let a = (x, y);
            let b = (
                f.axis.x(next.t),
                value_y(crate::curve::component(next.v, ed.curve_axis).unwrap_or(v)),
            );
            for (c, colour) in [(out, KEY), (in_, ACCENT)] {
                let (hx, hy) = crate::curve::handle_pos(a, b, c);
                line(batch, atlas, a.0, a.1, hx, hy, 1.0, FAINT);
                fill(
                    batch,
                    atlas,
                    Rect::new(hx - 2.5, hy - 2.5, 5.0, 5.0),
                    colour,
                );
            }
        }
    }
}

fn centered(batch: &mut HudBatch, atlas: &HudAtlas, f: &Frame, key: &str) {
    let label = Text::translate(key, &[]);
    let w = HudBatch::text_width(atlas, &label.plain());
    text(
        batch,
        atlas,
        f.curve_plot.x + (f.curve_plot.w - w) * 0.5,
        f.curve_plot.y + f.curve_plot.h * 0.5 - 4.0,
        &label,
        FAINT,
    );
}

fn draw_rows(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas) {
    for (i, row) in f.rows.iter().enumerate() {
        let band = Rect::new(f.lanes.x, row.label.y, f.lanes.w, row.label.h);
        if i % 2 == 1 {
            fill(batch, atlas, band, ROW_ALT);
        }
        draw_row_label(
            ed,
            row,
            batch,
            atlas,
            if i % 2 == 1 { ROW_ALT } else { PANEL_BG },
        );
        draw_keys(ed, f, row, batch, atlas);
    }
}

fn key_state(ch: &cinematix_anim::Channel, t: f64, fps: f32) -> u8 {
    if ch.keys.is_empty() {
        0
    } else if ch
        .keys
        .iter()
        .any(|k| (k.t - t).abs() <= 0.5 / f64::from(fps.max(1.0)))
    {
        2
    } else {
        1
    }
}

fn state_diamond(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, state: u8, inner: u32) {
    let diamond = |r: f32| [[cx, cy - r], [cx + r, cy], [cx, cy + r], [cx - r, cy]];
    match state {
        0 => {
            batch.fill_quad(atlas, diamond(4.0), OUTLINE);
            batch.fill_quad(atlas, diamond(3.0), inner);
        }
        1 => {
            batch.fill_quad(atlas, diamond(4.0), CHEVRON);
            batch.fill_quad(atlas, diamond(3.0), KEY_ANIM);
        }
        _ => batch.fill_quad(atlas, diamond(4.0), CHEVRON),
    }
}

fn draw_row_label(ed: &Editor, row: &Row, batch: &mut HudBatch, atlas: &HudAtlas, band: u32) {
    let Some(ch) = ed.doc.channel(row.chan) else {
        return;
    };
    let name = match ed.doc.node(ch.target.node) {
        Some(n) if !n.name.is_empty() => Text::literal(n.name.clone()),
        Some(n) => Text::translate(kind_key(&n.kind), &[]),
        None => Text::literal(String::new()),
    };
    let label = Text::translate(
        "cinematix.row.channel",
        &[name, Text::translate(prop_key(ch.target.prop), &[])],
    );
    let colour = if ed.selected_channel == Some(row.chan) {
        TEXT
    } else if ch.keys.is_empty() {
        FAINT
    } else {
        DIM
    };
    let state = key_state(ch, ed.time(), ed.fps());
    state_diamond(
        batch,
        atlas,
        row.label.x + GUTTER_W - 12.0,
        row.label.y + row.label.h * 0.5,
        state,
        band,
    );
    text(
        batch,
        atlas,
        row.label.x + 6.0,
        row.label.y + 4.0,
        &label,
        colour,
    );
    if ed.selected_channel == Some(row.chan) {
        batch.fill(atlas, row.label.x, row.label.y, 2.0, row.label.h, ACCENT);
    }
}

fn draw_keys(ed: &Editor, f: &Frame, row: &Row, batch: &mut HudBatch, atlas: &HudAtlas) {
    let Some(ch) = ed.doc.channel(row.chan) else {
        return;
    };
    let cy = row.keys.y + row.keys.h * 0.5;
    for (ki, k) in ch.keys.iter().enumerate() {
        let x = f.axis.x(k.t);
        if x < row.keys.x - 4.0 || x > row.keys.x + row.keys.w + 4.0 {
            continue;
        }
        let selected = ed.selected_key == Some((row.chan, ki));
        let colour = if selected { ACCENT } else { KEY };
        let r = if selected { 5.0 } else { 4.0 };
        batch.fill_quad(
            atlas,
            [[x, cy - r], [x + r, cy], [x, cy + r], [x - r, cy]],
            colour,
        );
    }
}

fn draw_playhead(ed: &Editor, f: &Frame, batch: &mut HudBatch, atlas: &HudAtlas) {
    let x = f.axis.x(ed.time());
    if x < f.ruler.x || x > f.ruler.x + f.ruler.w {
        return;
    }
    let top = f.ruler.y;
    let bottom = f.lanes.y + f.lanes.h;
    if bottom <= top {
        return;
    }
    batch.fill(atlas, x, top, 1.0, bottom - top, ACCENT);
    batch.fill_quad(
        atlas,
        [
            [x - 4.0, top],
            [x + 4.0, top],
            [x, top + 6.0],
            [x, top + 6.0],
        ],
        ACCENT,
    );
}

fn button_icon(b: Button, playing: bool) -> Option<Icon> {
    match b {
        Button::PlayPause if playing => Some(glyphs::PAUSE),
        Button::PlayPause => Some(glyphs::PLAY),
        Button::StepPrev => Some(glyphs::FRAME_PREV),
        Button::StepNext => Some(glyphs::FRAME_NEXT),
        Button::AddKey => Some(glyphs::ADD),
        Button::DeleteKey => Some(glyphs::REMOVE),
        Button::Fit => Some(glyphs::MAXIMIZE),
        Button::ZoomIn | Button::ZoomOut => None,
    }
}

fn button_key(b: Button, playing: bool) -> &'static str {
    match b {
        Button::PlayPause if playing => "cinematix.action.pause",
        Button::PlayPause => "cinematix.action.play",
        Button::StepPrev => "cinematix.action.step_back",
        Button::StepNext => "cinematix.action.step_forward",
        Button::AddKey => "cinematix.action.add_key",
        Button::DeleteKey => "cinematix.action.delete_key",
        Button::ZoomIn => "cinematix.action.zoom_in",
        Button::ZoomOut => "cinematix.action.zoom_out",
        Button::Fit => "cinematix.action.fit",
    }
}

fn draw_transport(
    ed: &Editor,
    f: &Frame,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cursor: (f32, f32),
) {
    batch.fill(
        atlas,
        f.transport.x,
        f.transport.y,
        f.transport.w,
        1.0,
        LINE,
    );
    for (b, r) in &f.buttons {
        let label = Text::translate(button_key(*b, ed.playing), &[]);
        let hover = r.contains(cursor.0, cursor.1);
        if hover {
            fill(batch, atlas, *r, HOVER);
        }
        match button_icon(*b, ed.playing) {
            Some(icon) => icon.draw(
                batch,
                atlas,
                r.x + (r.w - 16.0) * 0.5,
                r.y + (r.h - 16.0) * 0.5,
                16.0,
                TEXT,
            ),
            None => {
                let face = match *b {
                    Button::ZoomIn => "+",
                    Button::ZoomOut => "-",
                    _ => "",
                };
                text(
                    batch,
                    atlas,
                    r.x + r.w * 0.5 - 2.0,
                    r.y + 5.0,
                    &Text::literal(face),
                    TEXT,
                );
            }
        }
        register(&label, *r);
    }
    let last = f
        .buttons
        .last()
        .map(|(_, r)| r.x + r.w)
        .unwrap_or(f.transport.x);
    let y = f.transport.y + 8.0;
    let frame = Text::tr("cinematix.header.frame", &[&ed.frame_number()]);
    let time = Text::tr("cinematix.header.time", &[&format!("{:.2}", ed.time())]);
    let right = f.transport.x + f.transport.w - 8.0;
    let time_w = HudBatch::text_width(atlas, &time.plain());
    let frame_w = HudBatch::text_width(atlas, &frame.plain());
    let left = last + 12.0;
    if left + time_w <= right {
        text(batch, atlas, right - time_w, y, &time, DIM);
        if left + frame_w + 12.0 <= right - time_w {
            text(batch, atlas, left, y, &frame, TEXT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_reads_as_unkeyed_animated_or_keyed_at_the_playhead() {
        use cinematix_anim::{Channel, Key, Prop, PropPath, Value};
        use cinematix_scene::NodeId;
        let mut ch = Channel::new(PropPath {
            node: NodeId(1),
            prop: Prop::Fov,
        });
        assert_eq!(key_state(&ch, 0.0, 24.0), 0);
        ch.keys.push(Key {
            t: 0.0,
            v: Value::F32(70.0),
            interp: cinematix_anim::Interp::Linear,
        });
        assert_eq!(key_state(&ch, 0.0, 24.0), 2);
        assert_eq!(key_state(&ch, 0.01, 24.0), 2);
        assert_eq!(key_state(&ch, 0.1, 24.0), 1);
    }

    #[test]
    fn every_field_has_a_translation_key_and_a_bounded_range() {
        for section in Section::ALL {
            for f in crate::editor::fields_for(section) {
                assert!(f.label().starts_with("cinematix."));
                let (min, max, step) = f.range();
                assert!(min < max);
                assert!(step > 0.0);
            }
        }
    }

    #[test]
    fn every_section_has_a_full_and_a_short_translation_key() {
        for s in Section::ALL {
            assert!(s.key().starts_with("cinematix.section."));
            assert!(s.short_key().ends_with(".short"));
        }
    }
}
