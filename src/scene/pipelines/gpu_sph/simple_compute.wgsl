struct Particle {
    pos: vec4<f32>,
    vel: vec3<f32>,
    density: f32,
}

struct SimConfig {
    size: vec3<f32>,
    collision_damping: f32,
    dt: f32,
    gravity: f32,
    smoothing_radius: f32,
};

@group(0) @binding(0) var<uniform> config: SimConfig;

@group(1) @binding(0) var<storage, read> particles_prev: array<Particle>;
@group(1) @binding(1) var<storage, read_write> particles_next: array<Particle>;

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

@compute
@workgroup_size(64)
fn spatial_hash(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // setup
    var len = arrayLength(&particles_prev);
    var index = gid.x;
    
    if index >= len {
        return;
    }
    
    var p = particles_prev[index];
    var pos = p.pos.xyz;
    var vel = p.vel;
    var density = p.density;

    // apply gravity
    vel += -vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
    var predicted = pos + vel * config.dt;

    // compute spatial hash
    var cell = position_to_cell(predicted);
    var key = hash_cell(cell) % len;
    grid_keys[index] = key;
    particle_indices[index] = index;
}

@compute
@workgroup_size(64)
fn apply_pressure(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // setup
    var len = arrayLength(&particles_prev);
    var index = gid.x;
    
    if index >= len {
        return;
    }
    
    var p = particles_prev[index];
    var pos = p.pos.xyz;
    var vel = p.vel;
    var density = p.density;

    // apply gravity
    vel += -vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
    var predicted = pos + vel * config.dt;
    
    // integrate velocity
    pos += vel * config.dt;

    // handle collisions
    var half_bound_size = config.size / 2.0;
    var damping = config.collision_damping;
    var collided = abs(pos) > half_bound_size;
    pos = select(pos, sign(pos) * half_bound_size, collided);
    vel *= select(vec3(1.0), -vec3(1.0 - damping), collided);

    // update global ssbo
    particles_next[index] = Particle(vec4(pos, 0.0), vel, density);
}
