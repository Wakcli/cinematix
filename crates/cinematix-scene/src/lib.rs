use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AssetId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_translation(v: Vec3) -> Self {
        Self { translation: v, ..Default::default() }
    }

    pub fn matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CameraData {
    pub fov_deg: f32,
    pub near: f32,
    pub far: f32,
    pub focus_distance: f32,
    pub aperture: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightKind {
    Point { range: f32 },
    Spot { range: f32, inner_deg: f32, outer_deg: f32 },
    Area { width: f32, height: f32 },
    Sun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightData {
    pub kind: LightKind,
    pub color: [f32; 3],
    pub temperature_k: f32,
    pub intensity: f32,
    pub shadows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorData {
    pub rig: AssetId,
    pub skin: Option<AssetId>,
    pub cape: Option<AssetId>,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshInstance {
    pub asset: AssetId,
    pub material: Option<AssetId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Empty,
    Camera(CameraData),
    Light(LightData),
    Actor(ActorData),
    Mesh(MeshInstance),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub rest: Transform,
    pub kind: NodeKind,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetKind {
    Rig,
    Mesh,
    Skin,
    Texture,
    Sound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRef {
    pub kind: AssetKind,
    pub path: String,
    pub sha256: Option<String>,
    pub packed: bool,
}

/// The film's environment: each field `None` keeps the world's own value, so a film that sets
/// nothing renders the singleplayer world exactly as it is.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentData {
    #[serde(default)]
    pub time: Option<f32>,
    #[serde(default)]
    pub rain: Option<f32>,
    #[serde(default)]
    pub thunder: Option<f32>,
    #[serde(default)]
    pub fog_start: Option<f32>,
    #[serde(default)]
    pub fog_end: Option<f32>,
    #[serde(default)]
    pub sky_color: Option<[f32; 3]>,
    #[serde(default)]
    pub fog_color: Option<[f32; 3]>,
}

impl EnvironmentData {
    pub fn any(&self) -> bool {
        self.time.is_some()
            || self.rain.is_some()
            || self.thunder.is_some()
            || self.fog_start.is_some()
            || self.fog_end.is_some()
            || self.sky_color.is_some()
            || self.fog_color.is_some()
    }
}

/// A named span of the film, the Clips tab's shot list. Clips share the one timeline; they are a
/// way to jump between shots, not separate channel sets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Clip {
    #[serde(default)]
    pub name: String,
    pub start: f64,
    pub end: f64,
}

/// A deterministic camera shake: two sine terms at `frequency` and 1.7 times it, in degrees.
/// Time-based, so a preview and every rendered frame agree.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CameraFx {
    #[serde(default)]
    pub shake_amplitude: f32,
    #[serde(default)]
    pub shake_frequency: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneMeta {
    pub name: String,
    pub author: String,
    pub fps: f32,
    pub range: (f64, f64),
    pub resolution: (u32, u32),
    #[serde(default)]
    pub environment: EnvironmentData,
    #[serde(default)]
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub camera_fx: CameraFx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn chunk(self) -> (i32, i32) {
        (self.x >> 4, self.z >> 4)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockState {
    pub name: String,
    pub props: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockChange {
    Set(BlockState),
    Remove,
    Restore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockOp {
    pub pos: BlockPos,
    pub change: BlockChange,
    pub at: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldDeltaLayer {
    ops: Vec<BlockOp>,
    index: HashMap<BlockPos, Vec<usize>>,
}

impl WorldDeltaLayer {
    pub fn push(&mut self, op: BlockOp) {
        let i = self.ops.len();
        self.index.entry(op.pos).or_default().push(i);
        self.ops.push(op);
    }

    pub fn set(&mut self, pos: BlockPos, change: BlockChange, at: Option<f64>) {
        self.push(BlockOp { pos, change, at });
    }

    pub fn at(&self, pos: BlockPos, t: f64) -> Option<&BlockChange> {
        let slots = self.index.get(&pos)?;
        slots.iter().rev().find_map(|&i| {
            let op = &self.ops[i];
            if op.at.is_none_or(|a| a <= t) { Some(&op.change) } else { None }
        })
    }

    pub fn resolved(&self, pos: BlockPos, t: f64) -> Option<&BlockState> {
        match self.at(pos, t)? {
            BlockChange::Set(s) => Some(s),
            BlockChange::Remove | BlockChange::Restore => None,
        }
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn chunk_keys(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.index.keys().map(|p| p.chunk())
    }

    pub fn ops(&self) -> &[BlockOp] {
        &self.ops
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub meta: SceneMeta,
    pub nodes: HashMap<NodeId, Node>,
    pub roots: Vec<NodeId>,
    pub camera: NodeId,
    pub world: WorldDeltaLayer,
    pub assets: HashMap<AssetId, AssetRef>,
}

impl Scene {
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn parent_chain(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let mut cur = Some(id);
        std::iter::from_fn(move || {
            let n = self.nodes.get(&cur?)?;
            cur = n.parent;
            Some(n.id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn block(name: &str) -> BlockState {
        BlockState {
            name: name.into(),
            props: BTreeMap::new(),
        }
    }

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 64, z: 3 }
    }

    #[test]
    fn an_op_is_inactive_before_its_time_and_active_from_it() {
        let mut layer = WorldDeltaLayer::default();
        layer.set(pos(), BlockChange::Set(block("minecraft:gold_block")), Some(2.0));
        assert_eq!(layer.at(pos(), 1.9), None);
        assert!(matches!(layer.at(pos(), 2.0), Some(BlockChange::Set(_))));
    }

    #[test]
    fn the_latest_active_op_wins_and_restore_undoes_it() {
        let mut layer = WorldDeltaLayer::default();
        layer.set(pos(), BlockChange::Set(block("minecraft:stone")), None);
        layer.set(pos(), BlockChange::Set(block("minecraft:gold_block")), Some(1.0));
        assert!(matches!(
            layer.at(pos(), 1.5),
            Some(BlockChange::Set(s)) if s.name == "minecraft:gold_block"
        ));
        layer.set(pos(), BlockChange::Restore, Some(2.0));
        assert!(matches!(layer.at(pos(), 2.5), Some(BlockChange::Restore)));
        assert_eq!(layer.resolved(pos(), 2.5), None);
    }

    #[test]
    fn an_op_without_a_time_is_always_active() {
        let mut layer = WorldDeltaLayer::default();
        layer.set(pos(), BlockChange::Remove, None);
        assert!(matches!(layer.at(pos(), 0.0), Some(BlockChange::Remove)));
        assert!(matches!(layer.at(pos(), 1e9), Some(BlockChange::Remove)));
    }
}
