use std::collections::HashSet;

use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, OpaqueRendererMethod};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use game_models::entities::character::CharacterDisplay;
use game_models::states::{AppState, InGameStates};

const HSR_TOON_SHADER: &str = "shaders/test_toon.wgsl";


type HsrToonMaterial = ExtendedMaterial<StandardMaterial, HsrToonExtension>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct HsrToonExtension {
    #[uniform(100)]
    toon_params: Vec4,
    #[uniform(101)]
    rim_color: Vec4,
    #[uniform(102)]
    shadow_tint: Vec4,
}

impl Default for HsrToonExtension {
    fn default() -> Self {
        Self {
            toon_params: Vec4::new(0.55, 0.08, 3.0, 0.20),
            rim_color: Vec4::new(0.9, 0.9, 1.0, 1.0),
            shadow_tint: Vec4::new(0.60, 0.62, 0.75, 1.0),
        }
    }
}

impl MaterialExtension for HsrToonExtension {
    fn fragment_shader() -> ShaderRef {
        HSR_TOON_SHADER.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        HSR_TOON_SHADER.into()
    }
}

#[derive(Component)]
struct HsrToonMaterialApplied;

pub struct HsrToonPlugin;

impl Plugin for HsrToonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<HsrToonMaterial>::default());
        app.add_systems(
            Update,
            (
                apply_hsr_toon_materials,
                debug_uvs
            ).run_if(in_state(AppState::InGame(InGameStates::CharacterMenu))),
        );
    }
}

fn apply_hsr_toon_materials(
    mut commands: Commands,
    mut toon_materials: ResMut<Assets<HsrToonMaterial>>,
    standard_materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    display_query: Query<Entity, With<CharacterDisplay>>,
    parent_query: Query<&ChildOf>,
    mesh_query: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Without<HsrToonMaterialApplied>>,
) {
    if display_query.is_empty() {
        return;
    }

    let display_entities: HashSet<Entity> = display_query.iter().collect();

    for (entity, material_comp) in &mesh_query {
        if !is_descendant_of_display(entity, &display_entities, &parent_query) {
            continue;
        }

        let Some(original) = standard_materials.get(material_comp) else { continue; };

        let (img_loaded, img_size) = if let Some(tex) = &original.base_color_texture {
            match images.get(tex) {
                Some(img) => (true, Some(img.size())),
                None => (false, None),
            }
        } else {
            (false, None)
        };

        info!(
            "toon swap entity={:?} base_color_tex={} image_loaded={} image_size={:?}",
            entity,
            original.base_color_texture.is_some(),
            img_loaded,
            img_size
        );

        if original.base_color_texture.is_some() && !img_loaded {
            continue;
        }

        let mut base = original.clone();
        base.opaque_render_method = OpaqueRendererMethod::Forward;

        let toon_handle = toon_materials.add(HsrToonMaterial {
            base,
            extension: HsrToonExtension::default(),
        });

        commands.entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(toon_handle), HsrToonMaterialApplied));
    }
}

fn is_descendant_of_display(
    entity: Entity,
    display_entities: &HashSet<Entity>,
    parent_query: &Query<&ChildOf>,
) -> bool {
    let mut current = Some(entity);

    while let Some(entity) = current {
        if display_entities.contains(&entity) {
            return true;
        }

        current = parent_query.get(entity).ok().map(ChildOf::parent);
    }

    false
}

fn debug_uvs(
    meshes: Res<Assets<Mesh>>,
    q: Query<(Entity, &Mesh3d), With<CharacterDisplay>>,
) {
    for (e, mesh3d) in &q {
        if let Some(mesh) = meshes.get(mesh3d) {
            let has_uv0 = mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some();
            let has_pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some();
            info!("mesh entity={:?} has_pos={} has_uv0={}", e, has_pos, has_uv0);
        }
    }
}