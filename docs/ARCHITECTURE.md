# Cinematix architecture audit

## Current status

Phase 1 only. The real Horizon workspace is present and has not yet been given a Cinematix
implementation crate or gameplay module. This document describes current integration points
and boundaries; it does not claim a scene editor exists.

## Layer placement

Cinematix is a Horizon-only, finished extra. `docs/layers.md` places replay editing and other
finished features on top of platform APIs. The initial implementation should therefore add
platform-facing hooks only where Horizon itself lacks a public engine seam, and keep editor
state behind `horizon-extras`.

Within the game workspace, gameplay belongs primarily in `crates/horizon-client/src/game/`.
Reusable replay primitives belong under `crates/horizon-client/src/studio/` and renderer/UI
primitives under `crates/horizon-client-render/src/`. The existing Studio is deliberately the
first integration point rather than a parallel editor.

## Existing Replay Studio

The audited implementation includes:

- Replay recording and `.mcpr` playback: `crates/horizon-client/src/game/runtime/replay.rs`,
  `crates/horizon-client/src/studio/recorder.rs` and `crates/horizon-client/src/studio/mcpr.rs`
- Free camera and camera keyframe track: `crates/horizon-client/src/studio/camera.rs` and
  `crates/horizon-client/src/studio/spline.rs`
- Timeline UI and top bar: `crates/horizon-client/src/studio/timeline_ui.rs` and
  `crates/horizon-client/src/studio/topbar.rs`
- Gameplay integration: `crates/horizon-client/src/game/interface/studio.rs`
- Saved replay browser: `crates/horizon-client-render/src/studio_ui.rs`
- Scene/camera capture: `crates/horizon-client-render/src/gfx/capture.rs`

It is a functional replay/free-camera/timeline base, but it is not a general typed multichannel
scene animation backend. Phase 2 must reuse it and extend it; a second timeline or camera
implementation would be competing architecture.

## Scene model direction

Additions should be owned by an existing client or platform crate, not a new engine competing
with `horizon-client`. A future scene graph should use these conceptual models:

- stable scene and actor identifiers
- hierarchy and local/world transforms
- active camera and environment settings
- typed property channels with keyframes
- in-memory world override layer
- `.cmx` project metadata and asset references

Concrete structs are not being added in Phase 1. No API shape is asserted until implementation
traces a real caller and resource owner.

## Ownership and data flow

The preferred data flow is:

1. Gameplay lifecycle owns editor session state and input capture.
2. Existing replay/Studio modules own replay and camera playback state.
3. A future scene module supplies typed evaluated values.
4. Horizon renderer consumes camera/frame inputs and existing world/entity resources.
5. The world override layer applies temporary visual state only; persistent save code is not
   called unless the user explicitly uses normal gameplay save operations.

## Singleplayer lifecycle

Horizon's window is a client on an integrated server thread: `game/integrated.rs` runs the
server, while the window uses `remote` and `integrated` state. Cinematix must treat editor state
as client/session state and must not introduce networking or multiplayer synchronization.
