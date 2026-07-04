use crate::{
    config::DebugOverlayConfig,
    util::bounds::Bounds3,
    water::{
        debug::{line_batch::LineBatch, line_renderer::LineRenderer},
        pipelines::cpu_sph_particles::Particle,
    },
};
use cgmath::{Matrix4, Point3, Vector3};

const BOUNDS_LINE_COUNT: usize = 12;
const MAX_SPATIAL_GRID_LINES: usize = 128;

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

    pub fn resize_capacity(&mut self, device: &wgpu::Device, capacity: usize) {
        if self.line_renderer.capacity() == capacity {
            return;
        }

        self.line_renderer.resize_capacity(device, capacity);
    }

    pub fn required_capacity(
        &self,
        particle_count: usize,
        size: &Vector3<f32>,
        radius: f32,
    ) -> usize {
        Self::required_capacity_for_config(&self.config, particle_count, size, radius)
    }

    pub fn required_capacity_for_config(
        config: &DebugOverlayConfig,
        particle_count: usize,
        size: &Vector3<f32>,
        radius: f32,
    ) -> usize {
        if !config.enabled {
            return 0;
        }

        let mut capacity = 0;

        if config.velocity {
            capacity += particle_count;
        }

        if config.bounds {
            capacity += BOUNDS_LINE_COUNT;
        }

        if config.grid {
            capacity += Self::spatial_grid_line_count(size, radius);
        }

        capacity
    }

    pub fn rebuild_lines(&mut self, particles: &[Particle], size: &Vector3<f32>, radius: f32) {
        self.line_batch.clear();

        if !self.config.enabled {
            return;
        }

        let capacity = self.required_capacity(particles.len(), size, radius);
        self.line_batch.reserve_exact(capacity);

        if self.config.velocity {
            self.add_velocity_vectors(particles);
        }

        if self.config.bounds {
            self.add_bounds(size);
        }

        if self.config.grid {
            self.add_spatial_grid(size, radius);
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

    fn add_bounds(&mut self, size: &Vector3<f32>) {
        const BOUND_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
        const BOUND_WIDTH: f32 = 2.0;
        let half_extent = size * 0.5;
        let bounds = Bounds3::from_half_extent(Point3::new(0.0, 0.0, 0.0), half_extent);
        self.add_box(&bounds, BOUND_COLOR, BOUND_WIDTH);
    }

    fn add_box(&mut self, bounds: &Bounds3, color: [f32; 4], width_px: f32) {
        for (start, end) in bounds.edge_segments() {
            self.line_batch.push_segment(start, end, color, width_px);
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

    fn add_spatial_grid(&mut self, size: &Vector3<f32>, radius: f32) {
        const GRID_COLOR: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const GRID_WIDTH: f32 = 1.0;

        let Some(grid) = SpatialGridOverlay::new(size, radius) else {
            return;
        };

        for cell_x in grid.x_axis.iter_cells(grid.stride) {
            let x = cell_x as f32 * radius;
            self.line_batch.push_segment(
                Point3::new(x, grid.y_axis.min_world(), 0.0),
                Point3::new(x, grid.y_axis.max_world(), 0.0),
                GRID_COLOR,
                GRID_WIDTH,
            );
        }

        for cell_y in grid.y_axis.iter_cells(grid.stride) {
            let y = cell_y as f32 * radius;
            self.line_batch.push_segment(
                Point3::new(grid.x_axis.min_world(), y, 0.0),
                Point3::new(grid.x_axis.max_world(), y, 0.0),
                GRID_COLOR,
                GRID_WIDTH,
            );
        }
    }

    fn spatial_grid_line_count(size: &Vector3<f32>, radius: f32) -> usize {
        let Some(grid) = SpatialGridOverlay::new(size, radius) else {
            return 0;
        };

        grid.x_axis.sampled_count(grid.stride) + grid.y_axis.sampled_count(grid.stride)
    }
}

struct SpatialGridOverlay {
    x_axis: GridAxis,
    y_axis: GridAxis,
    stride: usize,
}

impl SpatialGridOverlay {
    fn new(size: &Vector3<f32>, radius: f32) -> Option<Self> {
        if !radius.is_finite() || radius <= f32::EPSILON {
            return None;
        }

        let x_axis = GridAxis::new(size.x, radius)?;
        let y_axis = GridAxis::new(size.y, radius)?;
        let line_count = x_axis.raw_count() + y_axis.raw_count();
        let stride = line_count.div_ceil(MAX_SPATIAL_GRID_LINES).max(1);

        Some(Self {
            x_axis,
            y_axis,
            stride,
        })
    }
}

struct GridAxis {
    start_cell: i32,
    end_cell: i32,
    radius: f32,
}

impl GridAxis {
    fn new(size: f32, radius: f32) -> Option<Self> {
        if !size.is_finite() || size.abs() <= f32::EPSILON {
            return None;
        }

        let half = size.abs() * 0.5;
        Some(Self {
            start_cell: (-half / radius).floor() as i32,
            end_cell: (half / radius).ceil() as i32,
            radius,
        })
    }

    fn raw_count(&self) -> usize {
        (self.end_cell - self.start_cell + 1).max(0) as usize
    }

    fn sampled_count(&self, stride: usize) -> usize {
        self.raw_count().div_ceil(stride)
    }

    fn iter_cells(&self, stride: usize) -> impl Iterator<Item = i32> + '_ {
        (self.start_cell..=self.end_cell).step_by(stride)
    }

    fn min_world(&self) -> f32 {
        self.start_cell as f32 * self.radius
    }

    fn max_world(&self) -> f32 {
        self.end_cell as f32 * self.radius
    }
}
