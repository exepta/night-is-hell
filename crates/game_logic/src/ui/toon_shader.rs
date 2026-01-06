use std::collections::HashSet;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, OpaqueRendererMethod};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures};
use bevy::core_pipeline::FullscreenShader;

use bevy::ecs::query::QueryItem;
use bevy::render::{
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::{RenderContext, RenderDevice},
    view::ViewTarget,
    RenderApp, RenderStartup,
};
use bevy::render::render_resource::binding_types::texture_2d_multisampled;
use game_models::entities::character::CharacterDisplay;
use game_models::states::{AppState, InGameStates};

const HSR_TOON_SHADER: &str = "shaders/test_toon.wgsl";
const OUTLINE_POST_SHADER: &str = "shaders/outline_postprocess.wgsl";

pub type HsrToonMaterial = ExtendedMaterial<StandardMaterial, HsrToonExtension>;

/// ─────────────────────────────────────────
/// TOON MATERIAL
/// ─────────────────────────────────────────
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct HsrToonExtension {
    #[uniform(100)]
    pub toon_params: Vec4,
    #[uniform(101)]
    pub rim_color: Vec4,
    #[uniform(102)]
    pub shadow_tint: Vec4,
}

impl Default for HsrToonExtension {
    fn default() -> Self {
        Self {
            toon_params: Vec4::new(0.55, 0.10, 3.0, 0.20),
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

/// ─────────────────────────────────────────
/// OUTLINE POSTPROCESS SETTINGS
/// ─────────────────────────────────────────
#[derive(Component, Clone, Copy, ShaderType, ExtractComponent, Reflect)]
#[reflect(Component)]
pub struct OutlinePostProcessSettings {
    /// Pixel-ish thickness in screen space
    pub thickness: f32,
    /// depth edge sensitivity
    pub depth_threshold: f32,
    /// normal edge sensitivity
    pub normal_threshold: f32,
    pub color: Vec4,
}

impl Default for OutlinePostProcessSettings {
    fn default() -> Self {
        Self {
            thickness: 0.5,
            depth_threshold: 0.0045,
            normal_threshold: 0.45,
            color: Vec4::new(0.05, 0.06, 0.08, 1.0),
        }
    }
}

pub struct HsrToonPlugin;

impl Plugin for HsrToonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<HsrToonMaterial>::default());

        app.add_plugins((
            ExtractComponentPlugin::<OutlinePostProcessSettings>::default(),
            UniformComponentPlugin::<OutlinePostProcessSettings>::default(),
        ));

        // RenderGraph / Pipeline Setup im RenderApp
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(RenderStartup, init_outline_post_pipeline);
        render_app
            .add_render_graph_node::<ViewNodeRunner<OutlinePostProcessNode>>(
                Core3d,
                OutlinePostProcessLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Tonemapping,
                    OutlinePostProcessLabel,
                    Node3d::EndMainPassPostProcessing,
                ),
            );

        app.add_systems(
            Update,
            (
                apply_hsr_toon_materials,
                ensure_outline_camera_components,
            )
                .run_if(in_state(AppState::InGame(InGameStates::CharacterMenu))),
        );
    }
}

/// ─────────────────────────────────────────
/// Toon-Material
/// ─────────────────────────────────────────
fn apply_hsr_toon_materials(
    mut commands: Commands,
    mut toon_materials: ResMut<Assets<HsrToonMaterial>>,
    standard_materials: Res<Assets<StandardMaterial>>,
    display_query: Query<Entity, With<CharacterDisplay>>,
    parent_query: Query<&ChildOf>,
    mesh_query: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        Without<HsrToonMaterialApplied>,
    >,
) {
    if display_query.is_empty() {
        return;
    }
    let display_entities: HashSet<Entity> = display_query.iter().collect();

    for (entity, mat_comp) in &mesh_query {
        if !is_descendant_of_display(entity, &display_entities, &parent_query) {
            continue;
        }

        let Some(original) = standard_materials.get(mat_comp) else { continue };

        let mut base = original.clone();
        base.opaque_render_method = OpaqueRendererMethod::Forward;

        let toon = toon_materials.add(HsrToonMaterial {
            base,
            extension: HsrToonExtension::default(),
        });

        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(toon), HsrToonMaterialApplied));
    }
}

fn ensure_outline_camera_components(
    mut commands: Commands,
    cameras: Query<Entity, With<Camera3d>>,
) {
    for cam in &cameras {
        commands
            .entity(cam)
            .insert((
                DepthPrepass,
                NormalPrepass,
                OutlinePostProcessSettings::default(),
            ));
    }
}

/// ─────────────────────────────────────────
/// Hierarchy helper
/// ─────────────────────────────────────────
fn is_descendant_of_display(
    entity: Entity,
    display_entities: &HashSet<Entity>,
    parent_query: &Query<&ChildOf>,
) -> bool {
    let mut current = Some(entity);
    while let Some(e) = current {
        if display_entities.contains(&e) {
            return true;
        }
        current = parent_query.get(e).ok().map(ChildOf::parent);
    }
    false
}

/// ─────────────────────────────────────────
/// RENDER APP: Outline Postprocess Node + Pipeline
/// ─────────────────────────────────────────
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct OutlinePostProcessLabel;

#[derive(Default)]
struct OutlinePostProcessNode;

impl ViewNode for OutlinePostProcessNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static OutlinePostProcessSettings,
        &'static DynamicUniformIndex<OutlinePostProcessSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, prepass, _settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some(depth_view) = prepass.depth_view() else { return Ok(()) };
        let Some(normal_view) = prepass.normal_view() else { return Ok(()) };

        let pipeline_res = world.resource::<OutlinePostProcessPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_res.pipeline_id) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<OutlinePostProcessSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        // source/destination swap
        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "outline_postprocess_bind_group",
            &pipeline_res.layout,
            &BindGroupEntries::sequential((
                post_process.source,          // scene color
                depth_view,                   // prepass depth (color attachment)
                normal_view,                  // prepass normals
                &pipeline_res.sampler,
                settings_binding.clone(),     // uniform buffer (dynamic)
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("outline_postprocess_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct OutlinePostProcessPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn init_outline_post_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    // BindGroupLayout:
    // 0 scene color
    // 1 depth prepass (color texture)
    // 2 normal prepass (color texture)
    // 3 sampler
    // 4 uniform
    let layout = render_device.create_bind_group_layout(
        "outline_postprocess_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),              // scene color (resolved)
                texture_2d_multisampled(TextureSampleType::Float { filterable: false }),// depth MSAA
                texture_2d_multisampled(TextureSampleType::Float { filterable: false }),// normal MSAA
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<OutlinePostProcessSettings>(true),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let shader = asset_server.load(OUTLINE_POST_SHADER);

    let pipeline_descriptor = RenderPipelineDescriptor {
        label: Some("outline_postprocess_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader,
            shader_defs: vec![],
            entry_point: Some("fragment".into()),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: false,
    };

    let pipeline_id = pipeline_cache.queue_render_pipeline(pipeline_descriptor);

    commands.insert_resource(OutlinePostProcessPipeline {
        layout,
        sampler,
        pipeline_id,
    });
}
