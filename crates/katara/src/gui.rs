use crate::config::CameraConfig;
use crate::debug_watch;
use crate::profiling::Profiler;
use crate::{scene::SceneConfig, scene::SceneStats};
use egui::{Color32, Context, FontFamily, FontId, Frame, Id, Margin, RichText, TextStyle};
use wgpu::{Device, Queue, TextureFormat};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct Gui {
    context: Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    debug_text: DebugText,
}

impl Gui {
    pub fn new(
        device: &Device,
        _queue: &Queue,
        window: &Window,
        surface_format: TextureFormat,
    ) -> Self {
        let context = Context::default();
        apply_editor_style(&context);
        let state = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(
            device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        Self {
            context,
            state,
            renderer,
            debug_text: DebugText::new(),
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) {
        if self.state.on_window_event(window, event).repaint {
            window.request_redraw();
        }
    }

    pub fn wants_mouse(&self) -> bool {
        self.context.egui_wants_pointer_input()
    }

    pub fn wants_keyboard(&self) -> bool {
        self.context.egui_wants_keyboard_input()
    }

    pub fn toggle_debug_text(&mut self) {
        self.debug_text.toggle();
    }

    pub fn render(
        &mut self,
        _dt: instant::Duration,
        device: &Device,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_size: [u32; 2],
        window: &Window,
        camera_settings: &mut CameraConfig,
        scene_config: &mut SceneConfig,
        scene_stats: &SceneStats,
        profiler: &mut Profiler,
    ) -> anyhow::Result<()> {
        let input = self.state.take_egui_input(window);
        let context = self.context.clone();
        let output = context.run_ui(input, |ui| {
            let context = ui.ctx().clone();
            self.debug_text.draw(&context, scene_stats);
            camera_settings.draw(&context);
            scene_config.draw(&context);
            profiler.draw(&context);
        });
        self.state
            .handle_platform_output(window, output.platform_output);

        for (id, image_delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        let pixels_per_point = context.pixels_per_point();
        let paint_jobs = context.tessellate(output.shapes, pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: target_size,
            pixels_per_point,
        };
        let command_buffers =
            self.renderer
                .update_buffers(device, queue, encoder, &paint_jobs, &screen);
        queue.submit(command_buffers);

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            .render(&mut render_pass.forget_lifetime(), &paint_jobs, &screen);

        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        Ok(())
    }
}

fn apply_editor_style(context: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::from_rgba_unmultiplied(15, 15, 15, 240);
    visuals.panel_fill = Color32::from_rgba_unmultiplied(15, 15, 15, 240);
    visuals.faint_bg_color = Color32::from_rgb(25, 25, 25);
    visuals.extreme_bg_color = Color32::from_rgb(10, 10, 10);
    visuals.selection.bg_fill = Color32::from_rgb(66, 100, 150);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 48);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(65, 65, 70);
    visuals.widgets.active.bg_fill = Color32::from_rgb(75, 105, 150);
    visuals.window_corner_radius = egui::CornerRadius::ZERO;
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(235, 235, 235);
    context.set_visuals(visuals);

    let mut style = (*context.global_style()).clone();
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(10.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(10.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(10.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(10.0, FontFamily::Monospace),
        ),
    ]
    .into();
    style.spacing.window_margin = Margin::same(4);
    style.spacing.item_spacing = egui::vec2(4.0, 2.0);
    style.spacing.button_padding = egui::vec2(3.0, 1.0);
    style.spacing.indent = 10.0;
    style.spacing.interact_size = egui::vec2(24.0, 12.0);
    style.spacing.slider_width = 70.0;
    style.spacing.combo_width = 70.0;
    context.set_global_style(style);
}

struct DebugText {
    open: bool,
}

impl DebugText {
    fn new() -> Self {
        Self { open: false }
    }

    fn toggle(&mut self) {
        self.open = !self.open;
    }

    fn draw(&mut self, context: &Context, scene_stats: &SceneStats) {
        let fps = context.input(|input| 1.0 / input.stable_dt.max(f32::EPSILON));
        let mut rows = vec![DebugRow::new(format!("FPS: {fps:.1}"))];

        if self.open {
            if !scene_stats.pipeline.rows.is_empty() {
                rows.push(DebugRow::new(format!("[{}]", scene_stats.pipeline.label)));
                rows.extend(scene_stats.pipeline.rows.iter().cloned().map(DebugRow::new));
            }

            let watches = debug_watch::snapshot();
            let mut current_group: Option<&str> = None;
            for (name, entry) in watches {
                let (group, label) = name.split_once('.').unwrap_or(("misc", name));

                if current_group != Some(group) {
                    current_group = Some(group);
                    rows.push(DebugRow::new(format!("[{group}]")));
                }

                rows.push(DebugRow {
                    text: format!("{label}: {}", entry.value),
                    tooltip: Some(format!(
                        "{}:{}\nlast seen frame: {}",
                        entry.file, entry.line, entry.last_seen_frame
                    )),
                });
            }
        }

        let font = FontId::new(10.0, FontFamily::Proportional);
        let width = context.fonts_mut(|fonts| {
            rows.iter()
                .map(|row| {
                    fonts
                        .layout_no_wrap(row.text.clone(), font.clone(), Color32::WHITE)
                        .size()
                        .x
                })
                .fold(0.0, f32::max)
        }) + 4.0;

        egui::Area::new(Id::new("Debug Text"))
            .fixed_pos(egui::pos2(10.0, 10.0))
            .order(egui::Order::Foreground)
            .show(context, |ui| {
                Frame::NONE.inner_margin(Margin::ZERO).show(ui, |ui| {
                    ui.set_min_width(width);
                    for row in rows {
                        let response = draw_text_with_bg(ui, &row.text);
                        if let Some(tooltip) = row.tooltip {
                            response.on_hover_text(tooltip);
                        }
                    }
                });
            });
    }
}

struct DebugRow {
    text: String,
    tooltip: Option<String>,
}

impl DebugRow {
    fn new(text: String) -> Self {
        Self {
            text,
            tooltip: None,
        }
    }
}

fn draw_text_with_bg(ui: &mut egui::Ui, text: &str) -> egui::Response {
    Frame::NONE
        .fill(Color32::from_black_alpha(240))
        .inner_margin(Margin::symmetric(2, 0))
        .show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(text)).extend())
        })
        .inner
}

pub trait Panel {
    fn draw(&mut self, context: &Context);
}
