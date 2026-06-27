use cgmath::Vector3;
pub struct Particle {
    pub pos: Vector3<f32>,
    pub vel: Vector3<f32>,
}

impl Particle {
    pub fn to_raw(&self) -> ParticleRaw {
        ParticleRaw {
            pos: self.pos.into(),
            vel: self.vel.into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    pos: [f32; 3],
    vel: [f32; 3],
}

impl ParticleRaw {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![5 => Float32x3, 6 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}
