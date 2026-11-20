use cinematix_scene::NodeId;
use glam::Quat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Prop {
    Translation,
    Rotation,
    Scale,
    Fov,
    Focus,
    Aperture,
    Intensity,
    Color,
    Visible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PropPath {
    pub node: NodeId,
    pub prop: Prop,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Value {
    F32(f32),
    Vec3([f32; 3]),
    Quat([f32; 4]),
    Color([f32; 4]),
    Bool(bool),
}

impl Value {
    pub fn lerp(self, other: Self, u: f32) -> Self {
        match (self, other) {
            (Value::F32(a), Value::F32(b)) => Value::F32(a + (b - a) * u),
            (Value::Vec3(a), Value::Vec3(b)) => Value::Vec3(lerp3(a, b, u)),
            (Value::Quat(a), Value::Quat(b)) => {
                let q = Quat::from_array(a).slerp(Quat::from_array(b), u);
                Value::Quat(q.normalize().to_array())
            }
            (Value::Color(a), Value::Color(b)) => Value::Color(lerp4(a, b, u)),
            (a, _) => a,
        }
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], u: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u]
}

fn lerp4(a: [f32; 4], b: [f32; 4], u: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
        a[3] + (b[3] - a[3]) * u,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Interp {
    Step,
    Linear,
    Smooth,
    /// Uniform Catmull-Rom through the neighbouring keys, the curve Studio's camera path uses.
    /// Positional values bend through their keys instead of easing on each axis.
    Spline,
    Bezier { out: [f32; 2], in_: [f32; 2] },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub t: f64,
    pub v: Value,
    pub interp: Interp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub target: PropPath,
    pub keys: Vec<Key>,
    pub muted: bool,
}

impl Channel {
    pub fn new(target: PropPath) -> Self {
        Self { target, keys: Vec::new(), muted: false }
    }

    pub fn sort(&mut self) {
        self.keys.sort_by(|a, b| a.t.total_cmp(&b.t));
    }

    pub fn eval(&self, t: f64) -> Option<Value> {
        if self.muted || self.keys.is_empty() {
            return None;
        }
        let i = self.keys.partition_point(|k| k.t <= t);
        if i == 0 {
            return Some(self.keys[0].v);
        }
        if i == self.keys.len() {
            return Some(self.keys[self.keys.len() - 1].v);
        }
        let a = &self.keys[i - 1];
        let b = &self.keys[i];
        let span = b.t - a.t;
        let u = if span <= f64::EPSILON { 1.0 } else { ((t - a.t) / span) as f32 };
        Some(blend(&self.keys, i, u))
    }
}

fn blend(keys: &[Key], i: usize, u: f32) -> Value {
    let a = &keys[i - 1];
    let b = &keys[i];
    let u = match a.interp {
        Interp::Step => 0.0,
        Interp::Linear => u,
        Interp::Smooth => u * u * (3.0 - 2.0 * u),
        Interp::Spline => return spline(keys, i, u),
        Interp::Bezier { out, in_ } => bezier(out, in_, u),
    };
    a.v.lerp(b.v, u)
}

/// Catmull-Rom through `a` and `b`, with the neighbouring keys as tangents. The end segments
/// extrapolate a virtual neighbour (`p1 - (p2 - p1)`), the same choice Studio's camera track
/// makes, so a two-key film is a straight line and a third key bends it.
fn spline(keys: &[Key], i: usize, u: f32) -> Value {
    let a = &keys[i - 1].v;
    let b = &keys[i].v;
    let p0 = if i >= 2 {
        keys[i - 2].v
    } else {
        extrapolate(*a, *b, true)
    };
    let p3 = if i + 1 < keys.len() {
        keys[i + 1].v
    } else {
        extrapolate(*a, *b, false)
    };
    let catmull = |p0: f32, p1: f32, p2: f32, p3: f32| {
        let t2 = u * u;
        let t3 = t2 * u;
        0.5 * ((2.0 * p1)
            + (-p0 + p2) * u
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    };
    match (p0, *a, *b, p3) {
        (Value::F32(p0), Value::F32(p1), Value::F32(p2), Value::F32(p3)) => {
            Value::F32(catmull(p0, p1, p2, p3))
        }
        (Value::Vec3(p0), Value::Vec3(p1), Value::Vec3(p2), Value::Vec3(p3)) => Value::Vec3([
            catmull(p0[0], p1[0], p2[0], p3[0]),
            catmull(p0[1], p1[1], p2[1], p3[1]),
            catmull(p0[2], p1[2], p2[2], p3[2]),
        ]),
        (Value::Color(p0), Value::Color(p1), Value::Color(p2), Value::Color(p3)) => Value::Color([
            catmull(p0[0], p1[0], p2[0], p3[0]),
            catmull(p0[1], p1[1], p2[1], p3[1]),
            catmull(p0[2], p1[2], p2[2], p3[2]),
            catmull(p0[3], p1[3], p2[3], p3[3]),
        ]),
        // Rotations have no meaningful Catmull-Rom; they slerp, as Studio's track does.
        _ => a.lerp(*b, u),
    }
}

fn extrapolate(a: Value, b: Value, backwards: bool) -> Value {
    let f = |a: f32, b: f32| if backwards { a - (b - a) } else { b + (b - a) };
    match (a, b) {
        (Value::F32(a), Value::F32(b)) => Value::F32(f(a, b)),
        (Value::Vec3(a), Value::Vec3(b)) => {
            Value::Vec3([f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])])
        }
        (Value::Color(a), Value::Color(b)) => Value::Color([
            f(a[0], b[0]),
            f(a[1], b[1]),
            f(a[2], b[2]),
            f(a[3], b[3]),
        ]),
        (a, _) => a,
    }
}

fn bezier(out: [f32; 2], in_: [f32; 2], t: f32) -> f32 {
    let (x1, y1) = (out[0].clamp(0.0, 1.0), out[1]);
    let (x2, y2) = (in_[0].clamp(0.0, 1.0), in_[1]);
    let x = t.clamp(0.0, 1.0);
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    let mut u = x;
    for _ in 0..24 {
        let cx = cubic(0.0, x1, x2, 1.0, u);
        if (cx - x).abs() < 1e-5 {
            break;
        }
        if cx < x {
            lo = u;
        } else {
            hi = u;
        }
        u = 0.5 * (lo + hi);
    }
    cubic(0.0, y1, y2, 1.0, u)
}

fn cubic(p0: f32, p1: f32, p2: f32, p3: f32, u: f32) -> f32 {
    let v = 1.0 - u;
    v * v * v * p0 + 3.0 * v * v * u * p1 + 3.0 * v * u * u * p2 + u * u * u * p3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> PropPath {
        PropPath { node: NodeId(1), prop: Prop::Translation }
    }

    fn chan(keys: Vec<Key>) -> Channel {
        let mut c = Channel::new(path());
        c.keys = keys;
        c
    }

    fn k(t: f64, v: f32, interp: Interp) -> Key {
        Key { t, v: Value::F32(v), interp }
    }

    fn v3(t: f64, x: f32, y: f32, z: f32, interp: Interp) -> Key {
        Key { t, v: Value::Vec3([x, y, z]), interp }
    }

    #[test]
    fn a_spline_passes_through_every_key() {
        let c = chan(vec![
            v3(0.0, 0.0, 64.0, 0.0, Interp::Spline),
            v3(1.0, 4.0, 66.0, 2.0, Interp::Spline),
            v3(2.0, 8.0, 64.0, 8.0, Interp::Spline),
        ]);
        for (t, want) in [(0.0, [0.0, 64.0, 0.0]), (1.0, [4.0, 66.0, 2.0]), (2.0, [8.0, 64.0, 8.0])] {
            let Some(Value::Vec3(got)) = c.eval(t) else { panic!("no value at {t}") };
            for axis in 0..3 {
                assert!((got[axis] - want[axis]).abs() < 1e-4, "t={t} axis={axis}: {got:?}");
            }
            assert!((c.eval(t).unwrap().lerp(Value::Vec3(want), 0.0) == Value::Vec3(want)));
        }
    }

    #[test]
    fn a_spline_bends_between_uneven_keys_where_a_straight_line_would_not() {
        let c = chan(vec![
            v3(0.0, 0.0, 64.0, 0.0, Interp::Spline),
            v3(1.0, 1.0, 64.0, 0.0, Interp::Spline),
            v3(2.0, 1.0, 64.0, 1.0, Interp::Spline),
        ]);
        let Some(Value::Vec3(mid)) = c.eval(0.5) else { panic!() };
        assert!(mid[0] > 0.5, "x should overshoot the chord: {mid:?}");
        assert!(mid[2] < 0.0, "z should dip below the first key: {mid:?}");
    }

    #[test]
    fn a_two_key_spline_is_the_straight_segment() {
        let c = chan(vec![
            v3(0.0, 0.0, 64.0, 0.0, Interp::Spline),
            v3(2.0, 10.0, 70.0, -4.0, Interp::Spline),
        ]);
        let Some(Value::Vec3(mid)) = c.eval(1.0) else { panic!() };
        assert!((mid[0] - 5.0).abs() < 1e-4, "{mid:?}");
        assert!((mid[1] - 67.0).abs() < 1e-4, "{mid:?}");
        assert!((mid[2] + 2.0).abs() < 1e-4, "{mid:?}");
    }

    #[test]
    fn spline_rotation_keys_still_slerp() {
        let a = Quat::from_rotation_y(0.0);
        let b = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let c = chan(vec![
            Key { t: 0.0, v: Value::Quat(a.to_array()), interp: Interp::Spline },
            Key { t: 1.0, v: Value::Quat(b.to_array()), interp: Interp::Spline },
        ]);
        let Some(Value::Quat(mid)) = c.eval(0.5) else { panic!() };
        let want = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4).to_array();
        for axis in 0..4 {
            assert!((mid[axis] - want[axis]).abs() < 1e-4, "{mid:?} vs {want:?}");
        }
    }

    #[test]
    fn holds_before_and_after() {
        let c = chan(vec![k(0.0, 1.0, Interp::Linear), k(2.0, 3.0, Interp::Linear)]);
        assert_eq!(c.eval(-1.0), Some(Value::F32(1.0)));
        assert_eq!(c.eval(5.0), Some(Value::F32(3.0)));
    }

    #[test]
    fn linear_midpoint() {
        let c = chan(vec![k(0.0, 0.0, Interp::Linear), k(2.0, 4.0, Interp::Linear)]);
        assert_eq!(c.eval(1.0), Some(Value::F32(2.0)));
    }

    #[test]
    fn step_holds_start_value() {
        let c = chan(vec![k(0.0, 0.0, Interp::Step), k(2.0, 4.0, Interp::Step)]);
        assert_eq!(c.eval(1.9), Some(Value::F32(0.0)));
        assert_eq!(c.eval(2.0), Some(Value::F32(4.0)));
    }

    #[test]
    fn bezier_is_monotonic_and_bounded() {
        let c = chan(vec![
            k(0.0, 0.0, Interp::Bezier { out: [0.8, 0.0], in_: [0.2, 1.0] }),
            k(1.0, 1.0, Interp::Linear),
        ]);
        let mut prev = -1.0;
        for i in 0..=100 {
            let v = match c.eval(i as f64 / 100.0).unwrap() {
                Value::F32(v) => v,
                _ => unreachable!(),
            };
            assert!(v >= prev - 1e-6);
            assert!((0.0..=1.0).contains(&v));
            prev = v;
        }
    }

    #[test]
    fn quat_slerp_stays_unit() {
        let a = Key { t: 0.0, v: Value::Quat(Quat::from_rotation_y(0.0).to_array()), interp: Interp::Linear };
        let b = Key { t: 1.0, v: Value::Quat(Quat::from_rotation_y(1.5).to_array()), interp: Interp::Linear };
        let c = chan(vec![a, b]);
        let Value::Quat(q) = c.eval(0.5).unwrap() else { panic!() };
        assert!((Quat::from_array(q).length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn muted_returns_none() {
        let mut c = chan(vec![k(0.0, 1.0, Interp::Linear)]);
        c.muted = true;
        assert_eq!(c.eval(0.0), None);
    }
}
