# Native UI and input audit

## Native framework

The in-game UI is drawn through Horizon's `horizon-client-render` HUD and `horizon-client`
game screens, not the launcher GPUI application. The relevant systems are:

- HUD immediate-mode batching: `crates/horizon-client-render/src/hud.rs`
- HUD kit: `crates/horizon-client-render/src/hud_kit/`
- Settings shell: `crates/horizon-client-render/src/settings_shell.rs`
- Game screen state and rendering: `crates/horizon-client/src/game/interface/screens.rs`,
  `game.rs`, `native_client_screens.rs`
- Focus and pointer capture: `game/interface/pointer_capture.rs`, `key_focus.rs`,
  `play_input.rs`, `play_keys.rs`
- Replay Studio topbar/timeline: `crates/horizon-client/src/studio/topbar.rs`,
  `timeline_ui.rs`

## Existing Replay Studio UI

The Replay Studio already opens a native script screen, frees mouse capture, renders a top bar
and timeline, and provides free camera, keyframe, path lock and replay playback actions. Its
gameplay integration is `game/interface/studio.rs`; browser UI is
`horizon-client-render/src/studio_ui.rs`.

Phase 2 must extend or host the existing Studio rather than add another screen stack.

## Input capture

`open_studio_screen` clears movement flags and calls `capture(false)`; `close_studio_screen`
calls `capture(true)`. Free camera activation captures the pointer, while deactivation restores
normal gameplay capture. Editor lifecycle code must preserve and restore prior capture state,
because merely toggling a boolean can change a different screen or UI session.

No lifecycle behavioral test is present yet for a Cinematix session. Tests must be added with
the smallest Phase 2 implementation.

## BBS-FS behavior targets

BBS-FS provides the interaction reference:

- Vertical workspace icons and hierarchical replay/track editors
- Resizable split/dock layouts
- Tab stacks and floating/docked panels
- Numeric properties and keyframe property panels
- Timeline ruler, dope-sheet and graph views
- Keyframe multiselection and tangent editing
- Context menus and undoable layout/sheet actions

Cinematix should recreate these interactions in Horizon native widgets and not copy Java
classes or LWJGL rendering.
