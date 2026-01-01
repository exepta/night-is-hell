use bevy::prelude::*;

#[derive(Component, Debug, Clone, Reflect)]
pub struct OrbitCamera {
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub target_radius: f32,
    pub target_yaw: f32,
    pub target_pitch: f32,
    pub rotation_smoothness: f32,
    pub zoom_smoothness: f32,
    pub position_smoothness: f32,
    pub follow_offset: Vec3,
}