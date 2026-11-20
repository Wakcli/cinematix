# Asset pipeline audit

## Existing infrastructure

Horizon has resource pack and model infrastructure in:

- `crates/horizon-assets`
- `crates/horizon-client-render/src/packs.rs`
- `crates/horizon-client-render/src/model_library.rs`
- `crates/horizon-client-render/src/sprites.rs`
- `crates/horizon-client-render/src/assets.rs`
- `crates/horizon-optifine`
- `crates/horizon-polytone`

It already loads vanilla/resource-pack models and textures and owns caching and GPU upload.
Cinematix must use these where an imported/procedural asset can be represented.

## External model import status

No first-party glTF/GLB, FBX or OBJ importer is established for Cinematix. Cargo dependencies
for those formats must not be chosen or added in this audit phase without tracing actual mesh
and material representations.

Assimp bindings are not used and would require explicit justification. Rust-native parsers or
existing Horizon loaders are preferred if measurements show they fit the resource and renderer
layers.

## Required future handling

Later imports must preserve geometry positions/normals/tangents/UVs, PBR materials, textures,
skeleton weights, embedded assets and coordinate normalization. Imported assets are project
data; they must not be committed from a player's installation or copied game directory.
