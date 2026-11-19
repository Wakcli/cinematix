# Cinematix development status

## Verification environment

The current environment contains the imported Horizon repository and the fetched BBS-FS
reference refs. Runtime resources and player data are not present. Only disposable directories
may be used for future game runs.

## Phase 1 checks

Required before implementation:

```sh
git rev-parse HEAD
git merge-base --is-ancestor 5ef7734fdd854add5efd6817178be3c988d8d050 HEAD
python3 tools/check_public_tree.py --history
cargo metadata --no-deps
cargo check -p horizon-client --lib
cargo test -p horizon-client --lib studio
```

Exact results must be recorded after running them. GPU and interactive runtime tests remain
separate and may not run without a compatible display and prepared resources.

## Recorded Phase 1 results

- `python3 tools/check_public_tree.py --history`: passed.
- `cargo metadata --no-deps --format-version 1`: passed.
- `cargo check -p horizon-client --lib`: passed after installing the upstream-documented
  `libasound2-dev`, `libudev-dev`, and `pkg-config` Linux prerequisites.
- `cargo test -p horizon-client --lib studio`: did not complete; the Rust test build was killed
  by the environment with SIGKILL. No test failure result is claimed.
- Interactive game launch, GPU rendering, screenshot, shader-pack, HDR and BBS-FS UI behavior
  were not run because no prepared disposable game resources or runtime display are available.

## Test policy

No source-string tests are acceptable for lifecycle/input behavior. Phase 2 tests must exercise
state transitions and captured/restored input/camera state. Later tests must evaluate typed
animation values, curves, camera constraints and world override restoration, not file contents.

## Commit policy

Baseline imports are separated from Cinematix implementation commits. Keep unrelated upstream
files intact and push only to `Wakcli/cinematix`.
