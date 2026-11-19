# Animation audit

## Existing animation systems

Horizon already has:

- Mod keyframe animation channels and interpolation:
  `crates/horizon-mod-vocab/src/client/keyframe_animation.rs`
- Mobs and entities: `crates/horizon-client/src/mobs/anim.rs`,
  `mobs/keyframes.rs`, `mobs/body_anim.rs`
- Block entity animation: `horizon-client-render/src/block_entity_anim.rs`
- Material, sprite and texture animation: `gfx/animate.rs`,
  `gfx/material_animate.rs`, `gfx/sprite_upload_tests.rs`
- Replay camera track and Catmull-Rom spline:
  `crates/horizon-client/src/studio/spline.rs`
- Timeline UI with playhead, keyframe selection, clips and split points:
  `crates/horizon-client/src/studio/timeline_ui.rs`

## Typed backend status

The existing Replay Studio spline is camera-specific and `TimelineUi` is an interaction state
for replay camera actions. Horizon's mod animation API is general in name but is for mod
animation channels and not a Cinematix scene's arbitrary property tracks.

There is no current scene-backed typed multichannel animation backend or curve editor for
Cinematix. These are Phase 3 work, not an existing implementation to fake in Phase 1.

## Intended integration

Phase 3 can extend the existing Studio data model rather than replacing it:

- typed scalar, vector and quaternion channels
- stable keyframe time in timeline units
- constant, linear, cubic, Bezier/Hermite and quaternion slerp
- explicit extrapolation and deterministic boundaries
- same keyframe data consumed by timeline and curve editor
- replay virtual clock separated from wall clock

Existing value sampling and replay seek behavior must be retained or extended, with unit tests
for evaluation and boundary behavior.
