mod config;
mod renderer;
mod sim;

use crate::scene::pipelines::PipelineId;
use renderer::BillboardRenderer;
use sim::{Sim, Stats};

pub use config::Config;

pub struct Pipeline {
    sim: Sim,
    renderer: BillboardRenderer,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout: &wgpu::BindGroupLayout,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let sim = Sim::new(device, &config.sim);

        let renderer = BillboardRenderer::new(
            device,
            surface_config,
            bind_group_layout,
            sim.particle_buffer(),
            config.sim.num_particles as usize,
            &config.render,
        )?;

        Ok(Self { sim, renderer })
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) {
        self.renderer.resize(device, config);
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        self.sim.reset(device);
        self.renderer
            .reset(self.sim.particle_buffer(), self.sim.num_particles());
    }

    pub fn update_fixed(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.sim.dispatch(encoder);
    }

    pub fn update_config(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &Config,
        dt: instant::Duration,
    ) {
        self.sim.update_config(device, &config.sim);
        self.sim.update_uniforms(queue, dt);
        self.renderer.update_config(&config.render);
        self.renderer
            .update_uniforms(queue, config.sim.target_density);
    }

    pub fn stats(&self) -> Stats {
        self.sim.get_stats()
    }

    pub fn id(&self) -> PipelineId {
        PipelineId::GpuSph
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        self.renderer
            .draw(encoder, target_view, camera_bind_group)?;
        Ok(())
    }
}
