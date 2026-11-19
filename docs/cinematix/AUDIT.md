# Cinematix source audit

## Status and provenance

This is a **source-backed, bounded audit**, not proof that Cinematix runs.
The destination contained only its project brief when inspected on 2026-11-19.
No Horizon baseline, Rust implementation, build or interactive verification is
included in this documentation contribution.

Audited checkouts:

- Horizon: `https://github.com/horizon-rust/horizon`,
  revision `5ef7734fdd854add5efd6817178be3c988d8d050`.
- BBS-FS: `https://github.com/Wemppy4/bbs-fs`,
  revision `13bd62dab1980b974b8ea0f78d037e8e82684b84`.

[`sources.json`](sources.json) records inspected files, SHA-256 digests and
revision-pinned source URLs. Paths below are relative to the corresponding
upstream repository, **not** files already imported into Cinematix. Files were
read selectively around implementations and their callers; inclusion does not
mean every line or every execution path was audited.

Baseline import is already being worked on in
[PR #1](https://github.com/Wakcli/cinematix/pull/1). This contribution does not
replace that work or modify its branch. Cinematix-specific documentation lives
under `docs/cinematix/` so importing upstream `docs/architecture.md`,
`docs/ui-kit.md` and other guidance does not overwrite either project's docs.

## Workspace, lifecycle and ownership

| Area | Actual implementation | Integration consequence |
|---|---|---|
| Build conventions | Root `Cargo.toml`: edition 2024, Rust minimum 1.95, LGPL-3.0-or-later. Renderer manifest uses wgpu 30.0.1, winit 0.30.13, glam 0.30. Root patches retain local wgpu-hal, naga and mimalloc fixes. | Import the real workspace and lockfile; do not create a substitute engine or replace its graphics dependencies. |
| Client split | `crates/horizon-client/src/lib.rs` re-exports the reusable library. `crates/horizon-client-lib/Cargo.toml` points its library root at `../horizon-client/src/client_lib.rs`. | The shared source layout is intentional; do not duplicate the reusable modules into a competing library. |
| Game state | `crates/horizon-client/src/game.rs`: `Game` owns world, renderer, Studio, replay and integrated-server state, `Arc<Resources>` and `PackResources`. | Put session ownership at a deliberate client lifecycle boundary, not in process-global mutable editor state. |
| Events | `crates/horizon-client/src/game/runtime/window_handler.rs` handles winit events and scripted-run input exclusion; `game.rs` drives tick/render/pause synchronization. | Follow actual event ordering and reuse existing screen/focus routing. |
| Local world | `game/world/world_session.rs` opens a local world on the integrated server; `game/world/integrated.rs` owns the server thread. | A singleplayer window has `remote` state too. `remote.is_some()` is **not** a test for unsupported multiplayer. |
| Pausing | `game/interface/pointer_capture.rs::pauses_game` checks local server, unpublished world and no splitscreen guests; frame processing calls `Integrated::set_paused`. The server suppresses pausing with joined split players. | Editor pause must use this path, not merely stop the window's tick. Initial scope should reject published/split sessions. |
| Worker data | `crates/horizon-client-render/src/streaming.rs` hands immutable `Arc` chunk snapshots to workers and rejects stale sequence results. | Overlay mesh jobs need immutable snapshots and their own revision/invalidations. Never write an editor delta into a canonical chunk to make a worker see it. |
| Saves | `game/world/saving.rs` stops the integrated server on exit and performs bounded snapshot work on other save paths. | Keep `.cmx` data and temporary render overrides outside world/player serialization. |

Actual gameplay sources have moved into `game/interface/`, `game/runtime/`,
`game/world/`, `game/network/` and `game/modding/`. Some upstream prose still
names the former flat `game/*.rs` paths. Resolve source locations from the pinned
tree rather than copying those old paths into an implementation plan.

The authoritative layer rules are upstream `docs/layers.md`: engine depends on
engine; platform can depend on engine/platform; product extras use platform and
product, not direct engine dependencies. Existing exceptions are not permission
to add new exceptions. `horizon-platform/src/lib.rs` exposes existing
`Hooks::frame`/`tick`, player pose/draw types and camera hooks through neutral
player data; `horizon-hooks/src/players.rs::CameraOverride` changes third-person
distance only, not arbitrary cinematic world pose. Rich editor access may require new neutral engine hooks
and platform-facing types. See [the proposed architecture](ARCHITECTURE.md).

## Renderer and shader packs

The renderer is `crates/horizon-client-render/src/gfx.rs`. Its `Renderer` owns GPU
resources, targets and temporal state; a borrowed `Frame` supplies camera, world
environment, entity vertices, optional GPU pose jobs, visual lights, gizmos and
HUD data. The current `Frame`/renderer seam is real, but it is not itself a
stable product-facing API.

Shader-pack parsing lives in `crates/horizon-shaders/src/lib.rs`; the renderer
runs the translated programs. `gfx/external.rs::ExternalPipeline` exposes
workspace-internal pass points, including shadow requests, world targets and
entity layers. Its documentation explicitly disclaims mod-facing stability.
It is a pipeline replacement seam: installing a competing Cinematix pipeline
there is **not** a safe way to preserve the active shader pack.

`gfx/compositor.rs` already composes same-device offscreen renderer frames.
However, `Compositor::new` requires `COPY_DST` and a `Bgra8Unorm` surface; it
copies already rendered frames into window regions. This is an SDR presentation
implementation, not evidence of an arbitrary HDR-compatible embedded viewport.
Do not create a second player/client/renderer for the editor merely because
splitscreen does so.

Upstream `docs/hdr-passes.md` and `gfx/hdr.rs` define the color contract:

1. World light, world post processing and effects precede tone mapping.
2. HUD and menus have a separate 8-bit layer on the float/HDR path.
3. HDR/API 0.2 packs may use `Rgba16Float`; standard `shaders/` packs have an
   8-bit picture. Float format alone does not identify gamma vs linear encoding.
4. Readbacks must use SDR mapping or an explicitly supported HDR encoding;
   clipping raw light to `[0, 1]` is not a valid export conversion.
5. Resize, encoding/pack changes and discontinuous camera/time changes require
   appropriate temporal-history invalidation.

For an embedded world rectangle, add the smallest renderer capability that
keeps the existing pipeline/output owner and gives the world its actual viewport
aspect. Validate every affected pass and pack path. The current compositor's
restrictions must be addressed, not described as solved.

## Existing cameras, replay and input

| Existing facility | Source and behavior | Missing Cinematix capability |
|---|---|---|
| Detached camera | `studio/camera.rs::StudioFreecam` owns separate double-precision position, yaw/pitch/roll/FOV, smoothed velocity and camera track. `game/interface/perspective.rs::camera_view` selects it; `turn_player` returns after turning freecam, without changing player yaw/pitch. | Live singleplayer activation, viewport control, retained editor state and explicit input ownership. |
| Camera paths | `studio/spline.rs::CameraTrack`: sorted insertion replaces equal-time keys; evaluation clamps endpoints, uses Catmull-Rom position, quaternion SLERP and FOV interpolation. Times are `u32` milliseconds. | General typed property tracks, tangent/easing editing, clips, exact sub-frame time, arc-length paths and constraints. |
| Studio screen | `game/interface/studio.rs::open_studio_screen` guards world/UI state, clears held keys/attack/use/mining and releases capture; close returns to Playing and captures. `Screen::Studio` is a script-window slot, not a scene editor. | Do not relabel it and claim a working Cinematix workspace. A dedicated lifecycle/input contract is needed. |
| Flight/input | Studio flight uses raw keys and only runs in `Screen::Playing`. `play_input.rs`'s replay gate also only applies while replaying in Playing; pointer look requires Playing/capture. | General editor input cannot inherit replay-only gates or rely on raw key constants for configurable controls. |
| Studio timeline | `studio/timeline_ui.rs`, `studio/topbar.rs`, `game/interface/studio.rs`: replay play/seek, camera-key moves, markers and clip split points. Step buttons currently move a second. | Scene-backed multichannel sequencer; replay transport is not animation evaluation. |
| Replay transport | `studio/replay.rs::ReplayDriver` accepts `Duration`, tracks a scaled cursor and processes timestamped packets. `seek_and_collect` starts from the recording's beginning; forward seek rejects rewind. Game replay rebuild logic lives in `game/runtime/replay.rs`. | Reversible scene scrubbing and deterministic simulation/export require an explicit reconstruction strategy. |

The `/studio freecam on` command requires a replay in the audited source.
Other internal Studio controls can activate the same camera. This distinction
matters: the camera implementation can be reused without claiming a documented
live-world editor entry point already exists.

## Mathematics

Existing Rust camera math includes both glam and deliberately parity-preserving
JOML-derived **Rust** helpers; that is not a reason to introduce a Java runtime
or a replacement math package.

`crates/horizon-client-render/src/camera.rs` establishes:

- Y is up; yaw 0 faces south (`+Z`); yaw 90 faces west (`-X`).
- World positions are doubles; camera-relative offsets are subtracted in doubles
  before float GPU transformation.
- Matrices are column-major; clip coordinates are projection × view × position.
- The game uses reversed depth and zero-to-one projection; its call sites swap
  the near/far arguments. Reuse them, do not substitute a conventional-Z matrix.
- Camera orientation has a parity-specific YXZ conversion. Camera path
  quaternions and model pose Euler conventions are not interchangeable.

`horizon-cosmetics/src/pose.rs` shows model-part translation in pixels divided by
16, rotations in radians, `T * R * S`, and parent × local composition. Import
normalization and the camera adapter need independent axis/roll tests before a
new shared scene quaternion convention is considered integrated.

## Assets, models and lighting

`horizon-assets/src/obj.rs` already parses OBJ/MTL; `resolve.rs` caches resolved
models and `bake.rs` consumes them. The audited parser stores position, UV,
object/material and quad faces, fan-triangulates larger polygons, and supports
negative indices. It does **not** retain source normals/tangents or a complete
PBR material: `parse_mtl` handles `map_Kd` mappings to sprite/slot references.
This is useful resource-model infrastructure, not full external scene importing.

`gfx/entity_skin.rs` already retains uploaded entity model geometry and uploads
part matrices/jobs per frame. Its current `SkinVertex` has an axis-normal and
single part index; eligible models are axis-facing ordinary opaque/cutout
entities. This is rigid model-part GPU posing, **not** general multi-weight
skinning, morph targets or an IK authoring system. Existing mobs and cosmetics
are integration references; do not create duplicate Steve/Alex renderers.

`dynamic_light.rs` has nearest-source selection (`MAX = 16`) and colored
position/radius/RGB/intensity entries. `game/modding/native_visuals.rs` merges
existing visual requests, particles and item lights; `Frame::colored_lights`
reaches the built-in terrain/entity shaders. These are client render lights,
not canonical block-light or save updates. They do not establish full
spot/area/directional editor lights or custom shadow support in every pack.

Targeted source searches did not identify a complete Cinematix FBX/glTF importer,
general IK editor, `.cmx` project store or FFmpeg export path. This is a bounded
negative finding, not a proof that no related code exists anywhere upstream.
Repeat the search against the imported baseline before creating replacements.

## World overrides and capture

`horizon-client-render/src/world.rs::World::block` reads canonical chunks and
falls back to air for an unloaded chunk. An editor resolver must distinguish
unloaded/unknown data from a known air cell. Altering canonical chunks or invoking
normal place/break/server commands cannot meet the non-destructive requirement.

Existing capture facilities have distinct purposes:

| Facility | Verified scope | Not established |
|---|---|---|
| `clock.rs::FixedStep`, `HORIZON_STILL` | Frozen initial world, repeatable visual clock, `/tick step` at whole simulation ticks. | Arbitrary scene subframes at 24/30/60/120 FPS. |
| `game/runtime/screenshot_sequence.rs` | Runs a pack's first N frames on set timing, writes **only the last** PNG. Requires frozen still mode, settled loading/meshing, no LOD/fade/shadow cache/TAA/frame generation. | A per-frame image sequence or completed video exporter. |
| `gfx/screenshot_readback.rs`, `gfx/hdr.rs` | Existing readback/PNG and HDR-to-SDR mapping infrastructure. | Ordered, bounded offline encoder queue and cancellation/recovery. |
| `gfx/capture.rs` | Captures renderer inputs and mesh bytes for same-build `scene_replay` debugging. | Portable `.cmx` scene/asset serialization. |

Export must address world readiness, exact frame time, simulation isolation,
temporal warmup/history, color mapping, ordered backpressure and encoder process
failure. It must never quietly drop frames.

## Verification and next gate

Inspection used Git, Python and direct source reads. Revision/file digests,
pinned links and destination documentation links are checked in
[DEVELOPMENT.md](DEVELOPMENT.md). No Cargo or Rust compiler was available in the
audit VM, no player-provided prepared resources were present, and no game was
launched. No compilation, GPU or interactive behavior is claimed.

Before Phase 2: reconcile this audit with the actual imported baseline, resolve
the viewport and platform input/screen seams, then build and test a single native
editor lifecycle slice. The audit is not a substitute for those checks.
