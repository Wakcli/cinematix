# Rigging audit

## Existing model and animation systems

Horizon has JSON/entity keyframe animation, CEM model rendering, model animation and bone
hierarchies in the client/mob assets crates. Relevant sources include:

- `crates/horizon-client/src/mobs/anim.rs`
- `crates/horizon-client/src/mobs/keyframes.rs`
- `crates/horizon-client/src/mobs/body_anim.rs`
- `crates/horizon-client/src/mobs/cem.rs`
- `crates/horizon-client/src/mobs/cem_anim.rs`
- `crates/horizon-client-render/src/block_entity_cem.rs`
- `crates/horizon-mod-vocab/src/client/keyframe_animation.rs`

## Status

There is no Cinematix-specific skeleton, FK/IK, skinning or clip library. BBS-FS contains
reference implementations for constraints and IK in Java, but they are not reusable as Rust
code and are not imported.

Future rigging work must add Rust-native scene ownership and use Horizon's model/part
representations where compatible. Imported glTF skeletons and weights are later pipeline work,
not safe assumptions of existing APIs.
