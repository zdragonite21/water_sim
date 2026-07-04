pub mod cpu_sph_particles;

use crate::config::WaterConfig;
use cgmath::Matrix4;
use cpu_sph_particles::{CpuSphParticlesPipeline, WaterSimStats};

pub enum WaterPipeline {
    CpuSphParticles(CpuSphParticlesPipeline),
}

impl WaterPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layout: &wgpu::BindGroupLayout,
        water_config: &WaterConfig,
    ) -> anyhow::Result<Self> {
        Ok(Self::CpuSphParticles(CpuSphParticlesPipeline::new(
            device,
            config,
            bind_group_layout,
            water_config,
        )?))
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) {
        match self {
            Self::CpuSphParticles(pipeline) => pipeline.resize(device, queue, config),
        }
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        match self {
            Self::CpuSphParticles(pipeline) => pipeline.reset(device),
        }
    }

    pub fn update_fixed(&mut self, dt: instant::Duration) {
        match self {
            Self::CpuSphParticles(pipeline) => pipeline.update_fixed(dt),
        }
    }

    pub fn upload_frame(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        match self {
            Self::CpuSphParticles(pipeline) => pipeline.upload_frame(queue, view_proj),
        }
    }

    pub fn update_config(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &WaterConfig,
    ) {
        match self {
            Self::CpuSphParticles(pipeline) => pipeline.update_config(device, queue, config),
        }
    }

    pub fn current_config(&self) -> WaterConfig {
        match self {
            Self::CpuSphParticles(pipeline) => pipeline.current_config(),
        }
    }

    pub fn stats(&self) -> WaterSimStats {
        match self {
            Self::CpuSphParticles(pipeline) => pipeline.stats(),
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        match self {
            Self::CpuSphParticles(pipeline) => {
                pipeline.render(encoder, target_view, camera_bind_group)
            }
        }
    }
}
