use super::texture::Texture;
use cgmath::Vector3;
use std::ops::Range;
use wgpu::util::DeviceExt;

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimpleVertex {
    pub position: [f32; 3],
}

impl SimpleVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
}

impl Vertex for SimpleVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl ModelVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[allow(unused)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

#[allow(unused)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Mesh {
    pub fn new<V>(name: String, vertices: &[V], indices: &[u32], device: &wgpu::Device) -> Self
    where
        V: Copy + Into<Vector3<f32>>,
    {
        let mut simple_verts = Vec::<SimpleVertex>::new();

        for v in vertices {
            let pos: Vector3<f32> = (*v).into();
            simple_verts.push(SimpleVertex {
                position: [pos.x, pos.y, pos.z],
            });
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&simple_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            name: name,
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
            material: 0,
        }
    }

    pub fn square(device: &wgpu::Device) -> Self {
        let vertices = [
            (-0.5, -0.5, 0.0),
            (0.5, -0.5, 0.0),
            (0.5, 0.5, 0.0),
            (-0.5, 0.5, 0.0),
        ];
        let indices: [u32; 6] = [0, 1, 2, 2, 3, 0];

        Self::new(String::from("square"), &vertices, &indices, device)
    }
}

#[allow(unused)]
pub trait DrawModel {
    fn draw_mesh(&mut self, mesh: &Mesh, material: &Material, camera_bind_group: &wgpu::BindGroup);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &Mesh,
        material: &Material,
        instances: Range<u32>,
        camera_bind_group: &wgpu::BindGroup,
    );
    fn draw_model(&mut self, model: &Model, camera_bind_group: &wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &Model,
        instances: Range<u32>,
        camera_bind_group: &wgpu::BindGroup,
    );
}

impl DrawModel for wgpu::RenderPass<'_> {
    fn draw_mesh(&mut self, mesh: &Mesh, material: &Material, camera_bind_group: &wgpu::BindGroup) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &Mesh,
        material: &Material,
        instances: Range<u32>,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(&mut self, model: &Model, camera_bind_group: &wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, camera_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &Model,
        instances: Range<u32>,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group);
        }
    }
}
