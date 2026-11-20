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
        Some(blend(a, b, u))
    }
}

fn blend(a: &Key, b: &Key, u: f32) -> Value {
    let u = match a.interp {
        Interp::Step => 0.0,
        Interp::Linear => u,
        Interp::Smooth => u * u * (3.0 - 2.0 * u),
        Interp::Bezier { out, in_ } => bezier(out, in_, u),
    };
    a.v.lerp(b.v, u)
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
