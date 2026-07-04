pub mod cpu_sph;

use crate::stats::DebugStats;
use cgmath::Matrix4;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineConfigs {
    pub cpu_sph: cpu_sph::Config,
}

impl Default for PipelineConfigs {
    fn default() -> Self {
        Self {
            cpu_sph: cpu_sph::Config::default(),
        }
    }
}

pub struct PipelineStats {
    pub label: &'static str,
    pub rows: Vec<String>,
}

pub enum Pipeline {
    CpuSph(cpu_sph::Pipeline),
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        cam_bind_group_layout: &wgpu::BindGroupLayout,
        pipeline_id: PipelineId,
        pipeline_configs: &PipelineConfigs,
    ) -> anyhow::Result<Self> {
        match pipeline_id {
            PipelineId::CpuSph => Ok(Self::CpuSph(cpu_sph::Pipeline::new(
                device,
                surface_config,
                cam_bind_group_layout,
                &pipeline_configs.cpu_sph,
            )?)),
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) {
        match self {
            Self::CpuSph(pipeline) => pipeline.resize(device, queue, surface_config),
        }
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        match self {
            Self::CpuSph(pipeline) => pipeline.reset(device),
        }
    }

    pub fn update_fixed(&mut self, dt: instant::Duration) {
        match self {
            Self::CpuSph(pipeline) => pipeline.update_fixed(dt),
        }
    }

    pub fn upload_frame(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        match self {
            Self::CpuSph(pipeline) => pipeline.upload_frame(queue, view_proj),
        }
    }

    pub fn update_config(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &PipelineConfigs,
    ) {
        match self {
            Self::CpuSph(pipeline) => pipeline.update_config(device, queue, &config.cpu_sph),
        }
    }

    pub fn id(&self) -> PipelineId {
        match self {
            Self::CpuSph(pipeline) => pipeline.id(),
        }
    }

    pub fn stats(&self) -> PipelineStats {
        let mut rows = Vec::new();
        match self {
            Self::CpuSph(pipeline) => pipeline.stats().append_debug_text_rows(&mut rows),
        }

        PipelineStats {
            label: self.id().label(),
            rows,
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        match self {
            Self::CpuSph(pipeline) => pipeline.render(encoder, target_view, camera_bind_group),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineId {
    #[serde(rename = "cpu_sph")]
    CpuSph = 0,
}

impl PipelineId {
    pub fn label(self) -> &'static str {
        match self {
            Self::CpuSph => "CPU SPH",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CpuSph => "cpu_sph",
        }
    }
}

impl Default for PipelineId {
    fn default() -> Self {
        Self::CpuSph
    }
}
