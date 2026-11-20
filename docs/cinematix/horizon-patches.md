# Upstream patches

Every edit to a file that came from Horizon is listed here with its size. This file is the upgrade
checklist: when the baseline moves, each entry is a place where a merge can conflict or silently
change behaviour.

Baseline: `1664a35852`, 2026-11-19. Imported 2026-11-20.

## Phase 1 — import

| File | Change | Lines |
|---|---|---|
| `Cargo.toml` | six `crates/cinematix-*` entries added to `members` | +6 |
| `.gitignore` | unchanged | 0 |

## Phase 2 — the editor screen

The editor's own code lives in `crates/cinematix-editor`; the Horizon tree only gains the screen
slot and the dispatch that reaches it. Every hook is one arm in an existing `match`, so an
upstream merge shows up as a conflict on that arm, not as silent drift.

| File | Change | Lines |
|---|---|---|
| `crates/horizon-client/src/game.rs` | `Screen::Cinematix` variant, id, `Game` fields, draw arm, HUD gate, cursor/wheel/key/tick hooks, camera orientation and effects | +35 −2 |
| `crates/horizon-client/src/game/interface/cinematix.rs` | new module: lifecycle, draw, input, camera, `.cmx` save/load, `/cinematix`, ctl | +488 (new) |
| `crates/horizon-client/src/game/interface/screens.rs` | `is_pause_screen`, `blurs_background` arms and their tests | +7 −4 |
| `crates/horizon-client/src/game/interface/perspective.rs` | `camera_view` returns the editor's preview pose | +5 |
| `crates/horizon-client/src/game/interface/fov.rs` | `fov_degrees` returns the preview field of view | +3 |
| `crates/horizon-client/src/game/interface/frame_geometry.rs` | the detached camera draws no hand | +1 −1 |
| `crates/horizon-client/src/game/interface/play_keys.rs` | F8 opens; Escape closes; keys stop at the editor | +9 |
| `crates/horizon-client/src/game/interface/chat.rs` | `/cinematix` suggestion grammar | +74 −1 |
| `crates/horizon-client/src/game/interface/debug_options.rs` | F3+F6 does not interrupt the editor | +1 |
| `crates/horizon-client/src/game/runtime/control.rs` | `cinematix` ctl dispatch, HELP, screen name; F7-F10 in the `key` parser | +11 |
| `crates/horizon-client/src/game/runtime/agent/mod.rs` | screen name | +1 |
| `crates/horizon-client/src/game/runtime/core_tests.rs` | the screen-id list and its uniqueness test | +1 |
| `crates/horizon-client/src/game/network/remote/mod.rs` | `/cinematix` reaches `cinematix_command` | +3 |
| `crates/horizon-client/src/bin/horizon-ctl/commands.rs` | the `cinematix` command in `horizon-ctl help` | +5 |
| `crates/horizon-client-input/src/keybinds.rs` | `key.cinematix` = F8 (`misc`) | +2 |
| `crates/horizon-client-input/src/actions.rs` | `Cinematix` action | +1 |
| `crates/horizon-client/Cargo.toml` | `cinematix-editor` dependency | +1 |
| `crates/horizon-client/assets/horizon/lang/en_us.json` | 68 `cinematix.*` / `horizon.cinematix.*` / `key.cinematix` strings | +69 −1 |
| `tools/layers.json` | six `cinematix-*` crates at `engine` | +10 −2 |

Deliberate deviations from the rules above:

- **No cargo feature.** Cinematix is not an optional extra in this fork: the screen, the key and
  the editor crate are always compiled. A feature would hide the code from `horizon-plain` builds
  and add a second configuration to test for no gain. The dependency is one line, so an upstream
  merge sees it immediately.
- **`chat.rs` and `en_us.json` exceed 50 lines.** One is a command grammar, the other is string
  data; both are append-only blocks, and splitting them would not make a merge any cleaner.

To move to a new Horizon baseline: take upstream on the files above (the new module and the six
crates are ours and merge without conflict), then re-add each arm. `cargo check -p horizon-client`
is the check: a lost arm fails to compile when the enum gains no variant, so also run
`cargo test -p horizon-client --lib core_tests` — the screen-id uniqueness test fails if a variant
is added without an id.
