use cgmath::Vector3;

use crate::config::WaterConfig;
pub struct Particle {
    pub pos: Vector3<f32>,
    pub vel: Vector3<f32>,
}

pub struct WaterSim {
    particles: Vec<Particle>,
    gravity: f32,
}

impl WaterSim {
    pub fn new(config: &WaterConfig) -> Self {
        let mut particles = Vec::with_capacity(config.num_particles as usize);
        let width = 20;
        for i in 0..config.num_particles {
            let pos = Vector3::new(
                i as f32 % width as f32,
                i as f32 / (width * width) as f32,
                (i as f32 / width as f32) % width as f32,
            );
            particles.push(Particle {
                pos,
                vel: Vector3::new(0.0, 0.0, 0.0),
            });
        }
        Self {
            particles,
            gravity: config.gravity,
        }
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn update(&mut self, dt: instant::Duration) {
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];
            p.vel.y -= self.gravity * dt.as_secs_f32();
            p.pos += p.vel * dt.as_secs_f32();
            self.resolve_collisions(i);
        }
    }

    fn resolve_collisions(&mut self, i: usize) {
        let half_bound_size = Vector3::new(10.0, 10.0, 10.0) * 2.0;
        let damping = 0.5;

        if self.particles[i].pos.x.abs() > half_bound_size.x {
            self.particles[i].pos.x = self.particles[i].pos.x.signum() * half_bound_size.x;
            self.particles[i].vel.x *= -(1.0 - damping);
        }
        if self.particles[i].pos.y.abs() > half_bound_size.y {
            self.particles[i].pos.y = self.particles[i].pos.y.signum() * half_bound_size.y;
            self.particles[i].vel.y *= -(1.0 - damping);
        }
        if self.particles[i].pos.z.abs() > half_bound_size.z {
            self.particles[i].pos.z = self.particles[i].pos.z.signum() * half_bound_size.z;
            self.particles[i].vel.z *= -(1.0 - damping);
        }
    }
}
