mod debug_overlay;
mod renderer;
mod sim;
pub mod config;

use cgmath::Matrix4;

use crate::scene::pipelines::PipelineId;

pub use debug_overlay::DebugOverlay;
pub use renderer::BillboardRenderer;
pub use sim::{Particle, Sim, Stats};

use serde::{Deserialize, Serialize};
use config::{DebugConfig, RenderConfig, SimConfig};
use crate::inspect::Inspect;


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub sim: SimConfig,
    pub render: RenderConfig,
    pub debug: DebugConfig,
}

impl Inspect for Config {
    fn inspect(&mut self, ui: &imgui::Ui) {
        ui.text("Simulation");
        self.sim.inspect(ui);
        ui.separator();
        ui.text("Rendering");
        self.render.inspect(ui);
        ui.separator();
        ui.text("Debug Overlay");
        self.debug.inspect(ui);
    }
}

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
        let mut sim = Sim::new(&config.sim);
        sim.reset();

        let renderer = BillboardRenderer::new(
            device,
            surface_config,
            bind_group_layout,
            config.sim.num_particles as usize,
            &config.render,
        )?;

        let debug_overlay = DebugOverlay::new(
            device,
            surface_config,
            bind_group_layout,
            DebugOverlay::required_capacity_for_config(
                &config.debug,
                sim.particles().len(),
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
            self.sim.smoothing_radius(),
        );
        self.debug_overlay.upload(queue, view_proj);
    }

    pub fn update_config(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &Config,
    ) {
        self.sim.update_config(&config.sim);
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
        PipelineId::CpuSph
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
            self.sim.smoothing_radius(),
        );
        self.debug_overlay.resize_capacity(device, capacity);
    }
}
