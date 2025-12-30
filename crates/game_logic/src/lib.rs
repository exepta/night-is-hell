#![feature(coverage_attribute)]

mod debug_logic;
mod camera_logic;

use bevy::prelude::*;
use game_models::states::AppState;
use crate::camera_logic::{orbit_camera_controls, setup_test_scene};
use crate::debug_logic::DebugLogicComponent;

pub struct GameLogicPlugin;

impl Plugin for GameLogicPlugin {

    #[coverage(off)]
    fn build(&self, app: &mut App) {
        app.add_plugins(DebugLogicComponent);
        app.add_systems(OnEnter(AppState::Preload), setup_test_scene);
        app.add_systems(Update, orbit_camera_controls.run_if(in_state(AppState::Preload)));
    }
}