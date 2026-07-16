use super::config::DebugConfig;
use crate::{
    scene::debug::{line::LineBatch, line_renderer::LineRenderer},
    util::bounds::Bounds3,
};
use cgmath::{Matrix4, Point3, Vector3};

const BOUNDS_LINE_COUNT: usize = 12;

pub struct DebugOverlay {
    config: DebugConfig,
    line_batch: LineBatch,
    line_renderer: LineRenderer,
}

impl DebugOverlay {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_layout: &wgpu::BindGroupLayout,
        debug_overlay_config: &DebugConfig,
    ) -> anyhow::Result<Self> {
        let line_renderer = LineRenderer::new(device, config, camera_layout, 0)?;

        Ok(Self {
            config: debug_overlay_config.clone(),
            line_batch: LineBatch::new(),
            line_renderer,
        })
    }

    pub fn resize(&mut self, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) {
        self.line_renderer.resize(queue, config);
    }

    fn resize_capacity(&mut self, device: &wgpu::Device, capacity: usize) {
        if self.line_renderer.capacity() == capacity {
            return;
        }

        self.line_renderer.resize_capacity(device, capacity);
    }

    fn required_capacity(&self, size: &Vector3<f32>, radius: f32) -> usize {
        Self::required_capacity_for_config(&self.config, size, radius)
    }

    fn required_capacity_for_config(
        config: &DebugConfig,
        size: &Vector3<f32>,
        radius: f32,
    ) -> usize {
        if !config.enabled {
            return 0;
        }

        let mut capacity = 0;

        if config.bounds {
            capacity += BOUNDS_LINE_COUNT;
        }

        if config.grid {
            capacity += Self::spatial_grid_line_count(size, radius);
        }

        capacity
    }

    fn rebuild_lines(&mut self, size: &Vector3<f32>, radius: f32) {
        self.line_batch.clear();

        if !self.config.enabled {
            return;
        }

        let capacity = self.required_capacity(size, radius);
        self.line_batch.reserve_exact(capacity);

        if self.config.bounds {
            self.add_bounds(size);
        }

        if self.config.grid {
            self.add_spatial_grid(size, radius);
        }
    }

    fn upload(&mut self, queue: &wgpu::Queue, view_proj: &Matrix4<f32>) {
        self.line_renderer
            .upload(queue, self.line_batch.lines(), view_proj);
    }

    pub fn upload_config(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: &Matrix4<f32>,
        size: &Vector3<f32>,
        radius: f32,
    ) {
        self.rebuild_lines(size, radius);
        self.resize_capacity(device, self.line_batch.lines().len());
        self.upload(queue, view_proj);
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

    pub fn update_config(&mut self, config: &DebugConfig) {
        if config == &self.config {
            return;
        }

        self.config = config.clone();
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

    fn add_spatial_grid(&mut self, size: &Vector3<f32>, radius: f32) {
        const GRID_COLOR: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const GRID_WIDTH: f32 = 1.0;

        let Some(grid) = SpatialGridOverlay::new(size, radius) else {
            return;
        };

        for cell_x in grid.x_axis.iter_cells() {
            let x = cell_x as f32 * radius;
            self.line_batch.push_segment(
                Point3::new(x, grid.y_axis.min_world(), 0.0),
                Point3::new(x, grid.y_axis.max_world(), 0.0),
                GRID_COLOR,
                GRID_WIDTH,
            );
        }

        for cell_y in grid.y_axis.iter_cells() {
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

        grid.count()
    }
}

struct SpatialGridOverlay {
    x_axis: GridAxis,
    y_axis: GridAxis,
    z_axis: GridAxis,
}

impl SpatialGridOverlay {
    fn new(size: &Vector3<f32>, radius: f32) -> Option<Self> {
        if !radius.is_finite() || radius <= f32::EPSILON {
            return None;
        }

        let x_axis = GridAxis::new(size.x, radius)?;
        let y_axis = GridAxis::new(size.y, radius)?;
        let z_axis = GridAxis::new(size.z, radius)?;

        Some(Self {
            x_axis,
            y_axis,
            z_axis,
        })
    }

    fn count(&self) -> usize {
        self.x_axis.raw_count() + self.y_axis.raw_count() + self.z_axis.raw_count()
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

    fn iter_cells(&self) -> impl Iterator<Item = i32> + '_ {
        self.start_cell..=self.end_cell
    }

    fn min_world(&self) -> f32 {
        self.start_cell as f32 * self.radius
    }

    fn max_world(&self) -> f32 {
        self.end_cell as f32 * self.radius
    }
}
