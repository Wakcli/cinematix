# Capture and export audit

## Existing deterministic mechanisms

Horizon has:

- `HORIZON_STILL=1` deterministic frame holds
- `/tick freeze` world state
- Screenshot sequences in `game/runtime/screenshot_sequence.rs`
- GPU screenshot/readback in `gfx/screenshot*.rs` and `gfx/readback.rs`
- Render scene capture in `gfx/capture.rs`
- Ordered screenshot output and HDR mapping

`HORIZON_STILL` and screenshot sequences require a frozen, loaded world and disable temporal
features that would depend on prior frames. This is reusable evidence for deterministic capture.

## Limits

The existing render capture does not capture entities, particles, weather, hands, outlines or
HUD. The 10,000-frame sequence command is bounded but no Cinematix offline renderer or FFmpeg
pipeline exists. Subframe motion blur and arbitrary resolution/export are not implemented.

## Future export boundary

Export belongs as a later slice after scene-backed timeline and independent viewport integration.
It must use real renderer frame outputs, preserve HDR mapping/pack behavior, bound memory,
validate frame order and handle cancellation/failure without silently dropping frames.
