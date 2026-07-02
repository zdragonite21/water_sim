use crate::inspect::inspect_config;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub camera: CameraConfig,
    pub window: WindowConfig,
    pub water: WaterConfig,
}

inspect_config! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(default)]
    pub struct WindowConfig {
        width: u32 = 1280;
        height: u32 = 720;
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
        slider accel: f32 = 100.0, 0.0, 300.0;
        slider damping: f32 = 5.0, 0.0, 20.0;
        slider mouse_sens: f32 = 0.005, 0.001, 0.02;
        slider scroll_sens: f32 = 0.1, 0.01, 2.0;
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct WaterSimConfig {
        slider gravity: f32 = 9.81;
        slider num_particles: u32 = 10000, 1000, 100000;
        vector3 size: [f32; 3] = [20.0, 20.0, 1.0];
        slider smoothing_radius: f32 = 0.5, 0.01, 5.0;
        slider target_density: f32 = 1.0, 0.5, 20.0;
        slider pressure_multiplier: f32 = 0.0, 10.0, 1000.0;
        slider mass: f32 = 1.0, 0.1, 10.0;
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct WaterRenderConfig {
        slider particle_size: f32 = 1.0, 0.05, 5.0;
        slider target_density: f32 = 1.0, 0.1, 10.0;
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct DebugOverlayConfig {
        checkbox enabled: bool = true;
        checkbox velocity: bool = true;
        checkbox bounds: bool = true;
        checkbox grid: bool = true;
        drag vector_width: f32 = 1.0, 0.1, 10.0;
        drag vector_scale: f32 = 0.5, 0.01, 5.0;
        color4 velocity_color: [f32; 4] = [0.0, 0.5, 1.0, 1.0];
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WaterConfig {
    pub sim: WaterSimConfig,
    pub render: WaterRenderConfig,
    pub debug: DebugOverlayConfig,
}
