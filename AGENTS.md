# Cinematix agent instructions

## Read first

Read docs/REQUIREMENTS.md. Inspect the actual upstream Horizon source, its AGENTS.md, CONTRIBUTING.md, docs/layers.md, docs/design-system.md, docs/architecture.md, docs/ui-kit.md and docs/hdr-passes.md before choosing integration points. Preserve upstream instructions when importing a Horizon baseline; merge these Cinematix rules rather than replacing upstream guidance.

## Non-negotiable constraints

- Native Rust integration into Horizon, not a Minecraft mod or a separate editor application.
- Use the engine's existing rendering, math, input, UI, asset and resource abstractions wherever suitable. Do not assume an API exists from documentation alone: follow its implementation and callers.
- Singleplayer only initially. Editor state and world overrides must not mutate permanent world saves.
- No Java, Kotlin, LWJGL, Forge or Fabric production implementation. Existing upstream development tooling can remain unchanged.
- No placeholder functionality, empty implementations, fabricated verification or disconnected UI prototypes.
- Never commit Minecraft jars, extracted assets, prepared resources, credentials or player data. Preserve upstream copyright, license files and notices.
- Inspect BBS-FS source and license for reference-driven UI behavior; do not copy restricted code or assets.
- Minimize unrelated changes, dependencies, clones and allocations. Keep modules focused and errors explicit. Preserve shader-pack and HDR behavior.
- All visible game strings use Horizon's translation system. Respect its architecture layers and feature gating.
- Read subsystem documentation and existing code before edits; update accurate implementation status, tests and documentation with code.
- Push small meaningful commits to this repository. Do not push to upstream Horizon, force-push or delete history.

## Initial work

First establish a traceable Horizon source baseline in this repository's working branch, retaining provenance, licensing and build conventions. Do not fabricate a mini-engine. Record the exact audited upstream revision. Keep Cinematix documentation and instructions while preserving upstream content.

Audit first, then document actual findings and shared interfaces. Begin Phase 2 only after that foundation exists, with one small functional in-game integration slice and relevant behavior tests. Do not attempt the complete suite in one pass.

## Verification

Follow upstream targeted-check and test guidance. Use disposable game directories, never real player data. Record commands, results and environment blockers. Compilation does not establish interactive rendering correctness. Mark unrun tests and unverified runtime behavior explicitly. Never silently weaken checks or remove existing behavior.
