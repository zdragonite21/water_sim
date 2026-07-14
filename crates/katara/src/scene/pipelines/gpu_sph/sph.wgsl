struct SimConfig {
    size: vec3<f32>,
    collision_damping: f32,
    dt: f32,
    gravity: f32,
    smoothing_radius: f32,
    stiffness: f32,
    viscosity_strength: f32,
    rest_density: f32,
    mass: f32,
};

    struct Interval {
    start_idx: u32,
    end_idx: u32,
}

// uniforms
@group(0) @binding(0) var<uniform> config: SimConfig;

// ping pong buffers
@group(1) @binding(0) var<storage, read_write> buff_a: array<vec4<f32>>;
@group(1) @binding(1) var<storage, read_write> buff_b: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read_write> buff_c: array<vec4<f32>>;
@group(1) @binding(3) var<storage, read_write> buff_d: array<vec4<f32>>;

// spatial hashing
@group(2) @binding(0) var<storage, read_write> keys: array<u32>;
@group(2) @binding(1) var<storage, read_write> indices: array<u32>;
@group(2) @binding(2) var<storage, read_write> intervals: array<Interval>;

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

fn get_key(cell: vec3<i32>, length: u32) -> u32 {
    var hash = hash_cell(cell);
    return hash % length;
}

fn apply_gravity(vel: vec3<f32>) -> vec3<f32> {
    return vel + -vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
}

fn smoothing_kernel(radius: f32, dst: f32) -> f32 {
    if dst >= radius {
        return 0.0;
    }
    var volume = PI * pow(radius, 4.0) / 6.0;
    var off = radius - dst;
    return off * off / volume;
}

fn smoothing_kernel_deriv(radius: f32, dst: f32) -> f32 {
    if dst >= radius {
        return 0.0;
    }
    var scale = 12.0 / (PI * pow(radius, 4.0));
    return (dst - radius) * scale;
}

fn viscosity_smoothing_kernel(radius: f32, dst: f32) -> f32 {
    if dst >= radius {
        return 0.0;
    }
    let volume = PI * pow(radius, 8.0) / 4.0;
    let value = max(radius * radius - dst * dst, 0.0);
    return value * value * value / volume;
}

fn density_to_pressure(density: f32) -> f32 {
    var density_error = density - config.rest_density;
    return config.stiffness * density_error;
}

const CELL_OFFSETS: array<vec2<i32>, 9> = array(
    vec2(-1, -1), vec2(0, -1), vec2(1, -1),
    vec2(-1, 0), vec2(0, 0), vec2(1, 0),
    vec2(-1, 1), vec2(0, 1), vec2(1, 1),
);

const PI: f32 = 3.14159265359;
const MAX_U32: u32 = 0xffffffffu;
const EPSILON: f32 = 0.0001;
const LOOK_AHEAD_NUM = 120.0;
const LOOK_AHEAD_FACTOR: f32 = 1.0 / LOOK_AHEAD_NUM;

@compute
@workgroup_size(64)
fn upload_keys(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // buffers
    let r_pos = &buff_a;
    let w_pos = &buff_b;
    let r_vel = &buff_c;
    let w_vel = &buff_d;

    // setup
    var len = arrayLength(r_pos);
    var index = gid.x;

    if index >= len {
        return;
    }

    var pos = (*r_pos)[index].xyz;
    var vel = (*r_vel)[index].xyz;
    vel = apply_gravity(vel);
    // apply gravity to get predicted
    var predicted = pos + vel * LOOK_AHEAD_FACTOR;

    // compute spatial hash
    var cell = position_to_cell(predicted);
    var key = get_key(cell, len);
    keys[index] = key;
    indices[index] = index;
    (*w_pos)[index]  = vec4<f32>(predicted, 0.0);
    (*w_vel)[index] = vec4<f32>(vel, 0.0);
}

// sort between passes

@compute
@workgroup_size(64)
fn upload_start_indices(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // buffers
    let r_pos = &buff_b;

    // setup
    var len = arrayLength(r_pos);
    var index = gid.x;

    if index >= len {
        return;
    }

    let key = keys[index];

    // 0 is our sentinel value
    if index == 0u {
        intervals[key].start_idx = 1u;
    } else {
        // most cases
        let prev_key = keys[index - 1u];

        if key != prev_key {
            intervals[prev_key].end_idx = index + 1;
            intervals[key].start_idx = index + 1;
        }
    }

    // Close the final interval
    if index == len - 1u {
        intervals[key].end_idx = len + 1u;
    }
}

@compute
@workgroup_size(64)
fn gather_particles(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // buffers
    let r_pos = &buff_b;
    let w_pos = &buff_a;
    let r_vel = &buff_d;
    let w_vel = &buff_c;

    // setup
    var len = arrayLength(r_pos);
    var index = gid.x;

    if index >= len {
        return;
    }

    var sorted_idx = indices[index];

    (*w_pos)[index] = (*r_pos)[sorted_idx];
    (*w_vel)[index] = (*r_vel)[sorted_idx];
}

@compute
@workgroup_size(64)
fn compute_density(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // buffers
    let r_pos = &buff_a;
    let w_pos_density = &buff_b;

    // setup
    var len = arrayLength(r_pos);
    var index = gid.x;

    if index >= len {
        return;
    }

    // fetch particle
    var predicted = (*r_pos)[index].xyz;

    // compute density
    var cell = position_to_cell(predicted);
    var sq_radius = config.smoothing_radius * config.smoothing_radius;

    var density = 0.0;
    for (var i = 0u; i < 9; i++) {
        var offset = CELL_OFFSETS[i];
        var curr_cell = cell + vec3<i32>(offset.x, offset.y, 0);
        var key = get_key(curr_cell, len);
        var ivl = intervals[key];
        if ivl.start_idx == 0u {
            continue;
        }

        var start_idx = ivl.start_idx - 1u;
        var end_idx = ivl.end_idx - 1u;

        for (var other_idx = start_idx; other_idx < end_idx; other_idx++) {
            var other_predicted = (*r_pos)[other_idx].xyz;
            var other_cell = position_to_cell(other_predicted);
            if any(other_cell != curr_cell) {
                continue;
            }

            var offset = other_predicted - predicted;
            var sq_dst = dot(offset, offset);
            if sq_dst < sq_radius {
                var dst = sqrt(sq_dst);
                var influence = smoothing_kernel(config.smoothing_radius, dst);
                density += config.mass * influence;
            }
        }
    }

    (*w_pos_density)[index] = vec4<f32>(predicted, density);
}

// sync computed density globally

@compute
@workgroup_size(64)
fn pressure_viscosity(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // buffers
    let r_pos_density = &buff_b;
    let w_pos_density = &buff_a;
    let r_vel = &buff_c;
    let w_vel = &buff_d;

    // setup
    var len = arrayLength(r_pos_density);
    var index = gid.x;

    if index >= len {
        return;
    }

    var pos_density  = (*r_pos_density)[index];
    var predicted = pos_density.xyz;
    var density = pos_density.w;
    var vel = (*r_vel)[index].xyz;

    // compute pressure force
    var cell = position_to_cell(predicted);
    var sq_radius = config.smoothing_radius * config.smoothing_radius;

    var pressure_force = vec3<f32>(0.0, 0.0, 0.0);
    var viscosity = vec3<f32>(0.0, 0.0, 0.0);
    var pressure = density_to_pressure(density);
    for (var i = 0u; i < 9; i++) {
        var offset = CELL_OFFSETS[i];
        var curr_cell = cell + vec3<i32>(offset.x, offset.y, 0);
        var key = get_key(curr_cell, len);
        var ivl = intervals[key];
        if ivl.start_idx == 0u {
            continue;
        }

        var start_idx = ivl.start_idx - 1u;
        var end_idx = ivl.end_idx - 1u;

        for (var other_idx = start_idx; other_idx < end_idx; other_idx++) {
            var other_predicted_density = (*r_pos_density)[other_idx];
            var other_predicted = other_predicted_density.xyz;
            var other_density = other_predicted_density.w;

            var other_cell = position_to_cell(other_predicted);
            if any(other_cell != curr_cell) {
                continue;
            }

            var offset = other_predicted - predicted;
            var sq_dst = dot(offset, offset);
            if sq_dst < sq_radius {
                if other_idx == index {
                    continue;
                }
                var dst = sqrt(sq_dst);
                var slope: f32 = smoothing_kernel_deriv(config.smoothing_radius, dst);

                // handle dst = 0.0 case (move in random non degeneerate dir)
                var dir: vec3<f32>;
                if dst == 0.0 {
                    // opposite dirs
                    dir = select(
                        vec3<f32>(-1.0, 0.0, 0.0),
                        vec3<f32>(1.0, 0.0, 0.0),
                        index < other_idx,
                    );
                } else {
                    dir = offset / dst;
                }

                // shared pressure + neighbor density
                var other_pressure = density_to_pressure(other_density);
                var shared_pressure = (pressure + other_pressure) * 0.5;
                
                pressure_force += shared_pressure * dir * slope * config.mass / max(other_density, EPSILON);

                // compute viscosity force
                var other_vel = (*r_vel)[other_idx].xyz;
                var influence = viscosity_smoothing_kernel(config.smoothing_radius, dst);
                var vel_diff = other_vel - vel;

                viscosity += vel_diff * influence;
            }
        }
    }

    // apply pressure and viscosity force
    let div = 1.0 / max(density, EPSILON);
    let pressure_accel = pressure_force * div;
    let viscosity_accel = viscosity * config.viscosity_strength * div;

    // integrate velocity
    var pos = predicted - vel * LOOK_AHEAD_FACTOR;

    vel += (pressure_accel + viscosity_accel) * config.dt;
    pos += vel * config.dt;

    // store ssbo
    (*w_pos_density)[index] = vec4<f32>(pos, density);
    (*w_vel)[index] = vec4<f32>(vel, 0.0);
}

@compute
@workgroup_size(64)
fn handle_collisions(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // buffers
    let rw_pos_density = &buff_a;
    let rw_vel = &buff_d;

    // setup
    var len = arrayLength(rw_pos_density);
    var index = gid.x;

    if index >= len {
        return;
    }

    var pos_density = (*rw_pos_density)[index];
    var pos = pos_density.xyz;
    var density = pos_density.w;

    var vel = (*rw_vel)[index].xyz;

    // handle collisions
    var half_bound_size = config.size * 0.5;
    var damping = config.collision_damping;
    var collided = abs(pos) > half_bound_size;
    pos = select(pos, sign(pos) * half_bound_size, collided);
    vel *= select(vec3(1.0), -vec3(1.0 - damping), collided);

    (*rw_pos_density)[index] = vec4<f32>(pos, density);
    (*rw_vel)[index] = vec4<f32>(vel, 0.0);
}
