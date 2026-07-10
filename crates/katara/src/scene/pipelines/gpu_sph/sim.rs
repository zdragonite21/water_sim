#![allow(dead_code)]
use std::num::NonZeroU32;

use super::config::SimConfig;
use crate::stats::debug_stats;
use cgmath::{Point3, Vector3};

use wgpu::util::DeviceExt;
use wgpu_sort::{GPUSorter, SortBuffers};

debug_stats! {
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub struct Stats {
        particle_count: usize = 0;
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    pub pos: [f32; 4],
}

impl ParticleRaw {
    const ATTRIBS: [wgpu::VertexAttribute; 1] =
        wgpu::vertex_attr_array![5 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
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
    stiffness: f32,
    rest_density: f32,
    mass: f32,
    _pad: [f32; 2],
}

impl SimUniform {
    pub fn new(config: &SimConfig, dt: f32) -> Self {
        Self {
            size: config.size,
            collision_damping: config.damping,
            dt,
            gravity: config.gravity,
            smoothing_radius: config.smoothing_radius,
            stiffness: config.pressure_multiplier,
            rest_density: config.target_density,
            mass: config.mass,
            _pad: [0.0; 2],
        }
    }
}

struct SimResources {
    uniform: wgpu::Buffer,
    particles_next: wgpu::Buffer,
    particles_prev: wgpu::Buffer,
    particle_vel_density: wgpu::Buffer,
    start_indices: wgpu::Buffer,
}

impl SimResources {
    fn new(device: &wgpu::Device, config: &SimConfig, particles: &[ParticleRaw]) -> Self {
        let sim_uniform = SimUniform::new(config, 0.0);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sim uniform buffer"),
            contents: bytemuck::bytes_of(&sim_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let particle_prev_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("prev_buffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: {
                use wgpu::BufferUsages;
                BufferUsages::STORAGE | BufferUsages::VERTEX
            },
        });

        let particle_next_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("next_buffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: {
                use wgpu::BufferUsages;
                BufferUsages::STORAGE | BufferUsages::VERTEX
            },
        });

        let particle_vel_density_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_vel_density_buffer"),
            size: (4 * std::mem::size_of::<f32>() * particles.len()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let start_indices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("start_indices_buffer"),
            size: (std::mem::size_of::<u32>() * particles.len()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniform: uniform_buffer,
            particles_next: particle_next_buffer,
            particles_prev: particle_prev_buffer,
            particle_vel_density: particle_vel_density_buffer,
            start_indices: start_indices_buffer,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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

        let spatial_upload_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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
    fn new(
        device: &wgpu::Device,
        layouts: &SimLayouts,
        resources: &SimResources,
        sorter: &Sorter,
    ) -> Self {
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: resources.particle_vel_density.as_entire_binding(),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: resources.particle_vel_density.as_entire_binding(),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: resources.start_indices.as_entire_binding(),
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

struct SphPipeline {
    compute_density: wgpu::ComputePipeline,
    main: wgpu::ComputePipeline,
    swap: bool,
}

impl SphPipeline {
    fn new(
        shader: &wgpu::ShaderModule,
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
        particle_layout: &wgpu::BindGroupLayout,
        spatial_upload_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // todo: naga oil shader composition of final compute shader

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sim Pipeline Layout"),
            bind_group_layouts: &[
                Some(uniform_layout),
                Some(particle_layout),
                Some(spatial_upload_layout),
            ],
            immediate_size: 0,
        });

        let compute_density = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute density pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("compute_density"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let main = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("main sim pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            compute_density,
            main,
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

        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_bind_group(1, particles, &[]);
            pass.set_bind_group(2, spatial_grid, &[]);

            pass.set_pipeline(&self.compute_density);
            pass.dispatch_workgroups(num_dispatches, 1, 1);

            pass.set_pipeline(&self.main);
            pass.dispatch_workgroups(num_dispatches, 1, 1);
        }

        self.swap = !self.swap;
    }
}

struct SpatialGridPipeline {
    upload: wgpu::ComputePipeline,
    start_indices: wgpu::ComputePipeline,
}

impl SpatialGridPipeline {
    fn new(
        shader: &wgpu::ShaderModule,
        device: &wgpu::Device,
        uniform_layout: &wgpu::BindGroupLayout,
        particle_layout: &wgpu::BindGroupLayout,
        spatial_upload_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Spatial Grid Pipeline Layout"),
            bind_group_layouts: &[
                Some(uniform_layout),
                Some(particle_layout),
                Some(spatial_upload_layout),
            ],
            immediate_size: 0,
        });

        let upload_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("spatial grid upload pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("upload_keys"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let start_indices_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("spatial grid start indices pipeline"),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some("upload_start_indices"),
                compilation_options: Default::default(),
                cache: Default::default(),
            });

        Self {
            upload: upload_pipeline,
            start_indices: start_indices_pipeline,
        }
    }

    fn dispatch(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        uniforms: &wgpu::BindGroup,
        particles: &wgpu::BindGroup,
        spatial_grid: &wgpu::BindGroup,
        start_indices: &wgpu::Buffer,
        sorter: &Sorter,
        num_particles: usize,
    ) {
        let num_items_per_workgroup = 64;
        let num_dispatches = num_particles.div_ceil(num_items_per_workgroup) as u32;

        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.upload);
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_bind_group(1, particles, &[]);
            pass.set_bind_group(2, spatial_grid, &[]);

            pass.dispatch_workgroups(num_dispatches, 1, 1);
        }

        sorter.sort(encoder, queue);

        {
            encoder.clear_buffer(start_indices, 0, Some(num_particles as u64));
        }

        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.start_indices);
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_bind_group(1, particles, &[]);
            pass.set_bind_group(2, spatial_grid, &[]);

            pass.dispatch_workgroups(num_dispatches, 1, 1);
        }
    }
}

pub struct Sorter {
    sorter: GPUSorter,
    sort_buffers: SortBuffers,
}

impl Sorter {
    fn new(device: &wgpu::Device, num_particles: usize) -> Self {
        let sorter = GPUSorter::new(device, 32);
        let sort_buffers =
            sorter.create_sort_buffers(&device, NonZeroU32::new(num_particles as u32).unwrap());

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
    sph_pipeline: SphPipeline,
    spatial_grid_pipeline: SpatialGridPipeline,

    bind_group_layouts: SimLayouts,
    bind_groups: SimBindGroups,
    resources: SimResources,
    sorter: Sorter,

    config: SimConfig,
}

impl Sim {
    const LOOK_AHEAD_FACTOR: f32 = 1.0 / 120.0;

    pub fn new(device: &wgpu::Device, config: &SimConfig) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("sph.wgsl"));

        let particles = Self::create_particles(config);
        let resources = SimResources::new(device, config, &particles);
        let bind_group_layouts = SimLayouts::new(device);
        let sorter = Sorter::new(device, config.num_particles as usize);
        let bind_groups = SimBindGroups::new(device, &bind_group_layouts, &resources, &sorter);

        let sph_pipeline = SphPipeline::new(
            &shader,
            device,
            &bind_group_layouts.uniform_layout,
            &bind_group_layouts.particle_layout,
            &bind_group_layouts.spatial_upload_layout,
        );

        let spatial_grid_pipeline = SpatialGridPipeline::new(
            &shader,
            device,
            &bind_group_layouts.uniform_layout,
            &bind_group_layouts.particle_layout,
            &bind_group_layouts.spatial_upload_layout,
        );

        Self {
            sph_pipeline,
            spatial_grid_pipeline,
            bind_group_layouts,
            bind_groups,
            resources,
            sorter,
            config: config.clone(),
        }
    }

    fn create_particles(config: &SimConfig) -> Vec<ParticleRaw> {
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
            particles.push(ParticleRaw {
                pos: [pos.x, pos.y, pos.z, 0.0],
            });
        }
        particles
    }

    pub fn resize_particle_buffers(&mut self, device: &wgpu::Device) {
        let particles = Self::create_particles(&self.config);
        self.resources = SimResources::new(device, &self.config, &particles);
        self.bind_groups = SimBindGroups::new(
            device,
            &self.bind_group_layouts,
            &self.resources,
            &self.sorter,
        );
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        let particles = Self::create_particles(&self.config);
        self.resources = SimResources::new(device, &self.config, &particles);
        self.bind_groups = SimBindGroups::new(
            device,
            &self.bind_group_layouts,
            &self.resources,
            &self.sorter,
        );
    }

    pub fn update_uniforms(&mut self, queue: &wgpu::Queue, dt: instant::Duration) {
        let dt = dt.as_secs_f32();

        let sim_uniform = SimUniform::new(&self.config, dt);
        queue.write_buffer(&self.resources.uniform, 0, bytemuck::bytes_of(&sim_uniform));
    }

    pub fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        self.spatial_grid_pipeline.dispatch(
            encoder,
            queue,
            &self.bind_groups.uniform_bind_group,
            &self.bind_groups.particle_bind_group_a,
            &self.bind_groups.spatial_upload_bind_group,
            &self.resources.start_indices,
            &self.sorter,
            self.config.num_particles as usize,
        );

        self.sph_pipeline.dispatch(
            encoder,
            &self.bind_groups.uniform_bind_group,
            if self.sph_pipeline.swap {
                &self.bind_groups.particle_bind_group_b
            } else {
                &self.bind_groups.particle_bind_group_a
            },
            &self.bind_groups.spatial_upload_bind_group,
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
        if self.sph_pipeline.swap {
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
