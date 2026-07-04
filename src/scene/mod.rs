pub mod debug;
pub mod pipelines;

use crate::{gui::Panel, scene::pipelines::{Pipeline, PipelineStats}};
use cgmath::Matrix4;

use crate::scene::pipelines::{PipelineConfigs, PipelineId};
use serde::{Deserialize, Serialize};
use crate::inspect::Inspect;

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
        ui.window("Water")
            .size([280.0, 125.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text(format!(
                    "Active Pipeline: {}",
                    self.active_pipeline.label()
                ));
                ui.separator();

                let _pipeline_id = ui.push_id(self.active_pipeline.as_str());
                self.pipeline_configs.cpu_sph.inspect(ui);
            });
    }
}

pub struct SceneStats {
    pub pipeline: PipelineStats,
}

pub struct Scene {
    pipeline: Pipeline,
    paused: bool,
    accumulator: instant::Duration,
    fixed_dt: instant::Duration,
    config: SceneConfig,
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
        let pipeline = Pipeline::new(
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

    pub fn config_mut(&mut self) -> &mut SceneConfig {
        &mut self.config
    }

    pub fn sync_pipeline(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
        cam_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<()> {
        if self.pipeline.id() != self.config.active_pipeline {
            self.pipeline = Pipeline::new(
                device,
                surface_config,
                cam_bind_group_layout,
                self.config.active_pipeline,
                &self.config.pipeline_configs,
            )?;
        } else {
            self.pipeline
                .update_config(device, queue, &self.config.pipeline_configs);
        }
        Ok(())
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
