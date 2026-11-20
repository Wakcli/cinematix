# Development

Everything here was executed on this machine. Where a command needs resources this repository does
not carry, it says so.

## Requirements

| | |
|---|---|
| Rust | 1.95 minimum, 1.99.1 used; edition 2024 |
| Linux packages | `libasound2-dev`, `libudev-dev` |
| GPU | Vulkan 1.4 capable; GTX 1080 used |
| Minecraft | your own 26.3 client install **and** the 26.3 `server.jar` |
| Encoder | `ffmpeg` on `PATH` for video output |

Minecraft files are never committed. `.gitignore` excludes `.local/` and `target/`;
`tools/check_public_tree.py` verifies the public tree.

## First build

```sh
cargo build --release -p horizon-game -p horizon-client-lib --bins
```

Measured: 18 m 44 s cold on 12 cores. For iteration use the fast profile, which links quickly:

```sh
cargo build --profile fast -p horizon-game
```

## Prepared resources

Horizon does not ship game assets. It derives its own resources from your installation. Both the
client and the server jar are required because the exporters run against the server.

```sh
mkdir -p ~/.minecraft/versions/26.3
curl -sSL https://piston-meta.mojang.com/v1/packages/11f1aefe473eefba0af62e3286c33b692807b2eb/26.3.json \
  -o ~/.minecraft/versions/26.3/26.3.json
# client.jar and the libraries also come from Mojang's manifest; a launcher install works too

./target/release/horizon-prepare --prepare \
  --minecraft-dir ~/.minecraft \
  --server-jar /path/to/server-26.3.jar
./target/release/horizon-prepare sounds --minecraft-dir ~/.minecraft
./target/release/horizon-prepare --check
```

**Save the version JSON with `curl` or `wget`, never re-serialised.** `horizon-prepare` verifies its
SHA-1 against the pin in `research-lock.json`; pretty-printing it changes the bytes and the prepare
fails with `is not a 26.3 version file Horizon knows`.

`--prepare` writes `.local/horizon-client/26.3` and takes about 15 s. `sounds` copies 4 962 files
(377 MB).

## Running

```sh
# windowed, disposable game directory, never your real saves
./target/release/horizon-client --test-terrain --world Scratch --seed 1 --game-dir /tmp/horizon-run

# headless: renders into a texture, nothing appears on the physical display
xvfb-run -a -s "-screen 0 1920x1080x24" \
  ./target/release/horizon-client --test-terrain --frames 2 --offscreen
```

`--offscreen` is deliberately absent from `--help`. It still needs `DISPLAY` or `WAYLAND_DISPLAY`
because winit creates an event loop first; without either, the process exits with
`neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set`.

## Driving the game

`horizon-ctl` launches and controls the game over a socket. This is how the editor is tested without
a human at the mouse.

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" bash -c '
  ./target/release/horizon-ctl --socket /tmp/ctl.sock launch \
    --world Scratch --seed 1 --offscreen --wait \
    --resources $PWD/.local/horizon-client/26.3
  ./target/release/horizon-ctl --socket /tmp/ctl.sock state | head -c 400
  ./target/release/horizon-ctl --socket /tmp/ctl.sock keys W 40      # hold W for 40 ticks
  ./target/release/horizon-ctl --socket /tmp/ctl.sock screenshot /tmp/shot.png
  ./target/release/horizon-ctl --socket /tmp/ctl.sock quit
'
```

`game/ui_control.rs` lets `horizon-ctl` reach registered controls by name, which is possible only
for controls drawn with a registering helper.

## The editor, headless

The editor is a screen inside the running game; this is the whole loop that opens it, proves it
drew, and closes it again, with no display:

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" bash -c '
  CTL=./target/fast/horizon-ctl
  SOCK=/tmp/opencode/cinematix.sock
  RES=/home/wak/Cinematix/horizon/.local/horizon-client/26.3
  $CTL --socket $SOCK launch --world CinematixScratch --seed 7 --offscreen --wait --resources $RES
  $CTL --socket $SOCK wait-ready 90
  $CTL --socket $SOCK key f8 tap                 # key.cinematix: the editor opens
  $CTL --socket $SOCK cinematix status           # open: true, screen: cinematix
  $CTL --socket $SOCK cinematix frame 48         # playhead at frame 48 (2.00 s at 24 fps)
  $CTL --socket $SOCK cinematix key              # one key on the selected channel
  $CTL --socket $SOCK key w down                 # fly the film camera forward (E/Q rise/fall)
  sleep 1
  $CTL --socket $SOCK key w up
  $CTL --socket $SOCK set field-of-view 0.5      # registered slider: the view widens to 90
  $CTL --socket $SOCK cinematix camera player    # back to the player's eye
  $CTL --socket $SOCK ui                         # the rail buttons and the FOV sliders, by id
  $CTL --socket $SOCK screenshot /tmp/editor.png
  $CTL --socket $SOCK key escape tap             # back to Playing
  $CTL --socket $SOCK cinematix status           # open: false, screen: playing
  $CTL --socket $SOCK quit
'
```

The same commands exist in chat as `/cinematix open|close|status|play|pause|frame N|seek S|key|
delkey|fit|section NAME`. `cinematix` is not `screenshot-sequence`: that command refuses any screen
but `Playing`, so `screenshot` is the capture path while the editor is open. The screenshot is the
surface size, not a requested resolution.

## Deterministic capture

```sh
./target/release/horizon-ctl --socket /tmp/ctl.sock options-set chunkSectionFadeInTime 0
./target/release/horizon-ctl --socket /tmp/ctl.sock wait-ready 60
./target/release/horizon-ctl --socket /tmp/ctl.sock screenshot-sequence 5 40 /tmp/seq.png
```

Requires `HORIZON_STILL=1`, a frozen world, chunk fade 0, no LOD and the shadow cache off. A run can
be rejected with `the camera, world, settings, shader pack or window size changed while it ran`;
retrying once succeeds. Output is the surface size, not a requested resolution.

## Tests

```sh
cargo test -p cinematix-scene -p cinematix-anim -p cinematix-ui -p cinematix-timeline -p cinematix-render --lib
```

30 tests, no GPU, no game required — this is the point of keeping those crates Horizon-free.

For the engine side, use upstream's self-check rather than ad-hoc commands:

```sh
python3 tools/pr_ready.py
cargo build --profile fast -p horizon-game
python3 tools/check_layers.py
python3 tools/check_public_tree.py
```

`tools/parity.py --changed origin/main` selects the vanilla-parity harnesses your change can affect.

## Conventions

- Compile with `cargo check -p <crate>` while working. Do not run `clippy` or a full build after
  every edit.
- User-facing strings are translation keys (`Text::tr`, `lang::get`). No English literals.
- Interactive controls use registering helpers (`ui::button`, `settings_shell::foot_button`);
  `graphics_style::button` and `screen_draw::button` only paint.
- New code respects the layers in `docs/layers.md`; `tools/check_layers.py` enforces it.
- Passes that read or redraw the world image follow `docs/hdr-passes.md`.
- Every change to an upstream Horizon file is listed in `docs/cinematix/horizon-patches.md`.

## Reporting

When claiming work is done, say what was executed and what was not. Compilation is not evidence of
correct rendering. This project records the difference between verified and read in
[AUDIT.md](AUDIT.md) and expects the same standard everywhere.
