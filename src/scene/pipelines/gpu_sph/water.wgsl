struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

struct WaterUniform {
    particle_size: f32,
    target_density: f32,
}

struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) velocity: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> water_config: WaterUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(2) velocity: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    var view_orient: mat4x4<f32> = camera.view;
    view_orient[3] = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    var billboard_offset = transpose(view_orient) * vec4<f32>(model.position * water_config.particle_size, 0.0);
    var pos = vec4<f32>(instance.position, 1.0) + billboard_offset;

    out.clip_position = camera.proj * camera.view * pos;
    out.uv = model.uv;
    out.velocity = instance.velocity;
    return out;
}

const RED: vec3<f32> = vec3<f32>(1.0, 0.0, 0.0);
const BLUE: vec3<f32> = vec3<f32>(0.0, 0.0, 1.0);
const GREEN: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);
const WHITE: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
const BLACK: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

struct ColorStop {
    pos: f32,          // 0.0 to 1.0
    color: vec3<f32>,
};

fn color_ramp(t_raw: f32) -> vec3<f32> {
    let t = clamp(t_raw, 0.0, 1.0);

    let stops = array<ColorStop, 4>(
        ColorStop(0.0, vec3<f32>(0.0, 0.0, 1.0)), // blue
        ColorStop(0.3, vec3<f32>(0.0, 1.0, 1.0)), // cyan
        ColorStop(0.7, vec3<f32>(1.0, 1.0, 0.0)), // yellow
        ColorStop(1.0, vec3<f32>(1.0, 0.0, 0.0)), // red
    );

    for (var i = 0u; i < 3u; i++) {
        let a = stops[i];
        let b = stops[i + 1u];

        if (t <= b.pos) {
            let local_t = (t - a.pos) / (b.pos - a.pos);
            return mix(a.color, b.color, local_t);
        }
    }

    return stops[3].color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var center_offset: vec2<f32> = (in.uv - 0.5) * 2.0;
    var sq_dist: f32 = center_offset.x * center_offset.x + center_offset.y * center_offset.y;
    if sq_dist > 1.0 {
        discard;
    }

    var z: f32 = sqrt(1.0 - sq_dist);
    var view_orient: mat4x4<f32> = camera.view;
    view_orient[3] = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    var nor: vec3<f32> = normalize(transpose(view_orient) * vec4(center_offset.x, center_offset.y, z, 1.0) * water_config.particle_size).xyz;

    var light_dir = normalize(vec3<f32>(0.0, 1.0, 1.0));
    var light_intensity: f32 = max(dot(nor, light_dir), 0.1);

    // target density
    // let denom = max(abs(water_config.target_density), 0.0001);
    // let error = clamp((water_config.target_density - in.density) / denom, -1.0, 1.0);

    // let strength = pow(abs(error), 0.5);

    // let color = select(
    //     mix(vec3(1.0), RED, strength),
    //     mix(vec3(1.0), BLUE, strength),
    //     error >= 0.0
    // );
    let mag = length(in.velocity);
    let strength = mag / 10.0;
    let color = color_ramp(strength);

    return light_intensity * vec4(color, 1.0);
}
