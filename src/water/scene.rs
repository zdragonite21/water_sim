use crate::{
    config::WaterConfig,
    water::{debug_overlay::DebugOverlay, renderer::WaterRenderer, sim::WaterSim},
};
use cgmath::Matrix4;

pub struct WaterScene {
    sim: WaterSim,
    renderer: WaterRenderer,
    debug_overlay: DebugOverlay,
    paused: bool,
    accumulator: instant::Duration,
    fixed_dt: instant::Duration,
}

impl WaterScene {
    const SIM_HZ: u64 = 120;
    const MAX_STEPS: u8 = 8;

    pub async fn new(
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
            paused: true,
            accumulator: instant::Duration::ZERO,
            fixed_dt: instant::Duration::from_secs_f32(1.0 / Self::SIM_HZ as f32),
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

    pub fn update(&mut self, queue: &wgpu::Queue, dt: instant::Duration, view_proj: &Matrix4<f32>) {
        if !self.paused {
            self.accumulator += dt;

            let mut steps = 0;

            while self.accumulator >= self.fixed_dt && steps < Self::MAX_STEPS {
                self.sim.update(self.fixed_dt);
                self.accumulator -= self.fixed_dt;
                steps += 1;
            }

            if steps == Self::MAX_STEPS {
                self.accumulator = instant::Duration::ZERO;
            }
        }
        self.upload_scene_data(queue, view_proj);
    }

    pub fn step(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        self.sim.update(self.fixed_dt);
        self.upload_scene_data(queue, view_proj);
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        self.sim.reset();
        let particle_count = self.sim.particles().len();
        self.renderer.resize_instance_buffer(device, particle_count);
        self.resize_debug_overlay_capacity(device);
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
        self.renderer.update_uniforms(queue);
    }

    pub fn current_config(&self) -> WaterConfig {
        WaterConfig {
            sim: self.sim.current_config(),
            render: self.renderer.current_config(),
            debug: self.debug_overlay.current_config(),
        }
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

    fn upload_scene_data(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        self.renderer.upload_particles(queue, self.sim.particles());
        self.debug_overlay.rebuild_lines(
            self.sim.particles(),
            &self.sim.bounds(),
            self.sim.current_config().smoothing_radius,
        );
        self.debug_overlay.upload(queue, view_proj);
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
