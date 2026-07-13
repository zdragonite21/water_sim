use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use wgpu_profiler::{GpuProfilerQuery, GpuProfilerSettings, GpuTimerQueryResult};

const HISTORY_SIZE: usize = 120;
const GPU_FRAME: &str = "GPU Frame Time";

/// Profiles one encoder/pass expression when GPU capture is active.
///
/// Use slash-separated labels to place a new measurement in the GUI tree:
/// `gpu_profile!(gpu, encoder, "GPU Frame Time/Simulation/My pass", { ... })`.
macro_rules! gpu_profile {
    ($gpu:expr, $recorder:expr, $label:expr, $body:block) => {{
        let query = $gpu.map(|gpu| gpu.begin_query($label, $recorder));
        let result = $body;
        if let (Some(gpu), Some(query)) = ($gpu, query) {
            gpu.end_query($recorder, query);
        }
        result
    }};
}
pub(crate) use gpu_profile;

/// A cheap per-frame handle to the crate-owned GPU profiler.
///
/// This is intentionally shared rather than threaded mutably through every
/// render function. `wgpu-profiler` owns query allocation and readback state.
#[derive(Clone)]
pub struct GpuFrameRecorder(Arc<Mutex<wgpu_profiler::GpuProfiler>>);

impl GpuFrameRecorder {
    pub fn begin_query<R: wgpu_profiler::ProfilerCommandRecorder>(
        &self,
        label: impl Into<String>,
        recorder: &mut R,
    ) -> GpuProfilerQuery {
        self.0.lock().unwrap().begin_query(label, recorder)
    }

    pub fn end_query<R: wgpu_profiler::ProfilerCommandRecorder>(
        &self,
        recorder: &mut R,
        query: GpuProfilerQuery,
    ) {
        self.0.lock().unwrap().end_query(recorder, query);
    }
}

#[derive(Clone, Copy, Default)]
struct GpuMeasurement {
    milliseconds: f64,
    calls: u32,
}

type GpuSample = HashMap<String, GpuMeasurement>;

pub struct TimingRow {
    pub label: String,
    pub average_ms: f64,
    pub maximum_ms: f64,
    pub percent: f64,
    pub calls: f64,
}

pub struct TimingNode {
    pub row: TimingRow,
    pub children: Vec<TimingNode>,
}

pub struct ProfileStats {
    pub cpu: Vec<TimingNode>,
    pub gpu: Vec<TimingNode>,
    pub cpu_samples: usize,
    pub gpu_samples: usize,
}

pub struct Profiler {
    capturing: bool,
    frame_capturing: bool,
    cpu: profiling::puffin::GlobalFrameView,
    gpu: Option<Arc<Mutex<wgpu_profiler::GpuProfiler>>>,
    gpu_history: VecDeque<GpuSample>,
}

impl Profiler {
    pub fn new(device: &wgpu::Device, gpu_supported: bool) -> Self {
        let gpu = gpu_supported.then(|| {
            Arc::new(Mutex::new(
                wgpu_profiler::GpuProfiler::new(device, GpuProfilerSettings::default()).unwrap(),
            ))
        });

        Self {
            capturing: false,
            frame_capturing: false,
            cpu: new_cpu_frame_view(),
            gpu,
            gpu_history: VecDeque::with_capacity(HISTORY_SIZE),
        }
    }

    pub fn begin_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<GpuFrameRecorder> {
        if let Some(gpu) = &self.gpu {
            let mut gpu = gpu.lock().unwrap();
            let _ = device.poll(wgpu::PollType::Poll);
            while let Some(results) = gpu.process_finished_frame(queue.get_timestamp_period()) {
                if self.capturing {
                    push_sample(&mut self.gpu_history, gpu_sample(results));
                }
            }
        }

        self.frame_capturing = self.capturing;
        profiling::puffin::set_scopes_on(self.frame_capturing);
        self.frame_capturing
            .then(|| self.gpu.as_ref().map(|gpu| GpuFrameRecorder(gpu.clone())))
            .flatten()
    }

    /// Adds the profiler's query resolve copies to this frame's encoder.
    pub fn resolve_gpu_queries(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.frame_capturing {
            if let Some(gpu) = &self.gpu {
                gpu.lock().unwrap().resolve_queries(encoder);
            }
        }
    }

    /// Must run after the command buffer containing the query resolves is submitted.
    pub fn finish_gpu_frame(&mut self) {
        if self.frame_capturing {
            if let Some(gpu) = &self.gpu {
                if let Err(error) = gpu.lock().unwrap().end_frame() {
                    log::warn!("Could not finish GPU profiler frame: {error}");
                }
            }
        }
    }

    pub fn finish_cpu_frame(&self) {
        if self.frame_capturing {
            profiling::finish_frame!();
        }
        profiling::puffin::set_scopes_on(false);
    }

    pub fn draw(&mut self, ui: &imgui::Ui) {
        let mut capturing = self.capturing;
        let stats = self.stats();
        let gpu_supported = self.gpu.is_some();

        ui.window("Profiler")
            .size([430.0, 470.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.checkbox("Capture profiling", &mut capturing);
                ui.text(if capturing { "Capturing" } else { "Frozen" });
                ui.same_line();
                ui.text(format!("{} frame history", HISTORY_SIZE));

                ui.separator();
                if stats.cpu_samples == 0 {
                    ui.text("CPU Frame Time");
                    ui.text("Waiting for CPU results");
                } else {
                    for node in &stats.cpu {
                        draw_timing_node(ui, node);
                    }
                }

                ui.separator();
                if !gpu_supported {
                    ui.text(GPU_FRAME);
                    ui.text("Unavailable on this device");
                } else if stats.gpu_samples == 0 {
                    ui.text(GPU_FRAME);
                    ui.text("Waiting for GPU results");
                } else {
                    for node in &stats.gpu {
                        draw_timing_node(ui, node);
                    }
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
            self.gpu_history.clear();
        }
    }

    fn stats(&self) -> ProfileStats {
        let (cpu, cpu_samples) = self.cpu_stats();
        ProfileStats {
            cpu,
            gpu: gpu_stats(&self.gpu_history),
            cpu_samples,
            gpu_samples: self.gpu_history.len(),
        }
    }

    fn cpu_stats(&self) -> (Vec<TimingNode>, usize) {
        let view = self.cpu.lock();
        let frames: Vec<_> = view
            .latest_frames(HISTORY_SIZE)
            .filter_map(|frame| frame.unpacked().ok())
            .collect();
        let sample_count = frames.len();
        if frames.is_empty() {
            return (Vec::new(), 0);
        }

        let Some(root_id) = view.scope_collection().fetch_by_name("CPU Frame Time") else {
            return (Vec::new(), sample_count);
        };
        let Some(thread) = frames
            .iter()
            .flat_map(|frame| frame.thread_streams.keys())
            .find(|thread| {
                profiling::puffin::merge_scopes_for_thread(view.scope_collection(), &frames, thread)
                    .is_ok_and(|scopes| scopes.iter().any(|scope| scope.id == *root_id))
            })
        else {
            return (Vec::new(), sample_count);
        };

        let scopes =
            profiling::puffin::merge_scopes_for_thread(view.scope_collection(), &frames, thread)
                .unwrap_or_default();
        let root_ns = scopes
            .iter()
            .find(|scope| scope.id == *root_id)
            .map_or(0, |scope| scope.duration_per_frame_ns);
        let nodes = scopes
            .iter()
            .map(|scope| timing_node(view.scope_collection(), scope, root_ns, sample_count))
            .collect();
        (nodes, sample_count)
    }
}

fn gpu_sample(results: Vec<GpuTimerQueryResult>) -> GpuSample {
    fn collect(result: GpuTimerQueryResult, sample: &mut GpuSample) {
        if let Some(time) = result.time {
            let measurement = sample.entry(result.label).or_default();
            measurement.milliseconds += (time.end - time.start) * 1_000.0;
            measurement.calls += 1;
        }
        for child in result.nested_queries {
            collect(child, sample);
        }
    }

    let mut sample = GpuSample::new();
    for result in results {
        collect(result, &mut sample);
    }
    sample
}

#[derive(Default)]
struct GpuTreeNode {
    full_label: String,
    children: BTreeMap<String, GpuTreeNode>,
}

fn gpu_stats(history: &VecDeque<GpuSample>) -> Vec<TimingNode> {
    let mut root = GpuTreeNode::default();
    for label in history.iter().flat_map(|sample| sample.keys()) {
        let mut node = &mut root;
        let mut path = String::new();
        for segment in label.split('/') {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(segment);
            node = node.children.entry(segment.to_owned()).or_default();
            node.full_label.clone_from(&path);
        }
    }

    let root_average = average(
        history
            .iter()
            .filter_map(|sample| sample.get(GPU_FRAME).map(|value| value.milliseconds)),
    );
    let mut nodes: Vec<_> = root
        .children
        .into_values()
        .map(|node| gpu_timing_node(node, history, root_average))
        .collect();
    sort_gpu_nodes(&mut nodes);
    nodes
}

fn gpu_timing_node(
    node: GpuTreeNode,
    history: &VecDeque<GpuSample>,
    root_average: f64,
) -> TimingNode {
    let measurements = history
        .iter()
        .filter_map(|sample| sample.get(&node.full_label));
    let average_ms = average(measurements.clone().map(|value| value.milliseconds));
    let mut children: Vec<_> = node
        .children
        .into_values()
        .map(|child| gpu_timing_node(child, history, root_average))
        .collect();
    sort_gpu_nodes(&mut children);

    TimingNode {
        row: TimingRow {
            label: node
                .full_label
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned(),
            average_ms,
            maximum_ms: maximum(measurements.clone().map(|value| value.milliseconds)),
            percent: if root_average > 0.0 {
                average_ms / root_average * 100.0
            } else {
                0.0
            },
            calls: average(measurements.map(|value| value.calls as f64)),
        },
        children,
    }
}

fn sort_gpu_nodes(nodes: &mut [TimingNode]) {
    fn order(label: &str) -> usize {
        match label {
            "GPU Frame Time" => 0,
            "Simulation" => 1,
            "Grid key upload" => 2,
            "Radix sort" => 3,
            "Grid start indices" => 4,
            "Density" => 5,
            "Pressure and viscosity" => 6,
            "Collision and integration" => 7,
            "Particle render" => 8,
            _ => 9,
        }
    }
    nodes.sort_by(|left, right| {
        order(&left.row.label)
            .cmp(&order(&right.row.label))
            .then_with(|| left.row.label.cmp(&right.row.label))
    });
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

fn timing_node(
    scopes: &profiling::puffin::ScopeCollection,
    scope: &profiling::puffin::MergeScope<'_>,
    root_ns: i64,
    frame_count: usize,
) -> TimingNode {
    let average_ms = scope.duration_per_frame_ns as f64 / 1_000_000.0;
    TimingNode {
        row: TimingRow {
            label: scopes
                .fetch_by_id(&scope.id)
                .map_or("Unknown scope", |details| details.name())
                .to_owned(),
            average_ms,
            maximum_ms: scope.max_duration_ns as f64 / 1_000_000.0,
            percent: if root_ns > 0 {
                scope.duration_per_frame_ns as f64 / root_ns as f64 * 100.0
            } else {
                0.0
            },
            calls: scope.num_pieces as f64 / frame_count as f64,
        },
        children: scope
            .children
            .iter()
            .map(|child| timing_node(scopes, child, root_ns, frame_count))
            .collect(),
    }
}

fn draw_timing_node(ui: &imgui::Ui, node: &TimingNode) {
    if node.children.is_empty() {
        ui.bullet_text(format!("{}  {:.3}", node.row.label, node.row.average_ms));
        draw_timing_tooltip(ui, &node.row);
    } else {
        let token = ui
            .tree_node_config(node.row.label.as_str())
            .label::<String, String>(format!("{}  {:.3}", node.row.label, node.row.average_ms))
            .default_open(true)
            .push();
        draw_timing_tooltip(ui, &node.row);
        if let Some(_token) = token {
            for child in &node.children {
                draw_timing_node(ui, child);
            }
        }
    }
}

fn draw_timing_tooltip(ui: &imgui::Ui, row: &TimingRow) {
    if ui.is_item_hovered() {
        ui.tooltip(|| {
            ui.text(format!(
                "Units: ms\nAverage: {:.3}\nMaximum: {:.3}\nPercentage: {:.1}%\nCalls: {:.1}",
                row.average_ms, row.maximum_ms, row.percent, row.calls
            ));
        });
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
    fn gpu_labels_automatically_build_the_gui_tree() {
        let mut history = VecDeque::new();
        history.push_back(HashMap::from([
            (
                GPU_FRAME.to_owned(),
                GpuMeasurement {
                    milliseconds: 10.0,
                    calls: 1,
                },
            ),
            (
                "GPU Frame Time/Simulation/New pass".to_owned(),
                GpuMeasurement {
                    milliseconds: 2.0,
                    calls: 3,
                },
            ),
        ]));

        let stats = gpu_stats(&history);
        assert_eq!(stats[0].row.label, GPU_FRAME);
        assert_eq!(stats[0].children[0].row.label, "Simulation");
        assert_eq!(stats[0].children[0].children[0].row.label, "New pass");
    }

    #[test]
    fn cpu_frame_view_uses_the_gui_history_size() {
        let view = new_cpu_frame_view();
        let frames = view.lock();
        assert_eq!(frames.max_recent(), HISTORY_SIZE);
        assert_eq!(frames.max_slow(), 0);
    }
}
