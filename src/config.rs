use crate::inspect::inspect_config;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub camera: CameraConfig,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub water: WaterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            let config = Self::default();
            config.save(path)?;
            return Ok(config);
        }

        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path.as_ref(), text)
            .with_context(|| format!("failed to write {}", path.as_ref().display()))
    }
}

inspect_config! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(default)]
    pub struct CameraConfig {
        accel: f32 = 100.0, 0.0, 300.0;
        damping: f32 = 5.0, 0.0, 20.0;
        mouse_sens: f32 = 0.005, 0.001, 0.02;
        scroll_sens: f32 = 0.1, 0.01, 2.0;
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct WaterSimConfig {
        gravity: f32 = 9.81;
        num_particles: u32 = 10000, 1000, 100000;
        size_x: f32 = 20.0, 1.0, 100.0;
        size_y: f32 = 20.0, 1.0, 100.0;
        smoothing_radius: f32 = 0.5, 0.01, 5.0;
        target_density: f32 = 1.0, 0.5, 20.0;
        pressure_multiplier: f32 = 0.0, 10.0, 1000.0;
        mass: f32 = 1.0, 0.1, 10.0;
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct WaterRenderConfig {
        particle_size: f32 = 1.0, 0.05, 5.0;
        target_density: f32 = 1.0, 0.1, 10.0;
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct DebugOverlayConfig {
        enabled: bool = true;
        velocity_vectors: bool = true;
        bounds: bool = true;
        spatial_grid: bool = true;
        vector_width: f32 = 1.0, 0.1, 10.0;
        vector_scale: f32 = 0.5, 0.01, 5.0;
        velocity_color: [f32; 4] = [0.0, 0.5, 1.0, 1.0];
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WaterConfig {
    pub sim: WaterSimConfig,
    pub render: WaterRenderConfig,
    pub debug: DebugOverlayConfig,
}
