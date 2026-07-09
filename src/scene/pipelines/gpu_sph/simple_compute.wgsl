struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
}

struct SimConfig {
    size: vec3<f32>,
    collision_damping: f32,
    dt: f32,
    gravity: f32,
};

@group(0) @binding(0) var<uniform> config: SimConfig;

@group(1) @binding(0) var<storage, read> particles_prev: array<Particle>;
@group(1) @binding(1) var<storage, read_write> particles_next: array<Particle>;



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
    
    var p = particles_prev[index];
    var pos = p.pos.xyz;
    var vel = p.vel.xyz;

    // apply gravity
    vel += -vec3<f32>(0.0, 1.0, 0.0) * config.gravity * config.dt;
    
    // integrate velocity
    pos += vel * config.dt;

    // handle collisions
    var half_bound_size = config.size / 2.0;
    var damping = config.collision_damping;
    var collided = abs(pos) > half_bound_size;
    pos = select(pos, sign(pos) * half_bound_size, collided);
    vel *= select(vec3(1.0), -vec3(1.0 - damping), collided);

    // update global ssbo
    particles_next[index] = Particle(vec4(pos, 0.0), vec4(vel, 0.0));
}
