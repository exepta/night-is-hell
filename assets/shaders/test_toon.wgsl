#import bevy_pbr::{
    pbr_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
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
fn saturate3(v: vec3<f32>) -> vec3<f32> { return clamp(v, vec3<f32>(0.0), vec3<f32>(1.0)); }

fn get_mat_id(in: VertexOutput) -> f32 {
#ifdef VERTEX_COLORS
    return clamp(in.color.r, 0.0, 1.0);
#else
    return 0.5;
#endif
}

fn get_shadow_shift(in: VertexOutput) -> f32 {
#ifdef VERTEX_COLORS
    return clamp(in.color.g, 0.0, 1.0);
#else
    return 0.5;
#endif
}

fn get_spec_mask(in: VertexOutput) -> f32 {
#ifdef VERTEX_COLORS
    return clamp(in.color.b, 0.0, 1.0);
#else
    return 1.0;
#endif
}

fn face_normal_from_world_pos(world_pos: vec3<f32>, is_front: bool) -> vec3<f32> {
    let dx = dpdx(world_pos);
    let dy = dpdy(world_pos);
    var n = normalize(cross(dx, dy));
    if (!is_front) { n = -n; }
    return n;
}

fn seam_aware_ramp_normal(n_smooth: vec3<f32>, n_face: vec3<f32>) -> vec3<f32> {
    let d = length(n_smooth - n_face);
    let t0 = 0.18;
    let t1 = 0.55;
    let seam = smoothstep(t0, t1, d) * 0.85;
    return normalize(mix(n_smooth, n_face, seam));
}

fn rim_term(N: vec3<f32>, V: vec3<f32>, power: f32) -> f32 {
    let ndv = saturate(dot(normalize(N), normalize(V)));
    return pow(1.0 - ndv, max(power, 0.001));
}

fn apply_rim(pbr_input: pbr_types::PbrInput, rgb: vec3<f32>, rim_mul: f32) -> vec3<f32> {
    let r = rim_term(pbr_input.N, pbr_input.V, hsr_params.z);
    let rim = r * hsr_params.w * rim_mul;
    return rgb + hsr_rim_color.rgb * rim;
}

fn toon_band(x: f32, edge: f32, width: f32) -> f32 {
    return smoothstep(edge - width, edge + width, x);
}

fn toon_specular_masked(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, mat_id: f32, spec_mask: f32) -> f32 {
    let H = normalize(V + L);
    let ndh = saturate(dot(normalize(N), H));

    let w_skin  = 1.0 - smoothstep(0.2, 0.35, mat_id);
    let w_hair  = smoothstep(0.65, 0.85, mat_id);
    let w_cloth = clamp(1.0 - w_skin - w_hair, 0.0, 1.0);

    let pow_skin = 34.0;
    let pow_cloth = 80.0;
    let pow_hair = 140.0;

    let p = pow_skin * w_skin + pow_cloth * w_cloth + pow_hair * w_hair;
    let raw = pow(ndh, p);

    let th = (0.30 * w_skin + 0.25 * w_cloth + 0.22 * w_hair);
    let soft = 0.055;

    let shaped = smoothstep(th - soft, th + soft, raw);
    let mask = saturate(mix(0.15, 1.20, spec_mask));
    return shaped * mask;
}

fn hair_band_highlight(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, mat_id: f32, spec_mask: f32) -> f32 {
    let w_hair = smoothstep(0.65, 0.85, mat_id);

    let H = normalize(V + L);
    let ndh = saturate(dot(normalize(N), H));

    // Slightly wider and softer than typical "anime sharp band"
    let band = smoothstep(0.84, 0.985, pow(ndh, 36.0));

    // Masked by vertex B so you can paint where the band is allowed
    let m = w_hair * saturate(spec_mask);

    return band * m;
}

fn endfield_tonemap(rgb: vec3<f32>) -> vec3<f32> {
    let x = max(rgb, vec3<f32>(0.0));
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    let y = (x * (a * x + vec3<f32>(b))) / (x * (c * x + vec3<f32>(d)) + vec3<f32>(e));
    return saturate3(y);
}

fn gamma_encode(rgb: vec3<f32>) -> vec3<f32> {
    return pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

fn soft_skin_sss(N: vec3<f32>, L: vec3<f32>, w_skin: f32) -> vec3<f32> {
    let ndl = saturate(dot(normalize(N), normalize(L)));
    let back = smoothstep(0.0, 0.70, 1.0 - ndl);
    let sss = back * w_skin;
    let sss_col = vec3<f32>(1.0, 0.80, 0.74);
    return sss_col * (sss * 0.07);
}

fn lift_midtones(w_shadow: f32, w_mid: f32, w_light: f32) -> vec3<f32> {
    // Returns (ws, wm, wl) renormalized with a midtone push.
    let mid_push = 0.10;

    let ws = saturate(w_shadow - mid_push);
    let wm = saturate(w_mid + mid_push);
    let wl = w_light;

    let norm = max(ws + wm + wl, 0.0001);
    return vec3<f32>(ws / norm, wm / norm, wl / norm);
}

fn toon_direct_lighting(
    in: VertexOutput,
    pbr_input: pbr_types::PbrInput,
    mat_id: f32,
    shadow_shift: f32,
    spec_mask: f32,
    is_front: bool
) -> vec4<f32> {
    let base = pbr_input.material.base_color;

    let w_skin  = 1.0 - smoothstep(0.2, 0.35, mat_id);
    let w_hair  = smoothstep(0.65, 0.85, mat_id);
    let w_cloth = clamp(1.0 - w_skin - w_hair, 0.0, 1.0);

    let N_face = face_normal_from_world_pos(in.world_position.xyz, is_front);
    let N_smooth = normalize(pbr_input.N);
    var N_ramp = seam_aware_ramp_normal(N_smooth, N_face);
    N_ramp = normalize(mix(N_ramp, N_face, 0.20 * w_skin));

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

    // Brighter baseline + a bit more headroom
    let ambient = lights.ambient_color.rgb * 0.18;
    let dir = dir_acc * 0.82;
    let light_color = clamp(ambient + dir, vec3<f32>(0.0), vec3<f32>(2.1));

    // Light wrap helps soften faces like Endfield
    let wrap = 0.18 + w_skin * 0.10 + w_hair * (-0.04);
    let nd = saturate((key_ndotl + wrap) / (1.0 + wrap));

    let pivot_base = hsr_params.x + w_skin * (-0.03) + w_hair * (0.02);
    let pivot = pivot_base + (shadow_shift - 0.5) * 0.10;

    // Slightly softer transitions than hard anime ramps
    let softness = max(hsr_params.y, 0.001) * (0.85 + w_skin * 0.35 + w_hair * (-0.25));

    // 3-zone banding
    let e0 = toon_band(nd, pivot - 0.18, softness * 0.60);
    let e2 = toon_band(nd, pivot + 0.16, softness * 0.95);

    let w_shadow = 1.0 - e0;
    let w_mid    = e0 * (1.0 - e2);
    let w_light  = e2;

    // Cooler and shallower shadows
    let tint_shadow_skin  = vec3<f32>(0.84, 0.74, 0.76);
    let tint_shadow_hair  = vec3<f32>(0.64, 0.70, 0.90);
    let tint_shadow_cloth = mix(hsr_shadow_tint.rgb, vec3<f32>(0.56, 0.60, 0.74), 0.35);

    let shadow_tint =
        tint_shadow_cloth * w_cloth +
        tint_shadow_skin  * w_skin  +
        tint_shadow_hair  * w_hair;

    let light_mul_skin  = vec3<f32>(1.04, 1.03, 1.01);
    let light_mul_cloth = vec3<f32>(1.08, 1.05, 1.02);
    let light_mul_hair  = vec3<f32>(1.10, 1.07, 1.02);

    let light_mul =
        light_mul_cloth * w_cloth +
        light_mul_skin  * w_skin  +
        light_mul_hair  * w_hair;

    let rgb_shadow = base.rgb * shadow_tint;
    let rgb_mid    = base.rgb;
    let rgb_light  = base.rgb * light_mul;

    // Midtone push and renormalize
    let w = lift_midtones(w_shadow, w_mid, w_light);
    var rgb = rgb_shadow * w.x + rgb_mid * w.y + rgb_light * w.z;

    // Apply lighting
    rgb *= light_color;

    // Skin soft subsurface feel
    rgb += soft_skin_sss(pbr_input.N, L_key, w_skin);

    // Controlled specular (reduce plastic)
    let spec = toon_specular_masked(pbr_input.N, pbr_input.V, L_key, mat_id, spec_mask);
    let spec_intensity = (0.006 * w_skin + 0.06 * w_cloth + 0.15 * w_hair);
    rgb += spec * spec_intensity;

    // Hair band highlight
    let hair_band = hair_band_highlight(pbr_input.N, pbr_input.V, L_key, mat_id, spec_mask);
    rgb += hair_band * vec3<f32>(0.88, 0.95, 1.00) * 0.55;

    // Rim (kept subtle)
    let rim_mul = 0.50 + w_hair * 0.50 + w_skin * (-0.10);
    rgb = apply_rim(pbr_input, rgb, rim_mul);

    // Slight skin lift and cap
    let skin_lift = 0.08;
    rgb = mix(rgb, rgb * (1.0 + skin_lift), w_skin);

    let skin_cap = 0.94;
    rgb = mix(rgb, min(rgb, vec3<f32>(skin_cap)), w_skin * 0.70);

    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(20.0));

    let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let shadow = 1.0 - smoothstep(0.25, 0.65, luma);
    rgb = mix(rgb, mix(vec3<f32>(luma), rgb, 0.92), shadow * 0.35);

    // Filmic output
    let tm = endfield_tonemap(rgb * 1.08);
    let out_rgb = gamma_encode(tm);

    return vec4<f32>(out_rgb, base.a);
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
    let shadow_shift = get_shadow_shift(in);
    let spec_mask = get_spec_mask(in);

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    apply_decals(&pbr_input);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;

    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = toon_direct_lighting(in, pbr_input, mat_id, shadow_shift, spec_mask, is_front);
    } else {
        out.color = pbr_input.material.base_color;
    }
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