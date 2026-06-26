use cgmath::*;
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

use crate::config::CameraConfig;

pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::from_cols(
    Vector4::new(1.0, 0.0, 0.0, 0.0),
    Vector4::new(0.0, 1.0, 0.0, 0.0),
    Vector4::new(0.0, 0.0, 0.5, 0.0),
    Vector4::new(0.0, 0.0, 0.5, 1.0),
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct Camera {
    pub position: Point3<f32>,
    pub yaw: Rad<f32>,
    pub pitch: Rad<f32>,
    pub velocity: Vector3<f32>,
    pub accel: f32,
    pub damping: f32,
    pub mouse_sens: f32,
    pub scroll_sens: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 5.0, 10.0),
            yaw: cgmath::Deg(-90.0).into(),
            pitch: cgmath::Deg(-20.0).into(),
            velocity: Vector3::zero(),
            accel: 100.0,
            damping: 5.0,
            mouse_sens: 0.002,
            scroll_sens: 0.3,
        }
    }
}

impl Camera {
    pub fn new(accel: f32, damping: f32, mouse_sens: f32, scroll_sens: f32) -> Self {
        Self {
            velocity: Vector3::zero(),
            accel,
            damping,
            mouse_sens,
            scroll_sens,
            ..Self::default()
        }
    }

    pub fn orient(&self) -> Quaternion<f32> {
        let yaw = Quaternion::from_angle_y(self.yaw);
        let pitch = Quaternion::from_angle_x(self.pitch);

        yaw * pitch
    }

    pub fn forward(&self) -> Vector3<f32> {
        self.orient() * -Vector3::unit_z()
    }

    pub fn right(&self) -> Vector3<f32> {
        self.orient() * Vector3::unit_x()
    }

    pub fn up(&self) -> Vector3<f32> {
        self.orient() * Vector3::unit_y()
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(self.position, self.forward(), Vector3::unit_y())
    }

    pub fn reset_view(&mut self) {
        *self = Self {
            accel: self.accel,
            damping: self.damping,
            mouse_sens: self.mouse_sens,
            scroll_sens: self.scroll_sens,
            ..Self::default()
        }
    }
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn proj_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Default, Debug)]
pub struct CameraInput {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl CameraInput {
    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => self.forward = pressed,
            KeyCode::KeyS | KeyCode::ArrowDown => self.backward = pressed,
            KeyCode::KeyA | KeyCode::ArrowLeft => self.left = pressed,
            KeyCode::KeyD | KeyCode::ArrowRight => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::ShiftLeft => self.down = pressed,
            _ => return false,
        }
        true
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn axis(positive: bool, negative: bool) -> f32 {
        positive as i32 as f32 - negative as i32 as f32
    }

    pub fn movement_vector(&self, camera: &Camera) -> Vector3<f32> {
        let dir = camera.forward() * Self::axis(self.forward, self.backward)
            + camera.right() * Self::axis(self.right, self.left)
            + Vector3::unit_y() * Self::axis(self.up, self.down);

        if dir.magnitude2() > 0.0 {
            dir.normalize()
        } else {
            Vector3::zero()
        }
    }
}

#[derive(Debug, Default)]
pub struct CameraController {
    input: CameraInput,
    captured: bool,
    rot_hor: f32,
    rot_vert: f32,
    scroll: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_captured(&mut self, captured: bool) {
        self.captured = captured;

        if !captured {
            self.input.clear();
        }
    }

    pub fn is_captured(&self) -> bool {
        self.captured
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        if self.is_captured() {
            self.input.process_keyboard(key, state)
        } else {
            false
        }
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.is_captured() {
            self.rot_hor += mouse_dx as f32;
            self.rot_vert += mouse_dy as f32;
        }
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        let dir;
        if self.is_captured() {
            camera.accel = (camera.accel - self.scroll * camera.scroll_sens * dt).clamp(0.0, 300.0);
            dir = self.input.movement_vector(camera);
        } else {
            camera.position += camera.forward() * -self.scroll * camera.scroll_sens * dt;
            dir = Vector3::zero();
        }

        camera.velocity += dir * camera.accel * dt;
        camera.velocity *= (-camera.damping * dt).exp();
        camera.position += camera.velocity * dt;

        camera.yaw += Rad(-self.rot_hor) * camera.mouse_sens;
        camera.pitch += Rad(-self.rot_vert) * camera.mouse_sens;
        camera.pitch = Rad(camera.pitch.0.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2));

        self.reset_frame_input();
    }

    fn reset_frame_input(&mut self) {
        self.rot_hor = 0.0;
        self.rot_vert = 0.0;
        self.scroll = 0.0;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use SquareMatrix;
        Self {
            view: Matrix4::identity().into(),
            proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view = camera.view_matrix().into();
        self.proj = projection.proj_matrix().into();
    }
}

pub struct CameraRig {
    pub camera: Camera,
    pub projection: Projection,
    pub controller: CameraController,
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraRig {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, config: &CameraConfig) -> Self {
        let camera = Camera::new(
            config.accel,
            config.damping,
            config.mouse_sens,
            config.scroll_sens,
        );
        let projection = Projection::new(width, height, cgmath::Deg(45.0), 0.1, 100.0);
        let controller = CameraController::new();

        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&camera, &projection);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("camera_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            camera,
            projection,
            controller,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: Duration) {
        self.controller.update_camera(&mut self.camera, dt);
        self.uniform
            .update_view_proj(&self.camera, &self.projection);

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }

    pub fn current_config(&self) -> CameraConfig {
        CameraConfig {
            accel: self.camera.accel,
            damping: self.camera.damping,
            mouse_sens: self.camera.mouse_sens,
            scroll_sens: self.camera.scroll_sens,
        }
    }
}
