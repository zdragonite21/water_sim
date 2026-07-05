use crate::render::{
    model::{Mesh, SimpleVertex, Vertex},
    texture::Texture,
};

use super::{Particle, config::RenderConfig};

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub density: f32,
}

impl InstanceRaw {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![5 => Float32x3, 6 => Float32x3, 7 => Float32];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }

    fn from_particle(p: &Particle) -> Self {
        Self {
            pos: p.pos.into(),
            vel: p.vel.into(),
            density: p.density,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct WaterUniform {
    particle_size: f32,
    target_density: f32,
    _pad: [u32; 2],
}

impl WaterUniform {
    pub fn new() -> Self {
        Self {
            particle_size: 1.0,
            target_density: 1.0,
            _pad: [0; 2],
        }
    }

    pub fn update(&mut self, config: &RenderConfig, target_density: f32) {
        self.particle_size = config.particle_size;
        self.target_density = target_density;
    }
}

pub struct BillboardRenderer {
    render_pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    particle_display: Mesh,
    depth_texture: Texture,
    num_instances: usize,

    water_uniform: WaterUniform,
    water_uniform_buffer: wgpu::Buffer,
    water_bind_group: wgpu::BindGroup,
    config: RenderConfig,
}

impl BillboardRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_layout: &wgpu::BindGroupLayout,
        num_instances: usize,
        render_config: &RenderConfig,
    ) -> anyhow::Result<Self> {
        #[include_wgsl_oil::include_wgsl_oil("compute.wgsl")]
        pub mod compute_shader{}

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute shader"),
            source: wgpu::ShaderSource::Wgsl(compute_shader::SOURCE.into()),
        });

        let depth_texture = Texture::create_depth_texture(device, config, "depth_texture");

        let mut water_uniform = WaterUniform::new();
        water_uniform.update(render_config, 0.0);

        let water_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Water Uniform Buffer"),
            contents: bytemuck::bytes_of(&water_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let water_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("water_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let water_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water_bind_group"),
            layout: &water_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: water_uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(camera_layout), Some(&water_bind_group_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SimpleVertex::desc(), InstanceRaw::desc()],
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

        log::debug!("water scene render pipeline created");

        let particle_display = Mesh::square(device);

        let instance_buffer = Self::new_instance_buffer(device, num_instances);

        Ok(Self {
            render_pipeline,
            instance_buffer,
            particle_display,
            depth_texture,
            num_instances,
            water_uniform,
            water_uniform_buffer,
            water_bind_group,
            config: render_config.clone(),
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.depth_texture = Texture::create_depth_texture(device, config, "depth_texture");
        log::debug!("depth texture rebuilt {}x{}", config.width, config.height);
    }

    pub fn new_instance_buffer(device: &wgpu::Device, num_instances: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (std::mem::size_of::<InstanceRaw>() * num_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn resize_instance_buffer(&mut self, device: &wgpu::Device, num_instances: usize) {
        if num_instances != self.num_instances {
            self.instance_buffer = Self::new_instance_buffer(device, num_instances);
            self.num_instances = num_instances;
        }
    }

    pub fn update_config(&mut self, config: &RenderConfig) {
        if config == &self.config {
            return;
        }

        self.config = config.clone();
    }

    pub fn update_uniforms(&mut self, queue: &wgpu::Queue, target_density: f32) {
        self.water_uniform.update(&self.config, target_density);
        queue.write_buffer(
            &self.water_uniform_buffer,
            0,
            bytemuck::bytes_of(&self.water_uniform),
        );
    }

    pub fn upload_particles(&mut self, queue: &wgpu::Queue, particles: &[Particle]) {
        // convert sim particles/cells into GPU instance data
        let instance_data = particles
            .iter()
            .map(InstanceRaw::from_particle)
            .collect::<Vec<_>>();
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }

    pub fn draw(
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
                        r: 0.02,
                        g: 0.06,
                        b: 0.02,
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
        render_pass.set_bind_group(1, &self.water_bind_group, &[]);
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
