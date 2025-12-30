use bevy::prelude::*;

#[derive(Component, Debug, Clone, Reflect)]
pub struct OrbitCamera {
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}