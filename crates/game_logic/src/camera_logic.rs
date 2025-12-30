use bevy::camera::visibility::RenderLayers;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::pbr::MeshMaterial3d;
use game_models::camera::OrbitCamera;
use game_models::entities::player::Player;

pub fn setup_test_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_test_player_cube(&mut commands, &mut meshes, &mut materials);
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera {
            radius: 8.0,
            yaw: 0.0,
            pitch: -0.3,
        },
        Camera {
            order: 0,
            ..default()
        },
        RenderLayers::from_layers(&[0]),
    ));
}

//TODO: Replace this temporary test player cube with the actual player spawn pipeline.
fn spawn_test_player_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.6, 0.9),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
        RenderLayers::from_layers(&[0, 1, 2]),
        Player,
    ));
}

pub fn orbit_camera_controls(
    mut motion_events: MessageReader<MouseMotion>,
    mut wheel_events: MessageReader<MouseWheel>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut cameras: Query<(&mut OrbitCamera, &mut Transform), With<Camera>>,
    targets: Query<&Transform, (With<Player>, Without<Camera>)>,
) {
    let target = match targets.single() {
        Ok(target) => target,
        Err(_) => return,
    };

    let mut motion_delta = Vec2::ZERO;
    for event in motion_events.read() {
        motion_delta += event.delta;
    }

    let mut scroll_delta = 0.0;
    for event in wheel_events.read() {
        scroll_delta += event.y;
    }

    for (mut orbit, mut transform) in cameras.iter_mut() {
        if mouse_buttons.pressed(MouseButton::Left) {
            orbit.yaw -= motion_delta.x * 0.005;
            orbit.pitch -= motion_delta.y * 0.005;
            orbit.pitch = orbit.pitch.clamp(-1.5, 1.5);
        }

        if scroll_delta.abs() > f32::EPSILON {
            orbit.radius = (orbit.radius - scroll_delta * 0.5).clamp(2.0, 20.0);
        }

        let rotation = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(orbit.pitch);
        let offset = rotation * Vec3::new(0.0, 0.0, orbit.radius);
        transform.translation = target.translation + offset;
        transform.look_at(target.translation, Vec3::Y);
    }
}