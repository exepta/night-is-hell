#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var scene_tex: texture_2d<f32>;

@group(0) @binding(1)
var depth_tex_ms: texture_depth_multisampled_2d;

@group(0) @binding(2)
var normal_tex_ms: texture_multisampled_2d<f32>;

@group(0) @binding(3)
var scene_samp: sampler;

struct OutlineSettings {
    thickness: f32,
    depth_threshold: f32,
    normal_threshold: f32,
    color: vec4<f32>,
};

@group(0) @binding(4)
var<uniform> settings: OutlineSettings;

fn clamp_px(p: vec2<i32>, max_xy: vec2<i32>) -> vec2<i32> {
    return vec2<i32>(
        clamp(p.x, 0, max_xy.x),
        clamp(p.y, 0, max_xy.y),
    );
}

fn sample_depth_px(px: vec2<i32>) -> f32 {
    return textureLoad(depth_tex_ms, px, 0);
}

fn decode_normal(enc: vec3<f32>) -> vec3<f32> {
    return normalize(enc * 2.0 - vec3<f32>(1.0));
}

fn sample_normal_px(px: vec2<i32>) -> vec3<f32> {
    let n = textureLoad(normal_tex_ms, px, 0).xyz;
    return decode_normal(n);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims_i = vec2<i32>(textureDimensions(scene_tex));
    let dims = vec2<f32>(dims_i);

    let uv = in.uv;
    let px_center = vec2<i32>(i32(uv.x * dims.x), i32(uv.y * dims.y));
    let px_max = dims_i - vec2<i32>(1, 1);

    let center_depth = sample_depth_px(clamp_px(px_center, px_max));
    let center_normal = sample_normal_px(clamp_px(px_center, px_max));

    let t = clamp(settings.thickness, 0.20, 1.50);
    let texel = 1.0 / dims;
    let step_uv = texel * t;

    var edge_strength = 0.0;

    for (var ox: i32 = -1; ox <= 1; ox = ox + 1) {
        for (var oy: i32 = -1; oy <= 1; oy = oy + 1) {
            if (ox == 0 && oy == 0) { continue; }

            let suv = uv + vec2<f32>(f32(ox), f32(oy)) * step_uv;
            let sp = vec2<i32>(i32(suv.x * dims.x), i32(suv.y * dims.y));
            let spc = clamp_px(sp, px_max);

            let d = sample_depth_px(spc);
            let n = sample_normal_px(spc);

            let depth_diff = abs(d - center_depth);
            let normal_diff = 1.0 - clamp(dot(n, center_normal), 0.0, 1.0);

            let depth_s = smoothstep(settings.depth_threshold, settings.depth_threshold * 2.0, depth_diff);
            let norm_s  = smoothstep(settings.normal_threshold, settings.normal_threshold * 1.6, normal_diff);

            edge_strength = max(edge_strength, max(depth_s, norm_s));
        }
    }

    let edge = smoothstep(0.30, 0.85, edge_strength);
    let base = textureSample(scene_tex, scene_samp, uv);

    // Less overpaint, more "ink"
    let a = edge * 0.85;
    return mix(base, settings.color, a);
}