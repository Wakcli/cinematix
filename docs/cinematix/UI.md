# BBS-FS interaction reference

## Scope and licensing

Reference revision: `13bd62dab1980b974b8ea0f78d037e8e82684b84` at
`https://github.com/Wemppy4/bbs-fs`.

This source inspection records behaviors to recreate in Horizon's native Rust
UI. It does not import BBS-FS's Java, Fabric, LWJGL, JOML runtime or assets.
No BBS-FS source or assets are copied by this contribution.

Root `LICENSE.md` carries MIT-style permission with copyright 2025 McHorse and
2026 Wemppy. Any later substantial code/asset reuse must retain the applicable
notices and add a third-party notice compatible with Horizon's LGPL license.
Inspect per-file/asset licensing too; a root license is not blanket provenance
for every bundled dependency.

Paths below are rooted at `src/client/java/mchorse/bbs_mod/ui/` unless a different
prefix is written. Revision-pinned URLs and digests are in
[sources.json](sources.json).

## Workspace structure, not a loose visual imitation

| Reference source | Observed behavior | Rust implementation acceptance target |
|---|---|---|
| `dashboard/UIDashboard.java` | Menu root contains panel manager and camera/orbit controls. Panel construction is queued for warmup rather than doing every expensive editor on the first key press. | Lazy panel/resource creation; no whole-suite allocation/update cost while closed. |
| `dashboard/panels/UIDashboardPanels.java` | 20-unit icon strip, pinned controls, scrollable panel buttons. Strip can dock bottom/top/left/right; **bottom is the reference default**, not a fixed vertical rail. Selected panel survives close; shown-state pairs appear/disappear. | User-requested vertical left rail as Cinematix's default, with one shared edge/orientation model. Closing preserves panel choice while unregistering active input. |
| `film/UIFilmPanel.java` | One shared `UIDockLayout` mounts edit area, main timeline, frameless preview, replay list and replay properties. Layout changes and settled layout notify preview sizing. | Native world viewport, hierarchy/inspector/timeline from one workspace; viewport projection resizes after layout settles. |
| `framework/elements/layout/UIDockLayout.java` | Registered panel IDs plus split/tree layout, tab/stack grouping, splitters, lock/reset, edge docking against panel or root, drag highlight and bounded layout undo. | Stable panel IDs and a single validated layout model; minimum sizes, hit testing and docking must agree with drawing. |
| `framework/elements/overlay/UIOverlay.java`, `UIOverlayPanel.java` | Floating overlay stack with retained size/offset; anchored corner-resize grip, constrained bounds, focus-sensitive closure. | Floating windows use the same panels/widgets and z-order input owner. Do not equate modal overlays with a confirmed arbitrary dock-to-floating conversion. |

The dock implementation's useful interaction parameters are evidence rather
than unconditional pixel requirements: 4-unit drag threshold, 80-unit minimum
panel size, 500 ms spring-load tab dwell, 300 ms splitter double-click and layout
undo cap of 32. Map them to Horizon GUI pixels and test at different GUI scales.
Do not create independent layout/input logic in each editor panel.

## Widgets, focus and gestures

`framework/elements/UIElement.java` keeps parent/children, area, layout resizers,
visibility/culling, focus and event propagation. Its own input handling and
context menus can consume an event; overlays offer top children first so
the resize grip does not leak clicks to covered content.

`framework/elements/input/UITrackpad.java` extends `UINumericInput`:

- Horizontal relative dragging changes a number using its current step; cursor
  wrapping keeps unbounded ranges reachable.
- Numeric value, range constraints and typed text editing live in the shared
  numeric base, not each property panel.
- A click can enter text focus; right-button release during drag restores the
  original value.
- Increment/decrement regions and configured wheel gestures share the same
  value/notification path.
- Drag start/end and change notification delimit editing transactions.

Recreate the relative scrub and cancel/commit semantics. Cursor warping is
platform-dependent: provide a supported relative-input alternative when winit
cannot warp; do not promise identical behavior on every window system.

Horizon already has `HudBatch`/atlas drawing, registering `ui::button`,
`action_button`, sliders, `Scroll`, `EditBox`/caret and HUD-kit primitives.
Adapt those, with one focus/pointer/keyboard owner. The launcher uses GPUI;
that does not make GPUI the running game's UI. Product-visible strings must use
Horizon translation keys. Draw helpers which only paint are not automatically
controller-focus registration.

## Timeline and curve editor

| Reference | Data/interaction relationship |
|---|---|
| `framework/elements/utils/UITimelineCanvas.java` | Shared time-axis canvas rather than a separate coordinate transform per timeline. |
| `framework/elements/input/keyframes/UIKeyframes.java` | Channel sheets, graph/dope-sheet mode, selection/drag/scrub state, context actions and copy/paste controller. Change callbacks update the data owner. Some bindings are explicitly scoped to the widget under the pointer so they do not steal another editor's shortcuts. |
| `framework/elements/input/keyframes/UIKeyframeSheet.java` | Sheet owns a reference to the actual channel and its selection. |
| `framework/elements/input/keyframes/graphs/UIKeyframeGraph.java` | Time/value mapping, key and Bézier-handle hit testing. Incoming/outgoing handle coordinates refer to the same key data used by interpolation. |
| `framework/elements/input/keyframes/graphs/UIKeyframeDopeSheet.java` | Alternate view of the channel/key selection, not a second animation store. |
| `src/main/java/mchorse/bbs_mod/utils/keyframes/KeyframeChannel.java` | Typed channel storage and evaluation; factories provide value/interpolation behavior. |

Cinematix's sequencer, curve editor and property inspector must edit **one**
typed animation backend. Copying keys into decorative graph state is not an
implementation. Undo transactions should cover a whole drag, not thousands of
mouse deltas.

Horizon's existing replay timeline is useful integration evidence but uses
replay transport and a camera-pose track. It is not this general backend.
Retain replay functionality and adapt its camera track without forking two
competing camera systems.

## First native slice and visual proof

The first implementation should open inside a loaded local world, own input,
show a reusable resizable workspace component and manipulate the real detached
camera while preserving player state. Disabled or absent advanced panels must
not masquerade as functional scene, light or export tools.

Before calling the slice complete, capture the actual Horizon editor with:
open/close, resize at multiple GUI scales, keyboard text focus, drag capture
outside the widget, right-click cancellation, overlays, focus loss, camera
movement and a world-exit cleanup case. Compilation alone does not establish
these interactions. None has been run by this documentation contribution.
