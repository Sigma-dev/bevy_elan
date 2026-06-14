# bevy_elan

A small, force-based **first-person character controller** and **FPS look camera**
for [Bevy](https://bevyengine.org), built on the [Avian](https://github.com/Jondolf/avian)
physics engine.

- **Floating-capsule controller** — hovers above the ground with a raycast spring,
  moves and jumps via forces, locks rotation so it stays upright.
- **FPS camera** — mouse look with yaw on the body and pitch on the camera when
  parented; grabs the cursor only while its camera is active, so multiple
  controllers/cameras can coexist and be toggled.

## Versions

| bevy_elan | Bevy | Avian |
|-----------|------|-------|
| 0.1       | 0.18 | 0.6   |

## Usage

```toml
[dependencies]
bevy_elan = { git = "https://github.com/Sigma-dev/bevy_elan" }
```

```rust
use bevy::prelude::*;
use bevy_elan::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, avian3d::PhysicsPlugins::default(), ElanPlugin))
        .add_systems(Startup, spawn_player)
        .run();
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        character_controller_bundle(),
        Transform::from_xyz(0.0, 2.0, 0.0),
        children![(
            Transform::from_translation(Vec3::Y * 0.3),
            FpsCamera::new(0.1),
        )],
    ));
}
```

Controls: **WASD** to move, **Space** to jump, mouse to look. Set
`CharacterController3d::enabled = false` (and deactivate the camera) to hand
control to another camera.

## License

MIT
