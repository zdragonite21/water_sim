use crate::{
    config::WaterConfig, water::{
        line_renderer::{LineRenderer, LineType, VectorType}, renderer::WaterRenderer, sim::WaterSim,
    },
};
use cgmath::{Matrix4, Point3};

pub struct WaterScene {
    sim: WaterSim,
    renderer: WaterRenderer,
    line_renderer: LineRenderer,
    paused: bool,
    accumulator: instant::Duration,
    fixed_dt: instant::Duration,
}

impl WaterScene {
    const FIXED_FPS: u64 = 120;
    const MAX_STEPS: u8 = 8;
    const EXTRA_LINE_CAPACITY: usize = 12;
    const VELOCITY_LINE_SCALE: f32 = 0.5;

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
        )
        .await?;

        let line_renderer = LineRenderer::new(
            device,
            config,
            bind_group_layout,
            Self::line_capacity(sim.particles().len()),
            &water_config.line_renderer,
        )
        .await?;

        Ok(Self {
            sim,
            renderer,
            line_renderer,
            paused: true,
            accumulator: instant::Duration::ZERO,
            fixed_dt: instant::Duration::from_secs_f32(1.0 / Self::FIXED_FPS as f32),
        })
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) {
        self.renderer.resize(device, config);
        self.line_renderer.resize(queue, config);
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: instant::Duration, view_proj: &Matrix4<f32>) {
        if !self.paused {
            self.accumulator += dt.min(instant::Duration::from_millis(Self::FIXED_FPS));

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
        self.line_renderer
            .resize_capacity(device, Self::line_capacity(particle_count));
    }

    pub fn update_config(&mut self, queue: &wgpu::Queue, config: &WaterConfig) {
        self.sim.update_config(&config.sim);
        self.renderer.update_config(&config.render);
        self.line_renderer.update_config(&config.line_renderer);
        self.renderer.update_uniforms(queue);
    }

    pub fn current_config(&self) -> WaterConfig {
        WaterConfig {
            sim: self.sim.current_config(),
            render: self.renderer.current_config(),
            line_renderer: self.line_renderer.current_config(),
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
        self.line_renderer
            .draw(encoder, target_view, camera_bind_group)?;
        Ok(())
    }

    fn upload_scene_data(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        self.renderer.upload(queue, &self.sim);
        self.rebuild_line_draw_list();
        self.line_renderer.upload(queue, view_proj);
    }

    fn rebuild_line_draw_list(&mut self) {
        self.line_renderer.clear();

        if !self.line_renderer.enabled() {
            return;
        }

        for particle in self.sim.particles() {
            self.line_renderer.push_vector(
                particle.pos,
                particle.vel,
                Self::VELOCITY_LINE_SCALE,
                VectorType::Velocity,
            );
        }

        self.push_sim_bounds_lines();
    }

    fn push_sim_bounds_lines(&mut self) {
        let config = self.sim.current_config();
        let half_x = config.size_x * 0.5;
        let half_y = config.size_y * 0.5;
        let z = 0.0;

        let bottom_left = Point3::new(-half_x, -half_y, z);
        let bottom_right = Point3::new(half_x, -half_y, z);
        let top_right = Point3::new(half_x, half_y, z);
        let top_left = Point3::new(-half_x, half_y, z);

        self.line_renderer.push_segment(bottom_left, bottom_right, LineType::Bounds);
        self.line_renderer.push_segment(bottom_right, top_right, LineType::Bounds);
        self.line_renderer.push_segment(top_right, top_left, LineType::Bounds);
        self.line_renderer.push_segment(top_left, bottom_left, LineType::Bounds);
    }

    fn line_capacity(particle_count: usize) -> usize {
        particle_count.saturating_add(Self::EXTRA_LINE_CAPACITY)
    }
}
