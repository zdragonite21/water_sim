use crate::scene::SceneConfig;
use crate::{
    gui::Panel,
    inspect::{Inspect, inspect_config},
};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub camera: CameraConfig,
    pub window: WindowConfig,
    pub scene: SceneConfig,
}

impl AppConfig {
    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path.as_ref(), text)
            .with_context(|| format!("failed to write {}", path.as_ref().display()))
    }
}

inspect_config! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(default)]
    pub struct WindowConfig {
        width: u32 = 1280;
        height: u32 = 720;
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

impl Panel for CameraConfig {
    fn draw(&mut self, context: &egui::Context) {
        egui::Window::new("Camera")
            .default_size([190.0, 90.0])
            .show(context, |ui| {
                self.inspect(ui);
            });
    }
}
