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
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 75.0,
        affects_lightmapped_meshes: false,
    });
    spawn_test_player_cube(&mut commands, &mut meshes, &mut materials);
    commands.spawn((
        PointLight {
            intensity: 12000.0,
            range: 30.0,
            color: Color::srgb(1.0, 0.95, 0.85),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(6.0, 10.0, 6.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera {
            radius: 8.0,
            yaw: 0.0,
            pitch: -0.3,
            target_radius: 8.0,
            target_yaw: 0.0,
            target_pitch: -0.3,
            rotation_smoothness: 12.0,
            zoom_smoothness: 10.0,
            position_smoothness: 14.0,
            follow_offset: Vec3::new(0.0, 1.2, 0.0),
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
    time: Res<Time>,
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
            orbit.target_yaw -= motion_delta.x * 0.005;
            orbit.target_pitch -= motion_delta.y * 0.005;
            orbit.target_pitch = orbit.target_pitch.clamp(-1.2, 1.2);
        }

        if scroll_delta.abs() > f32::EPSILON {
            orbit.target_radius = (orbit.target_radius - scroll_delta * 0.6).clamp(3.0, 18.0);
        }

        let dt = time.delta_secs();
        let rotation_t = 1.0 - (-orbit.rotation_smoothness * dt).exp();
        let zoom_t = 1.0 - (-orbit.zoom_smoothness * dt).exp();
        let position_t = 1.0 - (-orbit.position_smoothness * dt).exp();

        orbit.yaw = orbit.yaw.lerp(orbit.target_yaw, rotation_t);
        orbit.pitch = orbit.pitch.lerp(orbit.target_pitch, rotation_t);
        orbit.radius = orbit.radius.lerp(orbit.target_radius, zoom_t);

        let rotation = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(orbit.pitch);
        let offset = rotation * Vec3::new(0.0, 0.0, orbit.radius);
        let focus_point = target.translation + orbit.follow_offset;
        let desired_translation = focus_point + offset;
        transform.translation = transform.translation.lerp(desired_translation, position_t);
        transform.look_at(focus_point, Vec3::Y);
    }
}