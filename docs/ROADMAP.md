# Cinematix phased roadmap

## Phase 1: audit and baseline — current

Complete:

- Imported Horizon source history at revision `5ef7734fdd854add5efd6817178be3c988d8d050`
- Reconciled root agent guidance and destination brief
- Audited upstream architecture, renderer, UI, HDR and Replay Studio
- Audited BBS-FS revision `13bd62dab1980b974b8ea0f78d037e8e82684b84`
- Documented existing, reusable, missing and blocked capabilities

Phase 1 is complete only when its source-backed documentation and baseline build checks are
committed.

## Capability matrix

| Capability | Horizon status | Cinematix treatment |
|---|---|---|
| Replay recording/playback | Existing platform implementation | Reuse and extend |
| Replay Studio free camera | Existing Horizon extra | Reuse for independent viewport |
| Replay camera path/timeline/top bar | Existing | Reuse; add scene typing later |
| HORIZON_STILL/screenshot sequence | Existing | Reuse for deterministic capture |
| Render capture/readback | Existing partial scene capture | Extend only for real scene needs |
| Dynamic lights | Existing | Integrate through renderer APIs |
| Native UI/HUD/screen | Existing | First implementation in these systems |
| Dockable floating editor panels | BBS-FS reference only | Recreate in Rust |
| Typed multichannel scene timeline | New | Phase 3 |
| Scene hierarchy/gizmos/constraints | New | Phase 4 |
| Model import/skinning/IK | Partial/unknown | Audit then Phase 5 |
| World override save isolation | New | Phase 7 |
| Offline image/FFmpeg export | New | Phase 8 |
| `.cmx` project format | New | Phase 9 |

## Phase 2: smallest native editor slice

Not yet implemented. A safe first slice must:

- open from an existing menu entry and a configurable shortcut
- retain session/editor state
- capture and restore mouse/focus/gameplay input correctly
- reuse the existing Replay Studio free camera and timeline
- keep player state independent
- use the real renderer/view
- provide a reusable native workspace component
- include lifecycle/input restoration behavioral tests

Do not begin disconnected UI or timeline-only prototypes.
