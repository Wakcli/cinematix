# Cinematix architecture

Cinematix is a native subsystem of Horizon: an in-game cinematography, animation and
offline-rendering suite. It is not a mod, not a loader plugin, and not a second application. The
editor runs inside the running singleplayer client, on Horizon's own renderer and UI kit.

Evidence for every claim below is in [AUDIT.md](AUDIT.md).

## 1. Where the work happens

This repository is Horizon's tree plus Cinematix. There is no second renderer and no second window.

```
Game (singleplayer, server thread + client window)
└── Screen::Cinematix                      editor open
    ├── viewport      the live world, an editor camera, gizmos, path and light overlays
    ├── chrome        icon rail, docked panels, inspector, timeline, curve editor
    └── state         CinematixSession — scene, selection, playhead, undo
```

The editor is a `Screen`. Horizon already holds one at `game.rs:1147`, and
`settings_shell::Shell` proves a large screen can be described by data instead of hand-drawn per
variant. That is the model to copy: a screen that reads a state struct and emits commands.

When the editor is closed, the game is unchanged. No background updates, no retained GPU resources
beyond the asset store.

## 2. Crates

```
crates/
  cinematix-scene      scene graph, nodes, transforms, world delta layer      (no Horizon deps)
  cinematix-anim       channels, keys, interpolation, pure evaluation         (no Horizon deps)
  cinematix-timeline   time axis, zoom/pan, snapping, hit testing             (no Horizon deps)
  cinematix-ui         widget logic: dock tree, scrubber, panel state         (no Horizon deps)
  cinematix-render     render jobs, codecs, ffmpeg argument construction      (no Horizon deps)
  cinematix-editor     the in-game editor screen: drawing, input, commands    (Horizon-facing)
```

Dependency rule:

```
cinematix-editor → cinematix-ui, cinematix-timeline, cinematix-scene, cinematix-anim,
                   cinematix-render, horizon-client-render
everything else  → std, glam, serde
```

Only `cinematix-editor` may name a `horizon_*` crate. Everything below it is testable with
`cargo test` on a machine with no GPU, no game and no Minecraft install. That is what keeps the
animation, timeline and project logic verifiable while the engine side stays in flux.

`cinematix-editor` belongs in the engine layer, above `horizon-client-render` and below gameplay
wiring: it draws with `HudBatch` and `ui`, but never ticks the world.

## 3. What Horizon already provides, and must be used

| Need | Use | Not |
|---|---|---|
| Screen lifecycle, input routing | `Screen` + `Game`, `game/ui_control.rs` | a new event loop |
| Drawing | `HudBatch`, `HudAtlas`, `ui::*`, `hud_kit::*` | egui, a second GPU context |
| Panels and surfaces | `hud_kit::plate::Plate`, `hud_kit::room::Room` | a retained-mode toolkit |
| Curves, discs, arcs for gizmos | `hud_kit::shapes` | ad-hoc triangle math |
| Text fields, sliders, scroll | `ui::EditBox`, `ui::slider`, `ui::Scroll` | new controls |
| Camera free flight | `studio::StudioFreecam` | a second freecam |
| Camera path evaluation | extend `studio::CameraTrack` | a parallel spline type |
| Timeline interaction shape | `studio::TimelineUi` | a different interaction model |
| Euler↔quaternion boundary | `camera::Mat4::to_glam`, `view_rotation_matrix` | re-derived conventions |
| Frame stepping | `horizon-ctl screenshot-sequence`, `HORIZON_STILL` | a new capture path |
| Replay capture | `studio::recorder`, `.mcpr` | a new recording format |
| Strings | `Text::tr`, `lang::get` | English literals |
| Controls | `ui::button` (registers) | `screen_draw::button` (paints only) |

Upstream rules that apply to all Cinematix code: architecture layers
(`tools/check_layers.py`), translation keys, HDR pass discipline, and keeping pull requests small.

## 4. Editor activation and editing mode

Implemented in `game/interface/cinematix.rs` on top of `crates/cinematix-editor`.

Activation is `key.cinematix` (F8, `horizon-client-input/src/keybinds.rs`, one row in
`HORIZON_MAPPINGS`), `/cinematix open` in chat, and `horizon-ctl cinematix open`. The gate is the
same shape as Studio's (`cinematix_screen_opens`): it opens only over `Screen::Playing` with a
loaded world and a player, answers `true` when already open, and refuses every other screen.

While it is open:

1. **Input is the editor's.** `key_pressed` hands every key to `cinematix_key` before the gameplay
   mappings; mouse press, release, drag and the wheel are taken by `cinematix_mouse_input`,
   `cinematix_mouse_dragged` and `cinematix_wheel`; a click can never reach block breaking.
2. **The world keeps running.** `Screen::Cinematix.is_pause_screen()` is false, as for Studio and
   the emote wheel. Freezing the world for authoring is a later, deliberate feature.
3. **The HUD is hidden.** `draw_hud`'s `show_hud` excludes `Screen::Cinematix`, so the hotbar and
   crosshair do not sit under the viewport.
4. **Closing restores play.** Escape or F8 calls `close_cinematix_screen`: `Screen::Playing`,
   `capture(true)`. The `cinematix_editor::Editor` (and so the film) stays in `Game`, not in the
   screen, so closing and reopening the screen keeps the document.

5. **The film camera is what renders.** While the screen is open, `Game::camera_view`,
   `Game::camera_orientation` and `Game::fov_degrees` take the pose from
   `Editor::preview_view`/`preview_fov` (the camera node evaluated at the playhead) before any
   player or Studio camera. The hand and the hurt/bob camera effects are suppressed through
   `studio_draws_hand` and `camera_effects_enabled`. On opening, the camera adopts the player's
   eye and view angles unless its channels already hold keys.

In the editor, W/S/A/D fly horizontally in the camera's own frame, E and Q rise and fall, at
8 m/s; holding the right mouse button looks around (`Ctrl` slows the look). Moving an unkeyed
camera writes its rest pose; moving a keyed camera writes a key at the playhead, so an animated
camera stays animated. `cinematix camera player` (`/cinematix` as well) puts the camera back at
the player and keys it when the channels are keyed.

The player's position, rotation and camera must survive a full editor session unchanged. That is a
test, not an intention.

## 5. Time

Two clocks, never confused:

- **World tick** — Horizon's 20 Hz simulation. Frozen while authoring.
- **Timeline** — `f64` seconds computed as `frame / fps`. The only input to animation evaluation.

`evaluate(scene, t) -> Pose` is pure: no frame delta, no wall clock, no state carried between
frames. Playback and export call the same function and must never reuse each other's results.

Offline rendering evaluates at exactly `1/fps`, in `f64`, then snaps to the frame.

## 6. Scene and animation model

Already implemented and tested in `cinematix-scene` and `cinematix-anim`:

```rust
Scene { meta, nodes: HashMap<NodeId, Node>, roots, camera, world, assets }
Node  { id, name, parent, children, rest: Transform, kind, visible, locked }
NodeKind { Empty | Camera | Light | Actor | Mesh }
Channel { target: PropPath, keys: Vec<Key>, muted }
Key     { t: f64, v: Value, interp }
Interp  { Step | Linear | Smooth | Spline | Bezier { out, in_ } }
Value   { F32 | Vec3 | Quat | Color | Bool }
```

Design points that survive review:

- Channels are keyed by a stable property path, not embedded in objects. That is what makes one
  generic curve editor possible and it matches how BBS/McLib addresses properties.
- Rotation is quaternion keys with slerp. Euler is a display and authoring convenience, converted at
  the boundary using Horizon's YXZ convention.
- Keys are sorted; lookup is binary search. Missing channels fall back to rest state; muted channels
  are ignored entirely.
- A key's `interp` shapes the segment *after* it. `Spline` is the uniform Catmull-Rom Studio's
  camera track already used: the two neighbouring keys are the tangents, and an end segment
  extrapolates a virtual neighbour (`p1 - (p2 - p1)` on the first segment, `p2 + (p2 - p1)` on the
  last), so two keys give a straight line and a third bends it. Positions, scales and colours spline
  per component; rotations always slerp. New translation keys default to `Spline`; every other
  property defaults to `Smooth` (`cinematix_editor::default_interp`).
- `WorldDeltaLayer` is an ordered op list indexed by block position, with an optional activation
  time, so a wall can collapse on the timeline and the save is never touched.

## 7. World overrides

Non-destructive by construction. Nothing writes to a region file.

```
WorldDeltaLayer { ops: Vec<BlockOp>, index: HashMap<BlockPos, Vec<usize>> }
BlockOp { pos, change: Set | Remove | Restore, at: Option<f64> }
```

O(1) lookup per position; the renderer asks once per chunk rebuild; a dirty set drives remeshes.
Removing the layer restores the world because the world on disk never changed.

## 8. Rendering and lighting

Honest constraint, from the audit: Horizon's shader packs shade sky, material, entity, weather,
post, tonemap and shadow. They do **not** shade imported meshes.

Therefore:

- **Stage 1 (the target):** imported meshes and primitives draw through Horizon's `HudBatch`-style
  immediate path for editor overlays, and through a dedicated world-space pass for geometry, lit by
  Cinematix light objects, using the same sun direction and exposure as the world. No pack
  interaction. Visually consistent because both sides agree on lighting inputs.
- **Stage 2:** publish lights into a buffer a *Cinematix* pack reads. Requires our own pack, not a
  third-party one.
- **Stage 3 (research, not promised):** join the pack's shadow and ray-tracing passes. The test
  hardware here (GTX 1080) has no hardware ray queries, so `traced_*` packs cannot be validated on
  this machine at all.

Do not replace Horizon's pipeline and do not add a second renderer.

## 9. Export

Wrap what already works:

```
for frame in range:
    t = frame / fps
    pose = evaluate(scene, t)
    apply(world_delta, t)
    set camera from pose
    capture offscreen
    encode
```

`cinematix-render` already builds the job, sample times and ffmpeg arguments, and covers H.264,
HEVC, ProRes 422, ProRes 4444 and PNG sequences, with alpha only where the codec carries it.

Two limits found by running the engine, both recorded in [AUDIT.md](AUDIT.md):

- `screenshot-sequence` output is the **surface size**, so arbitrary 1440p/4K needs an explicit
  offscreen target that is not tied to the window. That is the first real renderer work.
- The deterministic guards reject a run that has not settled, and can reject the first attempt.
  A render job must retry the precondition rather than fail.

## 10. Project format

`.cmx` is a zip container:

```
scene.json        structure, channels, world ops, references
assets/<sha256>.<ext>
thumb.png
project.toml      format version, app version, timestamps
```

JSON for mergeability and because agents must be able to read it. Blobs addressed by content hash
for dedupe and integrity. `format_version` plus an explicit upcast chain, tested with fixtures.

Implemented in `crates/cinematix-editor/src/film.rs`: `project.toml` (`format_version`, `app`,
`app_version`, `saved_unix`) and `scene.json` (the `Scene`, every `Channel`). Save/load is reachable
as `cinematix save [PATH]`, `cinematix load PATH` and `cinematix new`, in chat and through
`horizon-ctl`. A file from a newer format is refused with its version; a channel whose node or
property does not exist is refused rather than silently dropped. `assets/` and `thumb.png` are
reserved in the container and not written yet: there are no assets or renders to put in them.

## 11. Documentation layout

Upstream Horizon already owns `docs/architecture.md`, `docs/ui-kit.md` and the rest of `docs/`.
Adding `docs/ARCHITECTURE.md` beside them is a collision on case-insensitive filesystems and a
confusing duplicate everywhere else. Cinematix documentation lives under `docs/cinematix/`:

```
docs/cinematix/AUDIT.md          what the engine actually does, with evidence
docs/cinematix/ARCHITECTURE.md   this file
docs/cinematix/UI.md             the editor's UI, on Horizon's widget kit
docs/cinematix/ROADMAP.md        phases, status, blockers
docs/cinematix/DEVELOPMENT.md    build, run and test commands
```

Further subsystem documents (`RENDERING.md`, `ANIMATION.md`, `CAMERA.md`, `RIGGING.md`,
`ASSET_PIPELINE.md`, `WORLD_MANIPULATION.md`, `SERIALIZATION.md`, `EXPORT.md`) are added when that
subsystem exists. A document for unbuilt code is a liability: it gets read, trusted and then
contradicted.

## 12. Rules for contributors and agents

- Read `AGENTS.md` (Cinematix rules, then upstream Horizon guidance) before editing.
- Read [AUDIT.md](AUDIT.md) before proposing an integration point. Several obvious-looking ones do
  not exist.
- No `unwrap` outside tests; no panics on user data; a malformed `.cmx` is a recoverable error.
- Public items get a doc comment; internal items do not.
- No `unsafe` outside the crate that must touch a foreign interface.
- Every crate keeps a fixture and a test for the behaviour it owns; determinism tests compare bytes.
- A change to a Horizon file is recorded in `docs/cinematix/horizon-patches.md` with its line count.
  That file is the upgrade checklist when the baseline moves.
- Never claim a runtime behaviour from compilation. Interactive rendering needs a run; this
  document distinguishes verified from read for exactly that reason.
