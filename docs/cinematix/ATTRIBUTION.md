# Third-party attribution

Cinematix reuses UI artwork and reads the UI source of two MIT-licensed projects. The MIT licence
requires this notice and the copyright lines to travel with the copied files; the full text sits
beside the asset.

## BBS mod (the film studio reference)

- Source: <https://github.com/Wemppy4/bbs-fs>, branch `dev`.
- Copyright 2025 McHorse, 2026 Wemppy. MIT.
- Copied so far: `src/client/resources/assets/bbs/assets/textures/icons.png` →
  `crates/horizon-client/assets/horizon/textures/gui/cinematix/icons.png` (256x256, a 16x16 grid of
  16 px GUI glyphs). Licence text beside it in
  `crates/horizon-client/assets/horizon/textures/gui/cinematix/LICENSE.txt`.
- Read for layout and behaviour: `src/client/java/mchorse/bbs_mod/ui/**` (dashboard, icon names,
  timeline and clip panels). No Java is compiled or shipped; the Rust editor reimplements the
  interactions.

## McLib

- Source: <https://github.com/mchorse/mclib>.
- Copyright (c) 2018 McHorse. MIT.
- Read for the dashboard shell (`client/gui/mclib/GuiAbstractDashboard`, `GuiDashboardPanels`,
  `client/gui/utils/Icons.java`) and the icon grid. No McLib file is copied yet; if one is, its
  licence notice goes beside it.

## Earlier claim corrected

`docs/cinematix/UI.md` used to say McLib's GitHub licence reads `NOASSERTION` and that nothing was
copied. Both repositories carry an MIT `LICENSE.md`; the GitHub metadata was wrong. Copying is
allowed with the notices above, and the editor is moving from a loose approximation to the BBS
layout and icon sheet.
