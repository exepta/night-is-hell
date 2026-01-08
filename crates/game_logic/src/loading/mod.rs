mod character_load;

use bevy::prelude::*;
use game_models::entities::{EntitiesData, EntityBaseInformation};
use game_models::states::{AppState, InGameStates};
use crate::loading::character_load::CharacterLoadComponent;

pub struct LoadingLogicComponent;

impl Plugin for LoadingLogicComponent {
    fn build(&self, app: &mut App) {
        app.add_plugins(CharacterLoadComponent);
        app.add_systems(OnEnter(AppState::InGame(InGameStates::CharacterMenu)), load_all_entities_data);
    }
}

pub(super) fn load_all_entities_data(mut entities: ResMut<EntitiesData>) {
    let results = EntityBaseInformation::fetch_all();
    if let Ok(entries) = results {
        entities.0 = entries;
    }

    debug!("Pre Loaded {} entities data", entities.0.len());
}