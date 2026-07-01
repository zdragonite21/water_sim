use crate::{
    config::DebugOverlayConfig,
    util::bounds::Bounds3,
    water::{
        line_batch::{Line3d, LineBatch},
        line_renderer::LineRenderer,
        sim::Particle,
    },
};
use cgmath::{Matrix4, Point3, Vector3};
pub struct DebugOverlay {
    config: DebugOverlayConfig,
    line_batch: LineBatch,
    line_renderer: LineRenderer,
}

impl DebugOverlay {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_layout: &wgpu::BindGroupLayout,
        line_capacity: usize,
        debug_overlay_config: &DebugOverlayConfig,
    ) -> anyhow::Result<Self> {
        let line_renderer = LineRenderer::new(device, config, camera_layout, line_capacity)?;

        Ok(Self {
            config: debug_overlay_config.clone(),
            line_batch: LineBatch::new(),
            line_renderer,
        })
    }

    pub fn resize(&mut self, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) {
        self.line_renderer.resize(queue, config);
    }

    pub fn rebuild_lines(&mut self, particles: &[Particle], size: &Vector3<f32>){
        const EXTRA_LINE_CAPACITY: usize = 12;
        self.line_batch.clear();
        
        if !self.config.enabled {
            return;
        }

        self.line_batch.reserve_exact(particles.len() + EXTRA_LINE_CAPACITY);

        if self.config.velocity_vectors {
            self.add_velocity_vectors(particles);
        }

        if self.config.bounds {
            self.add_bounds(size);
        }
    }

    pub fn upload(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        self.line_renderer
            .upload(queue, self.line_batch.lines(), view_proj);
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        self.line_renderer
            .draw(encoder, target_view, camera_bind_group)
    }

    pub fn update_config(&mut self, config: &DebugOverlayConfig) {
        if config == &self.config {
            return;
        }

        self.config = config.clone();
    }

    pub fn current_config(&self) -> DebugOverlayConfig {
        self.config.clone()
    }

    pub fn resize_capacity(&mut self, device: &wgpu::Device, capacity: usize) {
        self.line_renderer.resize_capacity(device, capacity);
    }

    fn add_bounds(&mut self, size: &Vector3<f32>) {
        const BOUND_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
        const BOUND_WIDTH: f32 = 2.0;
        let half_extent = size * 0.5;
        let bounds = Bounds3::from_half_extent(Point3::new(0.0, 0.0, 0.0), half_extent);
        self.add_box(&bounds, BOUND_COLOR, BOUND_WIDTH);
    }

    fn add_box(&mut self, bounds: &Bounds3, color: [f32; 4], width_px: f32) {
        for (start, end) in bounds.edge_segments() {
            self.line_batch.push_segment(
                start,
                end,
                color,
                width_px,
            );
        }
    }

    fn add_velocity_vectors(&mut self, particles: &[Particle]) {
        for particle in particles {
            self.line_batch.push_vector(
                particle.pos,
                particle.vel,
                self.config.velocity_color,
                self.config.vector_scale,
                self.config.vector_width,
            );
        }
    }
}
