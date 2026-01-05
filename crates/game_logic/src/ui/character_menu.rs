use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use game_models::config::GlobalConfig;
use game_models::entities::character::{Character, CharacterDisplay};
use game_models::entities::Characters;
use game_models::states::{AppState, InGameStates};

pub struct CharacterMenuLogicComponent;

#[derive(Resource, Default)]
struct CharacterRoulette {
    index: usize,
}

impl Plugin for CharacterMenuLogicComponent {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterRoulette>();
        app.add_systems(
            Update,
            (
                ensure_character_display,
                handle_character_roulette_input,
            )
                .run_if(in_state(AppState::InGame(InGameStates::CharacterMenu))),
        );
    }
}

fn ensure_character_display(
    mut commands: Commands,
    mut roulette: ResMut<CharacterRoulette>,
    characters: Res<Characters>,
    display_query: Query<Entity, With<CharacterDisplay>>,
    asset_server: Res<AssetServer>,
) {
    if characters.0.is_empty() {
        return;
    }

    if !display_query.is_empty() {
        return;
    }

    if roulette.index >= characters.0.len() {
        roulette.index = 0;
    }

    let character = characters.0[roulette.index].clone();
    spawn_character_display(&mut commands, &asset_server, character);
}

fn handle_character_roulette_input(
    mut commands: Commands,
    mut roulette: ResMut<CharacterRoulette>,
    characters: Res<Characters>,
    config: Res<GlobalConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    display_query: Query<Entity, With<CharacterDisplay>>,
    asset_server: Res<AssetServer>,
) {
    if characters.0.is_empty() {
        return;
    }

    let left_key = config.input_config.get_move_left_key();
    let right_key = config.input_config.get_move_right_key();
    let mut direction = 0_i32;

    if keyboard.just_pressed(left_key) {
        direction -= 1;
    }

    if keyboard.just_pressed(right_key) {
        direction += 1;
    }

    if direction == 0 {
        return;
    }

    let len = characters.0.len();
    if direction > 0 {
        roulette.index = (roulette.index + 1) % len;
    } else {
        roulette.index = (roulette.index + len - 1) % len;
    }

    for entity in display_query.iter() {
        commands.entity(entity).despawn();
    }

    let character = characters.0[roulette.index].clone();
    spawn_character_display(&mut commands, &asset_server, character);
}

fn spawn_character_display(
    commands: &mut Commands,
    asset_server: &AssetServer,
    mut character: Character,
) -> Entity {
    let (name, model_path) = match character.base_info.as_ref() {
        Some(info) => (info.name.clone(), info.model_path.clone()),
        None => ("unknown_character".to_string(), "".to_string()),
    };

    if model_path.is_empty() {
        warn!("Character '{name}' has no model_path set.");
        let entity = commands
            .spawn((
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLayers::from_layers(&[0]),
                CharacterDisplay,
                Name::new(format!("CharacterDisplay::{name}")),
            ))
            .id();

        character.id = Some(entity);
        commands.entity(entity).insert(character);
        return entity;
    }

    let scene: Handle<Scene> = asset_server.load(format!("{model_path}#Scene0"));

    let entity = commands
        .spawn((
            SceneRoot(scene),
            Transform::from_xyz(0.0, 0.0, 0.0),
            RenderLayers::from_layers(&[0]),
            CharacterDisplay,
            Name::new(format!("CharacterDisplay::{name}")),
        ))
        .id();

    character.id = Some(entity);
    commands.entity(entity).insert(character);
    entity
}