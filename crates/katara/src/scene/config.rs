use crate::inspect::{Inspect, inspect_config};
use crate::gui::Panel;
use serde::{Deserialize, Serialize};

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct RenderConfig {
        slider particle_size: f32 = 0.1, 0.001, 1.0;
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct DebugConfig {
        checkbox enabled: bool = true;
        checkbox xray: bool = false;
        checkbox velocity: bool = true;
        checkbox bounds: bool = true;
        checkbox grid: bool = true;
        drag vector_width: f32 = 1.0, 0.1, 10.0;
        drag vector_scale: f32 = 0.5, 0.01, 5.0;
        color4 velocity_color: [f32; 4] = [0.0, 0.5, 1.0, 1.0];
    }
}

inspect_config! {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct SimConfig {
        drag gravity: f32 = 9.81, 0.1;
        drag num_particles: u32 = 10000, 1000, 1000000;
        vector3 size: [f32; 3] = [20.0, 20.0, 1.0];
        drag smoothing_radius: f32 = 0.5, 0.01, 5.0;
        drag target_density: f32 = 1.0, 0.5, 20.0;
        drag pressure_multiplier: f32 = 0.0, 10.0, 1000.0;
        drag viscosity_strength: f32 = 0.1, 0.0, 10.0;
        drag mass: f32 = 1.0, 0.1, 10.0;
        drag damping: f32 = 0.05, 0.0, 1.0;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SceneConfig {
    pub sim: SimConfig,
    pub render: RenderConfig,
    pub debug: DebugConfig,
}

impl Inspect for SceneConfig {
    fn inspect(&mut self, ui: &mut egui::Ui) {
        ui.label("Simulation");
        self.sim.inspect(ui);
        ui.separator();
        ui.label("Rendering");
        self.render.inspect(ui);
        ui.separator();
        ui.label("Debug Overlay");
        self.debug.inspect(ui);
    }
}

impl Panel for SceneConfig {
    fn draw(&mut self, context: &egui::Context) {
        egui::Window::new("Scene")
            .default_size([190.0, 90.0])
            .show(context, |ui| {
                ui.push_id("mpm", |ui| {
                    self.inspect(ui);
                });
            });
    }
}
