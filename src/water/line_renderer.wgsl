struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct LineUniform {
    viewport_size: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> line_config: LineUniform;

struct InstanceInput {
    @location(5) start: vec2<f32>,
    @location(6) end: vec2<f32>,
    @location(7) color: vec4<f32>,
    @location(8) width_px: f32,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// @vertex
// fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
//     var out: VertexOutput;

//     let delta = instance.end - instance.start;
//     var dir = vec3<f32>(1.0, 0.0, 0.0);
//     let len = length(delta);
//     if len > 0.00001 {
//         dir = delta / len;
//     }

//     var normal = vec3<f32>(-dir.y, dir.x, 0.0);
//     if length(normal) <= 0.00001 {
//         normal = vec3<f32>(0.0, 1.0, 0.0);
//     }
//     normal = normalize(normal);

//     let width_world = instance.width_px * 0.02;
//     let world_pos = mix(instance.start, instance.end, model.uv.x)
//         + normal * model.position.y * width_world;

//     out.clip_position = camera.proj * camera.view * vec4<f32>(world_pos, 1.0);
//     out.color = instance.color;
//     return out;
// }
@vertex
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    let start_px = (instance.start * 0.5 + vec2<f32>(0.5)) * line_config.viewport_size;
    let end_px = (instance.end * 0.5 + vec2<f32>(0.5)) * line_config.viewport_size;

    let delta = end_px - start_px;
    var dir = vec2<f32>(1.0, 0.0);
    let len = length(delta);
    if len > 0.00001 {
        dir = delta / len;
    }

    var normal = vec2<f32>(-dir.y, dir.x);
    if length(normal) <= 0.00001 {
        normal = vec2<f32>(0.0, 1.0);
    }
    normal = normalize(normal);

    let width = instance.width_px;
    let pos_px = mix(start_px, end_px, in.uv.x)
        + normal * in.position.y * width;
    let pos = pos_px / line_config.viewport_size * 2.0 - vec2<f32>(1.0);

    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
