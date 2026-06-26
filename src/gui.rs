use crate::{camera::Camera, config::WaterConfig};
use wgpu::{Device, Queue, TextureFormat};
use winit::event::Event;
use winit::window::Window;

pub struct Gui {
    context: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
    renderer: imgui_wgpu::Renderer,

    debug_text: DebugText,
    camera: CameraPanel,
    water: WaterPanel,
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
            imgui_winit_support::HiDpiMode::Rounded,
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
            debug_text: DebugText::new(),
            camera: CameraPanel::new(),
            water: WaterPanel::new(),
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

    pub fn toggle_debug_text(&mut self) {
        self.debug_text.toggle();
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
        water: &mut WaterConfig,
    ) -> anyhow::Result<()> {
        self.context.io_mut().update_delta_time(dt);
        self.platform.prepare_frame(self.context.io_mut(), window)?;

        let ui = self.context.frame();

        self.debug_text.draw(ui);
        self.camera.draw(ui, camera);
        self.water.draw(ui, water);

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

struct DebugText {
    open: bool,
}

impl DebugText {
    fn new() -> Self {
        Self { open: true }
    }

    fn toggle(&mut self) {
        self.open = !self.open;
    }

    fn draw(&mut self, ui: &imgui::Ui) {
        if !self.open {
            return;
        }

        let _window_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.0, 0.0, 0.0, 0.0]);
        let _border = ui.push_style_color(imgui::StyleColor::Border, [0.0, 0.0, 0.0, 0.0]);
        let _padding = ui.push_style_var(imgui::StyleVar::WindowPadding([0.0, 0.0]));
        let _border_size = ui.push_style_var(imgui::StyleVar::WindowBorderSize(0.0));

        ui.window("Debug Text")
            .position([10.0, 10.0], imgui::Condition::Always)
            .no_decoration()
            .no_inputs()
            .always_auto_resize(true)
            .draw_background(false)
            .save_settings(false)
            .build(|| {
                draw_text_with_bg(ui, &format!("FPS: {:.1}", ui.io().framerate));
            });
    }
}

fn draw_text_with_bg(ui: &imgui::Ui, text: &str) {
    let text_size = ui.calc_text_size(text);
    let cursor_pos = ui.cursor_screen_pos();
    let padding = [4.0, 0.0];
    let background_max = [
        cursor_pos[0] + text_size[0] + padding[0],
        cursor_pos[1] + text_size[1] + padding[1],
    ];

    ui.get_window_draw_list()
        .add_rect(
            cursor_pos,
            background_max,
            imgui::ImColor32::from_rgba(0, 0, 0, 127),
        )
        .filled(true)
        .build();

    ui.text(text);
}

pub struct CameraPanel {
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
                F32Field::new("Acceleration", 0.0, 300.0).draw(ui, &mut camera.settings.accel);
                F32Field::new("Damping", 0.0, 20.0).draw(ui, &mut camera.settings.damping);
                F32Field::new("Mouse Sens", 0.0001, 0.02).draw(ui, &mut camera.settings.mouse_sens);
                F32Field::new("Scroll Sens", 0.01, 2.0).draw(ui, &mut camera.settings.scroll_sens);
            });
    }
}

pub struct WaterPanel {
    pub open: bool,
}

impl WaterPanel {
    pub fn new() -> Self {
        Self { open: true }
    }

    pub fn draw(&mut self, ui: &imgui::Ui, water: &mut WaterConfig) {
        if !self.open {
            return;
        }

        ui.window("Water")
            .opened(&mut self.open)
            .size([280.0, 95.0], imgui::Condition::FirstUseEver)
            .build(|| {
                F32Field::new("Gravity", 0.0, 30.0).draw(ui, &mut water.gravity);
                F32Field::new("Particle Size", 0.05, 5.0).draw(ui, &mut water.particle_size);
            });
    }
}

struct F32Field {
    label: &'static str,
    min: f32,
    max: f32,
}

impl F32Field {
    fn new(label: &'static str, min: f32, max: f32) -> Self {
        Self { label, min, max }
    }

    fn draw(&self, ui: &imgui::Ui, value: &mut f32) {
        ui.slider(self.label, self.min, self.max, value);
    }
}
