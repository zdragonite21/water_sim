struct Particle {
    pos: vec3<f32>,
    vel: vec3<f32>,
    _pad: vec2<f32>,
}

struct SimConfig {
    dt: f32,
    predict_dt: f32,
    smoothing_radius: f32,
    rest_density: f32,

    stiffness: f32,
    viscosity: f32,
    mass: f32,
    gravity: f32,

    size: vec3<f32>,
};

@group(0) @binding(0) var<storage, read> particles_prev: array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_next: array<Particle>;

@group(0) @binding(2)
var<uniform> config: SimConfig;

#import util.wgsl::{smoothing_kernel, smoothing_kernel_deriv};

fn density_to_pressure(density: f32) -> f32 {
    var density_error = density - config.rest_density;
    return config.stiffness * density_error;
}

@compute
@workgroup_size(64)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // todo spatial hashing
    var index = gid.x;
    var p = particles_prev[index];
    var len = arrayLength(&particles_prev);

    // apply gravity
    var vel = p.vel - vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
    var predicted = p.pos + vel * config.predict_dt;

    // compute density
    var density = 0.0;
    for (var i = 0u; i < len; i++) {
        var other = particles_prev[i];
        var dst = distance(predicted, other.pos);
        var influence: f32 = smoothing_kernel(config.smoothing_radius, dst);
        density += config.mass * influence;
    }

    // apply pressure
    var pressure_force: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var pressure = density_to_pressure(density);
    for (var i = 0u; i < len; i++) {
        var mask = select(0.0, 1.0, i != index);
        
        var other = particles_prev[i];
        var offset = other.pos - predicted;
        var dst = distance(predicted, other.pos);
        // todo handle dst = 0.0 case
        var dir = -offset / dst;

        var slope = smoothing_kernel_deriv(config.smoothing_radius, dst);
        
        // todo shared pressure + neighbor density

        pressure_force += pressure * dir * slope * config.mass / density * mask;
    }

    // todo apply viscosity

    // integrate velocity
    var pos = p.pos + vel * config.dt;

    // todo handle collisions

    particles_next[index] = Particle(pos, vel, vec2<f32>(0.0, 0.0));
}
