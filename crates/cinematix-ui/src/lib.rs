use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn split_v(&self, at: f32) -> (Rect, Rect) {
        let at = at.clamp(0.0, self.w);
        (
            Rect::new(self.x, self.y, at, self.h),
            Rect::new(self.x + at, self.y, self.w - at, self.h),
        )
    }

    pub fn split_h(&self, at: f32) -> (Rect, Rect) {
        let at = at.clamp(0.0, self.h);
        (
            Rect::new(self.x, self.y, self.w, at),
            Rect::new(self.x, self.y + at, self.w, self.h - at),
        )
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Dock {
    Leaf(u32),
    Split {
        axis: Axis,
        ratio: f32,
        a: Box<Dock>,
        b: Box<Dock>,
    },
}

impl Dock {
    pub fn leaf(id: u32) -> Self {
        Dock::Leaf(id)
    }

    pub fn split(axis: Axis, ratio: f32, a: Dock, b: Dock) -> Self {
        Dock::Split {
            axis,
            ratio: ratio.clamp(0.05, 0.95),
            a: Box::new(a),
            b: Box::new(b),
        }
    }

    pub fn layout(&self, area: Rect, out: &mut Vec<(u32, Rect)>) {
        match self {
            Dock::Leaf(id) => out.push((*id, area)),
            Dock::Split { axis, ratio, a, b } => {
                let (ra, rb) = match axis {
                    Axis::Horizontal => area.split_v(area.w * ratio),
                    Axis::Vertical => area.split_h(area.h * ratio),
                };
                a.layout(ra, out);
                b.layout(rb, out);
            }
        }
    }

    pub fn set_ratio_at(&mut self, path: &[bool], ratio: f32) {
        let Some((head, rest)) = path.split_first() else {
            return;
        };
        if let Dock::Split {
            axis: _,
            ratio: r,
            a,
            b,
        } = self
        {
            if rest.is_empty() {
                *r = ratio.clamp(0.05, 0.95);
            } else if *head {
                b.set_ratio_at(rest, ratio);
            } else {
                a.set_ratio_at(rest, ratio);
            }
        }
    }

    pub fn find(&self, id: u32) -> Option<Rect> {
        let mut v = Vec::new();
        self.layout(Rect::new(0.0, 0.0, 1.0, 1.0), &mut v);
        v.into_iter().find(|(i, _)| *i == id).map(|(_, r)| r)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Mods {
    pub fn scale(self) -> f64 {
        let mut s = 1.0;
        if self.shift {
            s *= 10.0;
        }
        if self.ctrl {
            s *= 0.1;
        }
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scrubber {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub px_per_step: f64,
}

impl Scrubber {
    pub fn new(value: f64, min: f64, max: f64, step: f64) -> Self {
        Self {
            value: value.clamp(min, max),
            min,
            max,
            step,
            px_per_step: 6.0,
        }
    }

    pub fn drag(&self, dx: f32, mods: Mods) -> f64 {
        let raw = self.value + (dx as f64 / self.px_per_step) * self.step * mods.scale();
        let v = if mods.ctrl { raw } else { self.quantise(raw) };
        v.clamp(self.min, self.max)
    }

    pub fn wheel(&self, ticks: f32, mods: Mods) -> f64 {
        let raw = self.value + ticks as f64 * self.step * mods.scale();
        let v = if mods.ctrl { raw } else { self.quantise(raw) };
        v.clamp(self.min, self.max)
    }

    pub fn quantise(&self, v: f64) -> f64 {
        if self.step <= 0.0 {
            return v;
        }
        (v / self.step).round() * self.step
    }

    pub fn frac(&self) -> f64 {
        if (self.max - self.min).abs() < f64::EPSILON {
            0.0
        } else {
            (self.value - self.min) / (self.max - self.min)
        }
    }

    pub fn from_frac(&self, f: f64) -> f64 {
        (self.min + f.clamp(0.0, 1.0) * (self.max - self.min)).clamp(self.min, self.max)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    pub row_h: f32,
    pub pad: f32,
    pub gap: f32,
    pub rail_w: f32,
    pub header_h: f32,
    pub font_small: f32,
    pub font_body: f32,
    pub font_title: f32,
}

impl Default for Tokens {
    fn default() -> Self {
        Self {
            row_h: 22.0,
            pad: 6.0,
            gap: 4.0,
            rail_w: 30.0,
            header_h: 30.0,
            font_small: 11.0,
            font_body: 13.0,
            font_title: 15.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubber_quantises_to_step() {
        let s = Scrubber::new(0.0, 0.0, 100.0, 0.5);
        assert_eq!(s.drag(6.0, Mods::default()), 0.5);
        assert_eq!(s.drag(3.0, Mods::default()), 0.5);
        assert_eq!(s.drag(-3.0, Mods::default()), 0.0);
    }

    #[test]
    fn modifiers_scale_speed() {
        let s = Scrubber::new(0.0, 0.0, 1000.0, 1.0);
        let coarse = s.drag(
            6.0,
            Mods {
                shift: true,
                ..Default::default()
            },
        );
        let fine = s.drag(
            6.0,
            Mods {
                ctrl: true,
                ..Default::default()
            },
        );
        assert_eq!(coarse, 10.0);
        assert!((fine - 0.1).abs() < 1e-9);
    }

    #[test]
    fn scrubber_clamps() {
        let s = Scrubber::new(9.0, 0.0, 10.0, 1.0);
        assert_eq!(s.drag(1000.0, Mods::default()), 10.0);
        assert_eq!(s.drag(-1000.0, Mods::default()), 0.0);
    }

    #[test]
    fn frac_round_trips() {
        let s = Scrubber::new(2.5, 0.0, 10.0, 0.001);
        assert!((s.from_frac(s.frac()) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn dock_layout_partitions_area() {
        let d = Dock::split(Axis::Horizontal, 0.25, Dock::leaf(1), Dock::leaf(2));
        let mut v = Vec::new();
        d.layout(Rect::new(0.0, 0.0, 100.0, 40.0), &mut v);
        assert_eq!(v.len(), 2);
        assert!((v[0].1.w - 25.0).abs() < 1e-6);
        assert!((v[1].1.w - 75.0).abs() < 1e-6);
        assert!((v[1].1.x - 25.0).abs() < 1e-6);
    }

    #[test]
    fn dock_ratio_is_clamped() {
        let d = Dock::split(Axis::Vertical, 5.0, Dock::leaf(1), Dock::leaf(2));
        let mut v = Vec::new();
        d.layout(Rect::new(0.0, 0.0, 10.0, 100.0), &mut v);
        assert!((v[0].1.h - 95.0).abs() < 1e-6);
    }
}
