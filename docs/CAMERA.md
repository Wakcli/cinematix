# Camera and viewport audit

## Existing implementation

Horizon has:

- Renderer cameras and projection in `crates/horizon-client-render/src/camera.rs`
- Free camera state and camera track in `crates/horizon-client/src/studio/camera.rs`
- Camera splines in `crates/horizon-client/src/studio/spline.rs`
- Gameplay camera pose hooks in `crates/horizon-client/src/game/native_hooks/camera_pose.rs`
- Replay Studio camera and path integration in `game/interface/studio.rs`
- Independent camera capture inputs in `gfx/capture.rs`

## Independent editor camera

The existing `StudioFreecam` does not alter player state; it owns a separate position/yaw/pitch,
FOV and optional track lock. The free camera is activated from the replay camera pose and
deactivated or reset through Studio actions.

This is a reusable base for the Phase 2 independent viewport camera. It must remain separate
from player position/orientation and must restore prior camera state on close.

## Missing pieces

The following are not Phase 1 implementations:

- Scene hierarchy and active-camera ownership
- Orbit, target, dolly and rail constraints
- Catmull-Rom/Bezier path editing and arc-length evaluation
- Gizmos and viewport frustum visualization
- Focal length/sensor/focus/aperture and depth-of-field scene properties
- Camera transitions and noise shake

The existing replay camera track is the reusable first slice for camera pose and path playback.
