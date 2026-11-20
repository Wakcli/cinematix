use crate::document::Document;
use crate::editor::{Button, Field, Section, fields_for};
use cinematix_scene::NodeId;
use cinematix_timeline::TimeAxis;
use cinematix_ui::{Rect, Tokens};

pub const ROW_H: f32 = 18.0;
pub const GUTTER_W: f32 = 156.0;
pub const TRANSPORT_H: f32 = 24.0;
pub const RULER_H: f32 = 18.0;

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
    pub axis: TimeAxis,
    pub rail_items: Vec<(Section, Rect)>,
    pub buttons: Vec<(Button, Rect)>,
    pub fields: Vec<(Field, Rect)>,
    pub rows: Vec<Row>,
    pub list: Vec<(NodeId, Rect)>,
    pub visible_rows: usize,
}

impl Frame {
    pub fn build(doc: &Document, section: Section, scroll: usize, size: (f32, f32)) -> Self {
        let (w, h) = size;
        let t = Tokens::default();
        let header_h = t.header_h;
        let rail_w = t.rail_w;
        let stage_h = (h - header_h).max(0.0);
        let timeline_h = (stage_h * 0.38)
            .clamp(96.0, 240.0)
            .min((stage_h - 48.0).max(0.0));
        let inspector_w = (w * 0.22).clamp(160.0, 300.0).min((w * 0.4).max(0.0));

        let header = Rect::new(0.0, 0.0, w, header_h);
        let rail = Rect::new(0.0, header_h, rail_w, stage_h - timeline_h);
        let inspector = Rect::new(w - inspector_w, header_h, inspector_w, stage_h - timeline_h);
        let viewport = Rect::new(
            rail_w,
            header_h,
            (w - rail_w - inspector_w).max(0.0),
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

        let mut fields = Vec::new();
        let mut fy = inspector.y + 30.0;
        for f in fields_for(section) {
            let rect = Rect::new(
                inspector.x + t.pad,
                fy,
                (inspector.w - t.pad * 2.0).max(20.0),
                20.0,
            );
            if rect.y + rect.h > inspector.y + inspector.h - t.pad {
                break;
            }
            fields.push((*f, rect));
            fy += 24.0;
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

        let mut list = Vec::new();
        let nodes = match section {
            Section::Actors => doc.nodes_of(|k| matches!(k, cinematix_scene::NodeKind::Actor(_))),
            Section::Models => doc.nodes_of(|k| matches!(k, cinematix_scene::NodeKind::Mesh(_))),
            Section::Lights => doc.nodes_of(|k| matches!(k, cinematix_scene::NodeKind::Light(_))),
            _ => Vec::new(),
        };
        let mut ly = inspector.y + 30.0;
        for id in nodes {
            if ly + ROW_H > inspector.y + inspector.h - t.pad {
                break;
            }
            list.push((
                id,
                Rect::new(inspector.x + t.pad, ly, inspector.w - t.pad * 2.0, ROW_H),
            ));
            ly += ROW_H;
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
            axis,
            rail_items,
            buttons,
            fields,
            rows,
            list,
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
        let f = Frame::build(&doc, Section::Camera, 0, (1280.0, 720.0));
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
            let f = Frame::build(&doc, Section::Project, 0, (w, h));
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
        let f = Frame::build(&doc, Section::Camera, 0, (1280.0, 720.0));
        assert_eq!(f.rail_items.len(), Section::ALL.len());
        for (s, r) in &f.rail_items {
            assert!(f.rail.contains(r.x, r.y), "{s:?}");
            assert!(f.rail.contains(r.x + r.w, r.y + r.h), "{s:?}");
        }
    }

    #[test]
    fn rows_cover_the_channels_from_the_scroll_position() {
        let doc = Document::new();
        let f = Frame::build(&doc, Section::Camera, 2, (1280.0, 720.0));
        assert_eq!(f.rows.first().map(|r| r.chan), Some(2));
        assert!(f.rows.len() <= f.visible_rows);
        for r in &f.rows {
            assert!(f.lanes.contains(r.keys.x, r.keys.y));
        }
    }

    #[test]
    fn the_time_axis_maps_the_window_onto_the_ruler() {
        let doc = Document::new();
        let f = Frame::build(&doc, Section::Camera, 0, (1280.0, 720.0)).with_window(0.0, 10.0);
        assert!((f.axis.x(0.0) - f.ruler.x).abs() < 0.01);
        assert!((f.axis.x(10.0) - (f.ruler.x + f.ruler.w)).abs() < 0.01);
    }
}
