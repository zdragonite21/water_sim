use crate::{config::AppConfig, state::State};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

const CONFIG_FILE: &str = "game_config.toml";

struct App {
    state: Option<State>,
    config: AppConfig,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    fn new() -> Self {
        let config = AppConfig::load(CONFIG_FILE).unwrap_or_else(|err| {
            log::warn!("failed to load {CONFIG_FILE}; using defaults: {err}");
            AppConfig::default()
        });

        Self {
            state: None,
            config,
        }
    }

    fn save_config(&self) {
        if let Some(state) = &self.state {
            if let Err(err) = state.current_config().save(CONFIG_FILE) {
                log::warn!("failed to save {CONFIG_FILE}: {err}");
            } else {
                log::debug!("saving {CONFIG_FILE}");
            }
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_inner_size(
            winit::dpi::PhysicalSize::new(self.config.window.width, self.config.window.height),
        );

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        {
            self.state = Some(pollster::block_on(State::new(window, &self.config)).unwrap());
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        self.state = Some(event);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        if let DeviceEvent::MouseMotion { delta } = event
            && state.camera.controller.is_captured()
        {
            state.camera.controller.handle_mouse(delta.0, delta.1);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        state.handle_event(&event);

        if window_id != state.window.id() {
            return;
        }

        if !(state.handle_shortcut(&event) || state.input(&event)) {
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => event_loop.exit(),
                WindowEvent::Resized(size) => state.resize(size.width, size.height),
                WindowEvent::RedrawRequested => match state.frame() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                },
                _ => {}
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.save_config();
    }
}

pub fn run() -> anyhow::Result<()> {
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("katara=info"))
            .init();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    {
        let mut app = App::new();
        event_loop.run_app(&mut app)?;
    }

    Ok(())
}
