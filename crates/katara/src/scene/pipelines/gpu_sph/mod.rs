mod config;
mod renderer;
mod sim;

use crate::{profiling::GpuFrameRecorder, scene::pipelines::PipelineId};
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
        gpu_recorder: Option<GpuFrameRecorder>,
    ) -> anyhow::Result<Self> {
        let sim = Sim::new(device, &config.sim, gpu_recorder);

        let renderer = BillboardRenderer::new(
            device,
            surface_config,
            bind_group_layout,
            &config.render,
            sim.velocity_buffers(),
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
        self.renderer.reset(device, self.sim.velocity_buffers());
    }

    pub fn update_fixed(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        self.sim.dispatch(encoder, queue);
    }

    pub fn update_config(
        &mut self,
        queue: &wgpu::Queue,
        config: &Config,
        fixed_dt: instant::Duration,
    ) {
        self.sim.update_config(&config.sim);
        self.sim.update_uniforms(queue, fixed_dt);
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
        self.renderer.draw(
            encoder,
            self.sim.particle_buffer(),
            self.sim.num_particles(),
            target_view,
            camera_bind_group,
            self.sim.velocity_buffer_index(),
        )?;
        Ok(())
    }
}
