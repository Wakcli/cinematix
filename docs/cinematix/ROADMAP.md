# Dependency-aware implementation roadmap

## Status

Only requirements and this source audit/design contribution exist on this
branch. Native engine integration, workspace widgets and timeline evaluation are
not implemented here. The audit references pinned upstream sources outside the
destination. PR #1 owns the baseline import; do not start a competing import or
push changes to its branch.

## Existing / extension / new matrix

| Capability | Reuse | Extension / new work | Dependency |
|---|---|---|---|
| Native singleplayer lifecycle | Integrated server, screen guards, input/capture and pause decisions | Explicit editor eligibility, focus/input lease, retained session and local pause | Baseline; neutral platform screen/input seam |
| Detached view | StudioFreecam, camera-view/turn selection, gizmos | Live editor entry, remappable controls, exact camera/time adapter | Lifecycle before activating camera |
| BBS-FS workspace | Native UI/HudBatch/focus primitives; inspected reference | Shared docking/floating/numeric widgets and actual world viewport | Native screen plus viewport capability |
| Timeline | Replay transport UI and camera key track | Typed scene channels, exact evaluator, transaction undo, curve editing | Shared scene IDs/time/property contracts |
| Scene/cameras | glam, existing camera math/path | Hierarchy, selection/gizmos, constraints, path arc length/blending | Evaluator and render camera adapter |
| Assets/rigging | OBJ/MTL resource resolver, model parts and GPU posing | Full import validation/materials; general weighted skin/morph/IK | Shared assets and compatible renderer vertex contracts |
| Lights/materials | Client-only colored lights and shader passes | Authored types, cone/area/shadow/PBR capabilities and pack tests | Scene, geometry, renderer seams |
| World layers | Canonical reads, snapshot meshing and revisions | Separate visual resolver/meshes, block-entity/light sampling, save isolation | Shared scene IDs plus mesh/read adapter |
| Export | FixedStep/still clock, capture/readbacks and color mapping | Arbitrary exact frames/subframes, simulation strategy, ordered sinks/FFmpeg | Evaluator, readiness and proven render/capture |
| `.cmx` | serde/json and existing assets/serialization practices | Versioned portable schema/container, bundling, validation/migration | Shared scene/asset schema |

These are source-backed classifications, not completion claims.
See [AUDIT.md](AUDIT.md) for restrictions and exact source paths.

## Sequence and acceptance gates

1. **Baseline reconciliation (existing PR #1).** Retain upstream history/license/
   notices/patches/lockfile, project brief and both sets of agent instructions.
   Record imported revision and compare with `sources.json`. Never copy game
   assets or force-push. Build the imported baseline before editor work.
2. **Phase 1 integration contract.** Review/reconcile the proposed layer,
   viewport, exact-time, model and ownership interfaces with the baseline.
   The current player camera hook changes third-person distance only;
   menu row APIs cannot host the whole editor. Add neutral APIs only when the
   first real use proves what is needed.
3. **Phase 2A lifecycle/camera slice.** Configurable shortcut and existing menu
   entry in a loaded local world; editor-exclusive input/cursor/focus;
   independent camera; retained session; safe cleanup and optional integrated
   server pause. Test releases and focus/world changes, not only a happy-path
   toggle. Reject unsupported sessions explicitly.
4. **Phase 2B workspace slice.** BBS-FS-referenced rail and one functional shared
   numeric/property control plus resizable panel on native UI. Embedded actual
   world viewport requires renderer extension and pack/HDR proof. Add
   docking/floating/inspectors incrementally, not dead placeholder panels.
5. **Phase 3 animation vertical slice.** Stable target/property IDs and exact
   time; scalar/vector/quaternion evaluators with boundary tests. Inspector,
   sequencer and curve editor mutate the same channels. Demonstrate a real
   scene/camera property changing on scrubbing; then extend clips/multiselect/
   copy/paste/undo/interpolation.
6. **Phase 4 scene/cameras.** Parent transforms, selection/gizmos and cameras/
   paths on existing rendering. Confirm player state preservation and camera
   axis/roll conversions.
7. **Phase 5 assets/rigging.** Start one verified import path through live world
   rendering; add material/texture normalization, procedural geometry, rigs,
   weights, morphs and IK/FK as separately tested increments. Do not call the
   existing OBJ or rigid part poser a complete asset/rigging pipeline.
8. **Phase 6 lighting/material integration.** Begin authored colored lights
   through established frame data, then required light types, shadows and PBR
   extensions. Explicitly expose unsupported pack/capability combinations.
9. **Phase 7 visual world layers.** Implement snapshot resolver and render
   invalidation without canonical mutation. Test overlay reveal against later
   underlying updates and prove world save unchanged.
10. **Phase 8 offline export.** Exact sampling, world readiness/simulation plan,
    color-safe ordered readback, PNG sink, FFmpeg process and cancel/recovery.
    Verify every requested frame and output count; the existing
    screenshot-sequence writes only its last frame.
11. **Phase 9 portable project management.** Versioned `.cmx`, relative/bundled
    assets, migrations and atomic saves. Foundational IDs/schema/persistence
    contracts may land earlier when actual dependencies require them.

Each gate leaves a buildable/testable baseline, with a meaningful commit and
accurate status. Continue existing replay features; no broad engine rewrite.

## Open blockers and explicit decisions needed

- Real Horizon baseline and compiler/resource setup are not on this branch.
- PR #1 is active; its changes and audited revision need reconciliation.
- Rich platform editor screen/input/camera ownership is not established by
  existing player/menu APIs. Choose concrete narrow neutral seams first.
- Embedded viewport must preserve HDR and shader-pack output; the current
  BGRA copy compositor cannot be described as that solution.
- Live server simulation is not freely rewindable. Decide supported export
  isolation/replay modes before claiming deterministic animated gameplay.
- Generic imported materials, normals/tangents, weighted skins and custom
  authored shadows need capability tests; existing partial systems are not
  proof of full support.
- No interactive game verification was performed in the audit VM.

An open blocker is not permission to add fake production logic. Document it,
implement the smallest dependency properly, and prove that slice before moving
to the next one.
