# Horizon and BBS-FS baseline

## Audited revisions

- Horizon: `5ef7734fdd854add5efd6817178be3c988d8d050` (`horizon/main` HEAD when audited).
- Horizon was merged into this repository with merge commit `b70a5128fa`.
- BBS-FS: `13bd62dab1980b974b8ea0f78d037e8e82684b84` (`bbs-fs/master` HEAD when audited).
- BBS-FS remains an external reference remote, not vendored Java source or assets.

## Horizon baseline provenance

The baseline is the actual upstream history, imported through `git fetch horizon --tags` and an
unrelated-history merge. It retains Horizon's root guidance, license, notices, workspace,
third-party vendor patches and source layout. The destination's brief remains in
`docs/REQUIREMENTS.md`; root `AGENTS.md` and `README.md` reconcile upstream text with
Cinematix constraints.

No Horizon commit is rewritten, and no upstream repository was modified.

## Mandatory upstream reading

Read by the initial audit:

- `AGENTS.md`
- `CONTRIBUTING.md`
- `docs/layers.md`
- `docs/design-system.md`
- `docs/architecture.md`
- `docs/ui-kit.md`
- `docs/hdr-passes.md`
- `docs/graphics.md`
- `docs/native-client.md`
- `docs/shaders.md`
- `docs/release-flow.md`

## BBS-FS license and reuse boundary

`LICENSE.md` at the audited revision says Copyright 2025 McHorse and Copyright 2026 Wemppy, and
grants MIT-style permission subject to the copyright and permission notice. No BBS-FS Java
runtime or restricted asset was copied. A Rust implementation may recreate behavior and layout
only by translating concepts to Horizon's native systems; reused substantial code would require
the MIT notice and a `THIRD_PARTY_NOTICES.md` entry.

## BBS-FS source audit

The principal audited behavior and source paths are:

- Dockable workspace and floating behavior: `src/client/java/mchorse/bbs_mod/ui/framework/elements/layout/UIDockLayout.java`
- Hierarchical film and replay editor: `src/client/java/mchorse/bbs_mod/ui/film/UIFilmPanel.java`
- Replay hierarchy: `src/client/java/mchorse/bbs_mod/ui/film/replays/UIReplaysEditor.java`
- Keyframe properties and timeline: `src/client/java/mchorse/bbs_mod/ui/framework/elements/input/keyframes/UIKeyframeEditor.java`
- Hierarchical track sheets: `src/client/java/mchorse/bbs_mod/ui/framework/elements/input/keyframes/UIKeyframeSheet.java`
- Value graph: `src/client/java/mchorse/bbs_mod/ui/framework/elements/input/keyframes/graphs/UIKeyframeGraph.java`
- Dopesheet: `src/client/java/mchorse/bbs_mod/ui/framework/elements/input/keyframes/graphs/UIKeyframeDopeSheet.java`
- Numeric and text input factories: `src/client/java/mchorse/bbs_mod/ui/framework/elements/input/keyframes/factories/`
- Timeline canvas utilities: `src/client/java/mchorse/bbs_mod/ui/framework/elements/utils/UITimelineCanvas.java`

The reference implements registered keyframe types, hierarchical track sheets, graph/dopesheet
selection modes, curve tangents, split/resize/dock layouts, panel drag/drop, floating panels,
tab stacks, spring-loaded tabs, maximize, layout undo and context/property panels. These are
behavioral targets, not dependencies or implementation source.
