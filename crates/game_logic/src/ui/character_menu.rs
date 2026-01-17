use bevy::asset::LoadState;
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

#[derive(Resource, Default)]
struct CharacterAnimationSelection {
    index: usize,
    needs_apply: bool,
}

#[derive(Resource, Default)]
struct CharacterMorphSelection {
    index: usize,
    needs_apply: bool,
}

#[derive(Resource, Default)]
struct PreloadedAnimationClips {
    display_entity: Option<Entity>,
    indices: Vec<usize>,
    clips: Vec<Handle<AnimationClip>>,
}

#[derive(Resource, Default)]
struct CharacterMorphTargetsCache {
    display_entity: Option<Entity>,
    targets: Vec<Entity>,
    target_count: usize,
}

#[derive(Component)]
struct CachedAnimNodes {
    nodes: Vec<AnimationNodeIndex>,
}

#[derive(Component)]
struct CharacterRigPlayer;

impl Plugin for CharacterMenuLogicComponent {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterRoulette>();
        app.init_resource::<CharacterAnimationSelection>();
        app.init_resource::<CharacterMorphSelection>();
        app.init_resource::<PreloadedAnimationClips>();
        app.init_resource::<CharacterMorphTargetsCache>();

        app.add_systems(
            Update,
            (
                ensure_character_display,
                handle_character_roulette_input,
                cache_character_morph_targets,
                handle_character_animation_input,
                handle_character_morph_input,
                mark_rig_player_when_ready,
                preload_character_animation_clips,
                build_graph_cache_when_loaded,
                apply_character_animation,
                apply_character_morph_targets,
            )
                .chain()
                .run_if(in_state(AppState::InGame(InGameStates::CharacterMenu))),
        );
    }
}

fn ensure_character_display(
    mut commands: Commands,
    mut roulette: ResMut<CharacterRoulette>,
    mut animation_selection: ResMut<CharacterAnimationSelection>,
    mut morph_selection: ResMut<CharacterMorphSelection>,
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

    animation_selection.index = 0;
    animation_selection.needs_apply = true;
    morph_selection.index = 0;
    morph_selection.needs_apply = true;
}

fn handle_character_roulette_input(
    mut commands: Commands,
    mut roulette: ResMut<CharacterRoulette>,
    mut animation_selection: ResMut<CharacterAnimationSelection>,
    mut morph_selection: ResMut<CharacterMorphSelection>,
    mut preload: ResMut<PreloadedAnimationClips>,
    mut morph_cache: ResMut<CharacterMorphTargetsCache>,
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

    *preload = PreloadedAnimationClips::default();
    *morph_cache = CharacterMorphTargetsCache::default();

    let character = characters.0[roulette.index].clone();
    spawn_character_display(&mut commands, &asset_server, character);

    animation_selection.index = 0;
    animation_selection.needs_apply = true;
    morph_selection.index = 0;
    morph_selection.needs_apply = true;
}

fn cache_character_morph_targets(
    mut morph_cache: ResMut<CharacterMorphTargetsCache>,
    display_query: Query<Entity, With<CharacterDisplay>>,
    children_query: Query<&Children>,
    morph_query: Query<&MorphWeights>,
) {
    let Some(display) = display_query.iter().next() else { return; };

    let needs_refresh = morph_cache.display_entity != Some(display) || morph_cache.targets.is_empty();
    if !needs_refresh {
        return;
    }

    fn walk(
        e: Entity,
        children_q: &Query<&Children>,
        morph_q: &Query<&MorphWeights>,
        targets: &mut Vec<Entity>,
        target_count: &mut usize,
    ) {
        if let Ok(weights) = morph_q.get(e) {
            if *target_count == 0 {
                *target_count = weights.weights().len();
            }
            targets.push(e);
        }
        if let Ok(children) = children_q.get(e) {
            for c in children.iter() {
                walk(c, children_q, morph_q, targets, target_count);
            }
        }
    }

    let mut targets = Vec::new();
    let mut target_count = 0;
    walk(
        display,
        &children_query,
        &morph_query,
        &mut targets,
        &mut target_count,
    );

    if targets.is_empty() || target_count == 0 {
        return;
    }

    morph_cache.display_entity = Some(display);
    morph_cache.targets = targets;
    morph_cache.target_count = target_count;
}

fn handle_character_animation_input(
    mut animation_selection: ResMut<CharacterAnimationSelection>,
    preload: Res<PreloadedAnimationClips>,
    config: Res<GlobalConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let animation_count = preload.clips.len();
    if animation_count == 0 {
        return;
    }

    let up_key = config.input_config.get_move_up_key();
    let down_key = config.input_config.get_move_down_key();
    let mut direction = 0_i32;

    if keyboard.just_pressed(up_key) {
        direction += 1;
    }
    if keyboard.just_pressed(down_key) {
        direction -= 1;
    }
    if direction == 0 {
        return;
    }

    if direction > 0 {
        animation_selection.index = (animation_selection.index + 1) % animation_count;
    } else {
        animation_selection.index = (animation_selection.index + animation_count - 1) % animation_count;
    }

    animation_selection.needs_apply = true;
}

fn handle_character_morph_input(
    mut morph_selection: ResMut<CharacterMorphSelection>,
    morph_cache: Res<CharacterMorphTargetsCache>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if morph_cache.target_count == 0 {
        return;
    }

    if !keyboard.just_pressed(KeyCode::KeyQ) {
        return;
    }

    morph_selection.index = (morph_selection.index + 1) % morph_cache.target_count;
    morph_selection.needs_apply = true;
}

fn mark_rig_player_when_ready(
    mut commands: Commands,
    display_query: Query<Entity, With<CharacterDisplay>>,
    children_query: Query<&Children>,
    player_query: Query<Entity, With<AnimationPlayer>>,
    already_marked: Query<(), With<CharacterRigPlayer>>,
    names: Query<&Name>,
) {
    if !already_marked.is_empty() {
        return;
    }
    let Some(display) = display_query.iter().next() else { return; };

    fn walk(
        e: Entity,
        depth: usize,
        children_q: &Query<&Children>,
        player_q: &Query<Entity, With<AnimationPlayer>>,
        out: &mut Vec<(Entity, usize)>,
    ) {
        if player_q.get(e).is_ok() {
            out.push((e, depth));
        }
        if let Ok(children) = children_q.get(e) {
            for c in children.iter() {
                walk(c, depth + 1, children_q, player_q, out);
            }
        }
    }

    let mut players = Vec::new();
    walk(display, 0, &children_query, &player_query, &mut players);
    if players.is_empty() {
        return;
    }

    players.sort_by_key(|(_, d)| *d);
    let (rig_player, depth) = players[players.len() - 1];

    if let Ok(n) = names.get(rig_player) {
        info!("Marking Rig AnimationPlayer {:?} depth={} name={}", rig_player, depth, n.as_str());
    } else {
        info!("Marking Rig AnimationPlayer {:?} depth={}", rig_player, depth);
    }

    commands.entity(rig_player).insert(CharacterRigPlayer);
}

fn preload_character_animation_clips(
    mut preload: ResMut<PreloadedAnimationClips>,
    display_query: Query<(Entity, &Character), With<CharacterDisplay>>,
    asset_server: Res<AssetServer>,
) {
    let Some((display_entity, character)) = display_query.iter().next() else { return; };
    let Some(base) = character.base_info.as_ref() else { return; };
    if base.model_path.is_empty() || base.animations.is_empty() {
        return;
    }

    if preload.display_entity == Some(display_entity) && !preload.clips.is_empty() {
        return;
    }

    preload.display_entity = Some(display_entity);
    preload.indices.clear();
    preload.clips.clear();

    for a in &base.animations {
        let clip: Handle<AnimationClip> =
            asset_server.load(format!("{}#Animation{}", base.model_path, a.index));
        preload.indices.push(a.index as usize);
        preload.clips.push(clip);
    }

    info!("Preloading {} animation clips for {}", preload.clips.len(), base.model_path);
}

fn build_graph_cache_when_loaded(
    preload: Res<PreloadedAnimationClips>,
    rig_player_query: Query<Entity, With<CharacterRigPlayer>>,
    cached_query: Query<(), With<CachedAnimNodes>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    let Ok(rig_player_entity) = rig_player_query.single() else {
        return;
    };

    if cached_query.get(rig_player_entity).is_ok() {
        return;
    }

    if preload.clips.is_empty() {
        return;
    }

    let all_loaded = preload.clips.iter().all(|h| {
        matches!(asset_server.get_load_state(h.id()), Some(LoadState::Loaded))
    });

    if !all_loaded {
        return;
    }

    let mut graph = AnimationGraph::new();
    let mut nodes = Vec::with_capacity(preload.clips.len());
    for h in preload.clips.iter().cloned() {
        nodes.push(graph.add_clip(h, 1.0, graph.root));
    }

    let graph_handle = graphs.add(graph);

    commands.entity(rig_player_entity).insert((
        AnimationGraphHandle(graph_handle),
        CachedAnimNodes { nodes: nodes.clone() },
    ));

    info!("Animation graph cached with {} nodes", nodes.len());
}

fn apply_character_animation(
    mut animation_selection: ResMut<CharacterAnimationSelection>,
    rig_player_query: Query<Entity, With<CharacterRigPlayer>>,
    mut players: Query<(&mut AnimationPlayer, Option<&CachedAnimNodes>)>,
) {
    if !animation_selection.needs_apply {
        return;
    }

    let Ok(rig_player_entity) = rig_player_query.single() else {
        return;
    };

    let Ok((mut player, cached_opt)) = players.get_mut(rig_player_entity) else {
        return;
    };

    let Some(cached) = cached_opt else {
        return;
    };

    if cached.nodes.is_empty() {
        return;
    }

    let idx = animation_selection
        .index
        .min(cached.nodes.len().saturating_sub(1));

    player.stop_all();
    player.play(cached.nodes[idx]).repeat();

    animation_selection.needs_apply = false;
}

fn apply_character_morph_targets(
    mut morph_selection: ResMut<CharacterMorphSelection>,
    morph_cache: Res<CharacterMorphTargetsCache>,
    mut morph_query: Query<&mut MorphWeights>,
) {
    if !morph_selection.needs_apply {
        return;
    }
    if morph_cache.target_count == 0 {
        return;
    }

    let index = morph_selection
        .index
        .min(morph_cache.target_count.saturating_sub(1));

    for entity in morph_cache.targets.iter().copied() {
        let Ok(mut weights) = morph_query.get_mut(entity) else { continue; };
        let weights = weights.weights_mut();
        for weight in weights.iter_mut() {
            *weight = 0.0;
        }
        if let Some(weight) = weights.get_mut(index) {
            *weight = 1.0;
        }
    }

    morph_selection.needs_apply = false;
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
