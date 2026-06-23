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
    pub speed: f32,
    pub sensitivity: f32,
}

impl Camera {
    pub fn new<V, Y, P>(position: V, yaw: Y, pitch: P, speed: f32, sensitivity: f32) -> Self
    where
        V: Into<Point3<f32>>,
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>,
    {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            speed,
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

#[derive(Debug, Default)]
pub struct CameraController {
    right_off: f32,
    up_off: f32,
    fwd_off: f32,
    rot_hor: f32,
    rot_vert: f32,
    scroll: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return true;
        }
        let amount = 1.0;
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.fwd_off += amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.fwd_off -= amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.right_off -= amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.right_off += amount;
                true
            }
            KeyCode::Space => {
                self.up_off += amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.up_off -= amount;
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rot_hor = mouse_dx as f32;
        self.rot_vert = mouse_dy as f32;
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        camera.position += camera.forward() * self.fwd_off * camera.speed * dt;
        camera.position += camera.right() * self.right_off * camera.speed * dt;
        camera.position += Vector3::unit_y() * self.up_off * camera.speed * dt;
        camera.position += camera.forward() * self.scroll * camera.speed * camera.sensitivity * dt;

        camera.yaw += Rad(self.rot_hor) * camera.sensitivity * dt;
        camera.pitch += Rad(-self.rot_vert) * camera.sensitivity * dt;

        self.reset();

        camera.pitch = Rad(camera.pitch.0.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2));
    }

    fn reset(&mut self) {
        *self = Self::default();
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
