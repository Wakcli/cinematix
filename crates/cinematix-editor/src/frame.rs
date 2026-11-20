use crate::document::Document;
use crate::editor::{Button, Field, Section, fields_for};
use cinematix_scene::NodeId;
use cinematix_timeline::TimeAxis;
use cinematix_ui::{Rect, Tokens};
use std::collections::HashSet;

pub const ROW_H: f32 = 18.0;
pub const GUTTER_W: f32 = 156.0;
pub const TRANSPORT_H: f32 = 24.0;
pub const RULER_H: f32 = 18.0;
pub const TAB_H: f32 = 20.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimelineTab {
    DopeSheet,
    Curves,
    Clips,
}

impl TimelineTab {
    pub const ALL: [TimelineTab; 3] = [
        TimelineTab::DopeSheet,
        TimelineTab::Curves,
        TimelineTab::Clips,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TimelineTab::DopeSheet => "cinematix.tab.dope_sheet",
            TimelineTab::Curves => "cinematix.tab.curves",
            TimelineTab::Clips => "cinematix.tab.clips",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InspectorTab {
    Properties,
    Scene,
}

impl InspectorTab {
    pub const ALL: [InspectorTab; 2] = [InspectorTab::Properties, InspectorTab::Scene];

    pub fn label(self) -> &'static str {
        match self {
            InspectorTab::Properties => "cinematix.panel.properties",
            InspectorTab::Scene => "cinematix.panel.scene",
        }
    }
}

/// The two camera vector rows above the scalar fields: a position (X/Y/Z) and a view direction
/// (yaw/pitch). Both edit the camera at the playhead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VectorRow {
    Position,
    Rotation,
}

impl VectorRow {
    pub fn label(self) -> &'static str {
        match self {
            VectorRow::Position => "cinematix.vector.position",
            VectorRow::Rotation => "cinematix.vector.rotation",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    PositionX,
    PositionY,
    PositionZ,
    Yaw,
    Pitch,
}

impl Axis {
    pub fn label(self) -> &'static str {
        match self {
            Axis::PositionX => "X",
            Axis::PositionY => "Y",
            Axis::PositionZ => "Z",
            Axis::Yaw => "Yaw",
            Axis::Pitch => "Pitch",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Axis::PositionX => "cinematix.axis.position_x",
            Axis::PositionY => "cinematix.axis.position_y",
            Axis::PositionZ => "cinematix.axis.position_z",
            Axis::Yaw => "cinematix.axis.yaw",
            Axis::Pitch => "cinematix.axis.pitch",
        }
    }

    pub fn is_position(self) -> bool {
        matches!(self, Axis::PositionX | Axis::PositionY | Axis::PositionZ)
    }

    pub fn component(self) -> usize {
        match self {
            Axis::PositionX => 0,
            Axis::PositionY => 1,
            Axis::PositionZ => 2,
            Axis::Yaw | Axis::Pitch => 0,
        }
    }

    pub fn range(self) -> (f64, f64, f64) {
        match self {
            Axis::PositionX | Axis::PositionY | Axis::PositionZ => {
                (-30_000_000.0, 30_000_000.0, 0.1)
            }
            Axis::Yaw => (-180.0, 180.0, 0.5),
            Axis::Pitch => (-90.0, 90.0, 0.5),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorKind {
    Sky,
    Fog,
}

impl ColorKind {
    pub const ALL: [ColorKind; 2] = [ColorKind::Sky, ColorKind::Fog];

    pub fn label(self) -> &'static str {
        match self {
            ColorKind::Sky => "cinematix.color.sky",
            ColorKind::Fog => "cinematix.color.fog",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Save,
    Undo,
    Redo,
    Render,
}

impl Tool {
    pub const ALL: [Tool; 4] = [Tool::Save, Tool::Undo, Tool::Redo, Tool::Render];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Save => "cinematix.tool.save",
            Tool::Undo => "cinematix.tool.undo",
            Tool::Redo => "cinematix.tool.redo",
            Tool::Render => "cinematix.tool.render",
        }
    }

    /// Whether the tool has a backend that can run now; Undo/Redo depend on the editor's stacks,
    /// so the draw and press paths ask `Editor` instead of this.
    pub fn implemented(self) -> bool {
        true
    }
}

pub struct Row {
    pub chan: usize,
    pub label: Rect,
    pub keys: Rect,
}

pub struct Frame {
    pub size: (f32, f32),
    pub header: Rect,
    pub rail: Rect,
    pub viewport: Rect,
    pub inspector: Rect,
    pub timeline: Rect,
    pub ruler: Rect,
    pub lanes: Rect,
    pub transport: Rect,
    pub split_h: Rect,
    pub split_v: Rect,
    pub tabs: Vec<(TimelineTab, Rect)>,
    pub tools: Vec<(Tool, Rect)>,
    pub inspector_tabs: Vec<(InspectorTab, Rect)>,
    pub inspector_header: Rect,
    pub inspector_redock: Option<Rect>,
    pub inspector_dock: Rect,
    pub inspector_floating: bool,
    /// Value-axis buttons (X/Y/Z/W) in the curve editor, for the selected channel's kind.
    pub curve_axes: Vec<(usize, Rect)>,
    /// Where the curve is plotted (the lane area, under the ruler).
    pub curve_plot: Rect,
    pub axis: TimeAxis,
    pub rail_items: Vec<(Section, Rect)>,
    pub buttons: Vec<(Button, Rect)>,
    pub fields: Vec<(Field, Rect)>,
    pub vectors: Vec<(VectorRow, Option<Rect>, Vec<(Axis, Rect)>)>,
    pub fps_presets: Vec<(u32, Rect)>,
    pub size_pair: Option<(Rect, Rect, Rect)>,
    pub color_rows: Vec<(ColorKind, Rect)>,
    pub clip_track: Option<Rect>,
    pub clip_handles: Vec<(u8, Rect)>,
    pub loop_check: Rect,
    pub free_check: Rect,
    pub rows: Vec<Row>,
    pub tree: Vec<(NodeId, usize, bool, Rect)>,
    pub tree_toggle: Vec<(NodeId, Rect)>,
    pub clip_add: Rect,
    pub clips: Vec<(usize, Rect)>,
    pub clip_delete: Vec<(usize, Rect)>,
    pub visible_rows: usize,
}

impl Frame {
    pub fn build(
        doc: &Document,
        section: Section,
        scroll: usize,
        size: (f32, f32),
        timeline_frac: f32,
        inspector_frac: f32,
        selected_channel: Option<usize>,
        expanded: &HashSet<NodeId>,
        inspector_tab: InspectorTab,
        inspector_float: Option<Rect>,
        selected_clip: Option<usize>,
    ) -> Self {
        let (w, h) = size;
        let t = Tokens::default();
        let header_h = t.header_h;
        let rail_w = t.rail_w;
        let stage_h = (h - header_h).max(0.0);
        let timeline_h = (stage_h * timeline_frac.clamp(0.2, 0.62))
            .clamp(96.0, 320.0)
            .min((stage_h - 80.0).max(0.0));
        let inspector_w = (w * inspector_frac.clamp(0.12, 0.4))
            .clamp(140.0, 340.0)
            .min((w * 0.45).max(0.0));

        let header = Rect::new(0.0, 0.0, w, header_h);
        let rail = Rect::new(0.0, header_h, rail_w, stage_h - timeline_h);
        let inspector_dock =
            Rect::new(w - inspector_w, header_h, inspector_w, stage_h - timeline_h);
        let float = inspector_float.map(|r| {
            let fw = r.w.clamp(190.0, (w - rail_w - 40.0).max(190.0));
            let fh = r.h.clamp(150.0, (h - header_h - 36.0).max(150.0));
            let fx = r.x.clamp(rail_w, (w - fw).max(rail_w));
            let fy = r.y.clamp(header_h, (h - fh).max(header_h));
            Rect::new(fx, fy, fw, fh)
        });
        let inspector_floating = float.is_some();
        let inspector = float.unwrap_or(inspector_dock);
        let viewport = Rect::new(
            rail_w,
            header_h,
            (w - rail_w - if inspector_floating { 0.0 } else { inspector_w }).max(0.0),
            stage_h - timeline_h,
        );
        let timeline = Rect::new(rail_w, h - timeline_h, (w - rail_w).max(0.0), timeline_h);
        let transport = Rect::new(
            timeline.x,
            timeline.y + timeline.h - TRANSPORT_H,
            timeline.w,
            TRANSPORT_H,
        );
        let ruler = Rect::new(
            timeline.x + GUTTER_W,
            timeline.y + 22.0,
            (timeline.w - GUTTER_W).max(0.0),
            RULER_H,
        );
        let lanes = Rect::new(
            timeline.x,
            ruler.y + ruler.h,
            timeline.w,
            (transport.y - (ruler.y + ruler.h)).max(0.0),
        );

        let axis = TimeAxis::new(0.0, 1.0, ruler.x, ruler.x + ruler.w);

        let split_h = Rect::new(rail_w, timeline.y - 2.0, (w - rail_w).max(0.0), 4.0);
        let split_v = if inspector_floating {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        } else {
            Rect::new(
                inspector.x - 2.0,
                header_h,
                4.0,
                (stage_h - timeline_h).max(0.0),
            )
        };

        let mut tabs = Vec::with_capacity(TimelineTab::ALL.len());
        let mut tab_x = timeline.x + 4.0;
        for tab in TimelineTab::ALL {
            let width = match tab {
                TimelineTab::DopeSheet => 84.0,
                TimelineTab::Curves => 88.0,
                TimelineTab::Clips => 56.0,
            };
            let rect = Rect::new(tab_x, timeline.y + 1.0, width, TAB_H);
            if rect.x + rect.w > timeline.x + timeline.w - 4.0 {
                break;
            }
            tabs.push((tab, rect));
            tab_x += rect.w + 2.0;
        }

        let inspector_header = Rect::new(inspector.x, inspector.y, inspector.w, 20.0);
        let mut inspector_tabs = Vec::new();
        let tab_right = inspector.x + inspector.w - if inspector_floating { 24.0 } else { 4.0 };
        let mut tab_x = inspector.x + 4.0;
        for tab in InspectorTab::ALL {
            let width = match tab {
                InspectorTab::Properties => 74.0,
                InspectorTab::Scene => 52.0,
            };
            if tab_x + width > tab_right {
                break;
            }
            inspector_tabs.push((tab, Rect::new(tab_x, inspector.y + 2.0, width, 16.0)));
            tab_x += width + 2.0;
        }
        let inspector_redock = inspector_floating.then(|| {
            Rect::new(
                inspector.x + inspector.w - 20.0,
                inspector.y + 2.0,
                16.0,
                16.0,
            )
        });

        let mut curve_axes = Vec::new();
        if let Some(ci) = selected_channel
            && let Some(kind) = doc.value_at(ci, 0.0)
        {
            let count = crate::curve::components(kind);
            let mut ax = timeline.x + 240.0;
            for i in 0..count {
                let rect = Rect::new(ax, timeline.y + 2.0, 16.0, 16.0);
                if rect.x + rect.w > timeline.x + timeline.w - 4.0 {
                    break;
                }
                curve_axes.push((i, rect));
                ax += 18.0;
            }
        }
        let curve_plot = Rect::new(
            lanes.x + 44.0,
            lanes.y + 3.0,
            (lanes.w - 48.0).max(0.0),
            (lanes.h - 6.0).max(0.0),
        );

        let mut tools = Vec::with_capacity(Tool::ALL.len());
        let mut tool_x = 72.0;
        for tool in Tool::ALL {
            let rect = Rect::new(tool_x, 4.0, 20.0, 20.0);
            if rect.x + rect.w > w - 8.0 {
                break;
            }
            tools.push((tool, rect));
            tool_x += 22.0;
        }

        let mut rail_items = Vec::with_capacity(Section::ALL.len());
        let mut y = rail.y + t.pad;
        for s in Section::ALL {
            let item = Rect::new(rail.x + 3.0, y, (rail.w - 6.0).max(0.0), 26.0);
            if item.y + item.h > rail.y + rail.h {
                break;
            }
            rail_items.push((s, item));
            y += item.h + t.gap;
        }

        let mut buttons = Vec::new();
        let mut x = transport.x + t.pad;
        for b in Button::ALL {
            let rect = Rect::new(x, transport.y + 3.0, b.width(), TRANSPORT_H - 6.0);
            if rect.x + rect.w > transport.x + transport.w - t.pad {
                break;
            }
            buttons.push((b, rect));
            x += rect.w + t.gap;
        }

        let loop_check = if timeline.w > 360.0 {
            Rect::new(timeline.x + 270.0, timeline.y + 2.0, 52.0, 16.0)
        } else {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        };
        let free_check = if timeline.w > 440.0 {
            Rect::new(loop_check.x + 56.0, loop_check.y, 72.0, 16.0)
        } else {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        };

        let mut fields = Vec::new();
        let mut vectors = Vec::new();
        let mut color_rows = Vec::new();
        let mut fps_presets = Vec::new();
        let mut size_pair = None;
        let pad = t.pad;
        let left = inspector.x + pad;
        let inner_w = (inspector.w - pad * 2.0).max(20.0);
        let limit = inspector.y + inspector.h - pad;
        let mut fy = inspector.y + 42.0;
        if inspector_tab == InspectorTab::Properties {
            for f in fields_for(section) {
                let rect = Rect::new(left, fy, inner_w, 20.0);
                if rect.y + rect.h > limit {
                    break;
                }
                fields.push((*f, rect));
                fy += 24.0;
            }
            if section == Section::Camera && fy + 44.0 < limit {
                fy += 4.0;
                for row in [VectorRow::Position, VectorRow::Rotation] {
                    let label = Rect::new(left, fy, inner_w, 12.0);
                    fy += 14.0;
                    let axes: &[Axis] = match row {
                        VectorRow::Position => &[Axis::PositionX, Axis::PositionY, Axis::PositionZ],
                        VectorRow::Rotation => &[Axis::Yaw, Axis::Pitch],
                    };
                    let gap = 4.0;
                    let bw =
                        ((inner_w - gap * (axes.len() as f32 - 1.0)) / axes.len() as f32).max(18.0);
                    let mut ax = left;
                    let mut rects = Vec::with_capacity(axes.len());
                    for axis in axes {
                        if fy + 22.0 > limit {
                            break;
                        }
                        rects.push((*axis, Rect::new(ax, fy, bw, 22.0)));
                        ax += bw + gap;
                    }
                    fy += 28.0;
                    vectors.push((row, Some(label), rects));
                }
            }
            if section == Section::Render {
                let label = Rect::new(left, fy, inner_w, 12.0);
                let label_w = 28.0;
                let gap = 4.0;
                let bw = ((inner_w - label_w - gap * 3.0) / 4.0).max(18.0);
                let mut bx = left + label_w;
                for fps in [24u32, 30, 60, 120] {
                    if bx + bw > left + inner_w + 0.01 {
                        break;
                    }
                    fps_presets.push((fps, Rect::new(bx, fy, bw, 22.0)));
                    bx += bw + gap;
                }
                let _ = label;
                fy += 32.0;
                if fy + 24.0 <= limit {
                    let link_w = 16.0;
                    let bw = ((inner_w - link_w - gap * 2.0) / 2.0).max(24.0);
                    let w = Rect::new(left, fy, bw, 24.0);
                    let link = Rect::new(left + bw + gap, fy + 4.0, link_w, 16.0);
                    let h = Rect::new(left + bw + gap + link_w + gap, fy, bw, 24.0);
                    size_pair = Some((w, h, link));
                    fy += 32.0;
                }
            }
            if section == Section::World {
                fy += 4.0;
                for kind in ColorKind::ALL {
                    if fy + 24.0 > limit {
                        break;
                    }
                    color_rows.push((kind, Rect::new(left, fy, inner_w, 24.0)));
                    fy += 28.0;
                }
            }
        }

        let clip_add = Rect::new(lanes.x + 4.0, lanes.y + 2.0, 76.0, 16.0);
        let mut clips = Vec::new();
        let mut clip_delete = Vec::new();
        let mut cy = lanes.y + ROW_H + 2.0;
        for (i, _) in doc.scene.meta.clips.iter().enumerate() {
            if cy + ROW_H > transport.y {
                break;
            }
            let row = Rect::new(lanes.x + 4.0, cy, (lanes.w - 8.0).max(20.0), ROW_H);
            clips.push((i, row));
            clip_delete.push((i, Rect::new(row.x + row.w - 18.0, cy + 2.0, 16.0, 16.0)));
            cy += ROW_H;
        }
        let mut clip_track = None;
        let mut clip_handles = Vec::new();
        if let Some(i) = selected_clip
            && let Some(clip) = doc.scene.meta.clips.get(i)
            && cy + 20.0 <= transport.y
        {
            let track = Rect::new(lanes.x + 8.0, cy + 4.0, (lanes.w - 16.0).max(20.0), 14.0);
            let (r0, r1) = doc.scene.meta.range;
            let span = (r1 - r0).max(1e-9);
            let x_at = |t: f64| track.x + (((t - r0) / span).clamp(0.0, 1.0) as f32) * track.w;
            for (edge, t) in [(0u8, clip.start), (1u8, clip.end)] {
                let x = x_at(t);
                clip_handles.push((edge, Rect::new(x - 2.0, track.y - 1.0, 4.0, track.h + 2.0)));
            }
            clip_track = Some(track);
        }

        let mut rows = Vec::new();
        let visible_rows = (lanes.h / ROW_H).floor().max(0.0) as usize;
        for (i, (chan, _)) in doc
            .channels
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_rows)
            .enumerate()
        {
            let y = lanes.y + i as f32 * ROW_H;
            rows.push(Row {
                chan,
                label: Rect::new(lanes.x, y, GUTTER_W.min(lanes.w), ROW_H),
                keys: Rect::new(lanes.x + GUTTER_W, y, (lanes.w - GUTTER_W).max(0.0), ROW_H),
            });
        }

        let mut tree = Vec::new();
        let mut tree_toggle = Vec::new();
        let mut ly = inspector.y + 26.0;
        let bottom = inspector.y + inspector.h - t.pad;
        fn walk(
            doc: &Document,
            id: NodeId,
            depth: usize,
            expanded: &HashSet<NodeId>,
            y: &mut f32,
            bottom: f32,
            x: f32,
            w: f32,
            tree: &mut Vec<(NodeId, usize, bool, Rect)>,
            toggles: &mut Vec<(NodeId, Rect)>,
        ) {
            let Some(node) = doc.node(id) else {
                return;
            };
            if *y + ROW_H > bottom {
                return;
            }
            let has_children = !node.children.is_empty();
            let row = Rect::new(x + depth as f32 * 10.0, *y, w - depth as f32 * 10.0, ROW_H);
            tree.push((id, depth, has_children, row));
            if has_children {
                toggles.push((id, Rect::new(row.x + 2.0, row.y + 4.0, 10.0, 10.0)));
            }
            *y += ROW_H;
            if has_children && expanded.contains(&id) {
                for child in node.children.clone() {
                    walk(
                        doc,
                        child,
                        depth + 1,
                        expanded,
                        y,
                        bottom,
                        x,
                        w,
                        tree,
                        toggles,
                    );
                }
            }
        }
        if inspector_tab == InspectorTab::Scene {
            for root in doc.scene.roots.clone() {
                walk(
                    doc,
                    root,
                    0,
                    expanded,
                    &mut ly,
                    bottom,
                    inspector.x + t.pad,
                    inspector.w - t.pad * 2.0,
                    &mut tree,
                    &mut tree_toggle,
                );
            }
        }

        Self {
            size: (w, h),
            header,
            rail,
            viewport,
            inspector,
            timeline,
            ruler,
            lanes,
            transport,
            split_h,
            split_v,
            tabs,
            tools,
            inspector_tabs,
            inspector_header,
            inspector_redock,
            inspector_dock,
            inspector_floating,
            curve_axes,
            curve_plot,
            axis,
            rail_items,
            buttons,
            fields,
            vectors,
            fps_presets,
            size_pair,
            color_rows,
            clip_track,
            clip_handles,
            loop_check,
            free_check,
            rows,
            tree,
            tree_toggle,
            clip_add,
            clips,
            clip_delete,
            visible_rows,
        }
    }

    /// The axis over the document's visible window, replacing the placeholder built above.
    pub fn with_window(mut self, t0: f64, t1: f64) -> Self {
        self.axis = TimeAxis::new(t0, t1, self.ruler.x, self.ruler.x + self.ruler.w);
        self
    }

    pub fn key_area(&self, chan: usize) -> Option<Rect> {
        self.rows.iter().find(|r| r.chan == chan).map(|r| r.keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_tile_without_overlapping() {
        let doc = Document::new();
        let f = Frame::build(
            &doc,
            Section::Camera,
            0,
            (1280.0, 720.0),
            0.38,
            0.22,
            None,
            &std::collections::HashSet::new(),
            InspectorTab::Properties,
            None,
            None,
        );
        assert_eq!(f.header.w, 1280.0);
        assert!(f.rail.x + f.rail.w <= f.viewport.x + 0.01);
        assert!(f.viewport.x + f.viewport.w <= f.inspector.x + 0.01);
        assert!(f.timeline.y >= f.viewport.y + f.viewport.h - 0.01);
        assert!(f.transport.y + f.transport.h <= f.timeline.y + f.timeline.h + 0.01);
        assert!(f.ruler.y >= f.timeline.y);
    }

    #[test]
    fn a_small_window_still_places_every_region_inside_it() {
        for (w, h) in [(320.0, 240.0), (640.0, 480.0), (3840.0, 2160.0)] {
            let doc = Document::new();
            let f = Frame::build(
                &doc,
                Section::Project,
                0,
                (w, h),
                0.38,
                0.22,
                None,
                &std::collections::HashSet::new(),
                InspectorTab::Properties,
                None,
                None,
            );
            for r in [
                f.header,
                f.rail,
                f.viewport,
                f.inspector,
                f.timeline,
                f.transport,
            ] {
                assert!(r.x >= -0.01 && r.y >= -0.01, "{r:?} at {w}x{h}");
                assert!(r.x + r.w <= w + 0.01, "{r:?} at {w}x{h}");
                assert!(r.y + r.h <= h + 0.01, "{r:?} at {w}x{h}");
            }
        }
    }

    #[test]
    fn every_rail_section_gets_a_button_that_stays_in_the_rail() {
        let doc = Document::new();
        let f = Frame::build(
            &doc,
            Section::Camera,
            0,
            (1280.0, 720.0),
            0.38,
            0.22,
            None,
            &std::collections::HashSet::new(),
            InspectorTab::Properties,
            None,
            None,
        );
        assert_eq!(f.rail_items.len(), Section::ALL.len());
        for (s, r) in &f.rail_items {
            assert!(f.rail.contains(r.x, r.y), "{s:?}");
            assert!(f.rail.contains(r.x + r.w, r.y + r.h), "{s:?}");
        }
    }

    #[test]
    fn the_camera_inspector_has_two_vector_rows_inside_the_panel() {
        let doc = Document::new();
        let f = Frame::build(
            &doc,
            Section::Camera,
            0,
            (1280.0, 720.0),
            0.38,
            0.22,
            None,
            &std::collections::HashSet::new(),
            InspectorTab::Properties,
            None,
            None,
        );
        assert_eq!(f.vectors.len(), 2);
        assert_eq!(f.vectors[0].2.len(), 3);
        assert_eq!(f.vectors[1].2.len(), 2);
        for (_, label, axes) in &f.vectors {
            if let Some(l) = label {
                assert!(f.inspector.contains(l.x, l.y));
            }
            for (_, r) in axes {
                assert!(
                    f.inspector.contains(r.x, r.y) && f.inspector.contains(r.x + r.w, r.y + r.h),
                    "{r:?}"
                );
            }
        }
    }

    #[test]
    fn the_render_inspector_has_presets_and_a_size_pair() {
        let doc = Document::new();
        let f = Frame::build(
            &doc,
            Section::Render,
            0,
            (1280.0, 720.0),
            0.38,
            0.22,
            None,
            &std::collections::HashSet::new(),
            InspectorTab::Properties,
            None,
            None,
        );
        assert!(f.fields.is_empty());
        assert!(f.free_check.w > 0.0);
        assert_eq!(f.fps_presets.len(), 4);
        let (w, h, link) = f.size_pair.unwrap();
        for r in [w, h, link] {
            assert!(f.inspector.contains(r.x, r.y));
        }
    }

    #[test]
    fn rows_cover_the_channels_from_the_scroll_position() {
        let doc = Document::new();
        let f = Frame::build(
            &doc,
            Section::Camera,
            2,
            (1280.0, 720.0),
            0.38,
            0.22,
            None,
            &std::collections::HashSet::new(),
            InspectorTab::Properties,
            None,
            None,
        );
        assert_eq!(f.rows.first().map(|r| r.chan), Some(2));
        assert!(f.rows.len() <= f.visible_rows);
        for r in &f.rows {
            assert!(f.lanes.contains(r.keys.x, r.keys.y));
        }
    }

    #[test]
    fn the_time_axis_maps_the_window_onto_the_ruler() {
        let doc = Document::new();
        let f = Frame::build(
            &doc,
            Section::Camera,
            0,
            (1280.0, 720.0),
            0.38,
            0.22,
            None,
            &std::collections::HashSet::new(),
            InspectorTab::Properties,
            None,
            None,
        )
        .with_window(0.0, 10.0);
        assert!((f.axis.x(0.0) - f.ruler.x).abs() < 0.01);
        assert!((f.axis.x(10.0) - (f.ruler.x + f.ruler.w)).abs() < 0.01);
    }
}
