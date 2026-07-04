use std::f32::consts::PI;

use cgmath::{InnerSpace, Point3, Vector3};
use rand::RngExt;
use rand::rngs::ThreadRng;

use super::config::SimConfig;
use crate::{dwatch, stats::debug_stats};

debug_stats! {
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub struct Stats {
        stat avg_density: f32 = 0.0;
        stat particle_count: usize = 0;
    }
}

pub struct Particle {
    pub pos: Point3<f32>,
    pub vel: Vector3<f32>,
    pub density: f32,
    pub predicted: Point3<f32>,
}

pub struct Sim {
    particles: Vec<Particle>,
    spatial_grid: SpatialGrid,
    config: SimConfig,
    rng: ThreadRng,
}

impl Sim {
    const LOOK_AHEAD_FACTOR: f32 = 1.0 / 120.0;

    pub fn new(config: &SimConfig) -> Self {
        Self {
            particles: Vec::new(),
            spatial_grid: SpatialGrid::new(),
            config: config.clone(),
            rng: rand::rng(),
        }
    }

    fn create_particles(&mut self) {
        let n = self.config.num_particles as usize;
        let mut particles = Vec::with_capacity(n);

        let scale = 0.5;

        let width = self.config.size[0] * scale;
        let height = self.config.size[1] * scale;

        let num_row = (n as f32 * width / height).sqrt().floor() as usize;
        let num_col = (n as f32 * height / width).sqrt().floor() as usize;

        for i in 0..n {
            let x = (i % num_row) as f32 / num_row as f32 * width;
            let y = (i / num_row) as f32 / num_col as f32 * height;

            let pos = Point3::new(x as f32 - width / 2.0, y as f32 - height / 2.0, 0.0);
            particles.push(Particle {
                pos,
                vel: Vector3::new(0.0, 0.0, 0.0),
                density: 0.0,
                predicted: Point3::new(0.0, 0.0, 0.0),
            });
        }
        self.particles = particles;
    }

    pub fn reset(&mut self) {
        self.create_particles();
        self.rebuild_spatial_grid();
    }

    pub fn update(&mut self, dt: instant::Duration) {
        let dt = dt.as_secs_f32();

        self.apply_gravity(dt);

        self.rebuild_spatial_grid();

        self.compute_densities();

        self.apply_pressure_viscosity_forces(dt);

        self.integrate_velocities(dt);
    }
}

impl Sim {
    fn apply_gravity(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.vel += -Vector3::unit_y() * self.config.gravity * dt;
            p.predicted = p.pos + p.vel * Self::LOOK_AHEAD_FACTOR;
        }
    }

    fn rebuild_spatial_grid(&mut self) {
        self.spatial_grid
            .update_spatial_lookup(&self.particles, self.config.smoothing_radius);
    }

    fn compute_densities(&mut self) {
        let particle_count = self.particles.len();
        let mut dbg_density_total = 0.0;
        let mut dbg_neighbor_total = 0;

        for i in 0..self.particles.len() {
            let mut density = 0.0;
            let sample_point = self.particles[i].predicted;

            dbg_neighbor_total += self.spatial_grid.for_each_neighbor(
                &self.particles,
                self.config.smoothing_radius,
                sample_point,
                |_neighbor_idx, _neighbor, _offset, dst| {
                    let influence = Self::smoothing_kernel(self.config.smoothing_radius, dst);
                    density += self.config.mass * influence;
                },
            );
            self.particles[i].density = density;

            dbg_density_total += density;
        }

        dwatch!("sim.avg_density", dbg_density_total / particle_count as f32);

        let volume = self.config.size[0] * self.config.size[1] * self.config.size[2];
        let exp_density = particle_count as f32 * self.config.mass / volume;
        dwatch!("sim.exp_density", exp_density);

        dwatch!(
            "sim.avg_neighbor_count",
            dbg_neighbor_total as f32 / particle_count as f32
        );

        let exp_neighbor_count =
            PI * self.config.smoothing_radius.powi(2) * exp_density / self.config.mass;
        dwatch!("sim.exp_neighbor_count", exp_neighbor_count);
    }

    fn apply_pressure_viscosity_forces(&mut self, dt: f32) {
        for i in 0..self.particles.len() {
            let pressure_force = -Self::calculate_pressure_force(
                &self.spatial_grid,
                &self.config,
                &self.particles,
                i,
                &mut self.rng,
            );
            let viscosity_force = Self::calculate_viscosity_force(
                &self.spatial_grid,
                &self.config,
                &self.particles,
                i,
            );
            let density = self.particles[i].density.max(f32::EPSILON);
            let pressure_accel = pressure_force / density;
            let viscosity_accel = viscosity_force / density;

            self.particles[i].vel += (pressure_accel + viscosity_accel) * dt;
        }
    }

    fn integrate_velocities(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.pos += p.vel * dt;
            Self::resolve_collisions(&self.config, p);
        }
    }

    fn resolve_collisions(config: &SimConfig, p: &mut Particle) {
        let half_bound_size = Vector3::from(config.size) / 2.0;
        let damping = config.damping;

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
        if dst >= radius {
            return 0.0;
        }
        let volume = PI * radius.powf(4.0) / 6.0;
        let off = radius - dst;
        off * off / volume
    }

    fn smoothing_kernel_deriv(radius: f32, dst: f32) -> f32 {
        if dst >= radius {
            return 0.0;
        }
        let scale = 12.0 / (PI * radius.powf(4.0));
        (dst - radius) * scale
    }

    fn viscosity_smoothing_kernel(radius: f32, dst: f32) -> f32 {
        if dst >= radius {
            return 0.0;
        }
        let volume = PI * radius.powf(8.0) / 4.0;
        let value = (radius * radius - dst * dst).max(0.0);
        value * value * value / volume
    }

    fn calculate_pressure_force(
        grid: &SpatialGrid,
        config: &SimConfig,
        particles: &[Particle],
        particle_idx: usize,
        rng: &mut ThreadRng,
    ) -> Vector3<f32> {
        let mut density_gradient = Vector3::new(0.0, 0.0, 0.0);
        let sample_point = particles[particle_idx].predicted;

        let pressure = Self::convert_density_to_pressure(config, particles[particle_idx].density);

        grid.for_each_neighbor(
            particles,
            config.smoothing_radius,
            sample_point,
            |neighbor_idx, neighbor, offset, dst| {
                if particle_idx == neighbor_idx {
                    return;
                }

                let dir = if dst == 0.0 {
                    Self::random_unit_dir(rng)
                } else {
                    -offset / dst
                };
                let slope = Self::smoothing_kernel_deriv(config.smoothing_radius, dst);
                let density = neighbor.density.max(f32::EPSILON);
                let neighbor_pressure = Self::convert_density_to_pressure(config, density);
                let shared_pressure = (pressure + neighbor_pressure) * 0.5;
                density_gradient += shared_pressure * dir * slope * config.mass / density;
            },
        );

        density_gradient
    }

    fn calculate_viscosity_force(
        grid: &SpatialGrid,
        config: &SimConfig,
        particles: &[Particle],
        particle_idx: usize,
    ) -> Vector3<f32> {
        let mut viscosity = Vector3::new(0.0, 0.0, 0.0);
        let sample_point = particles[particle_idx].predicted;

        grid.for_each_neighbor(
            particles,
            config.smoothing_radius,
            sample_point,
            |neighbor_idx, neighbor, _offset, dst| {
                if particle_idx == neighbor_idx {
                    return;
                }

                let influence = Self::viscosity_smoothing_kernel(config.smoothing_radius, dst);
                let velocity_diff = neighbor.vel - particles[particle_idx].vel;
                viscosity += velocity_diff * influence;
            },
        );

        viscosity * config.viscosity_strength
    }

    fn convert_density_to_pressure(config: &SimConfig, density: f32) -> f32 {
        let density_error = density - config.target_density;
        density_error * config.pressure_multiplier
    }

    fn random_unit_dir(rng: &mut ThreadRng) -> Vector3<f32> {
        let theta = rng.random_range(0.0..=2.0 * PI);
        let x = theta.cos();
        let y = theta.sin();
        Vector3::new(x, y, 0.0)
    }
}

impl Sim {
    pub fn get_stats(&self) -> Stats {
        let particle_count = self.particles.len();
        let density_sum: f32 = self.particles.iter().map(|p| p.density).sum();
        let avg_density = if particle_count > 0 {
            density_sum / particle_count as f32
        } else {
            0.0
        };

        Stats {
            avg_density,
            particle_count,
        }
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn smoothing_radius(&self) -> f32 {
        self.config.smoothing_radius
    }

    pub fn update_config(&mut self, config: &SimConfig) {
        if config == &self.config {
            return;
        }

        self.config = config.clone();
    }

    pub fn bounds(&self) -> Vector3<f32> {
        self.config.size.into()
    }
}

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq)]
struct GridEntry {
    cell_key: usize,
    particle_idx: usize,
}
struct SpatialGrid {
    spatial_lookup: Vec<GridEntry>,
    start_indices: Vec<usize>,
}

impl SpatialGrid {
    const CELL_OFFSETS: [(i32, i32); 9] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    fn new() -> Self {
        Self {
            spatial_lookup: Vec::new(),
            start_indices: Vec::new(),
        }
    }

    fn update_spatial_lookup(&mut self, particles: &[Particle], radius: f32) {
        self.spatial_lookup.resize(
            particles.len(),
            GridEntry {
                cell_key: 0,
                particle_idx: 0,
            },
        );
        self.start_indices.resize(particles.len(), usize::MAX);

        for i in 0..particles.len() {
            let (cell_x, cell_y) = Self::position_to_cell(particles[i].predicted, radius);
            let cell_key =
                Self::get_key_from_hash(Self::hash_cell(cell_x, cell_y), particles.len());
            self.spatial_lookup[i] = GridEntry {
                cell_key,
                particle_idx: i,
            };
            self.start_indices[i] = usize::MAX;
        }

        self.spatial_lookup.sort_by_key(|entry| entry.cell_key);

        // reverse order to get the first index
        for i in (0..particles.len()).rev() {
            let cell_key = self.spatial_lookup[i].cell_key;
            self.start_indices[cell_key] = i;
        }
    }

    fn position_to_cell(pos: Point3<f32>, radius: f32) -> (i32, i32) {
        let cell_x = (pos.x / radius).floor() as i32;
        let cell_y = (pos.y / radius).floor() as i32;
        (cell_x, cell_y)
    }

    fn hash_cell(cell_x: i32, cell_y: i32) -> u64 {
        const PRIME1: u64 = 73_856_093;
        const PRIME2: u64 = 19_349_663;

        let x = cell_x as u32 as u64;
        let y = cell_y as u32 as u64;

        x.wrapping_mul(PRIME1) ^ y.wrapping_mul(PRIME2)
    }

    fn get_key_from_hash(hash: u64, length: usize) -> usize {
        hash as usize % length
    }

    fn for_each_neighbor<F>(
        &self,
        particles: &[Particle],
        radius: f32,
        sample_point: Point3<f32>,
        mut f: F,
    ) -> u32
    where
        F: FnMut(usize, &Particle, Vector3<f32>, f32),
    {
        if particles.is_empty()
            || self.spatial_lookup.len() != particles.len()
            || self.start_indices.len() != particles.len()
        {
            return 0;
        }

        let (center_x, center_y) = Self::position_to_cell(sample_point, radius);
        let sq_radius = radius * radius;

        let mut num_neighbors = 0;

        for (off_x, off_y) in Self::CELL_OFFSETS {
            let curr_cell = (center_x + off_x, center_y + off_y);
            let key =
                Self::get_key_from_hash(Self::hash_cell(curr_cell.0, curr_cell.1), particles.len());

            let cell_start_idx = self.start_indices[key];
            if cell_start_idx == usize::MAX {
                continue;
            }

            for i in cell_start_idx..self.spatial_lookup.len() {
                if self.spatial_lookup[i].cell_key != key {
                    break;
                }

                let neighbor_pos = particles[self.spatial_lookup[i].particle_idx].predicted;
                let neighbor_cell = Self::position_to_cell(neighbor_pos, radius);
                if neighbor_cell != curr_cell {
                    continue;
                }

                let particle_idx = self.spatial_lookup[i].particle_idx;
                let offset = neighbor_pos - sample_point;
                let sq_dist = offset.magnitude2();
                if sq_dist < sq_radius {
                    f(
                        particle_idx,
                        &particles[particle_idx],
                        offset,
                        sq_dist.sqrt(),
                    );
                    num_neighbors += 1;
                }
            }
        }

        num_neighbors
    }
}
