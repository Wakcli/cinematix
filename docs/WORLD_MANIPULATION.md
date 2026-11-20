# World manipulation audit

## Existing world architecture

Horizon uses a server-side level model with client remote state. Relevant sources:

- `crates/horizon-client/src/game/world/`
- `crates/horizon-client/src/game/integrated.rs`
- `crates/horizon-world`
- `crates/horizon-client/src/game/blocks/`
- `crates/horizon-client/src/save.rs`

Singleplayer is a window joined to its own server thread, not a stateless local world
simulation. Permanent save mutation must remain behind normal save/world APIs.

## Phase 1 and Phase 2 boundary

The audit establishes no implementation yet for Cinematix world override layers. Editor mode
must not alter permanent saves in the first integration slice.

Future world deltas must be separate in-memory layers with:

- original block/state capture
- ordered place/remove/replace operations
- visibility and activation
- safe restoration
- no write in editor close/open lifecycle
- explicit save import only at a later defined action

No save serialization or world mutation is planned before the real scene/editor interfaces exist.
