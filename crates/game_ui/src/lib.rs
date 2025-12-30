#![feature(coverage_attribute)]

use bevy::prelude::*;
use bevy_extended_ui::ExtendedUiPlugin;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    
    #[coverage(off)]
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtendedUiPlugin);
    }
}