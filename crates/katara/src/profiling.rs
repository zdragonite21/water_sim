use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use wgpu_profiler::{GpuProfilerQuery, GpuProfilerSettings, GpuTimerQueryResult};

const HISTORY_SIZE: usize = 120;

macro_rules! gpu_profile {
    ($gpu:expr, $recorder:expr, $label:expr, $body:block) => {{
        let query = $gpu.and_then(|gpu| gpu.begin_query($label, $recorder));
        let result = $body;
        if let (Some(gpu), Some(query)) = ($gpu, query) {
            gpu.end_query($recorder, query);
        }
        result
    }};
}
pub(crate) use gpu_profile;

struct GpuRecorder {
    active: AtomicBool,
    profiler: Mutex<wgpu_profiler::GpuProfiler>,
}

#[derive(Clone)]
pub struct GpuFrameRecorder(Arc<GpuRecorder>);

impl GpuFrameRecorder {
    fn new(device: &wgpu::Device) -> Self {
        Self(Arc::new(GpuRecorder {
            active: AtomicBool::new(false),
            profiler: Mutex::new(
                wgpu_profiler::GpuProfiler::new(device, GpuProfilerSettings::default()).unwrap(),
            ),
        }))
    }

    fn set_active(&self, active: bool) {
        self.0.active.store(active, Ordering::Relaxed);
    }

    pub fn begin_query<R: wgpu_profiler::ProfilerCommandRecorder>(
        &self,
        label: impl Into<String>,
        recorder: &mut R,
    ) -> Option<GpuProfilerQuery> {
        self.0
            .active
            .load(Ordering::Relaxed)
            .then(|| self.0.profiler.lock().unwrap().begin_query(label, recorder))
    }

    pub fn end_query<R: wgpu_profiler::ProfilerCommandRecorder>(
        &self,
        recorder: &mut R,
        query: GpuProfilerQuery,
    ) {
        self.0.profiler.lock().unwrap().end_query(recorder, query);
    }
}

#[derive(Default)]
struct GpuSample {
    measurements: HashMap<String, GpuMeasurement>,
    order: Vec<String>,
}

#[derive(Clone, Copy, Default)]
struct GpuMeasurement {
    milliseconds: f64,
    calls: u32,
}

struct TimingRow {
    label: String,
    average_ms: f64,
    maximum_ms: f64,
    calls: f64,
}

pub struct Profiler {
    capturing: bool,
    frame_capturing: bool,
    cpu: profiling::puffin::GlobalFrameView,
    cpu_rows: Vec<TimingRow>,
    gpu: Option<GpuFrameRecorder>,
    gpu_history: VecDeque<GpuSample>,
    gpu_rows: Vec<TimingRow>,
}

impl Profiler {
    pub fn new(device: &wgpu::Device, gpu_supported: bool) -> Self {
        Self {
            capturing: false,
            frame_capturing: false,
            cpu: new_cpu_frame_view(),
            cpu_rows: Vec::new(),
            gpu: gpu_supported.then(|| GpuFrameRecorder::new(device)),
            gpu_history: VecDeque::with_capacity(HISTORY_SIZE),
            gpu_rows: Vec::new(),
        }
    }

    pub fn gpu_recorder(&self) -> Option<GpuFrameRecorder> {
        self.gpu.clone()
    }

    pub fn begin_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if let Some(gpu) = self.gpu.clone() {
            let mut profiler = gpu.0.profiler.lock().unwrap();
            let _ = device.poll(wgpu::PollType::Poll);
            while let Some(results) = profiler.process_finished_frame(queue.get_timestamp_period())
            {
                push_sample(&mut self.gpu_history, gpu_sample(results));
                self.gpu_rows = gpu_rows(&self.gpu_history);
            }
        }

        self.frame_capturing = self.capturing;
        if let Some(gpu) = &self.gpu {
            gpu.set_active(self.frame_capturing);
        }
        profiling::puffin::set_scopes_on(self.frame_capturing);
    }

    pub fn resolve_gpu_queries(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.frame_capturing {
            if let Some(gpu) = &self.gpu {
                gpu.0.profiler.lock().unwrap().resolve_queries(encoder);
            }
        }
    }

    pub fn finish_gpu_frame(&mut self) {
        if self.frame_capturing {
            if let Some(gpu) = &self.gpu {
                if let Err(error) = gpu.0.profiler.lock().unwrap().end_frame() {
                    log::warn!("Could not finish GPU profiler frame: {error}");
                }
            }
        }
    }

    pub fn finish_cpu_frame(&mut self) {
        if self.frame_capturing {
            profiling::finish_frame!();
            self.cpu_rows = cpu_rows(&self.cpu);
        }
        profiling::puffin::set_scopes_on(false);
    }

    pub fn draw(&mut self, ui: &imgui::Ui) {
        let mut capturing = self.capturing;

        ui.window("Profiler")
            .size([380.0, 420.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.checkbox("Capture profiling", &mut capturing);
                ui.text(if capturing { "Capturing" } else { "Frozen" });

                ui.separator();
                if let Some(frame) = frame_row(&self.cpu_rows) {
                    draw_section_header(ui, "CPU", frame);
                    ui.indent();
                    for row in self.cpu_rows.iter().filter(|row| row.label != "Frame") {
                        draw_cpu_timing_row(ui, row);
                    }
                    ui.unindent();
                } else {
                    ui.text("CPU");
                    ui.text("Waiting for results");
                }

                ui.separator();
                if self.gpu.is_none() {
                    ui.text("GPU");
                    ui.text("Unavailable on this device");
                } else if let Some(frame) = frame_row(&self.gpu_rows) {
                    draw_section_header(ui, "GPU", frame);
                    ui.indent();
                    for row in self.gpu_rows.iter().filter(|row| row.label != "Frame") {
                        draw_gpu_timing_row(ui, row, frame.average_ms);
                    }
                    ui.unindent();
                } else {
                    ui.text("GPU");
                    ui.text("Waiting for results");
                }
            });

        if capturing != self.capturing {
            self.set_capturing(capturing);
        }
    }

    fn set_capturing(&mut self, capturing: bool) {
        self.capturing = capturing;
        if capturing {
            self.cpu = new_cpu_frame_view();
            self.cpu_rows.clear();
            self.gpu_history.clear();
            self.gpu_rows.clear();
        }
    }
}

fn gpu_sample(results: Vec<GpuTimerQueryResult>) -> GpuSample {
    fn collect(result: GpuTimerQueryResult, sample: &mut GpuSample) {
        if let Some(time) = result.time {
            if !sample.measurements.contains_key(&result.label) {
                sample.order.push(result.label.clone());
            }
            let measurement = sample.measurements.entry(result.label).or_default();
            measurement.milliseconds += (time.end - time.start) * 1_000.0;
            measurement.calls += 1;
        }
        for child in result.nested_queries {
            collect(child, sample);
        }
    }

    let mut sample = GpuSample::default();
    for result in results {
        collect(result, &mut sample);
    }
    sample
}

fn gpu_rows(history: &VecDeque<GpuSample>) -> Vec<TimingRow> {
    let mut labels = Vec::new();
    for label in history.iter().flat_map(|sample| &sample.order) {
        if !labels.contains(label) {
            labels.push(label.clone());
        }
    }

    labels
        .into_iter()
        .map(|label| {
            let measurements = history
                .iter()
                .map(|sample| sample.measurements.get(&label).copied().unwrap_or_default());
            let average_ms = average(measurements.clone().map(|value| value.milliseconds));
            let maximum_ms = maximum(measurements.clone().map(|value| value.milliseconds));
            let calls = average(measurements.map(|value| value.calls as f64));
            TimingRow {
                label,
                average_ms,
                maximum_ms,
                calls,
            }
        })
        .collect()
}

fn new_cpu_frame_view() -> profiling::puffin::GlobalFrameView {
    let view = profiling::puffin::GlobalFrameView::default();
    {
        let mut frames = view.lock();
        frames.set_max_recent(HISTORY_SIZE);
        frames.set_max_slow(0);
        frames.set_pack_frames(false);
    }
    view
}

fn cpu_rows(view: &profiling::puffin::GlobalFrameView) -> Vec<TimingRow> {
    let view = view.lock();
    let frames: Vec<_> = view
        .latest_frames(HISTORY_SIZE)
        .filter_map(|frame| frame.unpacked().ok())
        .collect();
    if frames.is_empty() {
        return Vec::new();
    }

    let Some(frame_id) = view.scope_collection().fetch_by_name("Frame") else {
        return Vec::new();
    };
    let Some(thread) = frames
        .iter()
        .flat_map(|frame| frame.thread_streams.keys())
        .find(|thread| {
            profiling::puffin::merge_scopes_for_thread(view.scope_collection(), &frames, thread)
                .is_ok_and(|scopes| scopes.iter().any(|scope| scope.id == *frame_id))
        })
    else {
        return Vec::new();
    };

    let scopes =
        profiling::puffin::merge_scopes_for_thread(view.scope_collection(), &frames, thread)
            .unwrap_or_default();
    let mut rows = Vec::new();
    for scope in &scopes {
        collect_cpu_rows(view.scope_collection(), scope, frames.len(), &mut rows);
    }
    rows
}

fn collect_cpu_rows(
    scopes: &profiling::puffin::ScopeCollection,
    scope: &profiling::puffin::MergeScope<'_>,
    frame_count: usize,
    rows: &mut Vec<TimingRow>,
) {
    let label = scopes
        .fetch_by_id(&scope.id)
        .map_or("Unknown scope", |details| details.name());
    if is_cpu_scope(label) {
        rows.push(TimingRow {
            label: label.to_owned(),
            average_ms: scope.duration_per_frame_ns as f64 / 1_000_000.0,
            maximum_ms: scope.max_duration_ns as f64 / 1_000_000.0,
            calls: scope.num_pieces as f64 / frame_count as f64,
        });
    }
    for child in &scope.children {
        collect_cpu_rows(scopes, child, frame_count, rows);
    }
}

fn is_cpu_scope(label: &str) -> bool {
    matches!(
        label,
        "Frame" | "Update and command encoding" | "Submit and present"
    )
}

fn draw_timing_row(ui: &imgui::Ui, row: &TimingRow) {
    ui.text(format!("{}  {:.3} ms", row.label, row.average_ms));
}

fn draw_cpu_timing_row(ui: &imgui::Ui, row: &TimingRow) {
    draw_timing_row(ui, row);
    draw_timing_tooltip(ui, row, None);
}

fn draw_gpu_timing_row(ui: &imgui::Ui, row: &TimingRow, frame_ms: f64) {
    draw_timing_row(ui, row);
    draw_timing_tooltip(ui, row, Some(percentage(row.average_ms, frame_ms)));
}

fn draw_section_header(ui: &imgui::Ui, label: &str, row: &TimingRow) {
    ui.text(format!("{}  {:.3} ms", label, row.average_ms));
    draw_timing_tooltip(ui, row, None);
}

fn draw_timing_tooltip(ui: &imgui::Ui, row: &TimingRow, percent: Option<f64>) {
    if ui.is_item_hovered() {
        ui.tooltip(|| {
            ui.text(format!("Max: {:.3} ms", row.maximum_ms));
            ui.text(format!("Calls/frame: {:.1}", row.calls));
            if let Some(percent) = percent {
                ui.text(format!("{percent:.1}% of GPU frame"));
            }
        });
    }
}

fn frame_row(rows: &[TimingRow]) -> Option<&TimingRow> {
    rows.iter().find(|row| row.label == "Frame")
}

fn percentage(milliseconds: f64, frame_ms: f64) -> f64 {
    if frame_ms > 0.0 {
        milliseconds / frame_ms * 100.0
    } else {
        0.0
    }
}

fn push_sample<T>(history: &mut VecDeque<T>, sample: T) {
    if history.len() == HISTORY_SIZE {
        history.pop_front();
    }
    history.push_back(sample);
}

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0), |(sum, count), value| (sum + value, count + 1));
    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn maximum(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(label: &str, start: f64, end: f64) -> GpuTimerQueryResult {
        GpuTimerQueryResult {
            label: label.to_owned(),
            pid: 0,
            tid: std::thread::current().id(),
            time: Some(start..end),
            nested_queries: Vec::new(),
        }
    }

    #[test]
    fn history_keeps_the_newest_120_samples() {
        let mut history = VecDeque::new();
        for value in 0..125 {
            push_sample(&mut history, value);
        }
        assert_eq!(history.len(), 120);
        assert_eq!(history.front(), Some(&5));
        assert_eq!(history.back(), Some(&124));
    }

    #[test]
    fn new_gpu_labels_appear_automatically() {
        let history = VecDeque::from([gpu_sample(vec![result("New stage", 0.0, 0.002)])]);
        let rows = gpu_rows(&history);
        assert_eq!(rows[0].label, "New stage");
    }

    #[test]
    fn repeated_gpu_labels_are_combined_per_frame() {
        let sample = gpu_sample(vec![
            result("Density", 0.0, 0.002),
            result("Density", 0.002, 0.005),
        ]);
        let density = sample.measurements.get("Density").unwrap();
        assert_eq!(density.milliseconds, 5.0);
        assert_eq!(density.calls, 2);
    }

    #[test]
    fn gpu_rows_keep_pass_order() {
        let history = VecDeque::from([gpu_sample(vec![
            result("Frame", 0.0, 0.010),
            result("Grid key upload", 0.0, 0.001),
            result("Radix sort", 0.001, 0.004),
            result("Density", 0.004, 0.006),
        ])]);
        let labels: Vec<_> = gpu_rows(&history)
            .into_iter()
            .map(|row| row.label)
            .collect();
        assert_eq!(
            labels,
            ["Frame", "Grid key upload", "Radix sort", "Density"]
        );
    }

    #[test]
    fn missing_passes_count_as_zero_for_percentages() {
        let history = VecDeque::from([
            gpu_sample(vec![
                result("Frame", 0.0, 0.010),
                result("Density", 0.0, 0.002),
            ]),
            gpu_sample(vec![result("Frame", 0.0, 0.010)]),
        ]);
        let rows = gpu_rows(&history);
        let frame = frame_row(&rows).unwrap();
        let density = rows.iter().find(|row| row.label == "Density").unwrap();
        assert_eq!(density.average_ms, 1.0);
        assert_eq!(percentage(density.average_ms, frame.average_ms), 10.0);
    }

    #[test]
    fn cpu_scope_filter_keeps_only_the_three_main_scopes() {
        assert!(is_cpu_scope("Frame"));
        assert!(is_cpu_scope("Update and command encoding"));
        assert!(is_cpu_scope("Submit and present"));
        assert!(!is_cpu_scope("CommandEncoder::finish"));
    }
}
