struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

struct WaterUniform {
    particle_size: f32,
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
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    var view_orient: mat4x4<f32> = camera.view;
    view_orient[3] = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    var pos: vec4<f32> = transpose(view_orient) * vec4<f32>(model.position * water_config.particle_size, 1.0);
    pos += vec4<f32>(instance.position, 1.0);

    out.clip_position = camera.proj * camera.view * pos;
    out.uv = model.uv;
    return out;
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
    return light_intensity * vec4<f32>(0.0, 0.5, 1.0, 1.0);
}
