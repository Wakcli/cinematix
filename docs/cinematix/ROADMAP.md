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
| BBS-FS inspection | done | [UI.md](UI.md); both projects are MIT, so the icon sheet and behaviour are reproduced with [ATTRIBUTION](ATTRIBUTION.md) |
| Integration points identified | done | [ARCHITECTURE.md](ARCHITECTURE.md) §3 |
| Documentation and agent instructions | done | `AGENTS.md`, `docs/cinematix/` |
| Deterministic repeat runs proven | done | two static renders produced identical SHA-256 per frame (Phase 8) |

## Phase 2 — in-game editor shell

Deliverable: a screen that opens inside a running world, draws the rail and a panel, and closes
restoring everything.

| Item | Status |
|---|---|
| `cinematix-editor` crate, `Screen` variant, key binding | done — 25 tests; `Screen::Cinematix`, F8, `/cinematix` |
| Editor lifecycle: open, close, cursor, input restoration | done — `game/interface/cinematix.rs`; verified headless |
| Icon rail | done — eight sections with BBS mod's icon sheet (MIT, [ATTRIBUTION](ATTRIBUTION.md)); flat hover/active, tooltips, registered |
| Dock tree and panel widget | done — inspector/viewport/timeline split with draggable resizers and BBS panel headers; the inspector tears out into a floating panel by its header, moves, is swallowed over the timeline, and docks again by its button or at the right edge. The inspector has Properties and Scene tabs |
| Overlay card, context menu | partial — right-click menus on rows/keys/lanes with add/delete/interpolation/fit; mute, rename and paste are drawn dimmed and inert; no overlay card yet |
| Top toolbar (save/undo/redo/render) | done — Save writes the default `.cmx`, Undo/Redo drive the history, Render starts an MP4 of the film range at the document's resolution and fps (needs `--offscreen` and a still world, and says why otherwise) |
| Bottom dock tabs (Dope Sheet / Curve Editor / Clips) | done — Dope Sheet, Curve Editor and Clips; Clips is a shot list over the one timeline, Add clip takes the visible range, clicking a clip seeks and zooms to it, x deletes, both undo |
| Numeric scrubber | done — bounded fill bars and drag-number boxes from the supplied primitive kit; drag with Shift ×10 and Ctrl ÷10; camera Position and Yaw/Pitch vector rows |
| Viewport: live world + editor camera | done — the film camera renders while the screen is open; WASD/E/Q fly, right-drag look, FOV live |
| Timeline: ruler, playhead, key rows, transport | done — drawn, scrubbable, keys move in time |
| Channel list and key insertion/deletion | done — `Document`, `K`, `X`, `Delete`, rows |
| Save/load the film (`.cmx`) | done — zip with `project.toml` + `scene.json`; `cinematix save [PATH]`, `load PATH`, `new`; fixture tests for newer formats and bad targets |

Verified: `cargo test -p cinematix-editor` 25 pass; `cargo check -p horizon-client` clean. Headless:
F8 opened the screen (status `open: true, screen: "cinematix"`), the frame went to 48, a key was
inserted and drawn at the playhead, `ui` listed the eight rail buttons and the five sliders by id,
`screenshot` wrote 956x1032, Escape returned `screen: "playing"`, and typed `/cinematix status`
answered `Cinematix: frame 48.` in chat. A second session compared `state` before and after:
position, yaw, pitch and eye identical; `pointer_grabbed` true -> false -> true; `ticks` 30 -> 112
(the world kept running, as designed); the HUD is absent from the screenshot.

Camera slice, verified headless: on opening, `cinematix status` reported the camera at the player's
eye (`-4.5 66.615 6.5`, yaw 0, pitch 0, fov 70); holding W for one second moved it to z 14.57
(8.07 m at 8 m/s); `cinematix camera player` put it back at z 6.5; `set field-of-view 0.5` moved
the registered slider and `cinematix status` then reported fov 90, with the screenshot visibly
wider. The player's eye, yaw and pitch were identical before and after the session.

Spline keys, verified headless: camera keys at frames 0/48/96 at (0,64,0), (10,64,0), (10,64,10);
scrubbing reported exactly those poses at the keys, (5.625, 64, -0.625) at frame 24 and
(10.625, 64, 4.375) at frame 72, the values the Catmull-Rom formula gives (a straight line would be
(5,64,0) and (10,64,5)). Playing for about a second advanced to frame 24 with the camera at
(5.719, 64, -0.635); `pause` stopped it.

Rotation keys were checked separately with the position still: `cinematix channel rotation`, a key
at frame 0 (yaw 0) and a pose at frame 48 (yaw 90) gave yaw 0, 14.0625, 45, 75.9375 and 90 at frames
0, 12, 24, 36 and 48 — slerp under the `Smooth` easing, 4 keys total.

Film file, verified headless across two game processes: `cinematix save /tmp/session.cmx` wrote a
918-byte zip (`project.toml` format 1, `app_version 0.1.0`, `scene.json` with 10 channels and one
key); a second world loaded it, reported one key and the camera at the saved pose
(`8.5 72.619 8.5`); `cinematix new` reset to zero keys and adopted the new player's eye
(`72.5 68.62 41.5`); loading and saving again produced a 913-byte zip. `cinematix save` with no
path wrote to `<player dir>/cinematix/film.cmx`.


## Phase 3 — timeline and animation

| Item | Status |
|---|---|
| Channels, keys, interpolation, pure evaluation | done — `cinematix-anim`, 10 tests |
| Time axis, zoom, pan, snapping, hit testing | done — `cinematix-timeline`, 9 tests |
| Dope sheet editing | done — rows, keys, selection, drag in time, add/delete, per-key interpolation |
| Editing commands and undo | done — add/delete/move/interp, 64-step undo/redo (toolbar, Ctrl+Z/Y); live camera looks are not snapshotted |
| Curve editor with Bezier handles | done — curve view per channel with X/Y/Z/W axes, vertical key editing, and both Bezier control points draggable in time and value; Bezier is offered in the interpolation menu |
| Tree view (scene hierarchy) | done — disclosure, indentation, kind icons, selection |
| Viewport camera path overlay | done — yellow frustum/cross markers at the surrounding keys, the current segment and neighbours, and a green forward line plus cross at the playback pose; hidden during an export |

`Step`/`Linear`/`Smooth`/`Spline`/`Bezier` and quaternion slerp are implemented and tested.
`Spline` is centripetal Catmull-Rom through the neighbouring keys, with the keys' real times as the
knots — the same parameterisation as Flashback's SMOOTH mode. It keeps the path through a corner
inside the chord instead of the uniform formula's overshoot, and a two-key film stays the straight
segment. The film camera's position therefore follows a curve through its keys, not three
independent eased axes. Cubic and Hermite as *distinct* modes are still
absent; `Smooth` and `Bezier` cover the practical cases.

## Phase 4 — scene and camera

| Item | Status |
|---|---|
| Scene graph, node kinds, rest transforms | done — `cinematix-scene` |
| Camera data (fov, focus, aperture) | done as data |
| Camera rigs: free, orbit, dolly, rail, spline | partial — preview and free fly; `camera orbit` (centre, radius, frames, height), `camera dolly` (delta, frames) and `camera rail` (frames, points) generate Spline keys and look-at rotations, from chat and ctl; constraints and blending are todo |
| Constraints, shake, blending | partial — deterministic time-based camera shake (`Shake`, `Shake rate`) applied in the preview and therefore every rendered frame; constraints and blending are todo |
| Gizmos, selection, path rendering | partial — Studio's `studio/path_render.rs`, and Cinematix's own overlay: the film camera's current segment and one key either side, key frustums and crosses, and the green playback marker, drawn on top of the world |
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
| Environment overrides (time, weather, fog, sky) | done — the World section and `cinematix env` set time, rain, thunder, fog distances, sky and fog colours as film data; `view_day_time`/`view_weather` and the sampled sky read them while the editor is open or rendering, so the world, its save and PvP's own view stay untouched |
| Shadow and ray-traced integration | research, not promised; unverifiable on GTX 1080 |

## Phase 7 — world manipulation

| Item | Status |
|---|---|
| Override layer with time-activated ops | done — `WorldDeltaLayer`, indexed, 3 tests |
| Insertion into chunk read path | done — `Overrides` snapshot written into the mesher's padded block volume |
| Dirty-set driven remesh | done — the playhead diff marks each changed position's 3x3x3 sections |
| Animated structures | partial — time-activated single blocks and `world box` regions work; light propagation is not yet recomputed for overridden blocks |

Verified headless (fresh world, seed 41, camera at 0.5 120 0.5 looking +Z, gold at 0 120 5 active
from 1.0 s): frame 0 and frame 23 (t = 0.958 s) are byte-identical (SHA-256 `a2b7c5…`), frame 24
(t = 1.0 s) differs (`e143cf…`) and reads gold at the probe pixels; the render took 31 ms because
it waited two redraws for the remesh. Quitting and relaunching the same world with no layer
(`ops: 0`) and the same camera produced frame 0 with hash `a2b7c5…` again - the block never reached
the save. Region-file hashes change across a save because the server rewrites them; the reload
frame, not the file bytes, is what shows the saved blocks are unchanged.

## Phase 8 — offline rendering

| Item | Status |
|---|---|
| Job, range, subframe sampling, ffmpeg arguments | done — `cinematix-render`, 9 tests |
| Codec matrix incl. alpha | done |
| `cinematix render START END FPS WIDTH HEIGHT PATH` in chat and ctl | done |
| Arbitrary-resolution offscreen target | done — `Renderer::resize` retargets the offscreen frame; 320x180, 1920x1080 and 3840x2160 verified |
| Readback ring and encoder streaming | partial — one synchronous readback per frame; FFmpeg encodes the finished PNG sequence |
| Resume and cancellation | done — `cinematix render cancel` aborts with the frames kept; a job started with `--resume` skips frames already on disk, reports `resumed`, and `cinematix status` carries the job's start/end/next and output path |
| Byte-identical repeatability | proven — two runs of a static film produced identical SHA-256 per frame |

Verified headless (HORIZON_STILL=1, `--offscreen`, chunk fade 0): `cinematix render 0 6 24 320 180`
wrote 6 PNGs at 320x180, each 83 241 bytes; a second run to another folder was byte-identical
(equal SHA-256 for all three frames tested). 1920x1080 wrote two 747 831-byte frames, 3840x2160 one
2 272 269-byte frame, all with the surface still 956x1032 afterwards; `render ... /tmp/x.mp4`
produced a 320x180 24 fps 6-frame H.264 file (ffprobe). With the editor closed the command refuses,
and a world with 984 dirty sections is refused with that count instead of writing half a film.

## Phase 9 — project format

| Item | Status |
|---|---|
| Scene, channels and world ops are serialisable | done — serde derives |
| `.cmx` container, asset dedupe, migration | partial — zip with `project.toml` + `scene.json`, format 1 upcast and newer-version refusal; asset blobs deduped by sha256 are still todo because import does not exist yet |

## Blockers and risks

| Risk | Impact | Mitigation |
|---|---|---|
| Upstream moves fast — 554 commits touched the game loop in 30 days | integration drift | keep the diff small, keep `horizon-patches.md`, re-audit before each sync |
| Capture is surface-sized | 1440p/4K delivery | build an explicit offscreen target early in Phase 8 |
| No hardware ray queries on the dev GPU | cannot validate `traced_*` packs | test on RTX hardware; do not claim support |
| Shader packs do not shade custom meshes | lighting mismatch for imported models | own forward pass with shared sun/exposure, then own pack |
| UI kit has no dock/panel/tree/curve widgets | Phase 2 is genuinely new code | pure logic in `cinematix-ui`, drawing isolated in `cinematix-editor` |
| First deterministic sequence attempt can be rejected | export flakiness | treat guards as retryable preconditions |
