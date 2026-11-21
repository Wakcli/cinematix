use cinematix_scene::NodeId;
use glam::{Quat, Vec3};
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
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
    ]
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
    /// Displacement of the path midpoint from the straight segment.
    Quadratic {
        midpoint_offset: [f32; 3],
    },
    Bezier {
        out: [f32; 2],
        in_: [f32; 2],
    },
}

/// Timing only; leaves the segment's spatial shape unchanged.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ease {
    #[default]
    None,
    In,
    Out,
    InOut,
}

impl Ease {
    fn sample(self, u: f32) -> f32 {
        match self {
            Self::None => u,
            Self::In => u * u,
            Self::Out => u * (2.0 - u),
            Self::InOut => u * u * (3.0 - 2.0 * u),
        }
    }

    pub fn midpoint_time_fraction(self) -> f64 {
        match self {
            Self::In => 0.5_f64.sqrt(),
            Self::Out => 1.0 - 0.5_f64.sqrt(),
            _ => 0.5,
        }
    }

    fn is_none(&self) -> bool {
        *self == Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub t: f64,
    pub v: Value,
    pub interp: Interp,
    #[serde(default, skip_serializing_if = "Ease::is_none")]
    pub ease: Ease,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub target: PropPath,
    pub keys: Vec<Key>,
    pub muted: bool,
}

impl Channel {
    pub fn new(target: PropPath) -> Self {
        Self {
            target,
            keys: Vec::new(),
            muted: false,
        }
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
        let u = if span <= f64::EPSILON {
            1.0
        } else {
            ((t - a.t) / span) as f32
        };
        Some(blend(&self.keys, i, u))
    }
}

fn blend(keys: &[Key], i: usize, u: f32) -> Value {
    let a = &keys[i - 1];
    let b = &keys[i];
    let u = a.ease.sample(u);
    let u = match a.interp {
        Interp::Step => 0.0,
        Interp::Linear => u,
        Interp::Smooth => u * u * (3.0 - 2.0 * u),
        Interp::Spline => return spline(keys, i, u),
        Interp::Quadratic { midpoint_offset } => {
            let v = a.v.lerp(b.v, u);
            return match v {
                Value::Vec3(mut position) => {
                    for (component, offset) in position.iter_mut().zip(midpoint_offset) {
                        *component += 4.0 * u * (1.0 - u) * offset;
                    }
                    Value::Vec3(position)
                }
                _ => v,
            };
        }
        Interp::Bezier { out, in_ } => bezier(out, in_, u),
    };
    a.v.lerp(b.v, u)
}

/// Centripetal Catmull-Rom through the segment from `keys[i - 1]` to `keys[i]`, with the
/// neighbouring keys as controls and their real times as the knots. Flashback's SMOOTH mode
/// uses the same parameterisation; it keeps uneven key spacing from over-shooting the way the
/// uniform formula can. The end segments use the segment's own key twice, so a two-key film is
/// still the straight segment.
fn spline(keys: &[Key], i: usize, u: f32) -> Value {
    let a = keys[i - 1];
    let b = keys[i];
    let before = if i >= 2 { keys[i - 2] } else { a };
    let after = if i + 1 < keys.len() { keys[i + 1] } else { b };
    let time1 = (a.t - before.t) as f32;
    let time2 = (b.t - before.t) as f32;
    let time3 = (after.t - before.t) as f32;
    match (before.v, a.v, b.v, after.v) {
        (Value::F32(p0), Value::F32(p1), Value::F32(p2), Value::F32(p3)) => {
            Value::F32(catmull_scalar(p0, p1, p2, p3, time1, time2, time3, u))
        }
        (Value::Vec3(p0), Value::Vec3(p1), Value::Vec3(p2), Value::Vec3(p3)) => Value::Vec3(
            catmull_vec3(
                Vec3::from_array(p0),
                Vec3::from_array(p1),
                Vec3::from_array(p2),
                Vec3::from_array(p3),
                time1,
                time2,
                time3,
                u,
            )
            .to_array(),
        ),
        (Value::Color(p0), Value::Color(p1), Value::Color(p2), Value::Color(p3)) => {
            Value::Color(std::array::from_fn(|c| {
                catmull_scalar(p0[c], p1[c], p2[c], p3[c], time1, time2, time3, u)
            }))
        }
        // Rotations have no meaningful Catmull-Rom; they slerp, as Studio's track does.
        _ => a.v.lerp(b.v, u),
    }
}

fn catmull_knot(a: Vec3, b: Vec3) -> f32 {
    (b - a).length_squared().powf(0.25)
}

fn catmull_scalar_knot(a: f32, b: f32) -> f32 {
    (b - a).abs().sqrt()
}

/// The time-aware centripetal Catmull-Rom of Flashback's `CatmullRom.position`.
fn catmull_vec3(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    time1: f32,
    time2: f32,
    time3: f32,
    amount: f32,
) -> Vec3 {
    let tj1 = catmull_knot(p0, p1);
    let tj2 = catmull_knot(p1, p2);
    let tj3 = catmull_knot(p2, p3);
    let average = (tj1 + tj2 + tj3) / 3.0;
    if average <= 1e-9 {
        return p1;
    }
    let relation = (time3 / 3.0) / average;
    let delta = |tj: f32, dt: f32| {
        if dt <= 0.0 {
            0.0
        } else {
            (tj * relation) / ((tj * relation) / dt).clamp(0.4, 2.5)
        }
    };
    let dt1 = delta(tj1, time1);
    let dt2 = delta(tj2, time2 - time1);
    let dt3 = delta(tj3, time3 - time2);
    let (t0, t1) = (0.0, dt1);
    let t2 = t1 + dt2;
    let t3 = t2 + dt3;
    let t = t1 + (t2 - t1) * amount;
    let lerp = |a: Vec3, b: Vec3, u: f32| a.lerp(b, u.clamp(0.0, 1.0));
    let a1 = if t0 == t1 {
        lerp(p0, p1, 0.5)
    } else {
        lerp(p0, p1, (t - t0) / (t1 - t0))
    };
    let a2 = if t1 == t2 {
        lerp(p1, p2, 0.5)
    } else {
        lerp(p1, p2, (t - t1) / (t2 - t1))
    };
    let a3 = if t2 == t3 {
        lerp(p2, p3, 0.5)
    } else {
        lerp(p2, p3, (t - t2) / (t3 - t2))
    };
    let b1 = if t0 == t2 {
        lerp(a1, a2, 0.5)
    } else {
        lerp(a1, a2, (t - t0) / (t2 - t0))
    };
    let b2 = if t1 == t3 {
        lerp(a2, a3, 0.5)
    } else {
        lerp(a2, a3, (t - t1) / (t3 - t1))
    };
    if t1 == t2 {
        lerp(b1, b2, 0.5)
    } else {
        lerp(b1, b2, (t - t1) / (t2 - t1))
    }
}

fn catmull_scalar(
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    time1: f32,
    time2: f32,
    time3: f32,
    amount: f32,
) -> f32 {
    let tj1 = catmull_scalar_knot(p0, p1);
    let tj2 = catmull_scalar_knot(p1, p2);
    let tj3 = catmull_scalar_knot(p2, p3);
    let average = (tj1 + tj2 + tj3) / 3.0;
    if average <= 1e-9 {
        return p1;
    }
    let relation = (time3 / 3.0) / average;
    let delta = |tj: f32, dt: f32| {
        if dt <= 0.0 {
            0.0
        } else {
            (tj * relation) / ((tj * relation) / dt).clamp(0.4, 2.5)
        }
    };
    let dt1 = delta(tj1, time1);
    let dt2 = delta(tj2, time2 - time1);
    let dt3 = delta(tj3, time3 - time2);
    let (t0, t1) = (0.0, dt1);
    let t2 = t1 + dt2;
    let t3 = t2 + dt3;
    let t = t1 + (t2 - t1) * amount;
    let lerp = |a: f32, b: f32, u: f32| a + (b - a) * u.clamp(0.0, 1.0);
    let a1 = if t0 == t1 {
        lerp(p0, p1, 0.5)
    } else {
        lerp(p0, p1, (t - t0) / (t1 - t0))
    };
    let a2 = if t1 == t2 {
        lerp(p1, p2, 0.5)
    } else {
        lerp(p1, p2, (t - t1) / (t2 - t1))
    };
    let a3 = if t2 == t3 {
        lerp(p2, p3, 0.5)
    } else {
        lerp(p2, p3, (t - t2) / (t3 - t2))
    };
    let b1 = if t0 == t2 {
        lerp(a1, a2, 0.5)
    } else {
        lerp(a1, a2, (t - t0) / (t2 - t0))
    };
    let b2 = if t1 == t3 {
        lerp(a2, a3, 0.5)
    } else {
        lerp(a2, a3, (t - t1) / (t3 - t1))
    };
    if t1 == t2 {
        lerp(b1, b2, 0.5)
    } else {
        lerp(b1, b2, (t - t1) / (t2 - t1))
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
        PropPath {
            node: NodeId(1),
            prop: Prop::Translation,
        }
    }

    fn chan(keys: Vec<Key>) -> Channel {
        let mut c = Channel::new(path());
        c.keys = keys;
        c
    }

    fn k(t: f64, v: f32, interp: Interp) -> Key {
        Key {
            ease: Default::default(),
            t,
            v: Value::F32(v),
            interp,
        }
    }

    fn v3(t: f64, x: f32, y: f32, z: f32, interp: Interp) -> Key {
        Key {
            ease: Default::default(),
            t,
            v: Value::Vec3([x, y, z]),
            interp,
        }
    }

    #[test]
    fn timing_presets_do_not_change_the_quadratic_path() {
        let mut a = v3(
            0.0,
            0.0,
            0.0,
            0.0,
            Interp::Quadratic {
                midpoint_offset: [0.0, 4.0, 0.0],
            },
        );
        let b = v3(2.0, 8.0, 0.0, 0.0, Interp::Linear);
        for ease in [Ease::In, Ease::Out, Ease::InOut] {
            a.ease = ease;
            let channel = chan(vec![a, b]);
            let Value::Vec3(midpoint) = channel.eval(2.0 * ease.midpoint_time_fraction()).unwrap()
            else {
                panic!()
            };
            assert!((Vec3::from_array(midpoint) - Vec3::new(4.0, 4.0, 0.0)).length() < 1e-5);
            assert_eq!(channel.eval(0.0), Some(a.v));
            assert_eq!(channel.eval(2.0), Some(b.v));
        }
    }

    #[test]
    fn quadratic_midpoint_bends_without_moving_endpoints() {
        let keys = [
            Key {
                ease: Default::default(),
                t: 0.0,
                v: Value::Vec3([0.0, 0.0, 0.0]),
                interp: Interp::Quadratic {
                    midpoint_offset: [0.0, 4.0, 0.0],
                },
            },
            Key {
                ease: Default::default(),
                t: 2.0,
                v: Value::Vec3([8.0, 0.0, 0.0]),
                interp: Interp::Linear,
            },
        ];
        assert_eq!(blend(&keys, 1, 0.0), keys[0].v);
        assert_eq!(blend(&keys, 1, 0.25), Value::Vec3([2.0, 3.0, 0.0]));
        assert_eq!(blend(&keys, 1, 0.5), Value::Vec3([4.0, 4.0, 0.0]));
        assert_eq!(blend(&keys, 1, 1.0), keys[1].v);
    }

    #[test]
    fn a_spline_passes_through_every_key() {
        let c = chan(vec![
            v3(0.0, 0.0, 64.0, 0.0, Interp::Spline),
            v3(1.0, 4.0, 66.0, 2.0, Interp::Spline),
            v3(2.0, 8.0, 64.0, 8.0, Interp::Spline),
        ]);
        for (t, want) in [
            (0.0, [0.0, 64.0, 0.0]),
            (1.0, [4.0, 66.0, 2.0]),
            (2.0, [8.0, 64.0, 8.0]),
        ] {
            let Some(Value::Vec3(got)) = c.eval(t) else {
                panic!("no value at {t}")
            };
            for axis in 0..3 {
                assert!(
                    (got[axis] - want[axis]).abs() < 1e-4,
                    "t={t} axis={axis}: {got:?}"
                );
            }
            assert!((c.eval(t).unwrap().lerp(Value::Vec3(want), 0.0) == Value::Vec3(want)));
        }
    }

    #[test]
    fn a_spline_eases_between_uneven_keys_instead_of_using_the_chord() {
        let c = chan(vec![
            v3(0.0, 0.0, 64.0, 0.0, Interp::Spline),
            v3(1.0, 1.0, 64.0, 0.0, Interp::Spline),
            v3(2.0, 1.0, 64.0, 1.0, Interp::Spline),
        ]);
        let Some(Value::Vec3(mid)) = c.eval(0.5) else {
            panic!()
        };
        assert!(
            (mid[0] - 0.5).abs() > 0.01,
            "the curve eases instead of the straight chord: {mid:?}"
        );
    }

    #[test]
    fn a_spline_bows_off_the_chord_through_a_corner() {
        let c = chan(vec![
            v3(0.0, 0.0, 64.0, 0.0, Interp::Spline),
            v3(1.0, 5.0, 64.0, 5.0, Interp::Spline),
            v3(2.0, 10.0, 64.0, 0.0, Interp::Spline),
        ]);
        let Some(Value::Vec3(second)) = c.eval(1.5) else {
            panic!()
        };
        // The chord at this point is (7.5, 64, 2.5); a curved path is off it.
        let bow = ((second[0] - 7.5).powi(2) + (second[2] - 2.5).powi(2)).sqrt();
        assert!(
            bow > 0.01,
            "the path should bow around the corner: {second:?}"
        );
    }

    #[test]
    fn a_two_key_spline_is_the_straight_segment() {
        let c = chan(vec![
            v3(0.0, 0.0, 64.0, 0.0, Interp::Spline),
            v3(2.0, 10.0, 70.0, -4.0, Interp::Spline),
        ]);
        let Some(Value::Vec3(mid)) = c.eval(1.0) else {
            panic!()
        };
        assert!((mid[0] - 5.0).abs() < 1e-4, "{mid:?}");
        assert!((mid[1] - 67.0).abs() < 1e-4, "{mid:?}");
        assert!((mid[2] + 2.0).abs() < 1e-4, "{mid:?}");
    }

    #[test]
    fn spline_rotation_keys_still_slerp() {
        let a = Quat::from_rotation_y(0.0);
        let b = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let c = chan(vec![
            Key {
                ease: Default::default(),
                t: 0.0,
                v: Value::Quat(a.to_array()),
                interp: Interp::Spline,
            },
            Key {
                ease: Default::default(),
                t: 1.0,
                v: Value::Quat(b.to_array()),
                interp: Interp::Spline,
            },
        ]);
        let Some(Value::Quat(mid)) = c.eval(0.5) else {
            panic!()
        };
        let want = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4).to_array();
        for axis in 0..4 {
            assert!((mid[axis] - want[axis]).abs() < 1e-4, "{mid:?} vs {want:?}");
        }
    }

    #[test]
    fn holds_before_and_after() {
        let c = chan(vec![
            k(0.0, 1.0, Interp::Linear),
            k(2.0, 3.0, Interp::Linear),
        ]);
        assert_eq!(c.eval(-1.0), Some(Value::F32(1.0)));
        assert_eq!(c.eval(5.0), Some(Value::F32(3.0)));
    }

    #[test]
    fn linear_midpoint() {
        let c = chan(vec![
            k(0.0, 0.0, Interp::Linear),
            k(2.0, 4.0, Interp::Linear),
        ]);
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
            k(
                0.0,
                0.0,
                Interp::Bezier {
                    out: [0.8, 0.0],
                    in_: [0.2, 1.0],
                },
            ),
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
        let a = Key {
            ease: Default::default(),
            t: 0.0,
            v: Value::Quat(Quat::from_rotation_y(0.0).to_array()),
            interp: Interp::Linear,
        };
        let b = Key {
            ease: Default::default(),
            t: 1.0,
            v: Value::Quat(Quat::from_rotation_y(1.5).to_array()),
            interp: Interp::Linear,
        };
        let c = chan(vec![a, b]);
        let Value::Quat(q) = c.eval(0.5).unwrap() else {
            panic!()
        };
        assert!((Quat::from_array(q).length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn muted_returns_none() {
        let mut c = chan(vec![k(0.0, 1.0, Interp::Linear)]);
        c.muted = true;
        assert_eq!(c.eval(0.0), None);
    }
}
