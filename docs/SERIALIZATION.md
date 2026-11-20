# Serialization audit and `.cmx` strategy

## Current status

No Cinematix `.cmx` project format is implemented. No serialization dependency is being added
for a speculative format in Phase 1.

Horizon already uses Rust `serde` for captured scenes and metadata. It is a candidate for
future Cinematix project JSON, while future binary assets or keyframe payloads should use
explicit versioned formats.

## Intended `.cmx` design

A future `.cmx` project should contain:

- versioned JSON metadata and scene graph
- typed keyframe/channel data
- camera and environment configuration
- world delta records
- relative asset references and optional embedding policy
- asset deduplication and migration metadata

Implementation requires validation, corruption handling, migrations and portable path
resolution. It must not accidentally serialize permanent save state or prepared game resources.
