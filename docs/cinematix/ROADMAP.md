# Cinematix roadmap

Phases follow the project specification. Status is honest: `done` means code exists and its checks
pass, `partial` means some of it exists, `todo` means no code.

## Phase 1 — audit and architecture

| Item | Status | Evidence |
|---|---|---|
| Horizon source imported, revision recorded | done | `1664a35`, [AUDIT.md](AUDIT.md) |
| Build and boot verified | done | 18 m 44 s release build, first frame 463 ms |
| Prepared resources | done | `horizon-prepare`, `--check` exit 0, 4 962 sounds |
| Headless operation | done | `xvfb-run` + `--offscreen`, 305 ms |
| Rendering audit | done | [AUDIT.md](AUDIT.md) §UI, §Camera, §studio |
| UI audit | done | no `horizon-ui`; kit is `hud.rs` + `ui.rs` + `hud_kit/` |
| BBS-FS inspection | done at behaviour level | [UI.md](UI.md); McLib licence prevents code reuse |
| Integration points identified | done | [ARCHITECTURE.md](ARCHITECTURE.md) §3 |
| Documentation and agent instructions | done | `AGENTS.md`, `docs/cinematix/` |
| Deterministic repeat runs proven | **not proven** | one successful sequence; byte-identical repeat is the next check |

## Phase 2 — in-game editor shell

Deliverable: a screen that opens inside a running world, draws the rail and a panel, and closes
restoring everything.

| Item | Status |
|---|---|
| `cinematix-editor` crate, `Screen` variant, key binding | done — 25 tests; `Screen::Cinematix`, F8, `/cinematix` |
| Editor lifecycle: open, close, cursor, input restoration | done — `game/interface/cinematix.rs`; verified headless |
| Icon rail | done — eight sections, drawn and hit-tested |
| Dock tree and panel widget | partial — dock logic exists in `cinematix-ui`; the inspector is a fixed column, not dockable |
| Overlay card, context menu | todo |
| Numeric scrubber | done — drawn with a track; drag with Shift ×10 and Ctrl ÷10 |
| Viewport: live world + editor camera | partial — the editor draws over the live world; its own camera control is the next slice |
| Timeline: ruler, playhead, key rows, transport | done — drawn, scrubbable, keys move in time |
| Channel list and key insertion/deletion | done — `Document`, `K`, `X`, `Delete`, rows |
| Save/load the film (`.cmx`) | todo — the document is in memory only; closing the screen keeps it |

Verified: `cargo test -p cinematix-editor` 25 pass; `cargo check -p horizon-client` clean. Headless:
F8 opened the screen (status `open: true, screen: "cinematix"`), the frame went to 48, a key was
inserted and drawn at the playhead, `ui` listed the eight rail buttons and the five sliders by id,
`screenshot` wrote 956x1032, Escape returned `screen: "playing"`, and typed `/cinematix status`
answered `Cinematix: frame 48.` in chat. A second session compared `state` before and after:
position, yaw, pitch and eye identical; `pointer_grabbed` true -> false -> true; `ticks` 30 -> 112
(the world kept running, as designed); the HUD is absent from the screenshot.


## Phase 3 — timeline and animation

| Item | Status |
|---|---|
| Channels, keys, interpolation, pure evaluation | done — `cinematix-anim`, 6 tests |
| Time axis, zoom, pan, snapping, hit testing | done — `cinematix-timeline`, 9 tests |
| Dope sheet, read-only | todo |
| Editing commands and undo | todo |
| Curve editor with Bezier handles | todo |

`clamp`/`Step`/`Linear`/`Smooth`/`Bezier` and quaternion slerp are implemented and tested. Cubic and
Hermite interpolation from the specification are **not** yet implemented as distinct modes; the
current set covers the practical cases and adding modes is a small, tested change to `cinematix-anim`.

## Phase 4 — scene and camera

| Item | Status |
|---|---|
| Scene graph, node kinds, rest transforms | done — `cinematix-scene` |
| Camera data (fov, focus, aperture) | done as data |
| Camera rigs: free, orbit, dolly, rail, spline | partial — free and spline exist in `studio/` |
| Constraints, shake, blending | todo |
| Gizmos, selection, path rendering | partial — `studio/path_render.rs` |
| Spline paths, Catmull-Rom, arc length | partial — `studio/spline.rs`; arc-length parameterisation todo |

## Phase 5 — assets and rigging

| Item | Status |
|---|---|
| glTF/OBJ/FBX import | todo — no such dependency exists in the tree yet |
| Skins, capes, emissive layers | todo — vanilla skin loading exists, reuse its decoding |
| Skeleton, FK, skinning | todo |
| IK, constraints, morph targets | todo |
| Steve and Alex rigs | todo |

Hard constraint: Horizon has `glam` and no model-import crate. `gltf`, `tobj` and `russimp` must be
added deliberately, with FBX behind a feature, and the choice recorded here when made.

## Phase 6 — lighting and rendering

| Item | Status |
|---|---|
| Light data model | done — `cinematix-scene` |
| Custom mesh pass | todo |
| Light buffer and a Cinematix shader pack | todo |
| Environment overrides (time, weather, fog, sky) | todo |
| Shadow and ray-traced integration | research, not promised; unverifiable on GTX 1080 |

## Phase 7 — world manipulation

| Item | Status |
|---|---|
| Override layer with time-activated ops | done — `WorldDeltaLayer`, indexed, tested |
| Insertion into chunk read path | todo |
| Dirty-set driven remesh | todo |
| Animated structures | todo |

## Phase 8 — offline rendering

| Item | Status |
|---|---|
| Job, range, subframe sampling, ffmpeg arguments | done — `cinematix-render`, 9 tests |
| Codec matrix incl. alpha | done |
| Arbitrary-resolution offscreen target | todo — current capture is surface-sized |
| Readback ring and encoder streaming | todo |
| Resume and cancellation | partial — progress model exists |
| Byte-identical repeatability | not proven |

## Phase 9 — project format

| Item | Status |
|---|---|
| Scene, channels and world ops are serialisable | done — serde derives |
| `.cmx` container, asset dedupe, migration | todo |

## Blockers and risks

| Risk | Impact | Mitigation |
|---|---|---|
| Upstream moves fast — 554 commits touched the game loop in 30 days | integration drift | keep the diff small, keep `horizon-patches.md`, re-audit before each sync |
| Capture is surface-sized | 1440p/4K delivery | build an explicit offscreen target early in Phase 8 |
| No hardware ray queries on the dev GPU | cannot validate `traced_*` packs | test on RTX hardware; do not claim support |
| Shader packs do not shade custom meshes | lighting mismatch for imported models | own forward pass with shared sun/exposure, then own pack |
| UI kit has no dock/panel/tree/curve widgets | Phase 2 is genuinely new code | pure logic in `cinematix-ui`, drawing isolated in `cinematix-editor` |
| First deterministic sequence attempt can be rejected | export flakiness | treat guards as retryable preconditions |
