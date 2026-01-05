#![feature(coverage_attribute)]

mod debug_logic;
mod camera_logic;
mod loading;
mod ui;

use bevy::prelude::*;
use game_models::states::{AppState, InGameStates};
use crate::camera_logic::{orbit_camera_controls, setup_test_scene};
use crate::debug_logic::DebugLogicComponent;
use crate::loading::LoadingLogicComponent;
use crate::ui::UiLogicComponent;

pub struct GameLogicPlugin;

impl Plugin for GameLogicPlugin {

    #[coverage(off)]
    fn build(&self, app: &mut App) {
        app.add_plugins((DebugLogicComponent, LoadingLogicComponent, UiLogicComponent));
        app.add_systems(OnEnter(AppState::InGame(InGameStates::CharacterMenu)), setup_test_scene);
        app.add_systems(Update, orbit_camera_controls.run_if(in_state(AppState::InGame(InGameStates::CharacterMenu))));
    }
}