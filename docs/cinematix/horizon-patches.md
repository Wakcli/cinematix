# Upstream patches

Every edit to a file that came from Horizon is listed here with its size. This file is the upgrade
checklist: when the baseline moves, each entry is a place where a merge can conflict or silently
change behaviour.

Baseline: `1664a35852`, 2026-11-19. Imported 2026-11-20.

| File | Change | Lines |
|---|---|---|
| `Cargo.toml` | five `crates/cinematix-*` entries added to `members` | +5 |
| `.gitignore` | unchanged | 0 |

Nothing else in the Horizon tree has been modified. New behaviour lives in `crates/cinematix-*`
until Phase 2 needs a `Screen` variant, a key binding and an input hook.

When those land they are recorded here as, for example:

| File | Change | Lines |
|---|---|---|
| `crates/horizon-client/src/game.rs` | `Screen::Cinematix` variant dispatch; one match arm | ~8 |
| `crates/horizon-client-input/src/keybinds.rs` | one binding row | 1 |
| `crates/horizon-client/Cargo.toml` | optional `cinematix-editor` dependency behind a feature | 2 |

Rules for additions to this file:

- A patch stays under roughly 50 lines. If it grows past that, the integration point is wrong and
  the design should change instead.
- New behaviour goes behind a cargo feature so an upstream merge is mechanical.
- Every patch has a test or a harness that fails loudly if the merge moves the hook.
