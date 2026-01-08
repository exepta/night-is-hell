mod character_menu;
mod toon_shader;

use bevy::prelude::*;
use crate::ui::character_menu::CharacterMenuLogicComponent;
use crate::ui::toon_shader::HsrToonPlugin;

pub struct UiLogicComponent;

impl Plugin for UiLogicComponent {
    fn build(&self, app: &mut App) {
        app.add_plugins((CharacterMenuLogicComponent, HsrToonPlugin));
    }
}