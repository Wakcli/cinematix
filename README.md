# Cinematix

Cinematix is a planned native cinematography, scene-building, animation, lighting and offline-rendering subsystem for [Horizon](https://github.com/horizon-rust/horizon), implemented in Rust inside the running singleplayer game.

## Current status

Project brief recorded. Horizon source audit, architecture, implementation and runtime verification are not yet complete. This repository does not yet contain a working editor or an imported Horizon source baseline.

Read [the project requirements](docs/REQUIREMENTS.md) and [agent instructions](AGENTS.md) before making changes. Audit the actual engine before designing or implementing integrations. The UI interaction reference is [BBS-FS](https://github.com/Wemppy4/bbs-fs); inspect its source and licensing before any reuse.

## Development approach

Work incrementally: audit and architecture first, native editor integration second, then animation and subsequent subsystems. Push meaningful, reviewable commits with honest verification records. Never claim a feature works without evidence.
