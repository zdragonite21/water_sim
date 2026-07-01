use crate::{
    render::model::{Mesh, SimpleVertex, Vertex},
    water::line_batch::Line3d,
};
use cgmath::{Matrix4, Point3, Transform};
use wgpu::util::DeviceExt;

pub struct LineRenderer {
    render_pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    line_uniform: LineUniform,
    line_uniform_buffer: wgpu::Buffer,
    line_bind_group: wgpu::BindGroup,
    quad: Mesh,
    capacity: usize,
    visible_line_count: usize,
}

impl LineRenderer {
    pub async fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_layout: &wgpu::BindGroupLayout,
        capacity: usize,
    ) -> anyhow::Result<Self> {
        let shader = device.create_shader_module(wgpu::include_wgsl!("line_renderer.wgsl"));

        let line_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("line_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let line_uniform = LineUniform::new(config);
        let line_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Uniform Buffer"),
            contents: bytemuck::bytes_of(&line_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let line_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("line_bind_group"),
            layout: &line_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: line_uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Renderer Pipeline Layout"),
                bind_group_layouts: &[Some(camera_layout), Some(&line_bind_group_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Renderer Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SimpleVertex::desc(), LineInstanceRaw::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        log::debug!("line render pipeline created");

        let quad = Mesh::square(device);

        let instance_buffer = Self::new_instance_buffer(device, capacity);

        Ok(Self {
            render_pipeline,
            instance_buffer,
            line_uniform,
            line_uniform_buffer,
            line_bind_group,
            quad,
            capacity,
            visible_line_count: 0,
        })
    }

    pub fn resize(&mut self, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) {
        self.line_uniform.update_viewport(config);
        queue.write_buffer(
            &self.line_uniform_buffer,
            0,
            bytemuck::bytes_of(&self.line_uniform),
        );
    }

    fn new_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        let buffer_capacity = capacity.max(1);
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line Instance Buffer"),
            size: (std::mem::size_of::<LineInstanceRaw>() * buffer_capacity) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn resize_capacity(&mut self, device: &wgpu::Device, capacity: usize) {
        if capacity == self.capacity {
            return;
        }

        self.capacity = capacity;
        self.instance_buffer = Self::new_instance_buffer(device, capacity);
    }

    pub fn upload(&mut self, queue: &wgpu::Queue, lines: &[Line3d], view_proj: &Matrix4<f32>) {
        self.visible_line_count = 0;

        if lines.is_empty() {
            return;
        }

        let mut instance_data = Vec::with_capacity(lines.len());

        for line in lines {
            if self.visible_line_count >= self.capacity {
                break;
            }

            let screen_line = Line3d {
                start: view_proj.transform_point(line.start),
                end: view_proj.transform_point(line.end),
                color: line.color,
                width_px: line.width_px,
            };

            if !Self::is_point_visible(screen_line.start)
                || !Self::is_point_visible(screen_line.end)
            {
                continue;
            }
            instance_data.push(LineInstanceRaw::from_line(&screen_line));
            self.visible_line_count += 1;
        }

        if !instance_data.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );
        }

        let skipped_lines = lines.len() - self.visible_line_count;

        if skipped_lines > 0 {
            log::warn!(
                "LineRenderer capacity {} exceeded; skipped {} lines",
                self.capacity,
                skipped_lines
            );
        }
    }

    fn is_point_visible(point: Point3<f32>) -> bool {
        point.x.is_finite()
            && point.y.is_finite()
            && point.z.is_finite()
            && (0.0..=1.0).contains(&point.z)
    }

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        if self.visible_line_count == 0 {
            return Ok(());
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Line Renderer Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.line_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.quad.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(
            0..self.quad.num_elements,
            0,
            0..self.visible_line_count as u32,
        );

        drop(render_pass);

        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LineUniform {
    viewport_size: [f32; 2],
    _pad: [f32; 2],
}

impl LineUniform {
    fn new(config: &wgpu::SurfaceConfiguration) -> Self {
        Self {
            viewport_size: [config.width as f32, config.height as f32],
            _pad: [0.0; 2],
        }
    }

    fn update_viewport(&mut self, config: &wgpu::SurfaceConfiguration) {
        self.viewport_size = [config.width as f32, config.height as f32];
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineInstanceRaw {
    start: [f32; 2],
    end: [f32; 2],
    color: [f32; 4],
    width_px: f32,
    _pad: [f32; 3],
}

impl LineInstanceRaw {
    const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![5 => Float32x2, 6 => Float32x2, 7 => Float32x4, 8 => Float32];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }

    fn from_line(line: &Line3d) -> Self {
        Self {
            start: [line.start.x, line.start.y],
            end: [line.end.x, line.end.y],
            color: line.color,
            width_px: line.width_px,
            _pad: [0.0; 3],
        }
    }
}
