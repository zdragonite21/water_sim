use crate::{
    config::WaterConfig,
    water::{renderer::WaterRenderer, sim::WaterSim},
};

pub struct WaterScene {
    sim: WaterSim,
    renderer: WaterRenderer,
    paused: bool,
    accumulator: instant::Duration,
    fixed_dt: instant::Duration,
}

impl WaterScene {
    const FIXED_FPS: u64 = 120;
    const MAX_STEPS: u8 = 8;

    pub async fn new(
        device: &wgpu::Device,
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

        Ok(Self {
            sim,
            renderer,
            paused: true,
            accumulator: instant::Duration::ZERO,
            fixed_dt: instant::Duration::from_secs_f32(1.0 / Self::FIXED_FPS as f32),
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.renderer.resize(device, config);
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: instant::Duration) {
        if !self.paused {
            self.accumulator += dt.min(instant::Duration::from_millis(Self::FIXED_FPS));
            
            let mut steps = 0;

            while self.accumulator >= self.fixed_dt && steps < Self::MAX_STEPS {
                self.sim.update(self.fixed_dt);
                self.accumulator -= self.fixed_dt;
                steps += 1;
            }

            if steps == Self::MAX_STEPS {
                self.accumulator = instant::Duration::ZERO;
            }
        }
        self.renderer.upload(queue, &self.sim);
    }

    pub fn step(&mut self, queue: &wgpu::Queue) {
        self.sim.update(self.fixed_dt);
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
