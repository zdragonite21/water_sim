mod debug_overlay;
mod renderer;
mod sim;

use crate::config::WaterConfig;
use cgmath::Matrix4;

pub use debug_overlay::DebugOverlay;
pub use renderer::WaterRenderer;
pub use sim::{Particle, WaterSim, WaterSimStats};

pub struct CpuSphParticlesPipeline {
    sim: WaterSim,
    renderer: WaterRenderer,
    debug_overlay: DebugOverlay,
}

impl CpuSphParticlesPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layout: &wgpu::BindGroupLayout,
        water_config: &WaterConfig,
    ) -> anyhow::Result<Self> {
        let mut sim = WaterSim::new(&water_config.sim);
        sim.reset();

        let renderer = WaterRenderer::new(
            device,
            config,
            bind_group_layout,
            water_config.sim.num_particles as usize,
            &water_config.render,
        )?;

        let debug_overlay = DebugOverlay::new(
            device,
            config,
            bind_group_layout,
            DebugOverlay::required_capacity_for_config(
                &water_config.debug,
                sim.particles().len(),
                &sim.bounds(),
                water_config.sim.smoothing_radius,
            ),
            &water_config.debug,
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
        self.sim.reset();
        let particle_count = self.sim.particles().len();
        self.renderer.resize_instance_buffer(device, particle_count);
        self.resize_debug_overlay_capacity(device);
    }

    pub fn update_fixed(&mut self, dt: instant::Duration) {
        self.sim.update(dt);
    }

    pub fn upload_frame(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        self.renderer.upload_particles(queue, self.sim.particles());
        self.debug_overlay.rebuild_lines(
            self.sim.particles(),
            &self.sim.bounds(),
            self.sim.current_config().smoothing_radius,
        );
        self.debug_overlay.upload(queue, view_proj);
    }

    pub fn update_config(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &WaterConfig,
    ) {
        self.sim.update_config(&config.sim);
        self.renderer.update_config(&config.render);
        self.debug_overlay.update_config(&config.debug);
        self.resize_debug_overlay_capacity(device);
        self.renderer
            .update_uniforms(queue, config.sim.target_density);
    }

    pub fn current_config(&self) -> WaterConfig {
        WaterConfig {
            sim: self.sim.current_config(),
            render: self.renderer.current_config(),
            debug: self.debug_overlay.current_config(),
        }
    }

    pub fn stats(&self) -> WaterSimStats {
        self.sim.get_stats()
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

    fn resize_debug_overlay_capacity(&mut self, device: &wgpu::Device) {
        let bounds = self.sim.bounds();
        let capacity = self.debug_overlay.required_capacity(
            self.sim.particles().len(),
            &bounds,
            self.sim.current_config().smoothing_radius,
        );
        self.debug_overlay.resize_capacity(device, capacity);
    }
}
