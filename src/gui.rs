use crate::camera::Camera;
use wgpu::{Device, Queue, TextureFormat};
use winit::event::Event;
use winit::window::Window;

pub struct Gui {
    context: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
    renderer: imgui_wgpu::Renderer,

    pub debug: DebugPanel,
    pub camera: CameraPanel,
}

impl Gui {
    pub fn new(
        device: &Device,
        queue: &Queue,
        window: &Window,
        surface_format: TextureFormat,
    ) -> Self {
        let mut imgui = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            window,
            imgui_winit_support::HiDpiMode::Default,
        );

        let renderer = imgui_wgpu::Renderer::new(
            &mut imgui,
            device,
            queue,
            imgui_wgpu::RendererConfig {
                texture_format: surface_format,
                ..imgui_wgpu::RendererConfig::new()
            },
        );

        Self {
            context: imgui,
            platform,
            renderer,
            debug: DebugPanel::new(),
            camera: CameraPanel::new()
        }
    }

    pub fn handle_event<T>(&mut self, window: &Window, event: &Event<T>) {
        self.platform
            .handle_event(self.context.io_mut(), window, event);
    }

    pub fn wants_mouse(&self) -> bool {
        let io = self.context.io();
        io.want_capture_mouse
    }

    pub fn wants_keyboard(&self) -> bool {
        let io = self.context.io();
        io.want_capture_keyboard
    }

    pub fn render(
        &mut self,
        dt: instant::Duration,
        device: &Device,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        window: &Window,
        camera: &mut Camera,
    ) -> anyhow::Result<()> {
        self.context.io_mut().update_delta_time(dt);
        self.platform.prepare_frame(self.context.io_mut(), window)?;


        let ui = self.context.frame();

        self.debug.draw(ui);
        self.camera.draw(ui, camera);

        self.platform.prepare_render(ui, window);
        let draw_data = self.context.render();

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Debug GUI Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        self.renderer
            .render(draw_data, queue, device, &mut render_pass)?;

        Ok(())
    }
}

pub struct DebugPanel {
    pub open: bool,
}

impl DebugPanel {
    pub fn new() -> Self {
        Self { open: true }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn draw(&mut self, ui: &imgui::Ui) {
        if !self.open {
            return;
        }

        ui.window("Debug")
            .opened(&mut self.open)
            .size([260.0, 120.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text(format!("FPS: {:.1}", ui.io().framerate));
            });
    }
}

struct CameraPanel {
    pub open: bool,
}

impl CameraPanel {
    pub fn new() -> Self {
        Self { open: true }
    }
    
    pub fn draw(&mut self, ui: &imgui::Ui, camera: &mut Camera) {
        if !self.open {
            return;
        }

        ui.window("Camera")
            .opened(&mut self.open)
            .size([280.0, 145.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.slider("Acceleration", 0.0, 300.0, &mut camera.accel);
                ui.slider("Damping", 0.0, 20.0, &mut camera.damping);
                ui.slider("Mouse Sens", 0.0001, 0.02, &mut camera.mouse_sens);
                ui.slider("Scroll Sens", 0.01, 2.0, &mut camera.scroll_sens);
            });
    }
}
