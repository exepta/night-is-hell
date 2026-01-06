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

fn sample_normal_px(px: vec2<i32>) -> vec3<f32> {
    var n = vec3<f32>(0.0);
    n += textureLoad(normal_tex_ms, px, 0).xyz;
    n += textureLoad(normal_tex_ms, px, 1).xyz;
    n += textureLoad(normal_tex_ms, px, 2).xyz;
    n += textureLoad(normal_tex_ms, px, 3).xyz;
    return n * 0.25;
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

    var edge = 0.0;

    // 8-neighborhood, aber Offset in UV (subpixel)
    for (var ox: i32 = -1; ox <= 1; ox = ox + 1) {
        for (var oy: i32 = -1; oy <= 1; oy = oy + 1) {
            if (ox == 0 && oy == 0) { continue; }

            let sample_uv = uv + vec2<f32>(f32(ox), f32(oy)) * step_uv;

            // UV -> Pixel (und clamp)
            let sp = vec2<i32>(
                i32(sample_uv.x * dims.x),
                i32(sample_uv.y * dims.y),
            );
            let spc = clamp_px(sp, px_max);

            let d = sample_depth_px(spc);
            let n = sample_normal_px(spc);

            let depth_diff = abs(d - center_depth);
            let normal_diff = length(n - center_normal);

            if (depth_diff > settings.depth_threshold || normal_diff > settings.normal_threshold) {
                edge = 1.0;
            }
        }
    }

    let base = textureSample(scene_tex, scene_samp, uv);
    return mix(base, settings.color, edge);
}
