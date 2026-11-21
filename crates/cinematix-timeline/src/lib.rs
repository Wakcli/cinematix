use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimeAxis {
    pub t0: f64,
    pub t1: f64,
    pub x0: f32,
    pub x1: f32,
    pub min_span: f64,
}

impl TimeAxis {
    pub fn new(t0: f64, t1: f64, x0: f32, x1: f32) -> Self {
        let (t0, t1) = if t1 > t0 { (t0, t1) } else { (t0, t0 + 1.0) };
        Self { t0, t1, x0, x1, min_span: 1.0 / 120.0 }
    }

    pub fn span(&self) -> f64 {
        self.t1 - self.t0
    }

    pub fn px_per_sec(&self) -> f64 {
        let span = self.span();
        if span <= f64::EPSILON { 0.0 } else { (self.x1 - self.x0) as f64 / span }
    }

    pub fn x(&self, t: f64) -> f32 {
        let f = (t - self.t0) / self.span();
        self.x0 + (f as f32) * (self.x1 - self.x0)
    }

    pub fn t(&self, x: f32) -> f64 {
        let w = self.x1 - self.x0;
        if w.abs() < f32::EPSILON {
            return self.t0;
        }
        self.t0 + ((x - self.x0) / w) as f64 * self.span()
    }

    pub fn visible(&self) -> (f64, f64) {
        (self.t0, self.t1)
    }

    pub fn zoom_at(&self, anchor_x: f32, factor: f64) -> Self {
        let factor = factor.clamp(0.05, 20.0);
        let anchor_t = self.t(anchor_x);
        let mut span = (self.span() / factor).max(self.min_span);
        span = span.min(86_400.0);
        let frac = ((anchor_x - self.x0) / (self.x1 - self.x0)) as f64;
        let t0 = anchor_t - span * frac.clamp(0.0, 1.0);
        let mut out = *self;
        out.t0 = t0;
        out.t1 = t0 + span;
        out
    }

    pub fn pan_px(&self, dx: f32) -> Self {
        let dt = -(dx as f64) / self.px_per_sec().max(1e-9);
        let mut out = *self;
        out.t0 = self.t0 + dt;
        out.t1 = self.t1 + dt;
        out
    }

    pub fn pan_sec(&self, dt: f64) -> Self {
        let mut out = *self;
        out.t0 = self.t0 + dt;
        out.t1 = self.t1 + dt;
        out
    }

    pub fn fit(&self, t0: f64, t1: f64) -> Self {
        let pad = ((t1 - t0) * 0.02).max(1.0 / 120.0);
        Self::new(t0 - pad, t1 + pad, self.x0, self.x1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snap {
    pub frames: bool,
    pub keys: bool,
    pub markers: bool,
}

impl Default for Snap {
    fn default() -> Self {
        Self { frames: true, keys: true, markers: true }
    }
}

pub fn snap_t(t: f64, fps: f32, candidates: &[f64], tol: f64, snap: Snap) -> f64 {
    if snap.keys {
        if let Some(&c) = candidates.iter().min_by(|a, b| {
            (t - **a).abs().partial_cmp(&(t - **b).abs()).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            if (t - c).abs() <= tol {
                return c;
            }
        }
    }
    if snap.frames && fps > 0.0 {
        let f = (t * fps as f64).round();
        let ft = f / fps as f64;
        if (t - ft).abs() <= tol {
            return ft;
        }
    }
    t
}

pub fn hit_key(x: f32, axis: &TimeAxis, times: &[f64], tol_px: f32) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, t) in times.iter().enumerate() {
        let d = (axis.x(*t) - x).abs();
        if d <= tol_px && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

pub fn keys_in_range(times: &[f64], t0: f64, t1: f64) -> std::ops::Range<usize> {
    let a = times.partition_point(|t| *t < t0);
    let b = times.partition_point(|t| *t <= t1);
    a..b
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Playhead {
    pub t: f64,
    pub fps: f32,
}

impl Playhead {
    pub fn new(fps: f32) -> Self {
        Self { t: 0.0, fps }
    }

    pub fn frame(&self) -> i64 {
        (self.t * self.fps as f64).round() as i64
    }

    pub fn step(&mut self, frames: i64) {
        let f = self.frame() + frames;
        self.t = (f as f64 / self.fps as f64).max(0.0);
    }

    pub fn seek_frame(&mut self, f: i64) {
        self.t = f.max(0) as f64 / self.fps as f64;
    }

    pub fn quantise(&mut self) {
        self.t = self.frame() as f64 / self.fps as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_both_ways() {
        let a = TimeAxis::new(0.0, 10.0, 100.0, 600.0);
        assert!((a.x(5.0) - 350.0).abs() < 1e-3);
        assert!((a.t(350.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn zoom_keeps_anchor() {
        let a = TimeAxis::new(0.0, 10.0, 0.0, 500.0);
        let z = a.zoom_at(250.0, 2.0);
        assert!((z.t(250.0) - a.t(250.0)).abs() < 1e-6);
        assert!((z.span() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_respects_min_span() {
        let a = TimeAxis::new(0.0, 10.0, 0.0, 500.0);
        let mut z = a;
        for _ in 0..100 {
            z = z.zoom_at(250.0, 2.0);
        }
        assert!(z.span() >= a.min_span - 1e-12);
    }

    #[test]
    fn pan_moves_window_without_changing_span() {
        let a = TimeAxis::new(0.0, 10.0, 0.0, 500.0);
        let p = a.pan_px(50.0);
        assert!((p.span() - a.span()).abs() < 1e-9);
        assert!((p.x(0.0) - 50.0).abs() < 1e-3);
        assert!((p.t(0.0) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn snaps_to_nearby_key_then_frame() {
        let cands = [1.0, 3.0];
        assert_eq!(snap_t(3.02, 24.0, &cands, 0.05, Snap::default()), 3.0);
        assert!((snap_t(3.5 + 0.001, 24.0, &cands, 0.05, Snap::default()) - 3.5).abs() < 1e-9);
    }

    #[test]
    fn snap_can_be_disabled() {
        let cands = [1.0];
        assert_eq!(snap_t(1.001, 24.0, &cands, 0.05, Snap { frames: false, keys: false, markers: false }), 1.001);
    }

    #[test]
    fn hit_key_picks_nearest_within_tolerance() {
        let a = TimeAxis::new(0.0, 10.0, 0.0, 500.0);
        let times = [1.0, 5.0, 9.0];
        assert_eq!(hit_key(a.x(5.0) + 3.0, &a, &times, 6.0), Some(1));
        assert_eq!(hit_key(a.x(5.0) + 20.0, &a, &times, 6.0), None);
    }

    #[test]
    fn playhead_steps_by_frame() {
        let mut p = Playhead::new(24.0);
        p.step(1);
        assert_eq!(p.frame(), 1);
        p.seek_frame(48);
        assert!((p.t - 2.0).abs() < 1e-9);
    }

    #[test]
    fn range_query_is_half_open() {
        let times = [0.0, 1.0, 2.0, 3.0];
        assert_eq!(keys_in_range(&times, 1.0, 2.0), 1..3);
    }
}
