use crate::{config::LineRendererConfig, render::model::Mesh};
use cgmath::Vector3;
pub struct Line3d {
    pub start: Vector3<f32>,
    pub end: Vector3<f32>,
    pub color: [f32; 4],
    pub width_px: f32,
}

struct LineRenderer {
    render_pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    quad: Mesh,
    line_count: usize,
}

impl LineRenderer {
    pub async fn new(
        _device: &wgpu::Device,
        _config: &wgpu::SurfaceConfiguration,
        _camera_layout: &wgpu::BindGroupLayout,
        _max_lines: usize,
        _config_data: &LineRendererConfig,
    ) -> anyhow::Result<Self> {
        todo!("finish line renderer pipeline setup")
    }
    // pub fn clear(&mut self);
    // pub fn push_line(&mut self, line: Line3d);
    // pub fn push_vector(&mut self, origin: Vector3<f32>, vector: Vector3<f32>, scale: f32);
    // pub fn upload(&mut self, queue: &wgpu::Queue);
    // pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, camera_bind_group: &wgpu::BindGroup);
}

#[repr(C)]
pub struct LineInstanceRaw {
    start: [f32; 3],
    end: [f32; 3],
    color: [f32; 4],
    width_px: f32,
    _pad: [f32; 3],
}
