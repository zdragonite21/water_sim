use crate::{config::WaterConfig, water::{renderer::WaterRenderer, sim::WaterSim}};

pub struct WaterScene {
    sim: WaterSim,
    renderer: WaterRenderer,
    water_config: WaterConfig,
}

impl WaterScene {
    pub async fn new(
            device: &wgpu::Device,
            _queue: &wgpu::Queue,
            config: &wgpu::SurfaceConfiguration,
            bind_group_layout: &wgpu::BindGroupLayout,
            water_config: &WaterConfig,
        ) -> anyhow::Result<Self> {
            let sim = WaterSim::new(water_config.num_particles as usize);
            let renderer = WaterRenderer::new(
                device,
                config,
                bind_group_layout,
                water_config.num_particles as usize,
            ).await?;
            Ok(Self { sim, renderer, water_config: *water_config })
        }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.renderer.resize(device, config);
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: instant::Duration) {
        self.sim.update(dt);
        self.renderer.upload(queue, &self.sim);
    }

    pub fn config_mut(&mut self) -> &mut WaterConfig {
        &mut self.water_config
    }

    pub fn current_config(&self) -> WaterConfig {
        self.water_config
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> anyhow::Result<()> {
        self.renderer
            .render(encoder, target_view, camera_bind_group)
    }
}
