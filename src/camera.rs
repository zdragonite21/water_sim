use cgmath::*;
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

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
    pub sensitivity: f32,
}

impl Camera {
    pub fn new<V, Y, P>(
        position: V,
        yaw: Y,
        pitch: P,
        accel: f32,
        damping: f32,
        sensitivity: f32,
    ) -> Self
    where
        V: Into<Point3<f32>>,
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>,
    {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            velocity: Vector3::zero(),
            accel,
            damping,
            sensitivity,
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
            _ => return false
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
    pub input: CameraInput,
    rot_hor: f32,
    rot_vert: f32,
    scroll: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rot_hor += mouse_dx as f32;
        self.rot_vert += mouse_dy as f32;
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration, is_pressed: bool) {
        let dt = dt.as_secs_f32();

        let dir = if is_pressed {
            self.input.movement_vector(camera)
        } else {
            Vector3::zero()
        };

        camera.velocity += dir * camera.accel * dt;
        camera.velocity *= (-camera.damping * dt).exp();
        camera.position += camera.velocity * dt;

        camera.yaw += Rad(-self.rot_hor) * camera.sensitivity;
        camera.pitch += Rad(-self.rot_vert) * camera.sensitivity;
        camera.pitch = Rad(camera.pitch.0.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2));

        self.reset_frame_input();
    }

    pub fn clear_held_input(&mut self) {
        self.input.clear();
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
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use SquareMatrix;
        Self {
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_proj = (projection.proj_matrix() * camera.view_matrix()).into();
    }
}
