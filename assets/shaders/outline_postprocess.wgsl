#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var scene_tex: texture_2d<f32>;

@group(0) @binding(1)
var depth_tex_ms: texture_multisampled_2d<f32>;

@group(0) @binding(2)
var normal_tex_ms: texture_multisampled_2d<f32>;

@group(0) @binding(3)
var scene_samp: sampler;

struct OutlineSettings {
    thickness: f32,        // in "pixel units", erlaubt < 1.0
    depth_threshold: f32,
    normal_threshold: f32,
    color: vec4<f32>,
};

@group(0) @binding(4)
var<uniform> settings: OutlineSettings;

// clamp helper
fn clamp_px(p: vec2<i32>, max_xy: vec2<i32>) -> vec2<i32> {
    return vec2<i32>(
        clamp(p.x, 0, max_xy.x),
        clamp(p.y, 0, max_xy.y),
    );
}

fn sample_depth_px(px: vec2<i32>) -> f32 {
    // MSAA 4x: Mittelwert
    var d = 0.0;
    d += textureLoad(depth_tex_ms, px, 0).r;
    d += textureLoad(depth_tex_ms, px, 1).r;
    d += textureLoad(depth_tex_ms, px, 2).r;
    d += textureLoad(depth_tex_ms, px, 3).r;
    return d * 0.25;
}

fn decode_normal(enc: vec3<f32>) -> vec3<f32> {
    return normalize(enc * 2.0 - vec3<f32>(1.0));
}

fn sample_normal_px(px: vec2<i32>) -> vec3<f32> {
    var n = vec3<f32>(0.0);
    n += textureLoad(normal_tex_ms, px, 0).xyz;
    n += textureLoad(normal_tex_ms, px, 1).xyz;
    n += textureLoad(normal_tex_ms, px, 2).xyz;
    n += textureLoad(normal_tex_ms, px, 3).xyz;
    n = n * 0.25;
    return decode_normal(n);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims_i = vec2<i32>(textureDimensions(scene_tex));
    let dims = vec2<f32>(dims_i);

    // UV und center pixel
    let uv = in.uv;
    let px_center = vec2<i32>(i32(uv.x * dims.x), i32(uv.y * dims.y));
    let px_max = dims_i - vec2<i32>(1, 1);

    let center_depth = sample_depth_px(clamp_px(px_center, px_max));
    let center_normal = sample_normal_px(clamp_px(px_center, px_max));

    // thickness in pixel-space, aber subpixel erlaubt
    let t = clamp(settings.thickness, 0.15, 2.0);
    let texel = 1.0 / dims;
    let step_uv = texel * t;

    var edge_strength = 0.0;

    for (var ox: i32 = -1; ox <= 1; ox = ox + 1) {
        for (var oy: i32 = -1; oy <= 1; oy = oy + 1) {
            if (ox == 0 && oy == 0) { continue; }

            let sample_uv = uv + vec2<f32>(f32(ox), f32(oy)) * step_uv;

            let sp = vec2<i32>(
                i32(sample_uv.x * dims.x),
                i32(sample_uv.y * dims.y),
            );
            let spc = clamp_px(sp, px_max);

            let d = sample_depth_px(spc);
            let n = sample_normal_px(spc);

            let depth_diff = abs(d - center_depth);
            let normal_diff = 1.0 - clamp(dot(n, center_normal), 0.0, 1.0);

            // Stärke statt bool
            let depth_s = smoothstep(
                settings.depth_threshold,
                settings.depth_threshold * 2.0,
                depth_diff
            );
            let norm_s = smoothstep(
                settings.normal_threshold,
                settings.normal_threshold * 1.7,
                normal_diff
            );

            edge_strength = max(edge_strength, max(depth_s, norm_s));
        }
    }

    // (Anti-Alias-Look)
    let edge = smoothstep(0.25, 0.85, edge_strength);

    let base = textureSample(scene_tex, scene_samp, uv);

    // luminance (für light-dependent outline fade)
    let lum = dot(base.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));

    // 0.55..0.85
    let light_fade = 1.0 - smoothstep(0.55, 0.85, lum);

    // final mix factor
    let a = edge;

    return mix(base, settings.color, a);
}
