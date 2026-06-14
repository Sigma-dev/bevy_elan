//! A floating-capsule, force-based 3D character controller for Avian.

use avian3d::prelude::*;
use bevy::{math::FloatPow, prelude::*};

pub struct CharacterController3dPlugin;

impl Plugin for CharacterController3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedPreUpdate, (handle_grounded, handle_friction).chain())
            .add_systems(
                FixedUpdate,
                (handle_hover, handle_movement, handle_jump).chain(),
            );
    }
}

/// A floating-capsule character controller. Spawn it with
/// [`character_controller_bundle`], which supplies the rigid body, collider,
/// ground ray and friction/CCD settings.
#[derive(Component)]
pub struct CharacterController3d {
    /// Ride height: the body hovers this far above the ground.
    pub hover_height: f32,
    /// Horizontal movement strength.
    pub move_speed: f32,
    /// Upward launch velocity (m/s) applied on jump. Set as a velocity rather
    /// than a force so jump height doesn't depend on the body's mass.
    pub jump_velocity: f32,
    /// Multiplier on the velocity-based drag that slows the body down.
    pub drag_multiplier: f32,
    /// Grace period after leaving the ground during which a jump is still allowed.
    pub coyote_time: f32,
    /// Minimum time between jumps.
    pub jump_cooldown: f32,
    /// When `false`, movement and jump input is ignored (the body still hovers
    /// and settles). Useful for toggling control off, e.g. for a free camera.
    pub enabled: bool,
}

impl Default for CharacterController3d {
    fn default() -> Self {
        Self {
            hover_height: 1.0,
            move_speed: 1.0,
            jump_velocity: 5.0,
            drag_multiplier: 1.0,
            coyote_time: 0.1,
            jump_cooldown: 0.5,
            enabled: true,
        }
    }
}

#[derive(Component)]
pub struct Grounded;

#[derive(Component)]
pub struct LastGrounded(pub f32);

#[derive(Component)]
pub struct LastJump(pub f32);

#[derive(Component)]
pub struct DistanceToGround(pub f32);

/// The components that make up a character controller body.
pub fn character_controller_bundle() -> (
    RigidBody,
    CharacterController3d,
    Collider,
    RayCaster,
    GravityScale,
    LockedAxes,
    Friction,
    SweptCcd,
) {
    let capsule_height = 1.0;
    (
        RigidBody::Dynamic,
        CharacterController3d::default(),
        Collider::capsule(0.15, capsule_height),
        RayCaster::new(
            Vec3::new(0.0, -capsule_height / 2.0, 0.0),
            Dir3::new(-Vec3::Y).unwrap(),
        ),
        GravityScale(2.0),
        LockedAxes::ROTATION_LOCKED,
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        SweptCcd::default(),
    )
}

fn handle_grounded(
    time: Res<Time>,
    mut commands: Commands,
    controllers: Query<(Entity, &RayHits, &CharacterController3d)>,
) {
    for (entity, hits, controller) in controllers.iter() {
        let hit = hits.iter_sorted().next();
        let distance = hit.map(|hit| hit.distance).unwrap_or(f32::INFINITY);
        commands.entity(entity).insert(DistanceToGround(distance));
        if distance < controller.hover_height {
            commands.entity(entity).insert(Grounded);
            commands
                .entity(entity)
                .insert(LastGrounded(time.elapsed_secs()));
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

fn handle_hover(
    time: Res<Time>,
    mut controllers: Query<(
        &CharacterController3d,
        Forces,
        &DistanceToGround,
        Option<&LastJump>,
    )>,
) {
    for (controller, mut forces, distance_to_ground, last_jump) in controllers.iter_mut() {
        // Suppress the ground spring briefly after a jump so it doesn't cancel
        // the launch velocity before the body clears hover range.
        if last_jump.is_some_and(|last_jump| time.elapsed_secs() - last_jump.0 < 0.2) {
            continue;
        }
        let distance = distance_to_ground.0;
        if distance >= controller.hover_height {
            continue;
        }

        let diff = controller.hover_height - distance;
        let vertical_velocity = forces.linear_velocity().y;
        let bounce_force = Vec3::Y * diff * 100.0 * 60.0;
        let damp_force = Vec3::Y * -(vertical_velocity) * 10.0 * 60.0;
        let total_force = (damp_force + bounce_force) * 0.005;
        forces.apply_force(total_force);
    }
}

fn handle_movement(
    mut controllers: Query<(&Transform, Forces, &CharacterController3d, Option<&Grounded>)>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (transform, mut forces, controller, grounded) in controllers.iter_mut() {
        if !controller.enabled {
            continue;
        }
        let mut dir = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) {
            dir.z -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            dir.z += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if dir != Vec3::ZERO {
            dir = dir.normalize();
        } else {
            continue;
        }
        let grounded_mult = if grounded.is_some() { 1.0 } else { 0.2 };
        let force = transform.rotation * dir * grounded_mult * controller.move_speed * 7.0;
        if force != Vec3::ZERO {
            forces.apply_force(force);
        }
    }
}

fn handle_jump(
    time: Res<Time>,
    mut commands: Commands,
    mut controllers: Query<(
        Entity,
        Forces,
        &CharacterController3d,
        &LastGrounded,
        Option<&LastJump>,
    )>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (entity, mut forces, controller, last_grounded, last_jump) in controllers.iter_mut() {
        if !controller.enabled {
            continue;
        }
        if time.elapsed_secs() - last_grounded.0 > controller.coyote_time {
            continue;
        }
        if last_jump
            .is_some_and(|last_jump| time.elapsed_secs() - last_jump.0 < controller.jump_cooldown)
        {
            continue;
        }
        if keyboard.pressed(KeyCode::Space) {
            // Set the launch velocity directly: a fixed force would send a light
            // body flying, since acceleration is force / mass.
            forces.linear_velocity_mut().y = controller.jump_velocity;
            commands
                .entity(entity)
                .insert(LastJump(time.elapsed_secs()));
        }
    }
}

fn handle_friction(mut controllers: Query<(&CharacterController3d, Option<&Grounded>, Forces)>) {
    for (controller, grounded, mut forces) in controllers.iter_mut() {
        let velocity = forces.linear_velocity().with_y(0.0);
        let speed = velocity.length();
        if speed == 0.0 {
            continue;
        }
        let dir = -velocity.normalize();
        let grounded_mult = if grounded.is_some() { 1.0 } else { 0.05 };
        let fixed_friction_force = dir * if grounded.is_some() { 0.3 } else { 0.0 };
        let friction = speed.squared();

        let friction_force =
            friction * dir * grounded_mult * controller.drag_multiplier + fixed_friction_force;
        forces.apply_force(friction_force);
    }
}
