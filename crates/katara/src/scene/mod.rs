mod config;
mod debug_overlay;
mod line_renderer;
mod renderer;
mod sim;

use crate::profiling::GpuFrameRecorder;
use cgmath::Matrix4;
use debug_overlay::DebugOverlay;
use renderer::BillboardRenderer;
use sim::Sim;

pub use config::SceneConfig;

pub struct Scene {
    sim: Sim,
    renderer: BillboardRenderer,
    debug_overlay: DebugOverlay,
    
    paused: bool,
    accumulator: instant::Duration,
    fixed_dt: instant::Duration,
    config: SceneConfig,
    steps: u32,
}

impl Scene {
    const SIM_HZ: u64 = 120;
    const MAX_STEPS: u8 = 8;

    pub async fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        cam_bind_group_layout: &wgpu::BindGroupLayout,
        config: &SceneConfig,
        gpu_recorder: Option<GpuFrameRecorder>,
    ) -> anyhow::Result<Self> {
        let sim = Sim::new(device, &config.sim, gpu_recorder.clone());

        let renderer = BillboardRenderer::new(
            device,
            surface_config,
            cam_bind_group_layout,
            &config.render,
            sim.velocity_buffers(),
        )?;

        let debug_overlay =
            DebugOverlay::new(device, surface_config, cam_bind_group_layout, &config.debug)?;

        Ok(Self {
            sim,
            renderer,
            debug_overlay,
            paused: false,
            accumulator: instant::Duration::ZERO,
            fixed_dt: instant::Duration::from_secs_f32(1.0 / Self::SIM_HZ as f32),
            config: config.clone(),
            steps: 0,
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

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        dt: instant::Duration,
    ) {
        if !self.paused {
            self.accumulator += dt;

            let mut steps = 0;

            while self.accumulator >= self.fixed_dt && steps < Self::MAX_STEPS {
                self.update_fixed(encoder, queue);
                self.accumulator -= self.fixed_dt;
                steps += 1;
            }

            if steps == Self::MAX_STEPS {
                self.accumulator = instant::Duration::ZERO;
            }
        } else {
            for _ in 0..self.steps {
                self.update_fixed(encoder, queue);
            }
            self.steps = 0;
        }
    }

    pub fn update_fixed(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        self.sim.dispatch(encoder, queue);
    }

    pub fn step(&mut self) {
        self.steps += 1;
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        self.sim.reset(device);
        self.renderer.reset(device, self.sim.velocity_buffers());
    }

    pub fn config_mut(&mut self) -> &mut SceneConfig {
        &mut self.config
    }

    pub fn sync_config(
        &mut self,
        queue: &wgpu::Queue,
    ) {
        self.sim.update_config(&self.config.sim);
        self.sim.update_uniforms(queue, self.fixed_dt);
        self.renderer.update_config(&self.config.render);
        self.renderer
            .update_uniforms(queue, self.config.sim.target_density);
        self.debug_overlay.update_config(&self.config.debug);
    }

    pub fn current_config(&self) -> SceneConfig {
        self.config.clone()
    }

    pub fn stats(&self) -> SceneStats {
        SceneStats {
            sim: self.sim.get_stats(),
        }
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

        self.debug_overlay.draw(
            encoder,
            target_view,
            self.renderer.depth_texture_view(),
            camera_bind_group,
        )?;

        Ok(())
    }

    pub fn upload_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: &Matrix4<f32>,
    ) {
        let size = self.sim.bounds();
        let radius = self.sim.smoothing_radius();

        self.debug_overlay
            .upload_config(device, queue, view_proj, &size, radius);
    }
}

pub struct SceneStats {
    pub sim: sim::Stats,
}
