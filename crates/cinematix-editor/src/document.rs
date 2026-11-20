use cinematix_anim::{Channel, Interp, Key, Prop, PropPath, Value};
use cinematix_scene::{
    AssetId, CameraData, LightData, LightKind, Node, NodeId, NodeKind, Scene, SceneMeta, Transform,
};
use glam::{Quat, Vec3};
use std::collections::HashMap;

const CAMERA_PROPS: [Prop; 5] = [
    Prop::Translation,
    Prop::Rotation,
    Prop::Fov,
    Prop::Focus,
    Prop::Visible,
];
const LIGHT_PROPS: [Prop; 5] = [
    Prop::Translation,
    Prop::Rotation,
    Prop::Intensity,
    Prop::Color,
    Prop::Visible,
];
const NODE_PROPS: [Prop; 4] = [
    Prop::Translation,
    Prop::Rotation,
    Prop::Scale,
    Prop::Visible,
];

pub fn props_of(kind: &NodeKind) -> &'static [Prop] {
    match kind {
        NodeKind::Camera(_) => &CAMERA_PROPS,
        NodeKind::Light(_) => &LIGHT_PROPS,
        NodeKind::Empty | NodeKind::Actor(_) | NodeKind::Mesh(_) => &NODE_PROPS,
    }
}

pub fn prop_key(prop: Prop) -> &'static str {
    match prop {
        Prop::Translation => "cinematix.prop.translation",
        Prop::Rotation => "cinematix.prop.rotation",
        Prop::Scale => "cinematix.prop.scale",
        Prop::Fov => "cinematix.prop.fov",
        Prop::Focus => "cinematix.prop.focus",
        Prop::Aperture => "cinematix.prop.aperture",
        Prop::Intensity => "cinematix.prop.intensity",
        Prop::Color => "cinematix.prop.color",
        Prop::Visible => "cinematix.prop.visible",
    }
}

pub fn kind_key(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Empty => "cinematix.kind.empty",
        NodeKind::Camera(_) => "cinematix.kind.camera",
        NodeKind::Light(_) => "cinematix.kind.light",
        NodeKind::Actor(_) => "cinematix.kind.actor",
        NodeKind::Mesh(_) => "cinematix.kind.mesh",
    }
}

pub fn new_node(id: u64, kind: NodeKind) -> Node {
    Node {
        id: NodeId(id),
        name: String::new(),
        parent: None,
        children: Vec::new(),
        rest: Transform::default(),
        kind,
        visible: true,
        locked: false,
    }
}

pub fn camera_data() -> CameraData {
    CameraData {
        fov_deg: 70.0,
        near: 0.05,
        far: 512.0,
        focus_distance: 8.0,
        aperture: 2.8,
    }
}

pub fn key_light_data() -> LightData {
    LightData {
        kind: LightKind::Point { range: 16.0 },
        color: [1.0, 0.96, 0.9],
        temperature_k: 5600.0,
        intensity: 1.0,
        shadows: true,
    }
}

/// The interpolation a new key gets. A camera or node's position bends through its keys like
/// Studio's camera path; every other property eases (`Smooth`), which is the predictable choice
/// for a single number.
pub fn default_interp(prop: Prop) -> Interp {
    match prop {
        Prop::Translation => Interp::Spline,
        _ => Interp::Smooth,
    }
}

pub fn rest_value(node: &Node, prop: Prop) -> Option<Value> {
    match prop {
        Prop::Translation => Some(Value::Vec3(node.rest.translation.to_array())),
        Prop::Rotation => Some(Value::Quat(node.rest.rotation.to_array())),
        Prop::Scale => Some(Value::Vec3(node.rest.scale.to_array())),
        Prop::Visible => Some(Value::Bool(node.visible)),
        Prop::Fov => match &node.kind {
            NodeKind::Camera(c) => Some(Value::F32(c.fov_deg)),
            _ => None,
        },
        Prop::Focus => match &node.kind {
            NodeKind::Camera(c) => Some(Value::F32(c.focus_distance)),
            _ => None,
        },
        Prop::Aperture => match &node.kind {
            NodeKind::Camera(c) => Some(Value::F32(c.aperture)),
            _ => None,
        },
        Prop::Intensity => match &node.kind {
            NodeKind::Light(l) => Some(Value::F32(l.intensity)),
            _ => None,
        },
        Prop::Color => match &node.kind {
            NodeKind::Light(l) => Some(Value::Color([l.color[0], l.color[1], l.color[2], 1.0])),
            _ => None,
        },
    }
}

/// The editable document: the scene and one channel per animatable property of every node. A
/// channel with no keys still draws, dimmed, so what can be animated is visible before it is keyed.
#[derive(Debug, Clone)]
pub struct Document {
    pub scene: Scene,
    pub channels: Vec<Channel>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        let camera = new_node(1, NodeKind::Camera(camera_data()));
        let light = new_node(2, NodeKind::Light(key_light_data()));
        let mut nodes = HashMap::new();
        nodes.insert(camera.id, camera);
        nodes.insert(light.id, light);
        let scene = Scene {
            meta: SceneMeta {
                name: String::new(),
                author: String::new(),
                fps: 24.0,
                range: (0.0, 10.0),
                resolution: (1920, 1080),
                environment: Default::default(),
                clips: Vec::new(),
                camera_fx: Default::default(),
            },
            nodes,
            roots: vec![NodeId(1), NodeId(2)],
            camera: NodeId(1),
            world: Default::default(),
            assets: HashMap::<AssetId, _>::new(),
        };
        let mut doc = Self {
            scene,
            channels: Vec::new(),
        };
        doc.sync_channels();
        doc
    }

    /// One channel per animatable property, in node-id then property order, keeping the keys of
    /// channels that already exist.
    pub fn sync_channels(&mut self) {
        let mut ids: Vec<NodeId> = self.scene.nodes.keys().copied().collect();
        ids.sort();
        let mut out = Vec::with_capacity(self.channels.len().max(ids.len() * 4));
        for id in ids {
            let Some(node) = self.scene.nodes.get(&id) else {
                continue;
            };
            for &prop in props_of(&node.kind) {
                let target = PropPath { node: id, prop };
                match self.channels.iter().find(|c| c.target == target) {
                    Some(c) => out.push(c.clone()),
                    None => out.push(Channel::new(target)),
                }
            }
        }
        self.channels = out;
    }

    pub fn channel_index(&self, node: NodeId, prop: Prop) -> Option<usize> {
        self.channels
            .iter()
            .position(|c| c.target.node == node && c.target.prop == prop)
    }

    pub fn channel(&self, ci: usize) -> Option<&Channel> {
        self.channels.get(ci)
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.scene.nodes.get(&id)
    }

    pub fn node_name(&self, id: NodeId) -> String {
        match self.node(id) {
            Some(n) if !n.name.is_empty() => n.name.clone(),
            Some(n) => kind_key(&n.kind).to_owned(),
            None => String::new(),
        }
    }

    /// The value a key written now would hold: the channel's interpolated value at `t` when it has
    /// keys, so keying never moves the animation, and the node's rest value when it does not.
    pub fn value_at(&self, ci: usize, t: f64) -> Option<Value> {
        let ch = self.channels.get(ci)?;
        if let Some(v) = ch.eval(t) {
            return Some(v);
        }
        let node = self.node(ch.target.node)?;
        rest_value(node, ch.target.prop)
    }

    /// Writes `value` at `t`, replacing a key within a frame's epsilon. A new key takes the
    /// channel's default interpolation; a replaced one keeps the interpolation it had.
    pub fn upsert_key(&mut self, ci: usize, t: f64, value: Value) -> Option<usize> {
        let ch = self.channels.get_mut(ci)?;
        let interp = default_interp(ch.target.prop);
        if let Some(i) = ch.keys.iter().position(|k| (k.t - t).abs() < 1e-9) {
            ch.keys[i].v = value;
            return Some(i);
        }
        ch.keys.push(Key {
            t,
            v: value,
            interp,
        });
        ch.sort();
        ch.keys.iter().position(|k| (k.t - t).abs() < 1e-9)
    }

    /// Inserts a key at `t` holding what the channel already evaluates to, so keying never
    /// moves the animation. Returns its index.
    pub fn insert_key(&mut self, ci: usize, t: f64) -> Option<usize> {
        let value = self.value_at(ci, t)?;
        let ch = self.channels.get_mut(ci)?;
        let interp = default_interp(ch.target.prop);
        if let Some(i) = ch.keys.iter().position(|k| (k.t - t).abs() < 1e-9) {
            ch.keys[i].v = value;
            return Some(i);
        }
        ch.keys.push(Key {
            t,
            v: value,
            interp,
        });
        ch.sort();
        ch.keys.iter().position(|k| (k.t - t).abs() < 1e-9)
    }

    /// Moves a key to `t`, keeping the channel sorted, and returns its new index.
    pub fn move_key(&mut self, ci: usize, ki: usize, t: f64) -> Option<usize> {
        let ch = self.channels.get_mut(ci)?;
        if ki >= ch.keys.len() {
            return None;
        }
        let mut key = ch.keys.remove(ki);
        key.t = t.max(0.0);
        let at = ch.keys.partition_point(|k| k.t <= key.t);
        ch.keys.insert(at, key);
        Some(at)
    }

    pub fn delete_key(&mut self, ci: usize, ki: usize) -> bool {
        match self.channels.get_mut(ci) {
            Some(ch) if ki < ch.keys.len() => {
                ch.keys.remove(ki);
                true
            }
            _ => false,
        }
    }

    pub fn key_count(&self) -> usize {
        self.channels.iter().map(|c| c.keys.len()).sum()
    }

    pub fn nodes_of(&self, matches: impl Fn(&NodeKind) -> bool) -> Vec<NodeId> {
        let mut ids: Vec<NodeId> = self
            .scene
            .nodes
            .iter()
            .filter(|(_, n)| matches(&n.kind))
            .map(|(id, _)| *id)
            .collect();
        ids.sort();
        ids
    }

    pub fn camera(&self) -> Option<&Node> {
        self.node(self.scene.camera)
    }

    pub fn camera_data(&self) -> Option<&CameraData> {
        match &self.scene.nodes.get(&self.scene.camera)?.kind {
            NodeKind::Camera(c) => Some(c),
            _ => None,
        }
    }

    pub fn camera_data_mut(&mut self) -> Option<&mut CameraData> {
        match self.scene.nodes.get_mut(&self.scene.camera)?.kind {
            NodeKind::Camera(ref mut c) => Some(c),
            _ => None,
        }
    }

    /// The scene's own camera position and rotation, for the editor camera to fly from later.
    pub fn camera_transform(&self) -> Option<Transform> {
        Some(self.camera()?.rest)
    }

    pub fn set_camera_translation(&mut self, t: Vec3) -> bool {
        match self.scene.nodes.get_mut(&self.scene.camera) {
            Some(n) => {
                n.rest.translation = t;
                true
            }
            None => false,
        }
    }

    pub fn set_camera_rotation(&mut self, r: Quat) -> bool {
        match self.scene.nodes.get_mut(&self.scene.camera) {
            Some(n) => {
                n.rest.rotation = r.normalize();
                true
            }
            None => false,
        }
    }

    /// Whether the camera's pose is animated: either channel holds at least one key.
    pub fn camera_keyed(&self) -> bool {
        self.camera_channels()
            .iter()
            .any(|ci| self.channels[*ci].keys.len() > 0)
    }

    pub fn camera_channels(&self) -> Vec<usize> {
        let id = self.scene.camera;
        [Prop::Translation, Prop::Rotation]
            .into_iter()
            .filter_map(|p| self.channel_index(id, p))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_document_has_a_camera_and_a_light_and_their_channels() {
        let doc = Document::new();
        assert_eq!(doc.nodes_of(|k| matches!(k, NodeKind::Camera(_))).len(), 1);
        assert_eq!(doc.nodes_of(|k| matches!(k, NodeKind::Light(_))).len(), 1);
        assert_eq!(doc.channels.len(), CAMERA_PROPS.len() + LIGHT_PROPS.len());
        assert!(doc.channels.iter().all(|c| c.keys.is_empty()));
    }

    #[test]
    fn keying_twice_at_one_time_replaces_instead_of_stacking() {
        let mut doc = Document::new();
        let ci = doc.channel_index(NodeId(1), Prop::Translation).unwrap();
        doc.insert_key(ci, 1.0).unwrap();
        let i = doc.insert_key(ci, 1.0).unwrap();
        assert_eq!(doc.channels[ci].keys.len(), 1);
        assert_eq!(i, 0);
    }

    #[test]
    fn a_key_written_mid_curve_holds_the_curve_still() {
        let mut doc = Document::new();
        let ci = doc.channel_index(NodeId(1), Prop::Translation).unwrap();
        doc.insert_key(ci, 0.0).unwrap();
        doc.channels[ci].keys[0].v = Value::Vec3([0.0, 0.0, 0.0]);
        doc.insert_key(ci, 2.0).unwrap();
        doc.channels[ci].keys[1].v = Value::Vec3([10.0, 0.0, 0.0]);
        let before = doc.channels[ci].eval(1.0).unwrap();
        doc.insert_key(ci, 1.0).unwrap();
        let after = doc.channels[ci].eval(1.0).unwrap();
        assert_eq!(before, after);
        assert_eq!(doc.channels[ci].keys.len(), 3);
    }

    #[test]
    fn moving_a_key_past_its_neighbour_keeps_the_channel_sorted() {
        let mut doc = Document::new();
        let ci = doc.channel_index(NodeId(1), Prop::Focus).unwrap();
        doc.insert_key(ci, 0.0).unwrap();
        doc.insert_key(ci, 1.0).unwrap();
        doc.insert_key(ci, 2.0).unwrap();
        let new = doc.move_key(ci, 0, 5.0).unwrap();
        assert_eq!(new, 2);
        let times: Vec<f64> = doc.channels[ci].keys.iter().map(|k| k.t).collect();
        assert!(times.windows(2).all(|w| w[0] < w[1]), "{times:?}");
    }

    #[test]
    fn deleting_a_key_leaves_the_channel_empty_not_gone() {
        let mut doc = Document::new();
        let ci = doc.channel_index(NodeId(1), Prop::Fov).unwrap();
        doc.insert_key(ci, 0.5).unwrap();
        assert!(doc.delete_key(ci, 0));
        assert!(doc.channels[ci].keys.is_empty());
        assert!(!doc.delete_key(ci, 0));
    }

    #[test]
    fn syncing_channels_keeps_keys_when_nodes_are_added() {
        let mut doc = Document::new();
        let ci = doc.channel_index(NodeId(1), Prop::Rotation).unwrap();
        doc.insert_key(ci, 3.0).unwrap();
        let actor = new_node(
            3,
            NodeKind::Actor(cinematix_scene::ActorData {
                rig: AssetId(1),
                skin: None,
                cape: None,
                scale: 1.0,
            }),
        );
        doc.scene.roots.push(actor.id);
        doc.scene.nodes.insert(actor.id, actor);
        doc.sync_channels();
        let ci = doc.channel_index(NodeId(1), Prop::Rotation).unwrap();
        assert_eq!(doc.channels[ci].keys.len(), 1);
        assert_eq!(
            doc.channels.len(),
            CAMERA_PROPS.len() + LIGHT_PROPS.len() + NODE_PROPS.len()
        );
    }
}
