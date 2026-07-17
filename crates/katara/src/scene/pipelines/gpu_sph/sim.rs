#![allow(dead_code)]
use std::f32::consts::PI;
use std::num::NonZeroU32;

use super::config::SimConfig;
use crate::profiling::{GpuFrameRecorder, gpu_profile};
use crate::stats::debug_stats;
use cgmath::{Point3, Vector3};

use wgpu::util::DeviceExt;
use wgpu_sort::{GPUSorter, SortBuffers};

const WORKGROUP_SIZE: usize = 128;

fn create_layout(
    device: &wgpu::Device,
    label: &'static str,
    bindings: &[wgpu::BufferBindingType],
) -> wgpu::BindGroupLayout {
    let entries = bindings
        .iter()
        .enumerate()
        .map(|(binding, ty)| wgpu::BindGroupLayoutEntry {
            binding: binding as u32,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: *ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        })
        .collect::<Vec<_>>();

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &entries,
    })
}

fn create_bind_group(
    device: &wgpu::Device,
    label: &'static str,
    layout: &wgpu::BindGroupLayout,
    buffers: &[&wgpu::Buffer],
    swap_vel: bool,
) -> wgpu::BindGroup {
    let mut entries = buffers
        .iter()
        .enumerate()
        .map(|(binding, buffer)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: buffer.as_entire_binding(),
        })
        .collect::<Vec<_>>();

    if swap_vel {
        entries[2] = wgpu::BindGroupEntry {
            binding: 2,
            resource: buffers[3].as_entire_binding(),
        };
        entries[3] = wgpu::BindGroupEntry {
            binding: 3,
            resource: buffers[2].as_entire_binding(),
        };
    }

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &entries,
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    entry_point: &'static str,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry_point),
        layout: Some(layout),
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: Default::default(),
    })
}

debug_stats! {
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub struct Stats {
        stat particle_count: usize = 0;
        stat exp_density: f32 = 0.0;
        stat exp_neighbor_count: f32 = 0.0;
        stat exp_particles_per_cell: f32 = 0.0;
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    pub pos: [f32; 4],
}

impl ParticleRaw {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![5 => Float32x4];

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
    inv_smoothing_radius: f32,
    stiffness: f32,
    viscosity_strength: f32,
    rest_density: f32,
    mass: f32,
    inv_density_kernel_volume: f32,
    density_kernel_scale: f32,
    inv_viscosity_kernel_volume: f32,
    _pad: [f32; 1],
}

impl SimUniform {
    pub fn new(config: &SimConfig, dt: f32) -> Self {
        let radius = config.smoothing_radius;

        Self {
            size: config.size,
            collision_damping: config.damping,
            dt,
            gravity: config.gravity,
            smoothing_radius: config.smoothing_radius,
            inv_smoothing_radius: 1.0 / config.smoothing_radius,
            stiffness: config.pressure_multiplier,
            viscosity_strength: config.viscosity_strength,
            rest_density: config.target_density,
            mass: config.mass,
            inv_density_kernel_volume: 15.0 / (2.0 * PI * radius.powf(5.0)),
            density_kernel_scale: 15.0 / (PI * radius.powf(5.0)),
            inv_viscosity_kernel_volume: (315.0 / (64.0 * PI * radius.powf(9.0))),
            _pad: [0.0; 1],
        }
    }
}

struct SimResources {
    uniform: wgpu::Buffer,
    particles: [wgpu::Buffer; 4],
    intervals: wgpu::Buffer,
}

impl SimResources {
    fn new(device: &wgpu::Device, config: &SimConfig, particles: &[ParticleRaw]) -> Self {
        let particle_count = particles.len();
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sim uniform buffer"),
            contents: bytemuck::bytes_of(&SimUniform::new(config, 0.0)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let positions = bytemuck::cast_slice(particles);
        let velocities = vec![[0.0f32; 4]; particles.len()];
        let velocities = bytemuck::cast_slice(&velocities);
        let contents = [positions, positions, velocities, velocities];
        let labels = ["positions a", "positions b", "velocities a", "velocities b"];
        let particles = std::array::from_fn(|i| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(labels[i]),
                contents: contents[i],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            })
        });

        let intervals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spatial intervals"),
            size: (std::mem::size_of::<[u32; 2]>() * particle_count) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniform,
            particles,
            intervals,
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
        use wgpu::BufferBindingType::{Storage, Uniform};
        let read_write = Storage { read_only: false };
        let uniform_layout = create_layout(device, "sim uniforms", &[Uniform]);
        let particle_layout = create_layout(device, "sim particles", &[read_write; 4]);
        let spatial_upload_layout = create_layout(device, "spatial grid", &[read_write; 3]);

        Self {
            uniform_layout,
            particle_layout,
            spatial_upload_layout,
        }
    }
}

struct SimBindGroups {
    uniform: wgpu::BindGroup,
    particles: wgpu::BindGroup,
    particles_swapped: wgpu::BindGroup,
    spatial_grid: wgpu::BindGroup,
}

impl SimBindGroups {
    fn new(
        device: &wgpu::Device,
        layouts: &SimLayouts,
        resources: &SimResources,
        sorter: &Sorter,
    ) -> Self {
        Self {
            uniform: create_bind_group(
                device,
                "sim uniforms",
                &layouts.uniform_layout,
                &[&resources.uniform],
                false,
            ),
            particles: create_bind_group(
                device,
                "sim particles",
                &layouts.particle_layout,
                &resources.particles.iter().collect::<Vec<_>>(),
                false,
            ),
            particles_swapped: create_bind_group(
                device,
                "sim particles swapped",
                &layouts.particle_layout,
                &resources.particles.iter().collect::<Vec<_>>(),
                true,
            ),
            spatial_grid: create_bind_group(
                device,
                "spatial grid",
                &layouts.spatial_upload_layout,
                &[sorter.keys(), sorter.values(), &resources.intervals],
                false,
            ),
        }
    }
}

struct SphPipeline {
    compute_density: wgpu::ComputePipeline,
    pressure_viscosity: wgpu::ComputePipeline,
    collisions: wgpu::ComputePipeline,
}

impl SphPipeline {
    fn new(
        shader: &wgpu::ShaderModule,
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
    ) -> Self {
        Self {
            compute_density: create_pipeline(device, shader, layout, "compute_density"),
            pressure_viscosity: create_pipeline(device, shader, layout, "pressure_viscosity"),
            collisions: create_pipeline(device, shader, layout, "handle_collisions"),
        }
    }

    fn dispatch(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        uniforms: &wgpu::BindGroup,
        particles: &wgpu::BindGroup,
        spatial_grid: &wgpu::BindGroup,
        num_particles: usize,
        gpu_frame: Option<&GpuFrameRecorder>,
    ) {
        let num_dispatches = num_particles.div_ceil(WORKGROUP_SIZE) as u32;

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Water SPH / simulation"),
                ..Default::default()
            });
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_bind_group(1, particles, &[]);
            pass.set_bind_group(2, spatial_grid, &[]);

            gpu_profile!(gpu_frame, &mut pass, "Density", {
                pass.push_debug_group("Water SPH / density");
                pass.set_pipeline(&self.compute_density);
                pass.dispatch_workgroups(num_dispatches, 1, 1);
                pass.pop_debug_group();
            });

            gpu_profile!(gpu_frame, &mut pass, "Pressure and viscosity", {
                pass.push_debug_group("Water SPH / pressure and viscosity");
                pass.set_pipeline(&self.pressure_viscosity);
                pass.dispatch_workgroups(num_dispatches, 1, 1);
                pass.pop_debug_group();
            });

            gpu_profile!(gpu_frame, &mut pass, "Collision and integration", {
                pass.push_debug_group("Water SPH / collision and integration");
                pass.set_pipeline(&self.collisions);
                pass.dispatch_workgroups(num_dispatches, 1, 1);
                pass.pop_debug_group();
            });
        }
    }
}

struct SpatialGridPipeline {
    upload: wgpu::ComputePipeline,
    start_indices: wgpu::ComputePipeline,
    gather: wgpu::ComputePipeline,
}

impl SpatialGridPipeline {
    fn new(
        shader: &wgpu::ShaderModule,
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
    ) -> Self {
        Self {
            upload: create_pipeline(device, shader, layout, "upload_keys"),
            start_indices: create_pipeline(device, shader, layout, "upload_start_indices"),
            gather: create_pipeline(device, shader, layout, "gather_particles"),
        }
    }

    fn dispatch(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        uniforms: &wgpu::BindGroup,
        particles: &wgpu::BindGroup,
        spatial_grid: &wgpu::BindGroup,
        intervals: &wgpu::Buffer,
        sorter: &Sorter,
        num_particles: usize,
        gpu_frame: Option<&GpuFrameRecorder>,
    ) {
        let num_dispatches = num_particles.div_ceil(WORKGROUP_SIZE) as u32;

        gpu_profile!(gpu_frame, encoder, "Grid key upload", {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Water SPH / grid key upload"),
                ..Default::default()
            });
            pass.set_pipeline(&self.upload);
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_bind_group(1, particles, &[]);
            pass.set_bind_group(2, spatial_grid, &[]);
            pass.push_debug_group("Water SPH / grid key upload");
            pass.dispatch_workgroups(num_dispatches, 1, 1);
            pass.pop_debug_group();
        });

        gpu_profile!(gpu_frame, encoder, "Radix sort", {
            sorter.sort(encoder, queue);
        });

        gpu_profile!(gpu_frame, encoder, "Grid start indices", {
            encoder.clear_buffer(intervals, 0, None);
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Water SPH / grid start indices"),
                ..Default::default()
            });
            pass.set_pipeline(&self.start_indices);
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_bind_group(1, particles, &[]);
            pass.set_bind_group(2, spatial_grid, &[]);
            pass.push_debug_group("Water SPH / grid start indices");
            pass.dispatch_workgroups(num_dispatches, 1, 1);
            pass.pop_debug_group();
        });

        gpu_profile!(gpu_frame, encoder, "Gather particles", {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Water SPH / gather particles"),
                ..Default::default()
            });
            pass.set_pipeline(&self.gather);
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_bind_group(1, particles, &[]);
            pass.set_bind_group(2, spatial_grid, &[]);
            pass.push_debug_group("Water SPH / gather particles");
            pass.dispatch_workgroups(num_dispatches, 1, 1);
            pass.pop_debug_group();
        });
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
    num_particles: usize,
    gpu_recorder: Option<GpuFrameRecorder>,

    swap: bool,
}

impl Sim {
    pub fn new(
        device: &wgpu::Device,
        config: &SimConfig,
        gpu_recorder: Option<GpuFrameRecorder>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("sph.wgsl"));

        let particles = Self::create_particles(config);
        let resources = SimResources::new(device, config, &particles);
        let bind_group_layouts = SimLayouts::new(device);
        let sorter = Sorter::new(device, config.num_particles as usize);
        let bind_groups = SimBindGroups::new(device, &bind_group_layouts, &resources, &sorter);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simulation"),
            bind_group_layouts: &[
                Some(&bind_group_layouts.uniform_layout),
                Some(&bind_group_layouts.particle_layout),
                Some(&bind_group_layouts.spatial_upload_layout),
            ],
            immediate_size: 0,
        });
        let sph_pipeline = SphPipeline::new(&shader, device, &pipeline_layout);
        let spatial_grid_pipeline = SpatialGridPipeline::new(&shader, device, &pipeline_layout);

        Self {
            sph_pipeline,
            spatial_grid_pipeline,
            bind_group_layouts,
            bind_groups,
            resources,
            sorter,
            config: config.clone(),
            num_particles: config.num_particles as usize,
            gpu_recorder,
            swap: false,
        }
    }

    fn create_particles(config: &SimConfig) -> Vec<ParticleRaw> {
        let n = config.num_particles as usize;
        let mut particles = Vec::with_capacity(n);

        let scale = 0.5;

        let size = Vector3::new(
            config.size[0] * scale,
            config.size[1] * scale,
            config.size[2] * scale,
        );

        let cells_per_unit = (n as f32 / (size.x * size.y * size.z)).cbrt();

        let nx = (size.x * cells_per_unit).round().max(1.0) as usize;
        let ny = (size.y * cells_per_unit).round().max(1.0) as usize;
        let nz = n.div_ceil(nx * ny);

        for i in 0..n {
            let ix = i % nx;
            let iy = (i / nx) % ny;
            let iz = i / (nx * ny);

            let spacing = Vector3::new(size.x / nx as f32, size.y / ny as f32, size.z / nz as f32);

            let pos = Point3::new(
                (ix as f32 + 0.5) * spacing.x,
                (iy as f32 + 0.5) * spacing.y,
                (iz as f32 + 0.5) * spacing.z,
            ) - size * 0.5;

            particles.push(ParticleRaw {
                pos: [pos.x, pos.y, pos.z, 0.0],
            });
        }
        particles
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        let particles = Self::create_particles(&self.config);
        self.num_particles = self.config.num_particles as usize;

        self.resources = SimResources::new(device, &self.config, &particles);
        self.sorter = Sorter::new(device, self.num_particles);
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
        let particles = if self.swap {
            &self.bind_groups.particles_swapped
        } else {
            &self.bind_groups.particles
        };
        self.spatial_grid_pipeline.dispatch(
            encoder,
            queue,
            &self.bind_groups.uniform,
            particles,
            &self.bind_groups.spatial_grid,
            &self.resources.intervals,
            &self.sorter,
            self.num_particles,
            self.gpu_recorder.as_ref(),
        );

        self.sph_pipeline.dispatch(
            encoder,
            &self.bind_groups.uniform,
            particles,
            &self.bind_groups.spatial_grid,
            self.num_particles,
            self.gpu_recorder.as_ref(),
        );
        self.swap = !self.swap;
    }
}

impl Sim {
    pub fn get_stats(&self) -> Stats {
        let particle_count = self.num_particles;
        let r = self.config.smoothing_radius;
        let size = Vector3::new(
            self.config.size[0],
            self.config.size[1],
            self.config.size[2],
        );

        let volume = size.x * size.y * size.z;
        let exp_density = particle_count as f32 * self.config.mass / volume;
        let exp_neighbor_count = 4.0 / 3.0 * PI * r * r * r * particle_count as f32 / volume;

        let num_cells = (size.x / r).ceil() * (size.y / r).ceil() * (size.z / r).ceil();
        let exp_particles_per_cell = particle_count as f32 / num_cells;

        Stats {
            particle_count,
            exp_density,
            exp_neighbor_count,
            exp_particles_per_cell,
        }
    }

    pub fn smoothing_radius(&self) -> f32 {
        self.config.smoothing_radius
    }

    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        &self.resources.particles[0]
    }

    pub fn velocity_buffers(&self) -> [&wgpu::Buffer; 2] {
        [&self.resources.particles[2], &self.resources.particles[3]]
    }

    pub fn velocity_buffer_index(&self) -> usize {
        // `swap` has already been toggled after dispatch. The next input buffer
        // is the opposite one from the velocity buffer just written.
        if self.swap { 1 } else { 0 }
    }

    pub fn num_particles(&self) -> usize {
        self.num_particles
    }

    pub fn update_config(&mut self, config: &SimConfig) {
        if config == &self.config {
            return;
        }
        self.config = config.clone();
    }

    pub fn bounds(&self) -> Vector3<f32> {
        self.config.size.into()
    }
}
