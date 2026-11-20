# Cinematix

Cinematix is a planned native cinematography, scene-building, animation, lighting and offline-rendering subsystem for [Horizon](https://github.com/horizon-rust/horizon), implemented in Rust inside the running singleplayer game.

## Current status

The Horizon baseline `1664a35852` (2026-11-19) is imported into this repository as of 2026-11-20,
with full provenance in [docs/cinematix/AUDIT.md](docs/cinematix/AUDIT.md). The engine builds, boots
on Vulkan, derives its resources from your own Minecraft install, and runs headless.

Implemented and tested: the scene model, animation channels and evaluation, timeline time-axis math
and export job construction — 30 tests that need no GPU, no game and no Minecraft install.

Not yet implemented: the in-game editor screen, the timeline and curve UI, model import, lighting,
world-override insertion into the chunk path, and arbitrary-resolution offline capture.

See [docs/cinematix/ROADMAP.md](docs/cinematix/ROADMAP.md) for per-subsystem status.

Read [the project requirements](docs/REQUIREMENTS.md) and [agent instructions](AGENTS.md) before making changes. Audit the actual engine before designing or implementing integrations. The UI interaction reference is [BBS-FS](https://github.com/Wemppy4/bbs-fs); inspect its source and licensing before any reuse.

## Development approach

Work incrementally: audit and architecture first, native editor integration second, then animation and subsequent subsystems. Push meaningful, reviewable commits with honest verification records. Never claim a feature works without evidence.
