#import bevy_pbr::{
    pbr_types,
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
    decal::clustered::apply_decal_base_color,
    mesh_view_bindings::lights,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions,
    pbr_functions::main_pass_post_lighting_processing,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

#ifdef MESHLET_MESH_MATERIAL_PASS
#import bevy_pbr::meshlet_visibility_buffer_resolve::resolve_vertex_output
#endif

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif

#ifdef FORWARD_DECAL
#import bevy_pbr::decal::forward::get_forward_decal_info
#endif

/* ────────────────────────────────────────────────────────────── */
/* EXTENDED MATERIAL UNIFORMS — Bevy 0.17 compatible               */
/* ────────────────────────────────────────────────────────────── */

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> hsr_params: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var<uniform> hsr_rim_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var<uniform> hsr_shadow_tint: vec4<f32>;

/* ────────────────────────────────────────────────────────────── */

fn apply_contrast(rgb: vec3<f32>, c: f32) -> vec3<f32> {
    return (rgb - vec3<f32>(0.5)) * c + vec3<f32>(0.5);
}

fn apply_saturation(rgb: vec3<f32>, s: f32) -> vec3<f32> {
    let gray = vec3<f32>(dot(rgb, vec3<f32>(0.299, 0.587, 0.114)));
    return mix(gray, rgb, s);
}

fn apply_rim(pbr_input: pbr_types::PbrInput, rgb: vec3<f32>, rim_mul: f32) -> vec3<f32> {
    let ndv = max(dot(pbr_input.N, pbr_input.V), 0.0);
    let rim = pow(1.0 - ndv, hsr_params.z) * hsr_params.w * rim_mul;
    return rgb + (hsr_rim_color.rgb * rim);
}

// Soft light-wrap: lifts terminator zone, stronger at silhouettes.
fn apply_light_wrap(
    rgb: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    L: vec3<f32>,
    wrap_width: f32,
    wrap_strength: f32,
) -> vec3<f32> {
    let ndotl_raw = dot(N, L);
    let ndotv = max(dot(N, V), 0.0);

    // wrap near terminator (ndotl ~ 0) and slightly into shadow (negative ndotl)
    let wrap = smoothstep(-wrap_width, 0.0, ndotl_raw);

    // stronger near silhouette
    let view_fade = 1.0 - ndotv;

    // reduce wrap when facing the light strongly
    let ndotl = max(ndotl_raw, 0.0);
    let facing = 1.0 - smoothstep(0.25, 0.85, ndotl);

    let w = wrap * view_fade * facing * wrap_strength;

    return rgb + rgb * w;
}

// Read material id from Vertex Color R.
// If mesh has no vertex colors, default to cloth-ish (0.5).
fn get_mat_id(in: VertexOutput) -> f32 {
#ifdef VERTEX_COLORS
    // clamp to avoid weird imports
    return clamp(in.color.r, 0.0, 1.0);
#else
    return 0.5;
#endif
}

fn toon_direct_lighting_zzz(pbr_input: pbr_types::PbrInput, mat_id: f32) -> vec4<f32> {
    let base = pbr_input.material.base_color;

    // Overall scene tuning
    let AMBIENT_SCALE: f32 = 0.1;
    let DIR_SCALE: f32 = 0.00008;

    let ambient = lights.ambient_color.rgb * AMBIENT_SCALE;

    var dir_rgb = vec3<f32>(0.0);
    var L = vec3<f32>(0.0, 1.0, 0.0);

    if (lights.n_directional_lights > 0u) {
        let sun = lights.directional_lights[0];
        dir_rgb = sun.color.rgb * DIR_SCALE;
        L = normalize(sun.direction_to_light);
    }

    let ndotl = max(dot(pbr_input.N, L), 0.0);

    // ──────────────────────────────────────────
    // Material class weights (soft selection)
    // skin:  0.0
    // cloth: 0.5
    // hair:  1.0
    let w_skin  = 1.0 - smoothstep(0.2, 0.35, mat_id);
    let w_hair  = smoothstep(0.65, 0.85, mat_id);
    let w_cloth = clamp(1.0 - w_skin - w_hair, 0.0, 1.0);
    // ──────────────────────────────────────────

    // Per-material ramp tuning:
    // Hair: harder bands (smaller softness), Cloth: base, Skin: slightly softer.
    let pivot = hsr_params.x
        + w_skin * (-0.02)
        + w_hair * ( 0.02);

    let softness = max(hsr_params.y, 0.001)
        * (1.0
            + w_skin * 0.25   // softer
            + w_hair * -0.35  // harder
        );

    let t0 = pivot - softness;
    let t1 = pivot + softness;

    let s0 = smoothstep(t0, t0 + softness, ndotl);
    let s1 = smoothstep(t1, t1 + softness, ndotl);

    let w_shadow = 1.0 - s0;
    let w_mid    = s0 * (1.0 - s1);
    let w_light  = s1;

    // Per-material shadow tint:
    // - skin: warmer shadows
    // - hair: a bit cooler
    // - cloth: your base tint
    let skin_shadow_tint = vec3<f32>(0.78, 0.62, 0.56);
    let hair_shadow_tint = vec3<f32>(0.55, 0.60, 0.80);
    let cloth_shadow_tint = hsr_shadow_tint.rgb;

    let shadow_tint = cloth_shadow_tint * w_cloth
        + skin_shadow_tint * w_skin
        + hair_shadow_tint * w_hair;

    // Highlights:
    // Skin: less aggressive (avoid chalky white)
    // Hair: more punchy
    let skin_light_mul = vec3<f32>(1.04, 1.03, 1.01);
    let cloth_light_mul = vec3<f32>(1.07, 1.05, 1.02);
    let hair_light_mul  = vec3<f32>(1.10, 1.08, 1.03);

    let light_mul = cloth_light_mul * w_cloth
        + skin_light_mul * w_skin
        + hair_light_mul * w_hair;

    let shadow_rgb = base.rgb * shadow_tint;
    let mid_rgb    = base.rgb;
    let light_rgb  = base.rgb * light_mul;

    let band_rgb = shadow_rgb * w_shadow + mid_rgb * w_mid + light_rgb * w_light;

    var lit = band_rgb * (ambient + dir_rgb * ndotl);

    // Per-material post shaping:
    // Skin: less contrast, a bit warmer; Hair: more contrast; Cloth: neutral
    let contrast = 1.10
        + w_skin * (-0.06)
        + w_hair * ( 0.10);

    let saturation = 1.08
        + w_skin * (-0.03)
        + w_hair * ( 0.06);

    lit = apply_contrast(lit, contrast);
    lit = apply_saturation(lit, saturation);

    // Light wrap per material:
    // Skin gets more wrap, cloth less, hair moderate.
    let wrap_width = 0.35 + w_skin * 0.06 + w_hair * -0.03;
    let wrap_strength = 0.35 + w_skin * 0.18 + w_cloth * -0.10;

    lit = apply_light_wrap(lit, pbr_input.N, pbr_input.V, L, wrap_width, wrap_strength);

    // Rim per material:
    // Hair stronger rim, skin weaker, cloth normal
    let rim_mul = 1.0 + w_hair * 0.55 + w_skin * (-0.25);
    lit = apply_rim(pbr_input, lit, rim_mul);

    // Optional: stop skin from clipping into pure white a bit
    // (only affects skin mostly)
    let skin_cap = 0.92;
    lit = mix(lit, min(lit, vec3<f32>(skin_cap)), w_skin * 0.6);

    // Safety clamp
    lit = clamp(lit, vec3<f32>(0.0), vec3<f32>(50.0));

    return vec4<f32>(lit, base.a);
}

@fragment
fn fragment(
#ifdef MESHLET_MESH_MATERIAL_PASS
    @builtin(position) frag_coord: vec4<f32>,
#else
    vertex_output: VertexOutput,
    @builtin(front_facing) is_front: bool,
#endif
) -> FragmentOutput {
#ifdef MESHLET_MESH_MATERIAL_PASS
    let vertex_output = resolve_vertex_output(frag_coord);
    let is_front = true;
#endif

    var in = vertex_output;

#ifdef VISIBILITY_RANGE_DITHER
    pbr_functions::visibility_range_dither(in.position, in.visibility_range_dither);
#endif

#ifdef FORWARD_DECAL
    let forward_decal_info = get_forward_decal_info(in);
    in.world_position = forward_decal_info.world_position;
    in.uv = forward_decal_info.uv;
#endif

    // read mat id BEFORE pbr_input (so it's always available)
    let mat_id = get_mat_id(in);

    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Cutout alpha
    pbr_input.material.base_color =
        alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // Clustered decals
    pbr_input.material.base_color = apply_decal_base_color(
        in.world_position.xyz,
        in.position.xy,
        pbr_input.material.base_color,
    );

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;

    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = toon_direct_lighting_zzz(pbr_input, mat_id);
    } else {
        out.color = pbr_input.material.base_color;
    }

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

#ifdef OIT_ENABLED
    let alpha_mode =
        pbr_input.material.flags &
        pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;

    if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        oit_draw(in.position, out.color);
        discard;
    }
#endif

#ifdef FORWARD_DECAL
    out.color.a = min(forward_decal_info.alpha, out.color.a);
#endif

    return out;
}
