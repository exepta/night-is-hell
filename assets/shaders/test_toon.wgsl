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
var<uniform> hsr_params: vec4<f32>; // x=threshold, y=softness, z=rim_power, w=rim_strength

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var<uniform> hsr_rim_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var<uniform> hsr_shadow_tint: vec4<f32>;

/* ────────────────────────────────────────────────────────────── */

fn apply_hsr_rim(pbr_input: pbr_types::PbrInput, rgb: vec3<f32>) -> vec3<f32> {
    let ndv = max(dot(pbr_input.N, pbr_input.V), 0.0);
    let rim = pow(1.0 - ndv, hsr_params.z) * hsr_params.w;
    return rgb + (hsr_rim_color.rgb * rim);
}

fn toon_direct_lighting(pbr_input: pbr_types::PbrInput) -> vec4<f32> {
    let base = pbr_input.material.base_color;

    // --- TUNING (musst du an deine Szene anpassen) ---
    // Bevy-Lights sind HDR / physikalisch skaliert -> ohne Scale clippt alles.
    let AMBIENT_SCALE: f32 = 0.06;
    let DIR_SCALE: f32 = 0.00006;
    // -------------------------------------------------

    let ambient = lights.ambient_color.rgb * AMBIENT_SCALE;

    var dir_rgb = vec3<f32>(0.0);
    var L = vec3<f32>(0.0, 1.0, 0.0);

    if (lights.n_directional_lights > 0u) {
        let sun = lights.directional_lights[0];
        // In Bevy kann das schon "stark" sein (HDR). Daher DIR_SCALE.
        dir_rgb = sun.color.rgb * DIR_SCALE;
        L = normalize(sun.direction_to_light);
    }

    let ndotl = max(dot(pbr_input.N, L), 0.0);

    // Toon Ramp (x=threshold, y=softness)
    let t = hsr_params.x;
    let s = hsr_params.y;
    let ramp = smoothstep(t - s, t + s, ndotl);

    // Schattenfarbe -> weiß
    let shade = mix(hsr_shadow_tint.rgb, vec3<f32>(0.92), ramp);

    // Diffuse only (HSR Schritt 1)
    var lit = base.rgb * (ambient + dir_rgb * ndotl * shade);

    // Rim (additiv) – ok, aber kann auch clippen, daher optional clamp danach
    lit = apply_hsr_rim(pbr_input, lit);

    // Sicherheit: verhindert extremes Clippen, bevor Tonemapping kommt
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

    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Alpha discard (Cutout)
    pbr_input.material.base_color =
        alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // Clustered decals
    pbr_input.material.base_color = apply_decal_base_color(
        in.world_position.xyz,
        in.position.xy,
        pbr_input.material.base_color,
    );

#ifdef PREPASS_PIPELINE
    // Deferred: hier machen wir keinen post-light Toon (das geht so nicht sauber).
    // Du kannst später "pre-light" quantization in den GBuffer schreiben.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;

    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = toon_direct_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // Bevy Post-Processing / fog / tone mapping etc.
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
