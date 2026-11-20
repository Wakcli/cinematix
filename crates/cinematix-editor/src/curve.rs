//! Value-axis maths for the curve editor: which component of a key's value is edited, and the
//! range the plot covers.

use cinematix_anim::{Channel, Value};

pub fn components(v: Value) -> usize {
    match v {
        Value::Vec3(_) => 3,
        Value::Quat(_) | Value::Color(_) => 4,
        _ => 0,
    }
}

pub fn component(v: Value, axis: usize) -> Option<f64> {
    Some(match v {
        Value::F32(x) => f64::from(x),
        Value::Bool(b) => f64::from(u8::from(b)),
        Value::Vec3(a) => f64::from(*a.get(axis)?),
        Value::Quat(a) | Value::Color(a) => f64::from(*a.get(axis)?),
    })
}

pub fn set_component(v: &mut Value, axis: usize, value: f32) -> bool {
    match v {
        Value::F32(x) => {
            *x = value;
            true
        }
        Value::Vec3(a) => match a.get_mut(axis) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        },
        Value::Quat(a) | Value::Color(a) => match a.get_mut(axis) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        },
        Value::Bool(_) => false,
    }
}

/// The lowest and highest component across the channel's keys, with a margin so a flat curve is
/// still visible.
pub fn range(channel: &Channel, axis: usize) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for key in &channel.keys {
        if let Some(v) = component(key.v, axis) {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return None;
    }
    let pad = ((hi - lo) * 0.15).max(0.25);
    Some((lo - pad, hi + pad))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cinematix_anim::{Interp, Key, Prop, PropPath};
    use cinematix_scene::NodeId;

    fn channel(keys: Vec<Key>) -> Channel {
        let mut c = Channel::new(PropPath {
            node: NodeId(1),
            prop: Prop::Translation,
        });
        c.keys = keys;
        c
    }

    #[test]
    fn the_axis_edits_the_named_component_and_nothing_else() {
        let mut v = Value::Vec3([1.0, 2.0, 3.0]);
        assert_eq!(component(v, 1), Some(2.0));
        assert!(set_component(&mut v, 1, 9.0));
        assert_eq!(v, Value::Vec3([1.0, 9.0, 3.0]));
        let mut q = Value::Quat([0.0, 0.0, 0.0, 1.0]);
        assert!(set_component(&mut q, 3, 0.5));
        assert_eq!(component(q, 3), Some(0.5));
    }

    #[test]
    fn a_flat_channel_still_gets_a_window() {
        let c = channel(vec![
            Key {
                t: 0.0,
                v: Value::F32(4.0),
                interp: Interp::Linear,
            },
            Key {
                t: 1.0,
                v: Value::F32(4.0),
                interp: Interp::Linear,
            },
        ]);
        let (lo, hi) = range(&c, 0).unwrap();
        assert!(hi > lo && hi - lo >= 0.5);
    }
}
