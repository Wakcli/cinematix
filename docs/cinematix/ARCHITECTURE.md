# Cinematix integration design

## Status and decisions

This is the proposed implementation contract derived from [the source audit](AUDIT.md)
and [BBS-FS inspection](UI.md). **None of the new APIs or data types described
below is implemented by this contribution.** Names are design vocabulary, not
existing Rust symbols. Confirm them against the imported baseline before coding.

Use one native client session and one renderer. Preserve Replay Studio, normal
gameplay, shader packs and the vanilla save pipeline. Avoid a crate per logical
subsystem. The editor is a product feature behind `horizon-extras`, while neutral
render/input capabilities belong below it and are exposed through the platform.
No Minecraft mod packaging, Java integration or standalone editor is introduced.

The existing platform exports player-frame/pose/draw hooks, settings, menu pages,
HUD drawing and translation registration. Its camera hook only adjusts
third-person distance, not a general world pose. Those useful seams do not yet
establish an arbitrary editor screen/input/viewport API.

## Layer and module ownership

| Logical group | Owner and dependency direction | Initial scope |
|---|---|---|
| Render targets, view projection, geometry submission and readback | Existing engine renderer. Neutral capabilities consumed through a platform adapter, without naming Cinematix. | Extend only what a real first slice requires. No second renderer or pack replacement. |
| Input ownership, configurable actions, screen/focus and local pause | Engine input/screen lifecycle plus neutral platform-facing screen adapter. | One ownership gate; player action suppression is enforced before gameplay dispatch. |
| Reusable animation/evaluation and scene data | One cohesive native Cinematix library, classified in `tools/layers.json` according to actual dependencies. Pure evaluation may be a platform facility if exposed for reuse. | No engine dependency from the finished product layer, no invented dependency exceptions. |
| Session/workspace, panels, timeline and commands | Product editor on platform APIs; code stays in focused modules behind `horizon-extras`. | Per-client session, no global mutable project registry. |
| Assets/export/persistence | Editor-owned data and worker jobs, engine-managed GPU handles via platform. | CPU work can run off-thread; renderer mutation remains on its owning thread. |

Do not immediately create that library or add crate manifests. First establish
the neutral adapters and decide placement with the baseline owner. A small
group of modules is sufficient until independently reusable code justifies a
crate. Keep engine glue separate from product logic so a later layer extraction
does not change semantics or break existing mod point names/signatures.

## Shared scene vocabulary

The following interfaces should be agreed before scene, timeline and UI work:

- **Project/Scene:** versioned identity and metadata; stable object IDs; actors,
  objects, active camera ID, environment, lights, animation, world layers and
  render settings. Scene data never borrows a live `Game` or GPU resource.
- **Actor/Object:** local transform, parent ID, visibility, mesh/material IDs,
  optional skeleton and animation references. Resolve references through an
  asset store; do not duplicate mesh bytes per actor. Parenting must reject
  cycles and dangling IDs.
- **Transform:** double world translation, normalized `(x, y, z, w)` quaternion,
  local scale. Local composition is `T * R * S`; world composition is
  `parent_world * local`. GPU coordinates subtract the double camera origin.
  Normal transforms account for nonuniform scale.
- **Track:** stable target ID plus a typed property address, typed sorted keys,
  mute/lock and extrapolation. Scalar, vector and quaternion values cannot be
  accidentally assigned to an incompatible property.
- **Keyframe:** exact timeline position, typed value, interpolation, incoming/
  outgoing tangents and easing where applicable. Duplicate timestamps have one
  documented deterministic replacement/rejection policy.
- **Light:** type and transform, color/intensity, attenuation/range, requested
  shadows and type-specific cone/size parameters. A renderer capability result
  states supported vs unsupported behavior; no hidden degradation.
- **WorldDeltaLayer:** dimension-qualified block positions, replacement states,
  active/visible ordering and revision. Original states are metadata only where
  needed, not a mandate to apply-and-undo real world mutations.
- **Asset reference:** stable ID, relative source/embedded location, content hash,
  import metadata and validated model/material/rig payload. Runtime GPU handles
  are owned separately and never serialized.

Camera pose conversion is an adapter to existing Horizon camera conventions,
not a reuse of arbitrary model Euler angles. Position, quaternion, projection,
roll and focal parameters need independent test vectors. Reuse glam and existing
camera projection/parity helpers.

## Exact time and one animation backend

Keep authoring time and sampling time separate from wall-clock `Instant`.
Represent key times as checked integer ticks with a declared project timebase;
frame evaluation also carries an exact rational timestamp or checked fraction.
A proposed base of 120,000 ticks/second makes whole frames at 24/30/60/120 FPS
integral; sub-frame samples must remain exact even when their count does not
divide that base. This is a design choice to validate, not an upstream clock.

For frame `n` at rate `p/q`, the frame time is `n*q/p` seconds, calculated from
the frame index rather than repeated floating-point increments. A shutter
sample has an explicit rational offset. Convert to floats only inside
interpolation/rendering. Guard overflow and invalid denominators.

Evaluation reads authored data and writes an **evaluated scene snapshot**.
It does not mutate keys, advance server physics or write permanent blocks.
Inspector, dope sheet and curve editor send edit commands to the same tracks;
undo/redo records complete transactions. Drag preview may reevaluate but a
gesture produces one undo operation. The existing camera track gets an adapter,
not an unrelated second spline implementation.

Boundary behavior, constant/linear/cubic/Hermite/Bézier scalar/vector tracks,
quaternion normalization/shortest-arc SLERP, clip blending and extrapolation
must be specified and tested individually. Playback and offline rendering call
the same evaluator. Live-world simulation does not become reversible merely
because the animation evaluator is reversible.

## Native editor lifecycle contract

1. **Inactive:** retain project/session data, but do not evaluate, allocate
   frame scratch, build panels or submit render work.
2. **Opening:** accept only a loaded, unpublished singleplayer client with no
   split players; defer/refuse during loading, saves, resource reload or an
   incompatible screen. Capture previous screen/input/focus/cursor state.
3. **Active UI:** clear gameplay action state, attack/use/mining and transient
   drags. Editor owns pointer and keyboard. Text/modal/panel focus wins over
   viewport navigation; unconsumed events still do not reach player actions.
4. **Viewport navigation:** temporary capture uses the detached camera state.
   Gameplay position/yaw/pitch are never updated or restored by teleporting.
   Physical releases, controller inputs and focus loss obey the same gate.
5. **Paused simulation:** feed the existing local-server pause decision without
   stopping editor time/evaluation. If publication/split state changes, leave or
   refuse the editor safely rather than pausing other players.
6. **Closing/suspending:** cancel drags/capture, release render scratch/overrides,
   restore only valid focus/capture state, preserve project/camera/layout and
   reconcile current physical inputs. Never restore stale held keys or grab an
   unfocused window.
7. **World exit/dimension/resource change:** invalidate session bindings and GPU
   handles, detach overrides and cancel outstanding jobs. Project data can
   remain; it must not continue referring to the old live world.

Normal Escape, menu close, focus loss, failed opening, export cancellation and
world shutdown must share cleanup, rather than each ad hoc branch restoring a
different set of flags.

The platform screen adapter should provide a lifetime-bounded context with
session identity, translated native UI drawing, dimensions, input ownership,
camera override request and pause request. It must not hand the product a mutable
`Game`, canonical chunk map, GPU device or raw server command executor.

## Viewport/render data flow

```text
Native input -> editor commands -> authored project / undo
Exact time + authored project -> evaluated scene
Loaded world snapshot + evaluated world layers -> visual world view
Evaluated camera + meshes/lights + visual world view -> engine Frame adapter
Existing world/pack passes -> world color target -> existing HDR output + editor UI
```

The central viewport must render the actual loaded world with its own aspect.
The current full-window camera override can inform a first camera slice but is
not a completed embedded viewport. Split compositor copies are SDR-only.
Extend target/viewport handling within the existing pipeline and verify
projection, culling, sky, post effects, motion/history, HUD layering and capture.

Imported/procedural meshes should reuse entity/material upload paths where
their vertex contract fits. General normals/tangents, multiple materials and
weighted skinning need explicit renderer extensions. Submit shadow and pack
identities through existing pass contracts; arbitrary model geometry does not
automatically become compatible with every pack. Preserve the active
`ExternalPipeline`, do not overwrite it with an editor-specific renderer.

## Visual world isolation

Read-only resolution order is active editor layers, then the loaded canonical
world. Resolve unloaded data explicitly. Generate override section meshes and
light/culling inputs from immutable visual snapshots; track layer revisions and
neighbor section invalidation, and reject results from previous sessions/layers.
Removing an override reveals the **current** underlying block, not an old saved
copy that overwrites a legitimate world update.

Keep these render meshes and overrides outside integrated-server data,
`World::modified`, region/player writes, placement packets and gameplay
collision by default. Block entities, AO/light neighbor sampling and chunk
unloading require the same visual view, not only a last-minute draw of cubes.
An isolated save roundtrip with edits active must prove disk data unchanged.

## Export and persistence contracts

Export is a state machine: validate capabilities/readiness -> pin scene/resource
revisions -> warm temporal state -> evaluate exact frame/sample -> draw/capture
-> ordered bounded readback -> image/encoder sink -> finalize or fail/cancel.
Frame indices accompany every job; backpressure waits rather than dropping
frames. External FFmpeg receives an argument vector (not an interpolated shell
command), controlled stdin, drained stderr, checked exit status and guaranteed
termination/reaping on cancellation. No H.264/HEVC/ProRes support is claimed until
encoder availability and actual output tests establish it. Alpha is not
available just because an encoder name supports it.

The `.cmx` format is separate from same-build renderer debug capture and `.mcpr`.
Use versioned JSON scene metadata and relative/optional embedded binary assets;
reject path traversal, dangling refs, oversized/corrupt payloads and unsupported
versions. Establish a container choice and schema in the serialization slice,
not an empty format implementation now. Atomic save uses a sibling temporary
file and rename; distinguish recoverable validation errors from data loss.
Do not serialize GPU handles, live player identities or game resources that
the project has no right to redistribute.

## Tests and completion gates

- Pure behavior: transforms/parent cycles, camera axis/roll, typed interpolation,
  exact frame/subframe times, edit transactions, undo and schema validation.
- Lifecycle: local eligibility, input exclusivity, open/close/focus/exit cleanup,
  input held across opening, configurable pause and preserved player state.
- Render: real world viewport, resize, no-pack/standard/native pack paths,
  HDR mapping, mesh material/shadow participation and history reset.
- Persistence: world save bytes/semantic state unchanged with overrides active;
  `.cmx` save/load/corrupt/relative/embedded fixtures.
- Export: every indexed frame, bounded memory, cancellation, missing/broken
  encoder, stderr pressure, output frame count and repeatability.
- Performance: inactive editor has no scene evaluations/draw work; profile
  allocations/frame time on a representative scene rather than promise a
  percentage without measurement.

Buildability, tests and interactive proof are gates for each implementation
slice. [ROADMAP.md](ROADMAP.md) makes the dependencies and current blockers
explicit; [DEVELOPMENT.md](DEVELOPMENT.md) records what was actually checked.
