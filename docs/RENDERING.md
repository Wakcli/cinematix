# Rendering audit

## Existing implementation

- Graphics API: `wgpu 30.0.1`, with Glam `0.30`.
- Window: `winit 0.30.13`.
- World render entry: `crates/horizon-client-render/src/gfx.rs`.
- HDR: `crates/horizon-client-render/src/gfx/hdr.rs`.
- Offscreen/compositor work: `crates/horizon-client-render/src/gfx/compositor.rs`.
- Screenshot and readback: `gfx/readback.rs`, `gfx/screenshot.rs`, `gfx/screenshot_readback.rs`.
- Render capture: `gfx/capture.rs`.
- Dynamic lights: `crates/horizon-client-render/src/dynamic_light.rs`.
- Shader packs: `horizon-shader-api`, `horizon-shaders`, `horizon-optifine`,
  `horizon-polytone` and `gfx/pack_*`.

## HDR and passes

Horizon's frame is `Rgba16Float` for shader API 0.2 or HDR output without a pack. Ordinary
frames or `shaders/` packs can be 8-bit. The pass order and capture obligations are defined in
`docs/hdr-passes.md`: world, world post/post effects, tone curve, camera overlays, HUD, then
HDR output. Cinematix UI, capture and post effects must follow that order and must not clip HDR
light.

## Cameras and geometry

The renderer owns projection/frame inputs, world/section meshes, celestial, clouds, entities,
materials, block entities and GPU resources. `gfx/capture.rs` can retain CPU copies of uploaded
section and far-tile meshes for deterministic render capture. It also records camera, yaw/pitch,
FOV, environment, fog, frame and renderer settings in `scene.json`.

## Reusable capture

Horizon already has a deterministic frame-step and screenshot-sequence mechanism:
`HORIZON_STILL=1`, frozen tick state and `/horizon-ctl screenshot-sequence`. `gfx/capture.rs`
supports scene replay by captured meshes and frame inputs, but it explicitly does not include
entities, particles, weather, hands, outlines or HUD in its scene replay. Cinematix cannot
present it as complete frame capture until the missing scene inputs are added or excluded
truthfully.

## Import and editor implications

The existing renderer is the only production renderer for Cinematix. Any editor viewport,
gizmo, imported model or extra light must integrate through its frame/mesher/resource APIs.
No second wgpu or LWJGL renderer is permitted.
