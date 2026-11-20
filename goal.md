# goal.md

The goal and the operating instructions for work in this repository. Read with `AGENTS.md`; where
this file and `AGENTS.md` disagree, `AGENTS.md` governs and this file is corrected.

## 1. The goal

Build **Cinematix**: a native cinematography, scene-building, animation, lighting and
offline-rendering suite that runs *inside* the Horizon singleplayer client.

It is an engine subsystem, not a Minecraft mod, not a loader plugin, and not a second application.
There is no second window, no second renderer and no separate editor process. The user opens a world,
presses a key, and works inside the game they are already running.

The reference for interaction design is BBS-FS: an icon rail, dockable and floating panels, a
persistent timeline, a dope sheet, a curve editor, numeric scrubbers and context menus. That is a
*behavioural* reference. It is Java against OpenGL, its widget code lives in McLib whose licence is
`NOASSERTION`, and none of it can be linked. We reproduce the interaction model, never the code.

### The end state

A user can enter a singleplayer world, open Cinematix, build a scene, import and rig characters,
place animated lights, move cameras along splines, alter the world non-destructively, preview in real
time, and render a deterministic frame-exact video — without leaving Horizon and without altering the
world save on disk.

### Hard constraints

- Rust, native, inside Horizon's existing architecture and its existing UI and rendering systems.
- No Java, Kotlin, LWJGL, Forge, Fabric or Iris production code.
- Singleplayer only, initially. Editor state and world overrides never mutate permanent world saves.
- Shader-pack compatibility is preserved. Horizon's packs shade the world; they do not shade imported
  meshes, and any claim otherwise is false.
- No placeholders, no mock implementations, no fabricated verification, no disconnected UI prototypes.
- Never commit Minecraft jars, extracted assets, prepared resources, credentials or player data.
- All visible game strings are translation keys. Interactive controls use registering helpers.
- A change to an upstream Horizon file is listed in `docs/cinematix/horizon-patches.md` with its size.

## 2. Operating instructions

The workflow every agent follows in this repository:

> **Scheduler workflow: pick the lightest workflow that fits. Direct work: execute and verify
> proportionately. Otherwise: plan lanes → dispatch background specialists → track task IDs → await
> hook-driven completion → reconcile terminal results → verify.**

How that translates into practice here:

**Pick the lightest workflow that fits.** A one-line documentation fix, a version bump, a rebase of a
known patch, or a single obvious correction is done directly. Do not spin up a plan, lanes or
specialists for work whose whole shape is already visible; the coordination costs more than the edit.

**Direct work: execute and verify proportionately.** Do the thing, then check it at the level the
change actually risks. A doc edit is checked by reading it back. A code change that compiles is not
thereby verified — it is compiled. Anything touching rendering, input, camera or serialisation needs a
run, and the report says what was run and what was not.

**Otherwise, plan lanes.** When work is genuinely parallel — separate crates, separate subsystems, or
read-only audits that would each cost a long serial read — split it into lanes that do not share a
write scope. Two agents editing the same file is a merge conflict with extra steps.

**Dispatch background specialists.** Prefer a specialist for the lane that matches it: auditing,
external research, implementation, visual review. Give each lane its scope, the exact paths, and the
evidence format wanted back. Never delegate a decision that needs the user's intent.

**Track task IDs.** Record which lane owns which session so a result can be read, continued or
cancelled rather than blindly re-run.

**Await hook-driven completion.** Do not poll, sleep, or loop on a build, a harness or a child
session. Start it, do non-overlapping work, and take the completion notification. Waiting in loops is
where this project's predecessor sessions lost most of their time.

**Reconcile terminal results.** A specialist's summary is a claim, not evidence. Read the produced
artifacts. Missing scope and out-of-scope extras are both defects. Reconcile every lane before final
validation, and reuse evidence that is still valid instead of re-deriving it.

**Verify.** Then verify the whole, not just the parts. A green unit test over a renderer that cannot
produce a frame is the standard way a project of this kind dies.

## 3. What "done" means for a slice

A slice is complete only when all of these are true, and the report states each one explicitly:

1. The functionality exists and is reachable from the running game, not only from a test.
2. It integrates with the intended Horizon system rather than sitting beside it.
3. It builds.
4. Its tests pass, and the tests can fail — restore the bug, watch them go red, re-apply the fix.
5. GPU and CPU resources are released; nothing leaks on open, close or cancel.
6. Documentation reflects what was actually built, and `docs/cinematix/ROADMAP.md` status is updated.
7. No placeholder, stub, `todo!()` or fabricated result remains.

Verification is recorded the way `docs/cinematix/AUDIT.md` records it: what was executed, what was
only read, and what remains unproven.

## 4. Current position

Phase 1 is complete: the Horizon baseline is imported at `1664a35852`, the engine builds, boots and
runs headless under `xvfb-run` with `--offscreen`, and five engine-free crates carry 30 passing tests.
The audit, architecture, UI mapping and roadmap are in `docs/cinematix/`.

The next slice is Phase 2's first integration: a `Screen::Cinematix` that opens on a key binding
inside a running world, draws the icon rail and one plate panel with Horizon's own widgets, and closes
with input capture, cursor state and the player's camera restored exactly.

Do not begin the wider editor until that slice runs.
