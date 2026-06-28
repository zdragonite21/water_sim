use crate::{
    config::WaterConfig,
    water::{renderer::WaterRenderer, sim::WaterSim},
};

pub struct WaterScene {
    sim: WaterSim,
    renderer: WaterRenderer,
    paused: bool,
}

impl WaterScene {
    pub async fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layout: &wgpu::BindGroupLayout,
        water_config: &WaterConfig,
    ) -> anyhow::Result<Self> {
        let mut sim = WaterSim::new(&water_config.sim);
        sim.reset();
        let renderer = WaterRenderer::new(
            device,
            config,
            bind_group_layout,
            water_config.sim.num_particles as usize,
            &water_config.render,
        )
        .await?;
        Ok(Self { sim, renderer, paused: true })
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.renderer.resize(device, config);
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: instant::Duration) {
        if !self.paused {
            self.sim.update(dt);
        }
        self.renderer.upload(queue, &self.sim);
    }

    pub fn reset(&mut self, device: &wgpu::Device) {
        self.sim.reset();
        self.renderer
            .resize_instance_buffer(device, self.sim.current_config().num_particles as usize);
    }

    pub fn update_config(&mut self, queue: &wgpu::Queue, config: &WaterConfig) {
        self.sim.update_config(&config.sim);
        self.renderer.update_config(&config.render);
        self.renderer.update_uniforms(queue);
    }

    pub fn current_config(&self) -> WaterConfig {
        WaterConfig {
            sim: self.sim.current_config(),
            render: self.renderer.current_config(),
        }
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
