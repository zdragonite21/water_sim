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

// spatial hashing
@group(0) @binding(2) var<storage, read_write> grid_keys: array<u32>;
@group(0) @binding(3) var<storage, read_write> particle_indices: array<u32>;
@group(0) @binding(4) var<storage, read_write> start_indices: array<u32>;

@group(0) @binding(5)
var<uniform> config: SimConfig;

#import util.wgsl::{smoothing_kernel, smoothing_kernel_deriv};

const CELL_OFFSETS: array<vec2<i32>, 9> = array(
    vec2(-1, -1), vec2( 0, -1), vec2( 1, -1),
    vec2(-1,  0), vec2( 0,  0), vec2( 1,  0),
    vec2(-1,  1), vec2( 0,  1), vec2( 1,  1),
);

fn density_to_pressure(density: f32) -> f32 {
    var density_error = density - config.rest_density;
    return config.stiffness * density_error;
}

fn position_to_cell(pos: vec3<f32>) -> vec3<i32> {
    return vec3<i32>(floor(pos / config.smoothing_radius));
}

fn hash_cell(cell: vec3<i32>) -> u32 {
    var p1 = 73856093u;
    var p2 = 19349663u;
    var p3 = 83492791u;

    var x = u32(cell.x) * p1;
    var y = u32(cell.y) * p2;
    var z = u32(cell.z) * p3;

    return x ^ y ^ z;
}

const MAX: u32 = 0xffffffffu;

@compute
@workgroup_size(64)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    var index = gid.x;
    var p = particles_prev[index];
    var len = arrayLength(&particles_prev);

    // apply gravity
    var vel = p.vel - vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
    var predicted = p.pos + vel * config.predict_dt;

    // compute spatial hash
    var cell = position_to_cell(predicted);
    var key = hash_cell(cell) % len;
    grid_keys[index] = key;
    particle_indices[index] = index;
}


@compute
@workgroup_size(64)
fn compute_density_pressure(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    var index = gid.x;
    var p = particles_prev[index];
    var len = arrayLength(&particles_prev);

    var vel = p.vel - vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
    var predicted = p.pos + vel * config.predict_dt;

    // compute density
    var cell = position_to_cell(predicted);
    var sq_radius = config.smoothing_radius * config.smoothing_radius;

    var density = 0.0;
    for (var i = 0u; i < 9; i++) {
        var offset = CELL_OFFSETS[i];
        var curr_cell = cell + vec3<i32>(offset.x, offset.y, 0);
        var key = hash_cell(curr_cell) % len;
        var cell_start_idx = start_indices[key];
        if cell_start_idx == MAX {
            continue;
        }

        for (var j = cell_start_idx; j < len; j++) {
            if grid_keys[j] != key {
                break;
            }

            var other = particles_prev[particle_indices[j]];
            var other_cell = position_to_cell(other.pos);
            if (any(other_cell != curr_cell)) {
                continue;
            }

            var offset = other.pos - predicted;
            var sq_dst = dot(offset, offset);
            if sq_dst > sq_radius {
                var dst = sqrt(sq_dst);
                var influence: f32 = smoothing_kernel(config.smoothing_radius, dst);
                density += config.mass * influence;
            }
        }
    }

    // apply pressure
    var pressure_force: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var pressure = density_to_pressure(density);
    for (var i = 0u; i < 9; i++) {
        var offset = CELL_OFFSETS[i];
        var curr_cell = cell + vec3<i32>(offset.x, offset.y, 0);
        var key = hash_cell(curr_cell) % len;
        var cell_start_idx = start_indices[key];
        if cell_start_idx == MAX {
            continue;
        }

        for (var j = cell_start_idx; j < len; j++) {
            if grid_keys[j] != key {
                break;
            }

            var other = particles_prev[particle_indices[j]];
            var other_cell = position_to_cell(other.pos);
            if (any(other_cell != curr_cell)) {
                continue;
            }

            var offset = other.pos - predicted;
            var sq_dst = dot(offset, offset);
            if sq_dst > sq_radius {
                var mask = select(0.0, 1.0, j != index);
                var dst = sqrt(sq_dst);
                // todo handle dst = 0.0 case
                var dir = -offset / dst;

                var slope = smoothing_kernel_deriv(config.smoothing_radius, dst);
                
                // todo shared pressure + neighbor density

                pressure_force += pressure * dir * slope * config.mass / density * mask;
            }
        }
    }
    // todo apply viscosity

    // integrate velocity
    var pos = p.pos + vel * config.dt;

    // todo handle collisions

    particles_next[index] = Particle(pos, vel, vec2<f32>(0.0, 0.0));
}
