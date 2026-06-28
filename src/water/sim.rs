use cgmath::Vector3;
use rand::RngExt;
use rand::rngs::ThreadRng;

use crate::config::WaterSimConfig;
pub struct Particle {
    pub pos: Vector3<f32>,
    pub vel: Vector3<f32>,
}

pub struct WaterSim {
    particles: Vec<Particle>,
    config: WaterSimConfig,
    rng: ThreadRng,
}

impl WaterSim {
    pub fn new(config: &WaterSimConfig) -> Self {
        Self {
            config: config.clone(),
            particles: Vec::new(),
            rng: rand::rng(),
        }
    }

    fn create_particles(&mut self) {
        let n = self.config.num_particles as usize;
        let mut particles = Vec::with_capacity(n);

        for _ in 0..n {
            let xi_1 = self.rng.random_range(-1.0..=1.0);
            let xi_2 = self.rng.random_range(-1.0..=1.0);

            let pos = Vector3::new(xi_1 * self.config.size_x, xi_2 * self.config.size_y, 0.0) / 2.0;
            particles.push(Particle {
                pos,
                vel: Vector3::new(0.0, 0.0, 0.0),
            });
        }
        self.particles = particles;
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn update_config(&mut self, config: &WaterSimConfig) {
        if config == &self.config {
            return;
        }

        self.config = config.clone();
    }

    pub fn reset(&mut self) {
        self.create_particles();
    }

    pub fn current_config(&self) -> WaterSimConfig {
        self.config.clone()
    }

    pub fn update(&mut self, dt: instant::Duration) {
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];
            p.vel.y -= self.config.gravity * dt.as_secs_f32();
            p.pos += p.vel * dt.as_secs_f32();
            self.resolve_collisions(i);
        }
    }

    fn resolve_collisions(&mut self, i: usize) {
        let half_bound_size = Vector3::new(self.config.size_x, self.config.size_y, 10.0) / 2.0;
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
