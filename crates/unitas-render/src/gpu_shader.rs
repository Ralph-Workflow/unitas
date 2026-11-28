use super::VERTEX_STRIDE;

pub(super) const SHADER_SOURCE: &str = r"
struct GpuLightUniform {
    pos_range: vec4<f32>,
    dir_kind: vec4<f32>,
    color_intensity: vec4<f32>,
    spot_params: vec4<f32>,
};

struct Globals {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    camera_pos: vec4<f32>,
    shadow_params: vec4<f32>,
    ambient: vec4<f32>,
    lights: array<GpuLightUniform, 8>,
};

struct InstanceData {
    model: mat4x4<f32>,
    color: vec4<f32>,
    flags: vec4<f32>,
};

struct MaterialParams {
    params: vec4<f32>,
};

struct Light2DUniform {
    color_intensity: vec4<f32>,
    position_radius: vec4<f32>,
    direction_angles: vec4<f32>,
    kind: vec4<f32>,
};

struct Lights2D {
    count: vec4<f32>,
    lights: array<Light2DUniform, 16>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> lights_2d: Lights2D;
@group(1) @binding(0) var<uniform> inst: InstanceData;
@group(2) @binding(0) var albedo_tex: texture_2d<f32>;
@group(2) @binding(1) var normal_tex: texture_2d<f32>;
@group(2) @binding(2) var emissive_tex: texture_2d<f32>;
@group(2) @binding(3) var material_sampler: sampler;
@group(2) @binding(4) var<uniform> material: MaterialParams;
@group(3) @binding(0) var shadow_tex: texture_depth_2d;
@group(3) @binding(1) var shadow_sampler: sampler_comparison;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) world_pos: vec3<f32>,
    @location(5) alpha: f32,
};

@vertex
fn vs_shadow(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    let _unused = normal.x + uv.x + tangent.x;
    let world = inst.model * vec4<f32>(position, 1.0);
    return globals.light_view_proj * world;
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
) -> VertexOut {
    var out: VertexOut;
    let world = inst.model * vec4<f32>(position, 1.0);
    out.clip_position = globals.view_proj * world;
    let transformed = (inst.model * vec4<f32>(normal, 0.0)).xyz;
    let len = length(transformed);
    out.normal = select(vec3<f32>(0.0, 0.0, 0.0), transformed / len, len > 1e-8);
    out.tangent = (inst.model * vec4<f32>(tangent, 0.0)).xyz;
    out.color = inst.color.rgb;
    out.uv = uv;
    out.world_pos = world.xyz;
    out.alpha = inst.color.a;
    return out;
}

fn shadow_visibility(world_pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    if (globals.shadow_params.x < 0.5) {
        return 1.0;
    }
    let light_clip = globals.light_view_proj * vec4<f32>(world_pos, 1.0);
    if (light_clip.w <= 0.0) {
        return 1.0;
    }
    let ndc = light_clip.xyz / light_clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0) {
        return 1.0;
    }
    let cos_theta = clamp(dot(normal, globals.light_dir.xyz), 0.0, 1.0);
    let bias = max(0.005 * (1.0 - cos_theta), globals.shadow_params.w);
    let depth = ndc.z - bias;
    let texel = globals.shadow_params.z;
    var sum = 0.0;
    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
        for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
            let offset = vec2<f32>(f32(dx), f32(dy)) * texel;
            sum = sum + textureSampleCompareLevel(shadow_tex, shadow_sampler, uv + offset, depth);
        }
    }
    let center = textureSampleCompareLevel(shadow_tex, shadow_sampler, uv, depth);
    return select(center, sum / 9.0, globals.shadow_params.y >= 0.5);
}

fn surface_color(vs_out: VertexOut) -> vec3<f32> {
    let n_len = length(vs_out.normal);
    var n = select(vec3<f32>(0.0, 0.0, 0.0), vs_out.normal / n_len, n_len > 1e-8);
    let t_len = length(vs_out.tangent);
    if (t_len > 1e-4 && n_len > 1e-8) {
        var t = vs_out.tangent / t_len;
        t = normalize(t - n * dot(n, t));
        let b = cross(n, t);
        let nm = textureSample(normal_tex, material_sampler, vs_out.uv).xyz * 2.0 - 1.0;
        n = normalize(t * nm.x + b * nm.y + n * nm.z);
    }
    let albedo = textureSample(albedo_tex, material_sampler, vs_out.uv).rgb * vs_out.color;
    let emissive = textureSample(emissive_tex, material_sampler, vs_out.uv).rgb;
    let metallic = material.params.x;
    let smoothness = material.params.y;
    let lambert = max(dot(n, globals.light_dir.xyz), 0.0);
    let view_dir = normalize(globals.camera_pos.xyz - vs_out.world_pos);
    let half_dir = normalize(globals.light_dir.xyz + view_dir);
    let shininess = 8.0 + smoothness * 248.0;
    let specular = pow(max(dot(n, half_dir), 0.0), shininess) * metallic * smoothness;
    let diffuse = albedo * (1.0 - metallic);
    let shadow = shadow_visibility(vs_out.world_pos, n);
    var shade = (globals.light_color.w + globals.ambient.rgb) * albedo
        + globals.light_color.rgb * (lambert * diffuse + specular) * globals.light_dir.w * shadow;
    let light_count = u32(round(globals.ambient.w));
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        if (i >= light_count) {
            break;
        }
        let gl = globals.lights[i];
        let kind = gl.dir_kind.w;
        var l_dir = gl.dir_kind.xyz;
        var atten = 1.0;
        if (kind >= 0.5) {
            let to_light = gl.pos_range.xyz - vs_out.world_pos;
            let dist = length(to_light);
            let range = gl.pos_range.w;
            if (range <= 0.0 || dist >= range) {
                atten = 0.0;
            } else {
                l_dir = select(vec3<f32>(0.0, 1.0, 0.0), to_light / dist, dist > 1e-8);
                let norm_dist = clamp(dist / range, 0.0, 1.0);
                let falloff = 1.0 - norm_dist * norm_dist;
                atten = falloff * falloff;
                if (kind >= 1.5) {
                    let cos_theta = dot(l_dir, gl.dir_kind.xyz);
                    let cos_outer = gl.spot_params.x;
                    let cos_inner = max(gl.spot_params.y, cos_outer + 1e-5);
                    let spot_t = clamp((cos_theta - cos_outer) / (cos_inner - cos_outer), 0.0, 1.0);
                    atten = atten * spot_t * spot_t;
                }
            }
        }
        if (atten > 0.0 && gl.color_intensity.w > 0.0) {
            let l_lambert = max(dot(n, l_dir), 0.0);
            let l_half = normalize(l_dir + view_dir);
            let l_spec = pow(max(dot(n, l_half), 0.0), shininess) * metallic * smoothness;
            shade = shade + gl.color_intensity.rgb * (l_lambert * diffuse + l_spec) * gl.color_intensity.w * atten;
        }
    }
    return shade + emissive;
}

@fragment
fn fs_lit(vs_out: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(surface_color(vs_out), vs_out.alpha);
}

@fragment
fn fs_unlit(vs_out: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(vs_out.color, vs_out.alpha);
}

fn sprite_light(world_xy: vec2<f32>) -> vec3<f32> {
    var light = vec3<f32>(0.0);
    let count = u32(round(lights_2d.count.x));
    for (var index: u32 = 0u; index < 16u; index = index + 1u) {
        if (index >= count) {
            break;
        }
        let source = lights_2d.lights[index];
        let intensity = source.color_intensity.w;
        if (intensity <= 0.0) {
            continue;
        }
        if (source.kind.x < 0.5) {
            light = light + source.color_intensity.rgb * intensity;
            continue;
        }
        let to_light = source.position_radius.xy - world_xy;
        let distance = length(to_light);
        let inner = source.position_radius.z;
        let outer = source.position_radius.w;
        var falloff = 0.0;
        if (distance <= inner) {
            falloff = 1.0;
        } else if (outer > inner && distance < outer) {
            falloff = 1.0 - (distance - inner) / (outer - inner);
        }
        if (source.kind.x >= 1.5) {
            let to_fragment = -to_light;
            let span = length(to_fragment);
            let facing = select(0.0, dot(to_fragment / span, source.direction_angles.xy), span > 1e-8);
            let cos_outer = cos(radians(source.direction_angles.w));
            let cos_inner = cos(radians(source.direction_angles.z));
            var cone = 0.0;
            if (facing >= cos_outer) {
                cone = clamp((facing - cos_outer) / max(cos_inner - cos_outer, 1e-5), 0.0, 1.0);
            }
            falloff = falloff * cone;
        }
        light = light + source.color_intensity.rgb * (intensity * falloff);
    }
    return light;
}

@fragment
fn fs_sprite(vs_out: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo_tex, material_sampler, vs_out.uv);
    let light = sprite_light(vs_out.world_pos.xy);
    return vec4<f32>(texel.rgb * vs_out.color * (vec3<f32>(1.0) + light), texel.a * vs_out.alpha);
}

struct CaptureOut {
    @location(0) color: vec4<f32>,
    @location(1) id: u32,
};

@fragment
fn fs_capture_lit(vs_out: VertexOut) -> CaptureOut {
    var out: CaptureOut;
    out.color = vec4<f32>(surface_color(vs_out), 1.0);
    out.id = u32(round(inst.flags.x));
    return out;
}

@fragment
fn fs_capture_unlit(vs_out: VertexOut) -> CaptureOut {
    var out: CaptureOut;
    out.color = vec4<f32>(vs_out.color, 1.0);
    out.id = u32(round(inst.flags.x));
    return out;
}

@fragment
fn fs_capture_sprite(vs_out: VertexOut) -> CaptureOut {
    var out: CaptureOut;
    let texel = textureSample(albedo_tex, material_sampler, vs_out.uv);
    let light = sprite_light(vs_out.world_pos.xy);
    out.color = vec4<f32>(texel.rgb * vs_out.color * (vec3<f32>(1.0) + light), texel.a * vs_out.alpha);
    out.id = 0u;
    return out;
}
";

pub(super) const POST_SHADER_SOURCE: &str = r"
struct PostParams {
    threshold: f32,
    intensity: f32,
    exposure: f32,
    texel_x: f32,
    texel_y: f32,
    direction_x: f32,
    direction_y: f32,
    reserved: f32,
};
@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> params: PostParams;
@group(0) @binding(3) var bloom_tex: texture_2d<f32>;
struct PostVertex {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex
fn vs_post(@builtin(vertex_index) index: u32) -> PostVertex {
    var out: PostVertex;
    let corner = f32(index);
    let x = select(-1.0, 3.0, corner >= 1.0);
    let y = select(-1.0, 3.0, corner >= 2.0);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}
fn aces_fit_raw(value: f32) -> f32 {
    let numerator = value * (value + 0.0245786) - 0.000090537;
    let denominator = value * (0.983729 * value + 0.432951) + 0.238081;
    return numerator / denominator;
}
fn aces_fit(value: f32) -> f32 {
    return max(0.0, aces_fit_raw(value) / aces_fit_raw(1.0));
}
@fragment
fn fs_bright(vertex: PostVertex) -> @location(0) vec4<f32> {
    let color = textureSampleLevel(source_tex, source_sampler, vertex.uv, 0.0).rgb;
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    if (luma <= params.threshold) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return vec4<f32>(color, 1.0);
}
fn blur_sample(uv: vec2<f32>) -> vec3<f32> {
    let step = vec2<f32>(params.direction_x * params.texel_x, params.direction_y * params.texel_y);
    var sum = textureSampleLevel(source_tex, source_sampler, uv, 0.0).rgb * 0.22702703;
    sum = sum + (textureSampleLevel(source_tex, source_sampler, uv + step, 0.0).rgb
        + textureSampleLevel(source_tex, source_sampler, uv - step, 0.0).rgb) * 0.1945946;
    sum = sum + (textureSampleLevel(source_tex, source_sampler, uv + step * 2.0, 0.0).rgb
        + textureSampleLevel(source_tex, source_sampler, uv - step * 2.0, 0.0).rgb) * 0.12162162;
    sum = sum + (textureSampleLevel(source_tex, source_sampler, uv + step * 3.0, 0.0).rgb
        + textureSampleLevel(source_tex, source_sampler, uv - step * 3.0, 0.0).rgb) * 0.05405405;
    sum = sum + (textureSampleLevel(source_tex, source_sampler, uv + step * 4.0, 0.0).rgb
        + textureSampleLevel(source_tex, source_sampler, uv - step * 4.0, 0.0).rgb) * 0.01621622;
    return sum;
}
@fragment
fn fs_blur(vertex: PostVertex) -> @location(0) vec4<f32> {
    return vec4<f32>(blur_sample(vertex.uv), 1.0);
}
@fragment
fn fs_tonemap(vertex: PostVertex) -> @location(0) vec4<f32> {
    let scene = textureSampleLevel(source_tex, source_sampler, vertex.uv, 0.0).rgb;
    let bloom = textureSampleLevel(bloom_tex, source_sampler, vertex.uv, 0.0).rgb;
    let color = (scene + bloom * params.intensity) * params.exposure;
    return vec4<f32>(aces_fit(color.r), aces_fit(color.g), aces_fit(color.b), 1.0);
}
";
pub(super) const ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 12,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 24,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 32,
        shader_location: 3,
    },
];

pub(super) const VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: VERTEX_STRIDE,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &ATTRIBUTES,
};
