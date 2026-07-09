mod config;
mod debug_overlay;
mod renderer;
mod sim;

use crate::scene::pipelines::PipelineId;
use cgmath::Matrix4;
use debug_overlay::DebugOverlay;
use renderer::BillboardRenderer;
use sim::{Particle, Sim, Stats};

pub use config::Config;

pub struct Pipeline {
    sim: Sim,
    renderer: BillboardRenderer,
    debug_overlay: DebugOverlay,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout: &wgpu::BindGroupLayout,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let mut sim = Sim::new(device, &config.sim);
        // sim.reset();

        let renderer = BillboardRenderer::new(
            device,
            surface_config,
            bind_group_layout,
            sim.particle_buffer(),
            config.sim.num_particles as usize,
            &config.render,
        )?;

        let debug_overlay = DebugOverlay::new(
            device,
            surface_config,
            bind_group_layout,
            DebugOverlay::required_capacity_for_config(
                &config.debug,
                config.sim.num_particles as usize,
                &sim.bounds(),
                config.sim.smoothing_radius,
            ),
            &config.debug,
        )?;

        Ok(Self {
            sim,
            renderer,
            debug_overlay,
        })
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) {
        self.renderer.resize(device, config);
        self.debug_overlay.resize(queue, config);
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        // self.sim.reset();
        self.sim.resize_particle_buffers(device);
        self.resize_debug_overlay_capacity(device);
    }

    pub fn update_fixed(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, dt: instant::Duration) {
        self.sim.update_uniforms(queue, dt);
        self.sim.dispatch(device, queue);
    }

    pub fn upload_frame(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        // self.debug_overlay.rebuild_lines(
        //     self.sim.particles(),
        //     &self.sim.bounds(),
        //     self.sim.smoothing_radius(),
        // );
        self.debug_overlay.upload(queue, view_proj);
    }

    pub fn update_config(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, config: &Config) {
        self.sim.update_config(device, &config.sim);
        self.renderer.update_config(&config.render);
        self.debug_overlay.update_config(&config.debug);
        self.resize_debug_overlay_capacity(device);
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
        self.debug_overlay
            .draw(encoder, target_view, camera_bind_group)?;
        Ok(())
    }

    fn resize_debug_overlay_capacity(&mut self, _device: &wgpu::Device) {
        // let bounds = self.sim.bounds();
        // let capacity = self.debug_overlay.required_capacity(
        //     self.sim.particles().len(),
        //     &bounds,
        //     self.sim.smoothing_radius(),
        // );
        // self.debug_overlay.resize_capacity(device, capacity);
        // todo!();
    }
}
