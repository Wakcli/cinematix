# Developing and verifying Cinematix

## Read before edits

1. Root [AGENTS.md](../../AGENTS.md) and [requirements](../REQUIREMENTS.md).
2. The imported upstream instructions, contributing rules and actual layer/
   architecture/UI/HDR docs. This branch has not imported them yet.
3. [AUDIT.md](AUDIT.md), [ARCHITECTURE.md](ARCHITECTURE.md),
   [UI.md](UI.md) and [ROADMAP.md](ROADMAP.md).
4. Current code and callers for the chosen subsystem, plus open pull requests.

Keep these project-specific docs under `docs/cinematix/`; preserve upstream docs
at their original paths during baseline import. Merge root agent guidance rather
than replacing either project's rules. Do not add empty subsystem templates.

## Reproducing the source inspection

Clone the two public sources into separate audit directories, then check out
the exact revisions in [sources.json](sources.json). A shallow fetch of that
specific commit is sufficient for inspection; baseline import is a different
operation and must preserve accurate provenance/history.

```sh
git clone https://github.com/horizon-rust/horizon.git ../horizon-audit
git -C ../horizon-audit checkout --detach 5ef7734fdd854add5efd6817178be3c988d8d050
git clone https://github.com/Wemppy4/bbs-fs.git ../bbs-fs-audit
git -C ../bbs-fs-audit checkout --detach 13bd62dab1980b974b8ea0f78d037e8e82684b84
git diff --check
```

Check the manifest's file hashes and pinned URLs before relying on it. A changed
source revision requires a fresh audit of affected contracts, not quietly
rewriting the revision field. Selective file inspection is not runtime proof.

## Build and runtime gates after baseline import

The upstream workspace declares Rust **1.95 or newer**, edition 2024.
Linux default audio/controller features need ALSA/udev headers. Keep upstream
wgpu-hal/naga/mimalloc patches and its Cargo lockfile.

Follow actual upstream `tools/pr_ready.py` and touched-crate guidance:

```sh
cargo check -p <changed-crate>
python3 tools/pr_ready.py --base <review-base>
cargo build --profile fast -p horizon-game   # only when a game run is needed
```

Those are follow-on commands, **not successful runs reported by this branch**.
The final verification block must list exact commands, results and missing
prerequisites; do not remove tests or weaken feature/build gates to hide failures.

Interactive runs require prepared resources from the player's own Minecraft
copy. No game jars/assets/player data belong in the repository. Use disposable
game folders and explicitly isolate homes:

```sh
scratch=$(mktemp -d)
mkdir -p "$scratch/game"
export HORIZON_HOME="$scratch/horizon"
export HORIZON_LAUNCHER_HOME="$scratch/launcher"
export XDG_DATA_HOME="$scratch/xdg-data"
export XDG_CONFIG_HOME="$scratch/xdg-config"
export HORIZON_LAUNCHER_TRASH="$scratch/trash"
export HORIZON_LAUNCHER_CLIPBOARD="$scratch/clipboard"
export HORIZON_LAUNCHER_DESKTOP="$scratch/desktop"
export HORIZON_LAUNCHER_NO_SELF_UPDATE=1 HORIZON_NO_AUDIO=1
# Invoke the built game with --game-dir "$scratch/game" and an explicit
# --resources path to legally prepared resources. Never run against real saves.
```

A build does not verify capture/focus, actual viewport geometry, HDR/shader
compatibility or untouched world saves. Keep visual/runtime checks explicit.

## Verification record: this documentation contribution

- Source repos checked out at the pinned revisions.
- Static audit validation: **70** listed source files existed and their SHA-256
  digests and revision-pinned URLs matched both exact checkouts.
- Static documentation validation: **27** local Markdown links resolved within
  the destination. These are document/provenance checks, not behavior tests.
- Relevant implementations and callers inspected; provenance file hashes
  generated from those checkouts.
- Destination changes are audit/design docs and machine-readable provenance,
  not game functionality.
- `git diff --check` is the static whitespace gate.
- Rust compilation/tests: **not run**; no Cargo/rustc in the audit VM.
- Game/GPU/editor integration: **not run**; no prepared resources, no game
  launch and no accessible `/dev/dri` device found in this VM.
- FFmpeg executable was found at `/usr/bin/ffmpeg`; no encoder/export integration
  was implemented or tested.

This record is a limit on claims, not a recommendation to postpone tests.
Implementation commits must add the relevant behavior proof and update status.
