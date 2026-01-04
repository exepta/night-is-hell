use bevy::prelude::*;
use game_models::entities::{Characters, EntitiesData, EntityDataType};
use game_models::entities::character::Character;
use crate::loading::load_all_entities_data;

pub struct CharacterLoadComponent;

impl Plugin for CharacterLoadComponent {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load_character_data
            .after(load_all_entities_data)
            .run_if(resource_changed::<EntitiesData>));
    }
}

fn load_character_data(mut characters: ResMut<Characters>, entities_data: Res<EntitiesData>) {
    if entities_data.0.is_empty() { return; }
    for entity in entities_data.0.iter() {
        if entity._type == EntityDataType::Character {
            let character = Character::from_entity_base_information(entity.clone());
            if characters.0.contains(&character) { continue; }
            characters.0.push(character);
        }
    }

    debug!("Loaded {} characters", characters.0.len());
}

