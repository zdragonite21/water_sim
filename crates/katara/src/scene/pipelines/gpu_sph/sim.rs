#![allow(dead_code)]
use std::num::NonZeroU32;

use super::config::SimConfig;
use crate::stats::debug_stats;
use cgmath::{Point3, Vector3};

use wgpu::util::DeviceExt;
use wgpu_sort::{GPUSorter, SortBuffers, utils::guess_workgroup_size};

debug_stats! {
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub struct Stats {
        particle_count: usize = 0;
    }
}

pub struct Particle {
    pub pos: Point3<f32>,
    pub vel: Vector3<f32>,
    pub density: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    pub pos: [f32; 4],
    pub vel: [f32; 3],
    pub density: f32,
}

impl ParticleRaw {
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
            pos: [p.pos.x, p.pos.y, p.pos.z, 0.0],
            vel: p.vel.into(),
            density: p.density,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SimUniform {
    size: [f32; 3],
    collision_damping: f32,
    dt: f32,
    gravity: f32,
    smoothing_radius: f32,
    _pad: u32,
}

impl SimUniform {
    pub fn new(
        size: [f32; 3],
        collision_damping: f32,
        dt: f32,
        gravity: f32,
        smoothing_radius: f32,
    ) -> Self {
        Self {
            size,
            collision_damping,
            dt,
            gravity,
            smoothing_radius,
            _pad: 0,
        }
    }
}

struct SimResources {
    uniform: wgpu::Buffer,
    particles_next: wgpu::Buffer,
    particles_prev: wgpu::Buffer,
}

impl SimResources {
    fn new(device: &wgpu::Device, config: &SimConfig, particles: &[Particle]) -> Self {
        let sim_uniform = SimUniform::new(
            config.size,
            config.damping,
            0.0,
            config.gravity,
            config.smoothing_radius,
        );

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sim uniform buffer"),
            contents: bytemuck::bytes_of(&sim_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let particle_data = particles
            .iter()
            .map(ParticleRaw::from_particle)
            .collect::<Vec<_>>();

        let particle_next_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("input_buffer"),
            contents: bytemuck::cast_slice(&particle_data),
            usage: {
                use wgpu::BufferUsages;
                BufferUsages::COPY_DST
                    | BufferUsages::COPY_SRC
                    | BufferUsages::STORAGE
                    | BufferUsages::VERTEX
            },
        });

        let particle_prev_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prev_buffer"),
            contents: bytemuck::cast_slice(&particle_data),
            usage: {
                use wgpu::BufferUsages;
                BufferUsages::COPY_DST
                    | BufferUsages::COPY_SRC
                    | BufferUsages::STORAGE
                    | BufferUsages::VERTEX
            },
        });

        Self {
            uniform: uniform_buffer,
            particles_next: particle_next_buffer,
            particles_prev: particle_prev_buffer,
        }
    }
}

struct SimLayouts {
    uniform_layout: wgpu::BindGroupLayout,
    particle_layout: wgpu::BindGroupLayout,

    spatial_upload_layout: wgpu::BindGroupLayout,
}

impl SimLayouts {
    fn new(device: &wgpu::Device) -> Self {
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sim_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let particle_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let spatial_upload_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("spatial_upload_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        Self {
            uniform_layout,
            particle_layout,
            spatial_upload_layout,
        }
    }
}

struct SimBindGroups {
    uniform_bind_group: wgpu::BindGroup,
    particle_bind_group_a: wgpu::BindGroup,
    particle_bind_group_b: wgpu::BindGroup,
    spatial_upload_bind_group: wgpu::BindGroup,
}

impl SimBindGroups {
    fn new(device: &wgpu::Device, layouts: &SimLayouts, resources: &SimResources, sorter: &Sorter) -> Self {
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sim_bind_group"),
            layout: &layouts.uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: resources.uniform.as_entire_binding(),
            }],
        });

        let particle_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layouts.particle_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: resources.particles_next.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resources.particles_prev.as_entire_binding(),
                },
            ],
        });

        let particle_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layouts.particle_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resources.particles_next.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: resources.particles_prev.as_entire_binding(),
                },
            ],
        });

        let spatial_upload_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spatial_upload_bind_group"),
            layout: &layouts.spatial_upload_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sorter.keys().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sorter.values().as_entire_binding(),
                },
            ],
        });

        Self {
            uniform_bind_group,
            particle_bind_group_a,
            particle_bind_group_b,
            spatial_upload_bind_group,
        }
    }
}

struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    swap: bool,
}

impl ComputePipeline {
    fn new(
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
        particle_layout: &wgpu::BindGroupLayout,
        spatial_upload_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // todo: naga oil shader composition of final compute shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("simple_compute.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sim Pipeline Layout"),
            bind_group_layouts: &[Some(uniform_layout), Some(particle_layout), Some(spatial_upload_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute sim pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("apply_pressure"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            pipeline,
            swap: false,
        }
    }

    fn dispatch(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        uniforms: &wgpu::BindGroup,
        particles: &wgpu::BindGroup,
        spatial_grid: &wgpu::BindGroup,
        num_particles: usize,
    ) {
        let num_items_per_workgroup = 64;
        let num_dispatches = num_particles.div_ceil(num_items_per_workgroup) as u32;

        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, uniforms, &[]);
        pass.set_bind_group(1, particles, &[]);
        pass.set_bind_group(2, spatial_grid, &[]);

        pass.dispatch_workgroups(num_dispatches, 1, 1);

        self.swap = !self.swap;
    }
}

pub struct Sorter {
    sorter: GPUSorter,
    sort_buffers: SortBuffers,
}

impl Sorter {
    async fn new(device: &wgpu::Device, queue: &wgpu::Queue, num_particles: usize) -> Self {
        let subgroup_size = guess_workgroup_size(device, queue).await.unwrap();
        let sorter = GPUSorter::new(device, subgroup_size);
        let sort_buffers = sorter.create_sort_buffers(&device, NonZeroU32::new(num_particles as u32).unwrap());

        Self {
            sorter,
            sort_buffers,
        }
    }

    fn keys(&self) -> &wgpu::Buffer {
        self.sort_buffers.keys()
    }

    fn values(&self) -> &wgpu::Buffer {
        self.sort_buffers.values()
    }

    fn sort(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        self.sorter.sort(encoder, queue, &self.sort_buffers, None);
    }
}

pub struct Sim {
    compute_pipeline: ComputePipeline,
    bind_group_layouts: SimLayouts,
    bind_groups: SimBindGroups,
    resources: SimResources,
    sorter: Sorter,

    config: SimConfig,
}

impl Sim {
    const LOOK_AHEAD_FACTOR: f32 = 1.0 / 120.0;

    pub async fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &SimConfig) -> Self {
        let particles = Self::create_particles(config);
        let resources = SimResources::new(device, config, &particles);
        let bind_group_layouts = SimLayouts::new(device);
        let sorter = Sorter::new(device, queue, config.num_particles as usize).await;
        let bind_groups = SimBindGroups::new(device, &bind_group_layouts, &resources, &sorter);

        let compute_pipeline = ComputePipeline::new(
            device,
            &bind_group_layouts.uniform_layout,
            &bind_group_layouts.particle_layout,
            &bind_group_layouts.spatial_upload_layout,
        );

        Self {
            compute_pipeline,
            bind_group_layouts,
            bind_groups,
            resources,
            sorter,
            config: config.clone(),
        }
    }

    fn create_particles(config: &SimConfig) -> Vec<Particle> {
        let n = config.num_particles as usize;
        let mut particles = Vec::with_capacity(n);

        let scale = 0.5;

        let width = config.size[0] * scale;
        let height = config.size[1] * scale;

        let num_row = (n as f32 * width / height).sqrt().floor() as usize;
        let num_col = (n as f32 * height / width).sqrt().floor() as usize;

        for i in 0..n {
            let x = (i % num_row) as f32 / num_row as f32 * width;
            let y = (i / num_row) as f32 / num_col as f32 * height;

            let pos = Point3::new(x as f32 - width / 2.0, y as f32 - height / 2.0, 0.0);
            particles.push(Particle {
                pos,
                vel: Vector3::new(0.0, 0.0, 0.0),
                density: 0.0,
            });
        }
        particles
    }

    pub fn resize_particle_buffers(&mut self, device: &wgpu::Device) {
        let particles = Self::create_particles(&self.config);
        self.resources = SimResources::new(device, &self.config, &particles);
        self.bind_groups = SimBindGroups::new(device, &self.bind_group_layouts, &self.resources, &self.sorter);
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        let particles = Self::create_particles(&self.config);
        self.resources = SimResources::new(device, &self.config, &particles);
        self.bind_groups = SimBindGroups::new(device, &self.bind_group_layouts, &self.resources, &self.sorter);
    }

    pub fn update_uniforms(&mut self, queue: &wgpu::Queue, dt: instant::Duration) {
        let dt = dt.as_secs_f32();

        let sim_uniform = SimUniform::new(
            self.config.size,
            self.config.damping,
            dt,
            self.config.gravity,
            self.config.smoothing_radius,
        );
        queue.write_buffer(&self.resources.uniform, 0, bytemuck::bytes_of(&sim_uniform));
    }

    pub fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.compute_pipeline.dispatch(
            encoder,
            &self.bind_groups,
            self.config.num_particles as usize,
        );
    }
}

impl Sim {
    pub fn get_stats(&self) -> Stats {
        Stats {
            particle_count: self.config.num_particles as usize,
        }
    }

    pub fn smoothing_radius(&self) -> f32 {
        self.config.smoothing_radius
    }

    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        if self.compute_pipeline.swap {
            &self.resources.particles_prev
        } else {
            &self.resources.particles_next
        }
    }

    pub fn num_particles(&self) -> usize {
        self.config.num_particles as usize
    }

    pub fn update_config(&mut self, device: &wgpu::Device, config: &SimConfig) {
        if config == &self.config {
            return;
        }
        let old_num_particles = self.config.num_particles;
        self.config = config.clone();
        if old_num_particles != self.config.num_particles {
            self.resize_particle_buffers(device);
        }
    }

    pub fn bounds(&self) -> Vector3<f32> {
        self.config.size.into()
    }
}
