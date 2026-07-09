mod debug;
mod pipelines;

use crate::scene::pipelines::{PipelineConfigs, PipelineId};
use crate::{
    gui::Panel,
    scene::pipelines::{ActivePipeline, PipelineStats},
};
use cgmath::Matrix4;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneConfig {
    pub active_pipeline: PipelineId,
    pub pipeline_configs: PipelineConfigs,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            active_pipeline: PipelineId::default(),
            pipeline_configs: PipelineConfigs::default(),
        }
    }
}

impl Panel for SceneConfig {
    fn draw(&mut self, ui: &imgui::Ui) {
        let mut active_id = self.active_pipeline as usize;
        ui.window("Scene")
            .size([280.0, 125.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.combo(
                    "pipeline",
                    &mut active_id,
                    &PipelineId::ALL,
                    |pipeline_id| pipeline_id.label().into(),
                );
                self.active_pipeline = PipelineId::ALL[active_id];
                ui.separator();

                let _pipeline_id = ui.push_id(self.active_pipeline.as_str());
                self.pipeline_configs.draw(ui, self.active_pipeline);
            });
    }
}

pub struct SceneStats {
    pub pipeline: PipelineStats,
}

pub struct Scene {
    pipeline: ActivePipeline,
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
        scene_config: &SceneConfig,
    ) -> anyhow::Result<Self> {
        let pipeline = ActivePipeline::new(
            device,
            surface_config,
            cam_bind_group_layout,
            scene_config.active_pipeline,
            &scene_config.pipeline_configs,
        )?;

        Ok(Self {
            pipeline,
            paused: true,
            accumulator: instant::Duration::ZERO,
            fixed_dt: instant::Duration::from_secs_f32(1.0 / Self::SIM_HZ as f32),
            config: scene_config.clone(),
            steps: 0
        })
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) {
        self.pipeline.resize(device, queue, surface_config);
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn update(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, dt: instant::Duration, view_proj: &Matrix4<f32>) {
        if !self.paused {
            self.accumulator += dt;

            let mut steps = 0;

            while self.accumulator >= self.fixed_dt && steps < Self::MAX_STEPS {
                self.pipeline.update_fixed(encoder, self.fixed_dt);
                self.accumulator -= self.fixed_dt;
                steps += 1;
            }

            if steps == Self::MAX_STEPS {
                self.accumulator = instant::Duration::ZERO;
            }
        } else {
            for _ in 0..self.steps {
                self.pipeline.update_fixed(encoder, self.fixed_dt);
            }
            self.steps = 0;
        }
        self.upload_scene_data(queue, view_proj);
    }

    pub fn step(&mut self) {
        self.steps += 1;
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        self.pipeline.reset(device);
    }

    pub fn config_mut(&mut self) -> &mut SceneConfig {
        &mut self.config
    }

    pub fn sync_pipeline(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
        cam_bind_group_layout: &wgpu::BindGroupLayout,
        dt: instant::Duration,
    ) -> anyhow::Result<bool> {
        if self.pipeline.id() != self.config.active_pipeline {
            self.pipeline = ActivePipeline::new(
                device,
                surface_config,
                cam_bind_group_layout,
                self.config.active_pipeline,
                &self.config.pipeline_configs,
            )?;
            self.paused = true;
            Ok(true)
        } else {
            self.pipeline
                .update_config(device, queue, &self.config.pipeline_configs, dt);
            Ok(false)
        }
    }

    pub fn current_config(&self) -> SceneConfig {
        self.config.clone()
    }

    pub fn stats(&self) -> SceneStats {
        SceneStats {
            pipeline: self.pipeline.stats(),
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
