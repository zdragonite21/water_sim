use std::f32::consts::PI;

use cgmath::{InnerSpace, Vector3};
use rand::RngExt;
use rand::rngs::ThreadRng;

use crate::config::WaterSimConfig;
pub struct Particle {
    pub pos: Vector3<f32>,
    pub vel: Vector3<f32>,
    pub density: f32,
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
                density: 0.0,
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
        let dt = dt.as_secs_f32();

        for i in 0..self.particles.len() {
            let density = Self::calculate_density(&self.config, &self.particles, i);

            let p = &mut self.particles[i];
            p.vel += -Vector3::unit_y() * self.config.gravity * dt;
            p.density = density;
        }

        for i in 0..self.particles.len() {
            let pos = self.particles[i].pos;
            let pressure_force = -Self::calculate_pressure_force(&self.config, &self.particles, pos);
            let density = self.particles[i].density.max(f32::EPSILON);
            let pressure_accel = pressure_force / density;

            self.particles[i].vel += pressure_accel * dt;
        }

        for p in &mut self.particles {
            p.pos += p.vel * dt;
            Self::resolve_collisions(&self.config, p);
        }
    }

    fn resolve_collisions(config: &WaterSimConfig, p: &mut Particle) {
        let half_bound_size = Vector3::new(config.size_x, config.size_y, 10.0) / 2.0;
        let damping = 0.5;

        if p.pos.x.abs() > half_bound_size.x {
            p.pos.x = p.pos.x.signum() * half_bound_size.x;
            p.vel.x *= -(1.0 - damping);
        }
        if p.pos.y.abs() > half_bound_size.y {
            p.pos.y = p.pos.y.signum() * half_bound_size.y;
            p.vel.y *= -(1.0 - damping);
        }
        if p.pos.z.abs() > half_bound_size.z {
            p.pos.z = p.pos.z.signum() * half_bound_size.z;
            p.vel.z *= -(1.0 - damping);
        }
    }

    fn smoothing_kernel(radius: f32, dst: f32) -> f32 {
        let volume = PI * radius.powf(8.0) / 4.0;
        let value = (radius * radius - dst * dst).max(0.0);
        value * value * value / volume
    }

    fn smoothing_kernel_deriv(radius: f32, dst: f32) -> f32 {
        if dst >= radius {
            return 0.0;
        }
        let f = radius * radius - dst * dst;
        let scale = -24.0 * radius - dst * dst;
        scale * dst * f * f
    }

    fn calculate_density(config: &WaterSimConfig, particles: &[Particle], p_idx: usize) -> f32 {
        let mut density = 0.0;

        for i in 0..particles.len() {
            if i == p_idx {
                continue;
            }

            let offset = particles[i].pos - particles[p_idx].pos;
            let dst = offset.magnitude();
            

            let influence = Self::smoothing_kernel(config.smoothing_radius, dst);
            density += config.mass * influence;
        }
        density
    }

    fn calculate_pressure_force(
        config: &WaterSimConfig,
        particles: &[Particle],
        sample_point: Vector3<f32>,
    ) -> Vector3<f32> {
        let mut density_gradient = Vector3::new(0.0, 0.0, 0.0);

        for particle in particles {
            let offset = particle.pos - sample_point;
            let dst = offset.magnitude();
            if dst == 0.0 {
                continue;
            }

            let dir = offset / dst;
            let slope = Self::smoothing_kernel_deriv(config.smoothing_radius, dst);
            let density = particle.density.max(f32::EPSILON);
            density_gradient += -Self::convert_density_to_pressure(config, particle.density)
                * dir
                * slope
                * config.mass
                / density;
        }

        density_gradient
    }

    fn convert_density_to_pressure(config: &WaterSimConfig, density: f32) -> f32 {
        let density_error = density - config.target_density;
        density_error * config.pressure_multiplier
    }
}
