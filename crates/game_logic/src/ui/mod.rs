mod character_menu;

use bevy::prelude::*;
use crate::ui::character_menu::CharacterMenuLogicComponent;

pub struct UiLogicComponent;

impl Plugin for UiLogicComponent {
    fn build(&self, app: &mut App) {
        app.add_plugins(CharacterMenuLogicComponent);
    }
}