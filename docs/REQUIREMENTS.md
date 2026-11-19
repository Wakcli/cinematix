# Cinematix requirements and development brief

## Objective and sources

Build a professional cinematography and animation environment directly inside the running Horizon singleplayer client, in Rust. Horizon source: https://github.com/horizon-rust/horizon. UI source reference: https://github.com/Wemppy4/bbs-fs. This file records requested scope, not implemented functionality.

Cinematix is not a Minecraft mod, separate application, second client or separate rendering window. Preserve Horizon's shader-pack support; it is not a replacement for implementing Cinematix lighting, materials and geometry. Inspect the actual engine before selecting dependencies, graphics abstractions or UI integration.

## Audit before implementation

Inspect workspace/crates, lifecycle, game loop, threading, resource ownership, entity architecture, world/chunk/block access, player systems, singleplayer lifecycle and saving. Trace renderer passes, shader loading, shader-pack compatibility, cameras/projections, materials, textures and GPU buffers. Inspect UI layout/widgets, focus/input routing, windowing and compositing; model/texture loading, caching, paths and serialization. Study BBS-FS source for its hierarchy, widgets, docking/floating panels, numeric scrubbers, context menus, timeline, keyframes and curve editor.

Classify each proposed capability as existing/reusable, an extension or new work, with source paths and evidence. Specifically investigate in-world activation, independent camera/viewport, input capture, imported geometry alongside world geometry, simulation pause and deterministic frame capture. Record blockers honestly; do not assume unsupported capabilities exist.

## Architecture

Adapt logical subsystems to Horizon's real module structure: core, scene, animation, camera, rigging, assets, geometry, lighting, rendering, shader/material interfaces, world overrides, timeline, UI, viewport, export and serialization. Avoid unnecessary crates. Define shared interfaces, ownership, data flow and mathematical conventions before dependent implementations.

Scene: stable identity, metadata, actors/objects, active camera, lighting/environment, timeline, override layers and render configuration. Actors: transforms, meshes, materials, skeleton, animation, hierarchy, visibility and render properties. Transforms: translation/rotation/scale, local and world representations. Tracks: targets/properties, typed keyframes, interpolation/extrapolation and deterministic evaluation. Keyframes: time, value, interpolation, tangents and easing; support scalars, vectors and quaternions. Lights: type, transform, color/intensity, attenuation, shadows and specific settings. World delta layers: positions, replacement states, originals where needed, activation/visibility, animation and layer management without modifying the original save.

Reuse Horizon's math library. Document coordinates, units, quaternion convention, matrix order, transform hierarchy, projection/view matrices and animation time.

## Native editor and BBS-FS interaction

Configurable activation shortcut plus existing-menu entry, open/close and session restoration. Reproduce BBS-FS interactions/layout as closely as reasonably possible using Horizon's native systems and respecting licensing. Workspace: central actual-world viewport, vertical icon rail, hierarchy, context-sensitive inspector, timeline/sequencer, curve editor, toolbars, resizable dockable/floating panels and context menus. Shared widgets: numeric drag scrubbers, text inputs, dropdowns, searchable selectors, property sections, timeline rulers/track headers/keyframes, curve handles, docking/floating containers and icon buttons.

Independent editor camera; do not alter player position/orientation. Viewport selection, translation/rotation/scale gizmos, frustums, spline/light/rig visualization and overlays. Editor mode prevents player movement and routes keyboard/mouse/focus correctly, with configurable gameplay pause independent of timeline playback. Closing restores capture/input/gameplay state, releases temporary resources, preserves sessions and avoids unnecessary inactive updates.

## Animation and cameras

Real scene-backed multitrack/multichannel timeline, expandable hierarchy, selection/multiselection, move/duplicate/delete/copy/paste, undo/redo, scrub/play, snapping, zoom/scroll, clips, mute/lock. Channels: translation/rotation/scale components, quaternions, camera FOV/focal length/focus/aperture, light intensity/color, material/morph/environment/world-override properties. Constant, linear, cubic, Bezier, Hermite and quaternion spherical interpolation, editable tangents, deterministic boundaries. Curve editor edits this same data with visibility, selection, tangents, rulers, zoom and pan.

Cameras: free, orbit, target, dolly, rail and spline-following; parenting/look-at/tracking constraints, blending/transitions, Dutch angle, suitable noise-based shake. Animate pose, FOV, focal length, sensor, focus, aperture, depth of field and projection. Catmull-Rom and Bezier paths, tension, arc-length evaluation, orientation/targets, interactive controls and viewport feedback.

## Assets, rigging and geometry

Evaluate existing and Rust-native import infrastructure for FBX, OBJ/MTL, glTF/GLB; appropriate Rust Assimp bindings only if justified. Preserve positions/normals/tangents/UVs, materials/textures/embedded assets, skeletons/weights and animation with coordinate/scale normalization. No Java bindings.

Hierarchical skeletons, FK/IK, constraints, skinning/weights, morphs/facial animation, reusable clips/actions (idle/walk/run/jump/custom). Steve/Alex and mob configurations without hardcoding the architecture around player rigs. Skin support: 64x64, legacy 64x32, capes, emissive layers, hot-swappable textures. Procedural cubes, planes, spheres, cylinders, custom meshes, light cards/emissive screens with dimensions, UVs, materials and transforms.

## Lighting, rendering and world overrides

Actual point/spot/area/directional lights with relevant color/temperature/intensity/range/attenuation/cones/dimensions/shadows. Imported/procedural geometry and custom PBR materials; shadow/specular/AO/transparency/emissive integration using the existing renderer, not a second renderer. Preserve shader packs where possible and explicitly document limitations.

Ambient/sky/sun/moon/fog/volumetric/weather/time controls and animation. Non-destructive in-memory world layers for place/remove/replace, visibility, animated blocks/doors/structures, original-state restoration and large modifications. Animate time, sun/moon, rain/thunder/snow, fog color/density, sky gradients and wind where supported. Keep editor overlays out of persistent world data and saving paths.

## Offline export and projects

Exact timestamp evaluation independent of real-time FPS at 24/30/60/120 FPS, with explicit investigation of simulation isolation. 1080p/1440p/4K/custom resolution and aspect ratio, ordered frame capture, subframe motion-blur sampling, quality controls, image sequences, progress/cancel/recovery. External FFmpeg for H.264, HEVC, ProRes 422 and 4444 where alpha is actually supported, plus PNG sequences. Bound memory, preserve frame order, handle failures/termination/cleanup/partial renders; never silently drop frames.

.cmx projects: versioned JSON metadata plus binary assets where appropriate; serialize hierarchy, actors, models/materials/textures/skeletons, clips/keyframes, camera paths, lighting/environment, overrides and render settings. Relative paths, optional embedding, deduplication, migrations, validation/corruption handling and reliable portable save/load.

## Dependency-aware phases

1. Audit Horizon and BBS-FS; establish evidence-based architecture, interfaces, blockers, testing strategy and roadmap.
2. Functional native in-game editor lifecycle/input/viewport and reusable BBS-FS-style workspace widgets.
3. Timeline, typed animation backend, interpolation, playback/scrubbing and curve editing.
4. Scene hierarchy, transforms/objects, camera rigs/paths, selection and gizmos.
5. Model/texture/material imports, primitives, skeletons/skinning and IK/FK/clips.
6. Lighting/material/render integration, shadows/environment and shader compatibility.
7. World delta layer and animated world/environment changes.
8. Deterministic capture, sampling, image export and FFmpeg.
9. .cmx serialization, assets, migrations and project management; introduce foundational persistence earlier when a real dependency requires it.

Do not attempt every phase in one pass. Every increment must remain buildable/testable. Begin Phase 2 only once the audit and architecture are documented; no disconnected UI prototype.

## Documentation and quality

Dedicated docs/ with ARCHITECTURE.md, RENDERING.md, UI.md, ANIMATION.md, CAMERA.md, RIGGING.md, ASSET_PIPELINE.md, WORLD_MANIPULATION.md, SERIALIZATION.md, EXPORT.md, DEVELOPMENT.md and ROADMAP.md as their subjects become relevant. Do not create empty templates or present planned systems as implemented. Keep root/subsystem agent guidance and explicit implementation status current.

Meaningful unit tests for transforms, matrices/quaternions, splines, interpolation/evaluation, camera constraints, serialization and world deltas. Integration coverage for editor/input lifecycle, scene/assets/rendering/animation and save/load. Track inactive CPU/GPU/memory overhead, frame/render time and asset load cost. A subsystem is complete only when it works in the intended integration, builds, passes relevant tests, manages resources correctly and has accurate docs without placeholders.

Use small meaningful commits and push work to Wakcli/cinematix. Preserve working APIs, upstream licensing and unrelated functionality. Never claim successful tests or runtime behavior without verification. Initial scope explicitly excludes multiplayer synchronization, collaborative editing, remote scene control and new networking protocols; this does not prohibit using Horizon's existing singleplayer architecture internally.
