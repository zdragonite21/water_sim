use std::default::Default;
use std::sync::Arc;

use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use crate::{
    camera::CameraRig,
    config::{AppConfig, WindowConfig},
    debug_watch,
    scene::Scene,
};
use crate::{frame_clock::FrameClock, gui::Gui};

pub struct State {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,

    pub camera: CameraRig,
    scene: Scene,
    frame_clock: FrameClock,
    gui: Gui,
}

impl State {
    pub async fn new(window: Arc<Window>, app_config: &AppConfig) -> anyhow::Result<State> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
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
            "\ngpu: {}\nbackend: {}\ndriver: {} ({})\n",
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

        log::debug!(
            "surface capabilities:\nselected_format={:?}\npresent_modes={:?}\nalpha_modes={:?}",
            surface_format,
            surface_caps.present_modes,
            surface_caps.alpha_modes
        );

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

        let camera = CameraRig::new(&device, config.width, config.height, &app_config.camera);

        let scene = Scene::new(
            &device,
            &config,
            &camera.bind_group_layout,
            &app_config.scene,
        )
        .await?;
        log::debug!("water scene created");

        let frame_clock = FrameClock::new();

        let gui = Gui::new(&device, &queue, &window, surface_format);

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
            gui,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            self.camera.resize(width, height);
            self.scene.resize(&self.device, &self.queue, &self.config);
            log::debug!("surface resized to {}x{}", width, height);

            self.is_surface_configured = true;
        } else {
            log::debug!("ignored zero-sized surface resize: {}x{}", width, height);
        }
    }

    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        self.gui.handle_event(&self.window, event);
    }

    pub fn gui_wants_keyboard(&self) -> bool {
        self.gui.wants_keyboard()
    }

    pub fn gui_wants_mouse(&self) -> bool {
        self.gui.wants_mouse()
    }

    fn set_camera_capture(&mut self, captured: bool) {
        self.camera.controller.set_captured(captured);
        self.window.set_cursor_visible(!captured);

        if captured {
            match self
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
            {
                Ok(()) => log::debug!("cursor captured"),
                Err(err) => {
                    self.camera.controller.set_captured(false);
                    self.window.set_cursor_visible(true);
                    log::warn!("failed to capture cursor: {err}");
                }
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            log::debug!("cursor released");
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
            } => self.camera.controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. }
                if self.camera.controller.is_captured() || !self.gui_wants_mouse() =>
            {
                self.camera.controller.handle_mouse_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } if *state == ElementState::Pressed && !self.gui_wants_mouse() => {
                self.set_camera_capture(true);
                true
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } if *state == ElementState::Released && self.camera.controller.is_captured() => {
                self.set_camera_capture(false);
                true
            }
            WindowEvent::Focused(false) => {
                self.set_camera_capture(false);
                log::debug!("window lost focus, cursor released");
                true
            }
            _ => false,
        }
    }

    pub fn handle_shortcut(&mut self, event: &WindowEvent) -> bool {
        if self.gui_wants_keyboard() {
            return false;
        }

        let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        else {
            return false;
        };

        match key {
            KeyCode::F2 => {
                self.gui.toggle_debug_text();
                true
            }
            KeyCode::KeyH => {
                self.camera.camera.reset_view();
                true
            }
            KeyCode::KeyR => {
                self.scene.reset(&self.device);
                true
            }
            KeyCode::Enter => {
                self.scene.toggle_pause();
                true
            }
            KeyCode::ArrowRight => {
                if self.scene.paused() {
                    self.scene.step(&self.queue, &self.camera.view_proj());
                }
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: instant::Duration) {
        self.camera.update(&self.queue, dt);
        self.scene.update(&self.queue, dt, &self.camera.view_proj());
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            log::debug!("surface not configured yet; skipping render");
            return Ok(());
        }

        let (output, reconfigure_after_present) = {
            use wgpu::CurrentSurfaceTexture::*;

            match self.surface.get_current_texture() {
                Success(surface_texture) => (surface_texture, false),
                Suboptimal(surface_texture) => (surface_texture, true),
                Timeout => {
                    log::debug!("surface texture acquisition timed out; skipping frame");
                    return Ok(());
                }
                Occluded => {
                    log::debug!("surface is occluded; skipping frame");
                    return Ok(());
                }
                Validation => {
                    log::warn!("surface texture acquisition failed validation; skipping frame");
                    return Ok(());
                }
                Outdated => {
                    log::debug!("surface outdated; reconfiguring");
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

        let scene_stats = self.scene.stats();

        self.gui.render(
            self.frame_clock.dt,
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            &self.window,
            self.camera.config_mut(),
            self.scene.config_mut(),
            &scene_stats,
        )?;

        // changes take effect next frame
        let reset = self.scene.sync_pipeline(
            &self.device,
            &self.queue,
            &self.config,
            &self.camera.bind_group_layout,
        )?;

        if reset {
            self.frame_clock = FrameClock::new();
            debug_watch::clear();
        }

        self.queue.submit([encoder.finish()]);
        output.present();

        if reconfigure_after_present {
            log::debug!("surface suboptimal after present; reconfiguring");
            self.surface.configure(&self.device, &self.config);
        }

        Ok(())
    }

    pub fn frame(&mut self) -> anyhow::Result<()> {
        self.frame_clock.tick();
        debug_watch::begin_frame(self.frame_clock.frame_index, self.scene.paused());
        self.update(self.frame_clock.dt);
        self.render()
    }

    pub fn current_config(&self) -> AppConfig {
        AppConfig {
            camera: self.camera.current_config(),
            window: WindowConfig {
                width: self.config.width,
                height: self.config.height,
            },
            scene: self.scene.current_config(),
        }
    }
}
