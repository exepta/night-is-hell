pub mod player;
pub mod character;

use bevy::prelude::*;

pub struct EntitiesModule;

impl Plugin for EntitiesModule {

    #[coverage(off)]
    fn build(&self, _app: &mut App) {
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct EntityBaseInformation {
    pub id: Entity,
    pub display_name: String,
    pub localized_name: String
}

impl Default for EntityBaseInformation {
    fn default() -> Self {
        Self {
            id: Entity::PLACEHOLDER,
            display_name: String::new(),
            localized_name: String::new()
        }
    }
}