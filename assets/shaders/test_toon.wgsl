#import bevy_pbr::{
    pbr_types,
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
    decal::clustered::apply_decals,
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

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> hsr_params: vec4<f32>; // x=pivot, y=softness, z=rim_power, w=rim_intensity

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var<uniform> hsr_rim_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var<uniform> hsr_shadow_tint: vec4<f32>;

fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }

fn smooth_band(x: f32, edge0: f32, edge1: f32) -> f32 {
    return smoothstep(edge0, edge1, x);
}

fn get_mat_id(in: VertexOutput) -> f32 {
#ifdef VERTEX_COLORS
    return clamp(in.color.r, 0.0, 1.0);
#else
    return 0.5;
#endif
}

fn apply_rim(pbr_input: pbr_types::PbrInput, rgb: vec3<f32>, rim_mul: f32) -> vec3<f32> {
    let ndv = saturate(dot(pbr_input.N, pbr_input.V));
    let rim_raw = pow(1.0 - ndv, max(hsr_params.z, 0.001));
    let rim = rim_raw * hsr_params.w * rim_mul;
    return rgb + (hsr_rim_color.rgb * rim);
}

fn apply_light_wrap(
    rgb: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    L: vec3<f32>,
    wrap_width: f32,
    wrap_strength: f32,
) -> vec3<f32> {
    let ndotl_raw = dot(N, L);
    let ndotv = saturate(dot(N, V));

    let wrap = smoothstep(-wrap_width, 0.0, ndotl_raw);
    let view_fade = 1.0 - ndotv;

    let ndotl = max(ndotl_raw, 0.0);
    let facing = 1.0 - smoothstep(0.2, 0.85, ndotl);

    let w = wrap * view_fade * facing * wrap_strength;
    return rgb * (1.0 + w * 0.35);
}

fn toon_specular(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, mat_id: f32) -> f32 {
    let H = normalize(V + L);
    let ndoth = saturate(dot(N, H));

    let w_skin  = 1.0 - smoothstep(0.2, 0.35, mat_id);
    let w_hair  = smoothstep(0.65, 0.85, mat_id);
    let w_cloth = clamp(1.0 - w_skin - w_hair, 0.0, 1.0);

    let spec_pow = 28.0 * w_skin + 70.0 * w_cloth + 120.0 * w_hair;
    let spec_raw = pow(ndoth, spec_pow);

    let th = 0.30 * w_skin + 0.24 * w_cloth + 0.20 * w_hair;
    let soft = 0.065;
    return smoothstep(th - soft, th + soft, spec_raw);
}

fn face_normal_from_world_pos(world_pos: vec3<f32>, is_front: bool) -> vec3<f32> {
    let dpdx_p = dpdx(world_pos);
    let dpdy_p = dpdy(world_pos);
    var n = normalize(cross(dpdx_p, dpdy_p));
    if (!is_front) { n = -n; }
    return n;
}

// Adaptive softness boost based on ndotl gradients.
fn adaptive_softness(softness: f32, nd: f32) -> f32 {
    let dx = abs(dpdx(nd));
    let dy = abs(dpdy(nd));
    let grad = dx + dy;

    let scale = 7.0;       // 5..10
    let max_boost = 0.08;  // 0.04..0.12

    let k = clamp(grad * scale, 0.0, 1.0);
    return max(softness + k * max_boost, 0.001);
}

/*
    SEAM-AWARE RAMP NORMAL:

    - Use smooth normal for most of the surface (removes faceting on hair etc.)
    - Blend towards face-normal ONLY where smooth vs face diverge a lot
      (these are exactly the places that cause "stripes" / seams in toon ramps)
*/
fn seam_aware_ramp_normal(N_smooth: vec3<f32>, N_face: vec3<f32>) -> vec3<f32> {
    // difference measure (0..~2)
    let d = length(N_smooth - N_face);

    // thresholds: where to start/finish blending to face normal
    // lower -> more aggressive (less stripes, more faceting)
    let t0 = 0.18;
    let t1 = 0.55;

    // strength cap (0..1)
    let strength = 0.85;

    let seam = smoothstep(t0, t1, d) * strength;
    return normalize(mix(N_smooth, N_face, seam));
}

fn toon_direct_lighting(in: VertexOutput, pbr_input: pbr_types::PbrInput, mat_id: f32, is_front: bool) -> vec4<f32> {
    let base = pbr_input.material.base_color;

    let w_skin  = 1.0 - smoothstep(0.2, 0.35, mat_id);
    let w_hair  = smoothstep(0.65, 0.85, mat_id);
    let w_cloth = clamp(1.0 - w_skin - w_hair, 0.0, 1.0);

    let N_face = face_normal_from_world_pos(in.world_position.xyz, is_front);

    // Smooth normal: use pbr_input.N (includes normal map). This is what looks "nice" on hair.
    // If your normal maps are heavy and cause noise, we tame it below with seam-aware blending + adaptive softness.
    let N_smooth = normalize(pbr_input.N);

    // Our ramp normal: mostly smooth, only face-normal near seams/problems
    var N_ramp = seam_aware_ramp_normal(N_smooth, N_face);

    // Optional: keep ramp even more stable by slightly biasing toward face on skin only
    // (skin is where seams are most visible; hair should stay smooth)
    let skin_seam_boost = 0.20 * w_skin; // 0..0.20
    N_ramp = normalize(mix(N_ramp, N_face, skin_seam_boost));

    var dir_acc = vec3<f32>(0.0);
    var key_ndotl = 0.0;
    var L_key = vec3<f32>(0.0, 1.0, 0.0);

    let count = min(lights.n_directional_lights, 3u);
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let dl = lights.directional_lights[i];
        let L = normalize(dl.direction_to_light);

        let nd = max(dot(N_ramp, L), 0.0);
        key_ndotl = max(key_ndotl, nd);
        dir_acc += dl.color.rgb * nd;

        if (i == 0u) { L_key = L; }
    }

    let ambient = lights.ambient_color.rgb * 0.08;
    let dir = dir_acc * 0.65;
    let light_color = clamp(ambient + dir, vec3<f32>(0.0), vec3<f32>(1.35));

    let wrap = 0.18 + w_skin * 0.10 + w_hair * (-0.05);
    let ndotl_wrapped = saturate((key_ndotl + wrap) / (1.0 + wrap));

    let pivot = hsr_params.x + w_skin * (-0.02) + w_hair * (0.02);
    let softness_base = max(hsr_params.y, 0.001) * (1.0 + w_skin * 0.25 + w_hair * (-0.35));

    // Adaptive softness to hide residual banding
    let softness = adaptive_softness(softness_base, ndotl_wrapped);

    let s0 = smooth_band(ndotl_wrapped, pivot - softness, pivot);
    let s1 = smooth_band(ndotl_wrapped, pivot, pivot + softness);

    let w_shadow = 1.0 - s0;
    let w_mid    = s0 * (1.0 - s1);
    let w_light  = s1;

    let skin_shadow_tint  = vec3<f32>(0.78, 0.62, 0.56);
    let hair_shadow_tint  = vec3<f32>(0.55, 0.60, 0.80);
    let cloth_shadow_tint = hsr_shadow_tint.rgb;

    let shadow_tint =
        cloth_shadow_tint * w_cloth +
        skin_shadow_tint  * w_skin  +
        hair_shadow_tint  * w_hair;

    let skin_light_mul  = vec3<f32>(1.02, 1.01, 1.00);
    let cloth_light_mul = vec3<f32>(1.06, 1.04, 1.01);
    let hair_light_mul  = vec3<f32>(1.08, 1.06, 1.02);

    let light_mul =
        cloth_light_mul * w_cloth +
        skin_light_mul  * w_skin  +
        hair_light_mul  * w_hair;

    let shadow_rgb = base.rgb * shadow_tint;
    let mid_rgb    = base.rgb;
    let light_rgb  = base.rgb * light_mul;

    let band_rgb = shadow_rgb * w_shadow + mid_rgb * w_mid + light_rgb * w_light;

    var lit = band_rgb * light_color;

    let wrap_width = 0.35 + w_skin * 0.06 + w_hair * (-0.03);
    let wrap_strength = 0.12 + w_skin * 0.10 + w_cloth * (-0.05);
    lit = apply_light_wrap(lit, N_ramp, pbr_input.V, L_key, wrap_width, wrap_strength);

    // Spec & rim on full normal (nice detail)
    let spec = toon_specular(pbr_input.N, pbr_input.V, L_key, mat_id);
    let spec_intensity = 0.01 * w_skin + 0.08 * w_cloth + 0.18 * w_hair;
    lit += spec * spec_intensity;

    let rim_mul = 0.65 + w_hair * 0.35 + w_skin * (-0.10);
    lit = apply_rim(pbr_input, lit, rim_mul);

    let skin_lift = 0.10;
    lit = mix(lit, lit * (1.0 + skin_lift), w_skin);

    let skin_cap = 0.92;
    lit = mix(lit, min(lit, vec3<f32>(skin_cap)), w_skin * 0.70);

    lit = clamp(lit, vec3<f32>(0.0), vec3<f32>(20.0));
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

    let mat_id = get_mat_id(in);
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    pbr_input.material.base_color =
        alpha_discard(pbr_input.material, pbr_input.material.base_color);

    apply_decals(&pbr_input);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;

    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = toon_direct_lighting(in, pbr_input, mat_id, is_front);
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
