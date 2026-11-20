# Horizon audit

Evidence for every integration decision in this repository. Reading order: this file, then
[ARCHITECTURE.md](ARCHITECTURE.md), then [UI.md](UI.md).

Anything marked **verified** was executed on this machine. Anything marked **read** was established
by reading the named code. Nothing here is inferred from documentation alone.

## Baseline

| | |
|---|---|
| Upstream | `horizon-rust/horizon` |
| Revision | `1664a35852` — "bug(launcher): a downloaded launcher update installs when the window closes (#18629)" |
| Date | 2026-11-19 23:33 +0100 |
| Branch | `main` |
| Imported | 2026-11-20, full tree minus `.git/`, `target/`, `.local/` |
| Size | 208 MB, 11 577 files, 80 workspace crates |
| Toolchain | edition 2024, `rust-version` 1.95, rustc 1.99.1 |
| Licence | MIT (`LICENSE`); `LICENSES/` retained |

`.local/` holds prepared Minecraft resources and is ignored by `.gitignore` line 1. No Mojang file
is in this repository: `python3 tools/check_public_tree.py` passes.

## Build and boot (verified)

| Step | Command | Result |
|---|---|---|
| Release build | `cargo build --release -p horizon-game -p horizon-client-lib --bins` | clean, **18 m 44 s** |
| Artifacts | | `horizon-client` 168.6 MB, `horizon-ctl` 1.1 MB, `horizon-prepare` |
| Fast build | `cargo build --profile fast -p horizon-game` | documented in upstream `AGENTS.md`; use this for iteration |
| Prepared resources | `horizon-prepare --prepare --minecraft-dir ~/.minecraft --server-jar <26.3 server.jar>` | **15.1 s**, writes `.local/horizon-client/26.3` |
| Sounds | `horizon-prepare sounds --minecraft-dir ~/.minecraft` | 4 962 files, 377.3 MB |
| Resource check | `horizon-prepare --check` | exit 0 |
| Boot | `horizon-client --test-terrain --frames 5` | Vulkan, GTX 1080, first frame **463 ms** |

### Resource preparation has three traps

1. **The version JSON must be Mojang's own bytes.** `horizon-prepare` checks the SHA-1 of
   `versions/26.3/26.3.json` against the pin in `research-lock.json`. Re-serialising the JSON — even
   with identical content — changes its bytes and fails with
   `... is not a 26.3 version file Horizon knows`. Fetch the file raw.
2. **26.3 is the release the pin expects.** `minecraft.version` is `26.3`; `accepted_manifest_sha1`
   lists eight republished manifests that differ only in `assetIndex`, so several launchers work.
3. **`--prepare` needs the server jar**, not just the client install, because the Java exporters run
   against the server.

## Headless operation (verified)

`--offscreen` renders into a texture instead of a window
(`crates/horizon-client/src/game/runtime/launch_options.rs:41,264`). It is **not listed in `--help`**.

It still needs a display server, because winit creates an event loop before the renderer decides
where to draw:

```
os error at winit-0.30.13/src/platform_impl/linux/mod.rs:765:
neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set.
```

The working recipe, confirmed booting to first frame in **305 ms** with no window on the physical
display, is a virtual X server:

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" \
  ./target/release/horizon-client --test-terrain --frames 2 --offscreen
```

Driving the game, including `--offscreen`, is `horizon-ctl`:

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" bash -c '
  ./target/release/horizon-ctl --socket /tmp/ctl.sock launch \
    --world SmokeTest --seed 1 --offscreen --wait \
    --resources $PWD/.local/horizon-client/26.3'
```

## Deterministic frames (partially verified)

`horizon-ctl screenshot-sequence FRAMES STEP_MS FILE` produced a 629 605-byte PNG at 956×1032 with
`HORIZON_STILL=1`. Its guards live in
`crates/horizon-client/src/game/runtime/screenshot_sequence.rs:55-110` and reject the run unless all
of the following hold:

- `HORIZON_STILL=1`, and the world frozen — not stepping, not sprinting
- no title, no loading entry, `Screen::Playing`, a player present
- chunks loaded, `streaming.pending() == 0`, no pending uploads
- `dirty_light` and `dirty_sections` empty
- no LOD tiles and far-terrain distance 0
- `chunkSectionFadeInTime == 0` (default 0.75; set with
  `horizon-ctl options-set chunkSectionFadeInTime 0`)
- shadow cache off and the pack's own guards satisfied

Two consequences for the export pipeline, discovered by running it:

1. **A sequence can still be rejected on the first attempt** with
   `the camera, world, settings, shader pack or window size changed while it ran`; an immediate
   retry succeeds. The render queue must treat this as a retryable precondition, not a fatal error.
2. **Output is the surface size**, not a requested resolution. Arbitrary 4K/1440p export is *not*
   demonstrated by this path and needs an explicit offscreen target — see
   [ARCHITECTURE.md](ARCHITECTURE.md).

Byte-identical repeatability across two *successful* runs was not captured in this session (the
first attempt was rejected by the guard above); it is the next runtime check, not a claim.

## UI framework (read)

There is no `horizon-ui` crate. The UI is a vanilla-style immediate-mode widget layer inside
`horizon-client-render`:

| Layer | File | Contents |
|---|---|---|
| Batch | `crates/horizon-client-render/src/hud.rs` | `HudAtlas` (72), `HudBatch` (1692), `GuiScale` (1686), `quad` (1865), `quad_3d` (1897), `sprite` (1977), `sprite_scaled` (1982), `rect_uv` (1970), `centered_origin` (1859), `depth_since` (1934), `blend_since` (1786), atlas decode (168, 275, 292) |
| Widgets | `crates/horizon-client-render/src/ui.rs` (2 893 lines) | `nine_slice` (167), `blit_sprite` (191), `text_scaled` (347), `hovered` (362), `centered_text` (375), `button` (546), `action_button` (564), `button_as` (598), `slider` (707), `menu_background` (822), `separators` (843), `list_background` (883), `Scroll` (935), `scrollbar` (995), `tooltip` family (1052-1267), `EditBox` (1267) with `Caret` (1307), `append_field` (1450), `delete_words` (1490) |
| Layout kit | `crates/horizon-client-render/src/hud_kit/` | `room` (`Room`, `Keep`), `plate` (`Plate`), `shapes` (triangles, fans, arcs, discs, radial gradients, `alpha_since`, `translate_since`), `motion` (`Clock`, `Tween`), `icon` (`Icon`, `IconPainter`), `fit` (`ellipsize`, `wrap`, `chord`, `fit_in_circle`), `tabs` (`collapsed`, `tab_strip`), `hint` (`HintRow`) |
| Screen shell | `settings_shell.rs` | `Shell`: a searchable settings screen described by data — the precedent for a large described screen |
| Style | `graphics_style` | tokens used as `graphics_style as style` |

Two upstream rules that constrain every widget we add, from `AGENTS.md`:

- **Controls must register.** Draw interactive controls with `ui::button` or
  `settings_shell::foot_button`. `graphics_style::button` and `screen_draw::button` only paint;
  using them loses controller and agent-control reach.
- **No hardcoded user-facing strings.** Use translation keys (`Text::tr`, `lang::get`).

## Screens and input (read)

- `struct Game` holds the active screen: `screen: Screen` at `crates/horizon-client/src/game.rs:1147`.
  Screens are large enums with their own state variants (for example
  `crates/horizon-client/src/multiplayer.rs:98`), drawn and clicked per variant.
- Key bindings are a table of `(translation key, default binding, category)` in
  `crates/horizon-client-input/src/keybinds.rs:19-54`, with `KeyBinds` at line 377 and `KeyAction`
  at `keymapping.rs:157`.
- Player input is consumed in gameplay (`game/play_keys.rs`, `game/play_input.rs`), with mouse look
  in `game/pointer_capture.rs`.
- `game/control.rs`, `game/agent.rs`, `game/events.rs` and `game/ui_control.rs` expose the game to
  `horizon-ctl` and MCP, including driving UI controls by name. This is how editor screens will be
  tested without a human.

## Camera (read)

Two math worlds, and the boundary between them matters:

- `crates/horizon-client-render/src/camera.rs` (1 304 lines) mirrors JOML to stay bit-compatible with
  vanilla: its own `Quat` (`rotation_yxz` at 116 — Minecraft's YXZ order), `Mat4`
  (`translation`, `rotate_x/y/z`, `affine`, `to_glam` at 454), `view_rotation_matrix` (465),
  `view_matrix` (480), `project_point` (491), plus `java_min`, `java_max`, `lerp_d`, `clamp`.
- Cinematography code should use `glam` (`DVec3`, `Quat`) and convert through `to_glam`/`affine`
  at the renderer boundary, never re-derive the Euler convention.

## Existing studio code (read)

`crates/horizon-client/src/studio/` is Horizon's in-game video studio, and it already implements
part of what this project needs:

| File | Lines | What it provides |
|---|---|---|
| `camera.rs` | 211 | `StudioFreecam`: `activate`, `deactivate`, `turn`, `adjust_speed`, `update_flight_motion`, `update_spline_playback`, `add_keyframe_at_current` |
| `spline.rs` | 320 | `CameraKeyframe`, `CameraTrack`, `EvaluatedPose`; centripetal Catmull-Rom positions, quaternion slerp rotation, `sample_path`; unit-tested |
| `timeline_ui.rs` | 942 | `TimelineUi` with `render`, `handle_mouse_down`, `handle_mouse_drag`, `handle_mouse_up -> TimelineAction`; scrubbing, keyframe insert/select/delete, markers, clip split |
| `topbar.rs` | 490 | `TopBarUi`, `TopBarAction` |
| `path_render.rs` | 138 | camera path visualisation |
| `recorder.rs`, `replay.rs`, `mcpr.rs` | 557 / 466 / 1 658 | `.mcpr` replay recording, recovery, playback |

**Reuse, do not rebuild.** Cinematix's camera evaluation is a superset of `CameraTrack`
(more channels, per-channel interpolation, constraints), and its timeline must follow the same
`render` + `handle_mouse_*` + action-enum shape. The replay recorder is the existing answer to
"record what happened", and the offline renderer should not duplicate it.

## Dependency facts (read)

- `Cargo.toml` defines **no** `[workspace.dependencies]`; crates declare versions individually.
  `glam = "0.30"` and `serde = { version = "1", features = ["derive"] }` are the ordinary forms.
- `cargo` does not compile a crate twice for different features, so adding
  `glam/serde` is a workspace-wide feature unification decision.
- `horizon-optifine` exists (508 KB) and is the layer under everything that reads OptiFine-format
  packs, including `cem/` custom entity models. It is not Iris.

## Unresolved

| Question | Why it matters | How to settle |
|---|---|---|
| Byte-identical repeat runs of `screenshot-sequence` | It is the core promise of offline rendering | Two successful sequences, compare SHA-1 |
| Rendering at an arbitrary resolution (1440p/4K) | Delivery formats in the spec | Find or add an offscreen target not tied to surface size |
| Whether custom geometry can join shader-pack shadow/post passes | Lighting quality | Read `gfx/pack_*` stage contracts; test with `traced_shadows` |
| GTX 1080 has no hardware ray queries | `traced_shadows`/`traced_water` cannot be validated here | Validate on RTX-class hardware; do not claim support |
| Exact `Screen` enum location for the client | Editor needs a variant | `grep -rn "enum Screen" crates/` during Phase 2 |
