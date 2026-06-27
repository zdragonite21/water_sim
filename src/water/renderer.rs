use crate::{
    render::{
        model::{Mesh, SimpleVertex, Vertex},
        texture::Texture,
    }, water::{particle::{Particle, ParticleRaw}, sim::WaterSim},
};

pub struct WaterRenderer {
    render_pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    particle_display: Mesh,
    depth_texture: Texture,
    num_instances: usize,
}

impl WaterRenderer {
    pub async fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_layout: &wgpu::BindGroupLayout,
        num_instances: usize,
    ) -> anyhow::Result<Self> {
        let shader = device.create_shader_module(wgpu::include_wgsl!("water_scene.wgsl"));
        
        let depth_texture = Texture::create_depth_texture(device, config, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(camera_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SimpleVertex::desc(), ParticleRaw::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        log::debug!("demo scene render pipeline created");

        let particle_display = Mesh::square(device);

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (std::mem::size_of::<ParticleRaw>() * num_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            render_pipeline,
            instance_buffer,
            particle_display,
            depth_texture,
            num_instances
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.depth_texture = Texture::create_depth_texture(device, config, "depth_texture");
        log::debug!("depth texture rebuilt {}x{}", config.width, config.height);
    }

    pub fn upload(&mut self, queue: &wgpu::Queue, sim: &WaterSim) {
        // convert sim particles/cells into GPU instance data
        let instance_data = sim
            .particles
            .iter()
            .map(Particle::to_raw)
            .collect::<Vec<_>>();
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        // draw only; no physics decisions
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.particle_display.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.particle_display.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(
            0..self.particle_display.num_elements,
            0,
            0..self.num_instances as u32,
        );

        drop(render_pass);

        Ok(())
    }
}
