#![feature(coverage_attribute)]

mod debug_logic;

use bevy::prelude::*;
use crate::debug_logic::DebugLogicComponent;

pub struct GameLogicPlugin;

impl Plugin for GameLogicPlugin {

    #[coverage(off)]
    fn build(&self, app: &mut App) {
        app.add_plugins(DebugLogicComponent);
    }
}