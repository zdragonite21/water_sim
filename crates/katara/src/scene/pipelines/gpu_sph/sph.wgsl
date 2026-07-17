const WORK_GROUP_SIZE: u32 = 128u;

struct SimConfig {
    size: vec3<f32>,
    collision_damping: f32,
    dt: f32,
    gravity: f32,
    smoothing_radius: f32,
    inv_smoothing_radius: f32,
    stiffness: f32,
    viscosity_strength: f32,
    rest_density: f32,
    mass: f32,
    inv_density_kernel_volume: f32,
    density_kernel_scale: f32,
    inv_viscosity_kernel_volume: f32,
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
    return vec3<i32>(floor(pos * config.inv_smoothing_radius));
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

fn hash_u32(x_in: u32) -> u32 {
    var x = x_in;
    x ^= x >> 16u;
    x *= 0x7feb352du;
    x ^= x >> 15u;
    x *= 0x846ca68bu;
    x ^= x >> 16u;
    return x;
}

fn random_f32(seed: u32) -> f32 {
    // Uses 24 bits, matching roughly the precision available in f32.
    return f32(hash_u32(seed) >> 8u) * (1.0 / 16777216.0);
}

fn random_vec3(seed: u32) -> vec3<f32> {
    return vec3<f32>(
        random_f32(seed),
        random_f32(seed ^ 0x9e3779b9u),
        random_f32(seed ^ 0x85ebca6bu),
    );
}

fn random_unit_vec3_fast(seed: u32) -> vec3<f32> {
    let v = random_vec3(seed) * 2.0 - 1.0;
    return v * inverseSqrt(max(dot(v, v), 1e-12));
}

fn smoothing_kernel(dst: f32) -> f32 {
    let radius = config.smoothing_radius;
    if dst >= radius {
        return 0.0;
    }
    var off = radius - dst;
    return off * off * config.inv_density_kernel_volume;
}

fn smoothing_kernel_deriv(dst: f32) -> f32 {
    let radius = config.smoothing_radius;
    if dst >= radius {
        return 0.0;
    }
    return (dst - radius) * config.density_kernel_scale;
}

fn viscosity_smoothing_kernel(dst: f32) -> f32 {
    let radius = config.smoothing_radius;
    if dst >= radius {
        return 0.0;
    }
    let value = max(radius * radius - dst * dst, 0.0);
    return value * value * value * config.inv_viscosity_kernel_volume;
}

fn density_to_pressure(density: f32) -> f32 {
    var density_error = density - config.rest_density;
    return config.stiffness * density_error;
}

const PI: f32 = 3.14159265359;
const MAX_U32: u32 = 0xffffffffu;
const EPSILON: f32 = 0.0001;
const LOOK_AHEAD_NUM = 120.0;
const LOOK_AHEAD_FACTOR: f32 = 1.0 / LOOK_AHEAD_NUM;

fn apply_gravity(vel: vec3<f32>) -> vec3<f32> {
    return vel + -vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
}

fn apply_vortex(
    pos: vec3<f32>,
    vel: vec3<f32>,
    center: vec3<f32>,
    axis_in: vec3<f32>,
    radius: f32,
    spin: f32,
    pull: f32,
    lift: f32,
) -> vec3<f32> {
    // assume axis_in is normalized
    let axis = axis_in;
    let relative = pos - center;

    let axial_offset = dot(relative, axis);
    let radial = relative - axis * axial_offset;
    let distance = length(radial);

    if distance >= radius || distance < 0.0001 {
        return vel;
    }

    let radial_dir = radial / distance;
    let tangent = normalize(cross(axis, radial_dir));

    let x = 1.0 - distance / radius;
    let falloff = x * x;

    let acceleration = tangent * spin * falloff
        - radial_dir * pull * falloff
        + axis * lift * falloff;

    return vel + acceleration * config.dt;
}

@compute
@workgroup_size(WORK_GROUP_SIZE)
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
    (*w_pos)[index] = vec4<f32>(predicted, 0.0);
    (*w_vel)[index] = vec4<f32>(vel, 0.0);
}

// sort between passes

@compute
@workgroup_size(WORK_GROUP_SIZE)
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
@workgroup_size(WORK_GROUP_SIZE)
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
@workgroup_size(WORK_GROUP_SIZE)
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
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                var offset = vec3<i32>(x, y, z);
                var curr_cell = cell + offset;
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
                        var influence = smoothing_kernel(dst);
                        density += config.mass * influence;
                    }
                }
            }
        }
    }

    (*w_pos_density)[index] = vec4<f32>(predicted, density);
}

// sync computed density globally

@compute
@workgroup_size(WORK_GROUP_SIZE)
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

    var pos_density = (*r_pos_density)[index];
    var predicted = pos_density.xyz;
    var density = pos_density.w;
    var vel = (*r_vel)[index].xyz;

    // compute pressure force
    var cell = position_to_cell(predicted);
    var sq_radius = config.smoothing_radius * config.smoothing_radius;

    var pressure_force = vec3<f32>(0.0, 0.0, 0.0);
    var viscosity = vec3<f32>(0.0, 0.0, 0.0);
    var pressure = density_to_pressure(density);

    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                var offset = vec3<i32>(x, y, z);
                var curr_cell = cell + offset;
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
                        var slope = smoothing_kernel_deriv(dst);

                        // handle dst = 0.0 case (move in random non degeneerate dir)
                        var dir: vec3<f32>;
                        if dst == 0.0 {
                            // random, equal and opposite dirs
                            dir = random_unit_vec3_fast(other_idx ^ index);
                            dir *= select(1.0, -1.0, index > other_idx);
                        } else {
                            dir = offset / dst;
                        }

                        // shared pressure + neighbor density
                        var other_pressure = density_to_pressure(other_density);
                        var shared_pressure = (pressure + other_pressure) * 0.5;

                        pressure_force += shared_pressure * dir * slope * config.mass / max(other_density, EPSILON);

                        // compute viscosity force
                        var other_vel = (*r_vel)[other_idx].xyz;
                        var influence = viscosity_smoothing_kernel(dst);
                        var vel_diff = other_vel - vel;

                        viscosity += vel_diff * influence;
                    }
                }
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
@workgroup_size(WORK_GROUP_SIZE)
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
