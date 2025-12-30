#![feature(coverage_attribute)]

pub mod states;
pub mod v_ram_detection;
pub mod config;
pub mod key_utils;
pub mod debug;
pub mod entities;

use bevy::prelude::*;
use crate::entities::EntitiesModule;

/// Core of all game relevant resources and structures. This Plugin initializes resources
/// with `init_resource` from bevy. This Plugin is registered at [`ManagerPlugin`] which is
/// a part of the main.rs file.
pub struct GameCorePlugin;

impl Plugin for GameCorePlugin {

    #[coverage(off)]
    fn build(&self, app: &mut App) {
        app.add_plugins(EntitiesModule);
    }

}