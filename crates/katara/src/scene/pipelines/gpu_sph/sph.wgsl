struct Particle {
    pos: vec4<f32>,
}

struct VelocityDensity {
    vel: vec3<f32>,
    density: f32,
}

struct ParticleData {
    pos: vec3<f32>,
    vel: vec3<f32>,
    predicted: vec3<f32>,
    density: f32,
}

struct SimConfig {
    size: vec3<f32>,
    collision_damping: f32,
    dt: f32,
    gravity: f32,
    smoothing_radius: f32,
    stiffness: f32,
    rest_density: f32,
    mass: f32,
};

// uniforms
@group(0) @binding(0) var<uniform> config: SimConfig;

// ping pong buffers
@group(1) @binding(0) var<storage, read> particles_prev: array<Particle>;
@group(1) @binding(1) var<storage, read_write> particles_next: array<Particle>;
@group(1) @binding(2) var<storage, read_write> particle_vel_density: array<VelocityDensity>;


// spatial hashing
@group(2) @binding(0) var<storage, read_write> grid_keys: array<u32>;
@group(2) @binding(1) var<storage, read_write> particle_indices: array<u32>;
@group(2) @binding(2) var<storage, read_write> start_indices: array<u32>;

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

fn get_start_idx(key: u32) -> u32 {
    return start_indices[key] - 1;
}

fn apply_gravity(vel: vec3<f32>) -> vec3<f32> {
    return vel + -vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
}

fn fetch_particle(index: u32, fetch_velocity: bool, fetch_density: bool) -> ParticleData {
    var p = particles_prev[index];
    var pos = p.pos.xyz;

    // compute velocity from previous position (dt is fixed), data race in main pass
    var vel: vec3<f32>;
    var density: f32;
    if !fetch_velocity {
        var p_old = particles_next[index];
        vel = (pos - p_old.pos.xyz) / config.dt;
        density = select(
            0.0,
            particle_vel_density[index].density,
            fetch_density
        );
        vel = apply_gravity(vel);
    } else {
        var vel_density = particle_vel_density[index];
        vel = vel_density.vel;
        density = select(
            0.0,
            vel_density.density,
            fetch_density
        );
    }
    // apply gravity to get predicted
    var predicted = pos + vel * config.dt;

    return ParticleData(pos, vel, predicted, density);
}

fn store_particle(index: u32, p: ParticleData) {
    particles_next[index].pos = vec4<f32>(p.pos, 1.0);
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

fn density_to_pressure(density: f32) -> f32 {
    var density_error = density - config.rest_density;
    return config.stiffness * density_error;
}

const CELL_OFFSETS: array<vec2<i32>, 9> = array(
    vec2(-1, -1), vec2( 0, -1), vec2( 1, -1),
    vec2(-1,  0), vec2( 0,  0), vec2( 1,  0),
    vec2(-1,  1), vec2( 0,  1), vec2( 1,  1),
);

const PI: f32 = 3.14159265359;
const MAX_U32: u32 = 0xffffffffu;
const EPSILON: f32 = 0.0001;

@compute
@workgroup_size(64)
fn upload_keys(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // setup
    var len = arrayLength(&particles_prev);
    var index = gid.x;
    
    if index >= len {
        return;
    }
    
    var p = fetch_particle(index, false, false);

    // compute spatial hash
    var cell = position_to_cell(p.predicted);
    var key = get_key(cell, len);
    grid_keys[index] = key;
    particle_indices[index] = index;
}

// sort between passes

@compute
@workgroup_size(64)
fn upload_start_indices(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // setup
    var len = arrayLength(&particles_prev);
    var index = gid.x;
    
    if index >= len {
        return;
    }

    var key = grid_keys[index];

    // first particle always starts at 0
    // if the key is different than the previous, this is a new start index
    if index == 0u || key != grid_keys[index - 1u] {
        start_indices[key] = index + 1;
    }
}

// sync start indices

@compute
@workgroup_size(64)
fn compute_density(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // setup
    var len = arrayLength(&particles_prev);
    var index = gid.x;
    
    if index >= len {
        return;
    }

    var p = fetch_particle(index, false,false);

    // compute density
    var cell = position_to_cell(p.predicted);
    var sq_radius = config.smoothing_radius * config.smoothing_radius;

    for (var i = 0u; i < 9; i++) {
        var offset = CELL_OFFSETS[i];
        var curr_cell = cell + vec3<i32>(offset.x, offset.y, 0);
        var key = get_key(curr_cell, len);
        var cell_start_idx = get_start_idx(key);
        if cell_start_idx == MAX_U32 {
            continue;
        }

        for (var j = cell_start_idx; j < len; j++) {
            if grid_keys[j] != key {
                break;
            }
            
            var other_idx = particle_indices[j];
            var other = fetch_particle(other_idx, false, false);
            var other_cell = position_to_cell(other.predicted);
            if (any(other_cell != curr_cell)) {
                continue;
            }

            var offset = other.predicted - p.predicted;
            var sq_dst = dot(offset, offset);
            if sq_dst < sq_radius {
                var dst = sqrt(sq_dst);
                var influence = smoothing_kernel(config.smoothing_radius, dst);
                p.density += config.mass * influence;
            }
        }
    }

    // for (var i = 0u; i < len; i++) {
    //     var other = fetch_particle(i, false);
    //     var offset = other.predicted - p.predicted;
    //     var sq_dst = dot(offset, offset);
    //     if sq_dst < sq_radius {
    //         var dst = sqrt(sq_dst);
    //         var influence = smoothing_kernel(config.smoothing_radius, dst);
    //         p.density += config.mass * influence;
    //     }
    // }

    particle_vel_density[index].vel = p.vel;
    particle_vel_density[index].density = p.density;
}

// sync computed density globally

@compute
@workgroup_size(64)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // setup
    var len = arrayLength(&particles_prev);
    var index = gid.x;
    
    if index >= len {
        return;
    }

    var p = fetch_particle(index, true, true);
    // todo: it's possible that some threads write to memory before other threads read for copmuting velocity
    // solve this by dipatching per cell, not per particle
    // workgroupBarrier();

    // compute pressure force
    var cell = position_to_cell(p.predicted);
    var sq_radius = config.smoothing_radius * config.smoothing_radius;

    var pressure_force: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var pressure = density_to_pressure(p.density);
    for (var i = 0u; i < 9; i++) {
        var offset = CELL_OFFSETS[i];
        var curr_cell = cell + vec3<i32>(offset.x, offset.y, 0);
        var key = get_key(curr_cell, len);
        var cell_start_idx = get_start_idx(key);
        if cell_start_idx == MAX_U32 {
            continue;
        }

        for (var j = cell_start_idx; j < len; j++) {
            if grid_keys[j] != key {
                break;
            }
            
            var other_idx = particle_indices[j];
            var other = fetch_particle(other_idx, true, true);
            var other_cell = position_to_cell(other.predicted);
            if (any(other_cell != curr_cell)) {
                continue;
            }

            var offset = other.predicted - p.predicted;
            var sq_dst = dot(offset, offset);
            if sq_dst < sq_radius {
                if j == index {
                    continue;
                }
                var dst = sqrt(sq_dst);
                var slope: f32 = smoothing_kernel_deriv(config.smoothing_radius, dst);

                // handle dst = 0.0 case (move in random non degeneerate dir)
                var dir = select(offset / dst, vec3<f32>(1.0, 0.0, 0.0), dst == 0.0);
                
                // shared pressure + neighbor density
                var other_pressure = density_to_pressure(other.density);
                var shared_pressure = (pressure + other_pressure) / 2.0;

                pressure_force += shared_pressure * dir * slope * config.mass / max(p.density, EPSILON);
            }
        }
    }

    // apply pressure force
    let pressure_accel = pressure_force / max(p.density, EPSILON);
    p.vel += pressure_accel * config.dt;

    // integrate velocity
    p.pos += p.vel * config.dt;

    // handle collisions
    var half_bound_size = config.size / 2.0;
    var damping = config.collision_damping;
    var collided = abs(p.pos) > half_bound_size;
    p.pos = select(p.pos, sign(p.pos) * half_bound_size, collided);
    p.vel *= select(vec3(1.0), -vec3(1.0 - damping), collided);

    // store ssbo
    store_particle(index, p);
}
