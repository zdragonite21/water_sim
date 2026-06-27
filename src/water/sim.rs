use cgmath::Vector3;
use crate::water::renderer::InstanceRaw;
pub struct Particle {
    pub pos: Vector3<f32>,
    pub vel: Vector3<f32>,
}

impl Particle {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            pos: self.pos.into(),
            vel: self.vel.into(),
        }
    }
}

pub struct WaterSim {
    pub particles: Vec<Particle>,
}

impl WaterSim {
    pub fn new(num_particles: usize) -> Self {
        let mut particles = Vec::with_capacity(num_particles);
        let width = 20;
        for i in 0..num_particles {
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
        Self { particles }
    }

    pub fn update(&mut self, _dt: instant::Duration) {
        // physics rules live here
    }
}
