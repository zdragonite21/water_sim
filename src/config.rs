use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub camera: CameraConfig,
}

#[derive(Serialize, Deserialize)]
pub struct CameraConfig {
    pub accel: f32,
    pub damping: f32,
    pub mouse_sens: f32,
    pub scroll_sens: f32,
}
