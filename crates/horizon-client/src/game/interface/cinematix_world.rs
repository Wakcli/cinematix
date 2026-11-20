//! `cinematix world`: the film's `WorldDeltaLayer` resolved at the playhead and handed to the
//! mesher as a read-path overlay.
//!
//! The world, the chunks and the save are never written. The layer maps positions to a block (or
//! to air for `Remove`, or to nothing for `Restore`); `sync_cinematix_world` resolves it at the
//! playhead, diffs it against what the mesher had, marks every changed position's sections dirty
//! (the same 3x3x3 neighbourhood a block edit remeshes), and publishes the new set to
//! `Streaming`, whose mesh jobs copy it into the padded block volume before meshing. Gameplay
//! block queries keep seeing the real world; only the picture changes.

use super::{Game, Screen};
use cinematix_editor::{BlockChange, BlockPos, FilmBlock};
use horizon_client::Result;
use horizon_world::registry::{BlockRegistry, BlockStateId};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

/// What the mesher currently draws for the film.
#[derive(Default)]
pub(super) struct Applied {
    blocks: HashMap<[i32; 3], u32>,
}

impl Applied {
    pub(super) fn len(&self) -> usize {
        self.blocks.len()
    }

    fn snapshot(&self) -> Option<std::sync::Arc<horizon_client_render::mesher::Overrides>> {
        if self.blocks.is_empty() {
            return None;
        }
        Some(std::sync::Arc::new(
            horizon_client_render::mesher::Overrides::new(
                self.blocks.iter().map(|(pos, state)| (*pos, *state)),
            ),
        ))
    }
}

fn block_id(
    registry: &BlockRegistry,
    block: &FilmBlock,
) -> std::result::Result<BlockStateId, String> {
    let state = horizon_world::chunk::BlockState {
        name: block.name.clone(),
        properties: block.props.clone().into_iter().collect(),
    };
    registry
        .resolve(&state)
        .map_err(|error| format!("{}: {error}", block.name))
}

fn describe(change: &BlockChange) -> String {
    match change {
        BlockChange::Set(block) => block.name.clone(),
        BlockChange::Remove => "air".to_owned(),
        BlockChange::Restore => "restore".to_owned(),
    }
}

impl Game<'_> {
    /// Resolves the layer at the playhead and remeshes what changed. Cheap when the playhead has
    /// not moved since the last call. Returns whether any position changed and was remeshed.
    pub(super) fn sync_cinematix_world(&mut self) -> bool {
        if self.screen != Screen::Cinematix {
            return false;
        }
        let t = self.cinematix.playhead.t;
        if self.cinematix_world_time == Some(t) {
            return false;
        }
        self.cinematix_world_time = Some(t);

        let next: HashMap<[i32; 3], u32> = {
            let layer = &self.cinematix.doc.scene.world;
            let mut seen = HashSet::new();
            let mut next = HashMap::new();
            for op in layer.ops() {
                let pos = [op.pos.x, op.pos.y, op.pos.z];
                if !seen.insert(pos) {
                    continue;
                }
                let Some(change) = layer.at(op.pos, t) else {
                    continue;
                };
                let state = match change {
                    BlockChange::Set(block) => block_id(&self.pack.assets.registry, block).ok(),
                    BlockChange::Remove => Some(BlockStateId(0)),
                    BlockChange::Restore => None,
                };
                if let Some(state) = state {
                    next.insert(pos, state.0);
                }
            }
            next
        };

        let mut changed: Vec<[i32; 3]> = Vec::new();
        for pos in self.cinematix_world.blocks.keys() {
            if next.get(pos).copied() != self.cinematix_world.blocks.get(pos).copied() {
                changed.push(*pos);
            }
        }
        for (pos, state) in &next {
            if self.cinematix_world.blocks.get(pos) != Some(state) {
                changed.push(*pos);
            }
        }
        let remeshed = !changed.is_empty();
        for pos in changed {
            super::viewed_worlds::meshing::block_changed(&mut self.world, pos);
        }
        self.cinematix_world = Applied { blocks: next };
        self.streaming
            .set_overrides(self.cinematix_world.snapshot());
        remeshed
    }

    /// Drops the overlay (the editor closed): every overridden position goes back to the world's
    /// own block and its sections remesh.
    pub(super) fn clear_cinematix_world(&mut self) {
        self.cinematix_world_time = None;
        let positions: Vec<_> = self.cinematix_world.blocks.keys().copied().collect();
        for pos in positions {
            super::viewed_worlds::meshing::block_changed(&mut self.world, pos);
        }
        self.cinematix_world = Applied::default();
        self.streaming.set_overrides(None);
    }

    fn push_world_op(
        &mut self,
        pos: BlockPos,
        change: BlockChange,
        at: Option<f64>,
    ) -> Result<Value> {
        let layer = &mut self.cinematix.doc.scene.world;
        layer.set(pos, change, at);
        self.cinematix_world_time = None;
        self.sync_cinematix_world();
        Ok(json!({
            "ok": true,
            "position": [pos.x, pos.y, pos.z],
            "ops": self.cinematix.doc.scene.world.len(),
            "active": self.cinematix_world.len(),
        }))
    }

    /// `/cinematix world ...` and `horizon-ctl cinematix world ...`.
    pub(super) fn cinematix_world_control(
        &mut self,
        words: &[&str],
    ) -> Result<Option<Value>> {
        let Some(sub) = words.first().copied() else {
            return Ok(Some(json!({
                "ok": false,
                "error": "world needs set, remove, restore, clear, list or status",
            })));
        };
        let number = |i: usize| words.get(i).and_then(|w| w.parse::<i32>().ok());
        let at = |i: usize| match words.get(i) {
            None => Ok(None),
            Some(w) => match w.parse::<f64>() {
                Ok(t) if t >= 0.0 && t.is_finite() => Ok(Some(t)),
                _ => Err(format!("activation {w:?} is not seconds from the start")),
            },
        };
        let pos = || -> std::result::Result<BlockPos, String> {
            match (number(1), number(2), number(3)) {
                (Some(x), Some(y), Some(z)) => Ok(BlockPos { x, y, z }),
                _ => Err("world needs block coordinates X Y Z".into()),
            }
        };
        match sub {
            "set" => {
                let pos = pos()?;
                let Some(name) = words.get(4).copied() else {
                    return Ok(Some(json!({ "ok": false, "error": "set needs a block name" })));
                };
                let block = FilmBlock {
                    name: name.to_owned(),
                    props: BTreeMap::new(),
                };
                block_id(&self.pack.assets.registry, &block).map_err(|e| format!("unknown block {e}"))?;
                let at = at(5)?;
                Ok(Some(self.push_world_op(pos, BlockChange::Set(block), at)?))
            }
            "remove" => {
                let pos = pos()?;
                let at = at(4)?;
                Ok(Some(
                    self.push_world_op(pos, BlockChange::Remove, at)?,
                ))
            }
            "restore" => {
                let pos = pos()?;
                let at = at(4)?;
                Ok(Some(
                    self.push_world_op(pos, BlockChange::Restore, at)?,
                ))
            }
            "clear" => {
                let count = self.cinematix.doc.scene.world.len();
                let active = self.cinematix_world.len();
                if count > 0 {
                    self.cinematix.doc.scene.world = Default::default();
                    self.cinematix_world_time = None;
                    self.clear_cinematix_world();
                }
                Ok(Some(json!({ "ok": true, "cleared": count, "was_active": active })))
            }
            "list" => {
                let ops: Vec<Value> = self
                    .cinematix
                    .doc
                    .scene
                    .world
                    .ops()
                    .iter()
                    .map(|op| {
                        json!({
                            "position": [op.pos.x, op.pos.y, op.pos.z],
                            "block": describe(&op.change),
                            "at": op.at,
                        })
                    })
                    .collect();
                Ok(Some(json!({ "ok": true, "ops": ops })))
            }
            "status" => Ok(Some(json!({
                "ok": true,
                "ops": self.cinematix.doc.scene.world.len(),
                "active": self.cinematix_world.len(),
                "time": self.cinematix.playhead.t,
                "dirty_sections": self.world.dirty_sections.len(),
                "uploads": self.pending_uploads.len(),
                "meshing": self.streaming.busy(),
            }))),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{describe, FilmBlock};
    use cinematix_editor::BlockChange;
    use std::collections::BTreeMap;

    #[test]
    fn a_change_reads_as_the_block_it_draws() {
        let block = FilmBlock {
            name: "minecraft:gold_block".into(),
            props: BTreeMap::new(),
        };
        assert_eq!(describe(&BlockChange::Set(block)), "minecraft:gold_block");
        assert_eq!(describe(&BlockChange::Remove), "air");
        assert_eq!(describe(&BlockChange::Restore), "restore");
    }
}
