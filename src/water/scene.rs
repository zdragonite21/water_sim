use crate::{
    config::WaterConfig,
    water::pipelines::{WaterPipeline, cpu_sph_particles::WaterSimStats},
};
use cgmath::Matrix4;

pub struct WaterSceneStats {
    pub sim: WaterSimStats,
}

pub struct WaterScene {
    pipeline: WaterPipeline,
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
        let pipeline = WaterPipeline::new(device, config, bind_group_layout, water_config)?;

        Ok(Self {
            pipeline,
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
        self.pipeline.resize(device, queue, config);
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
                self.pipeline.update_fixed(self.fixed_dt);
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
        self.pipeline.update_fixed(self.fixed_dt);
        self.upload_scene_data(queue, view_proj);
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        self.pipeline.reset(device);
    }

    pub fn update_config(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &WaterConfig,
    ) {
        self.pipeline.update_config(device, queue, config);
    }

    pub fn current_config(&self) -> WaterConfig {
        self.pipeline.current_config()
    }

    pub fn stats(&self) -> WaterSceneStats {
        WaterSceneStats {
            sim: self.pipeline.stats(),
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        self.pipeline
            .render(encoder, target_view, camera_bind_group)
    }

    fn upload_scene_data(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        self.pipeline.upload_frame(queue, view_proj);
    }
}
