# Cinematix UI

The editor is an in-game screen drawn with Horizon's UI kit. This document maps the BBS-FS
interaction model onto the widgets that actually exist, and states what must be written.

[BBS-FS](https://github.com/Wemppy4/bbs-fs) is the visual and behavioural reference. Both it and
McLib (`mchorse/mclib`) are MIT licensed; the icon sheet is copied with the notice in
[ATTRIBUTION.md](ATTRIBUTION.md), and the Rust editor reuses the layout, colours and interaction
rules (the Java itself is not compiled or shipped). See `ui/dashboard` and `ui/utils/icons` on the
`dev` branch for the reference the current work follows.

## 0. The control language

[`UIprim.svg`](UIprim.svg) is the supplied primitive sheet: a bounded fill bar, a bipolar
centre-zero bar, a drag-number box, a three-axis vector row, a linked dimension pair, stepped
presets, a dual-handle range track, a cycle/time gradient, a colour swatch with hex, and the
checkbox/keyframe-rail states. The editor's inspector and timeline use it directly:

| Primitive | Used by |
|---|---|
| Bounded fill bar, drag-number box | every scalar `Field` in the inspector |
| Bipolar centre-zero | camera Yaw/Pitch rows |
| Three-axis vector row | camera Position X/Y/Z and Rotation Yaw/Pitch |
| Linked dimension pair | render Width/Height, with the chain toggle |
| Stepped presets | render FPS 24/30/60/120 |
| Dual-handle range track | the selected clip's start/end in the Clips tab |
| Cycle/time gradient | the World section's Time |
| Colour swatch + hex | World Sky and Fog colours |
| Checkbox | the transport's Loop and Free view |
| Keyframe rail states | the dope-sheet row gutter: unkeyed, animated, keyed at the playhead |

## 1. What Horizon already gives us

Verified in [AUDIT.md](AUDIT.md). Do not reimplement any of this.

| Piece | Source | Use in the editor |
|---|---|---|
| `HudBatch` | `hud.rs:1692` | every quad, sprite and glyph |
| `HudAtlas` | `hud.rs:72` | sprites, glyphs, item and block icons |
| `ui::nine_slice` | `ui.rs:167` | panel and button frames from `gui.scaling` metadata |
| `ui::button`, `action_button` | `ui.rs:546,564` | all clickable controls; **these register** for controller and agent control |
| `ui::slider`, `slider_ext` | `ui.rs:707,726` | bounded numeric input with a track |
| `ui::EditBox` | `ui.rs:1267` | text entry with caret, selection and scrolling |
| `ui::Scroll`, `scrollbar` | `ui.rs:935,995` | list and panel scrolling |
| `ui::tooltip*` | `ui.rs:1052-1267` | hover text, focused and under-pointer variants |
| `ui::menu_background`, `separators`, `list_background` | `ui.rs:822,843,883` | chrome |
| `hud_kit::plate::Plate` | `hud_kit/plate.rs` | panel body and backing |
| `hud_kit::room::Room` | `hud_kit/room.rs` | usable area, keep-outs |
| `hud_kit::shapes` | `hud_kit/shapes.rs` | arcs, discs, fans, triangles, radial gradients — curves, gizmo rings, light falloff |
| `hud_kit::tabs`, `hud_kit::hint`, `hud_kit::fit` | `hud_kit/` | section tabs, key hints, ellipsize and wrap |
| `hud_kit::motion::Tween` | `hud_kit/motion.rs` | quiet motion as a pure function of frame time |
| `graphics_style` | crate module | colour and spacing tokens |

Two rules from upstream, both non-negotiable:

- **Register interactive controls.** `ui::button` and `settings_shell::foot_button` register;
  `graphics_style::button` and `screen_draw::button` only paint. A painted-only control is dead to
  controllers and to `horizon-ctl`.
- **No English literals.** Every visible string is a translation key.

## 2. What must be written

These do not exist anywhere in Horizon. They go in `cinematix-ui` (logic, no drawing) and
`cinematix-editor` (drawing, input, commands).

| Widget | Logic | Drawing |
|---|---|---|
| Dock tree (split, drag ratio, reset) | `cinematix-ui` — exists and is tested | frame + splitter grip |
| Panel (title, collapse, scroll, pin) | state machine | nine-slice header, plate body |
| Icon rail | selection + hit test | icon column, active marker, tooltip |
| Floating overlay card | placement + focus trap | dimmed backdrop, plate, title |
| Context menu | open/close, item hit test, keyboard nav | menu plate, separators, disabled rows |
| Numeric scrubber | drag/wheel/type, `shift`×10, `ctrl`×0.1, clamping — exists and is tested | field, unit suffix, optional track |
| Tree view (scene hierarchy) | flatten, expand, indent, select, rename | disclosure triangles, rows, guides |
| Timeline ruler | tick spacing by zoom, label thinning | ticks, labels, playhead |
| Dope sheet rows | visible-range culling, row layout, key hit test | diamonds, selection, group rows |
| Curve editor | value axis, segment sampling, handle hit test | sampled polylines, handles, grid |
| Colour field | hex parse/format, alpha | swatch, hex box, alpha bar |

The two pieces that are hardest and least forgiving are the tree view and the curve editor. Both
are pure hit-testing and layout over a `TimeAxis` or a value axis, which is why that logic lives in
`cinematix-ui`/`cinematix-timeline` where it can be unit-tested with no GPU.

## 3. Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│ header   scene · fps · frame      [play] [loop] [render]             │
├──────┬───────────────────────────────────────────────┬───────────────┤
│ rail │                                               │  inspector    │
│  ▣   │                  viewport                     │  transform    │
│  ▣   │                                               │  material     │
│  ▣   │                                               │  light        │
├──────┴───────────────────────────────────────────────┴───────────────┤
│ outliner │            timeline · dope sheet · curves                │
├──────────────────────────────────────────────────────────────────────┤
│ transport   frame · time · range · fps · snap · zoom                 │
└──────────────────────────────────────────────────────────────────────┘
```

Rules that make it feel like one application:

- The rail is always visible and is the only navigation. It switches the panel set, never the
  screen.
- The timeline is always visible and is the only way to change time.
- Panels float over the viewport and collapse to a title bar; the viewport is never permanently
  covered.
- Every panel is a pure function of `(scene, selection, time)` and emits commands. No panel mutates
  the scene.
- The layout is a `Dock` tree with ratios clamped to `0.05..0.95`, so a bad drag cannot destroy a
  panel.

## 4. Rail sections and their panels

| Section | Panels |
|---|---|
| Camera | lens (fov, focus, aperture, roll), rig type (free/orbit/dolly/handheld), shake, path list |
| Actors | rig picker, bone tree, morph sliders, skin and cape, action library |
| Models | imported assets, material overrides, UV scale, primitives |
| Lights | light list, type parameters, colour temperature, shadows, environment overrides |
| World | block delta list, time of day, weather, fog, sky |
| Film | shots, clip sequencer, markers, notes |
| Render | resolution, fps, codec, range, samples, output path, progress |
| Project | save mode, assets, migration warnings, metadata |

## 5. Timeline

Three stacked views over one time axis and one selection set:

1. **Dope sheet** — rows are channels grouped by node, keys are diamonds. Move, copy, delete,
   box-select, set interpolation from a context menu.
2. **Curve editor** — selected channels only, value axis with manual zoom, draggable Bezier handles.
3. **Clip sequencer** — shots as blocks with duration, camera binding and render state.

Shared behaviour:

- One playhead, one range, one selection (`NodeId` + channel index).
- Zoom by `Z`/`X` or ctrl+scroll; vertical zoom only in the curve editor.
- Snap to frames, keys, markers and range ends; hold `Alt` to suspend.
- Every edit is a command on an undo stack, coalesced per drag.
- A channel with no keys draws as a dimmed rest-state row, so users see what *can* be animated.

Rendering is custom painting in one region, with visible-range culling and a per-frame layout cache.
Target: 1 000 channels and 20 000 keys must still scroll smoothly.

The interaction shape is not invented: `studio/timeline_ui.rs` already does
`render`, `handle_mouse_down`, `handle_mouse_drag`, `handle_mouse_up -> Action`. Follow it.

## 6. Interaction conventions

- **Left drag on a value scrubs. Left click without drag edits text.** This is the single most
  important convention and must be identical on every numeric field.
- `Space` play/pause, `K` key at playhead, `I`/`O` set range, `F` frame selection,
  `Delete` remove keys, `Ctrl+Z`/`Ctrl+Y` undo/redo, `Tab` cycle panel focus.
- Right-click always opens a context menu, never a properties dialog.
- Nothing is modal except destructive confirmation and file pickers.
- Escape closes an overlay before it closes the editor.

## 7. Editing mode and input

Opening the editor pushes a screen, which is how Horizon already suppresses gameplay input. The
editor must additionally:

- release or capture the cursor deliberately, and restore the previous capture on close;
- keep the editor camera independent of the player, restoring the player's view exactly;
- leave the world running or frozen per a setting, with the default frozen during authoring.

## 8. Milestones

1. Editor screen opens and closes on a binding, draws the rail and one plate panel, and restores
   input. No scene yet.
2. Dope sheet, read-only, over a fixture scene: ruler, rows, playhead, zoom, pan.
3. Selection and editing commands with undo, then the curve editor.
4. Outliner tree, inspector fields, numeric scrubbers wired to the scene.
5. Remaining rail sections, one at a time, each with a test.

The render spine comes first only in the sense that the editor must never be the reason a frame
cannot be produced. A beautiful timeline over a renderer that cannot render is the standard failure
mode for projects of this kind.
