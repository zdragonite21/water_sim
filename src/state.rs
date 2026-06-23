use std::default::Default;
use std::sync::Arc;

use winit::{
    event::{KeyEvent, MouseButton, WindowEvent},
    keyboard::PhysicalKey,
    window::{CursorGrabMode, Window},
};

use crate::camera::CameraRig;
use crate::scene::DemoScene;
use crate::frame_clock::FrameClock;

pub struct State {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,

    pub camera: CameraRig,
    scene: DemoScene,
    frame_clock: FrameClock,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let info = adapter.get_info();
        log::info!(
            "\nGPU: {} \nbackend: {:?} \ndriver: {} \ndriver info: {}",
            info.name,
            info.backend,
            info.driver,
            info.driver_info,
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config: wgpu::wgt::SurfaceConfiguration<Vec<wgpu::TextureFormat>> =
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

        let camera = CameraRig::new(&device, config.width, config.height);

        let scene = DemoScene::new(&device, &queue, &config, &camera.bind_group_layout).await?;
        
        let frame_clock = FrameClock::new();

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            camera,
            scene,
            frame_clock,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            self.camera.resize(width, height);
            self.scene.resize(&self.device, &self.config);

            self.is_surface_configured = true;
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.camera.controller.input.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera.controller.handle_mouse_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                let mouse_pressed = *state == winit::event::ElementState::Pressed;
                self.window.set_cursor_visible(!mouse_pressed);
                self.camera.controller.set_captured(mouse_pressed);

                if mouse_pressed {
                    let _ = self
                        .window
                        .set_cursor_grab(CursorGrabMode::Locked)
                        .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
                } else {
                    let _ = self.window.set_cursor_grab(CursorGrabMode::None);
                }

                true
            }
            WindowEvent::Focused(false) => {
                self.camera.controller.set_captured(false);
                self.window.set_cursor_visible(true);
                let _ = self.window.set_cursor_grab(CursorGrabMode::None);
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: instant::Duration) {
        self.camera.update(&self.queue, dt);
        self.scene.update(dt);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let (output, reconfigure_after_present) = {
            use wgpu::CurrentSurfaceTexture::*;

            match self.surface.get_current_texture() {
                Success(surface_texture) => (surface_texture, false),
                Suboptimal(surface_texture) => (surface_texture, true),
                Timeout | Occluded | Validation => {
                    return Ok(());
                }
                Outdated => {
                    self.surface.configure(&self.device, &self.config);
                    return Ok(());
                }
                Lost => {
                    anyhow::bail!("Lost device");
                }
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.scene
            .render(&mut encoder, &view, &self.camera.bind_group)?;

        self.queue.submit([encoder.finish()]);
        output.present();

        if reconfigure_after_present {
            self.surface.configure(&self.device, &self.config);
        }

        Ok(())
    }

    pub fn frame(&mut self) -> anyhow::Result<()> {
        self.frame_clock.tick();
        self.update(self.frame_clock.dt);
        self.render()
    }
}
