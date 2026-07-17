use crate::{config::AppConfig, state::State};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};
#[cfg(not(target_arch = "wasm32"))]
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

#[cfg(not(target_arch = "wasm32"))]
const CONFIG_FILE: &str = "game_config.toml";
#[cfg(target_arch = "wasm32")]
const DEFAULT_CONFIG: &str = include_str!("../../../100k_config.toml");

struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
    config: AppConfig,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());

        #[cfg(not(target_arch = "wasm32"))]
        let config = AppConfig::load(CONFIG_FILE).unwrap_or_else(|err| {
            log::warn!("failed to load {CONFIG_FILE}; using defaults: {err}");
            AppConfig::default()
        });

        #[cfg(target_arch = "wasm32")]
        let config = toml::from_str(DEFAULT_CONFIG).expect("failed to parse 100k_config.toml");
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
            config,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(not(target_arch = "wasm32"))]
        let window_attributes = Window::default_attributes().with_inner_size(
            winit::dpi::PhysicalSize::new(self.config.window.width, self.config.window.height),
        );

        #[cfg(target_arch = "wasm32")]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element))
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window, &self.config)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                let config = self.config.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window, &config)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
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
                #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(not(target_arch = "wasm32"))]
        self.save_config();
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("katara=info"))
            .init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn check_webgpu() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let gpu = js_sys::Reflect::get(&window.navigator(), &JsValue::from_str("gpu"))?;

    if gpu.is_null() || gpu.is_undefined() {
        return Err(JsValue::from_str(
            "WebGPU is unavailable: enable WebGPU in this browser or use a browser with WebGPU support.",
        ));
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    check_webgpu()?;
    run().unwrap_throw();

    Ok(())
}
