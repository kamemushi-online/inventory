//! Opt-in native lifecycle/render soak. Actions call the same application methods
//! as controls, but are scripted state changes, not OS pointer/keyboard input.
//! No raw buffers or FFT vectors are retained by the validation recorder.

use crate::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Weak;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    plot_config_path: String,
    output_jsonl: String,
    #[serde(default = "default_duration")]
    duration_seconds: f64,
    #[serde(default = "default_cycles")]
    cycles: usize,
    #[serde(default = "default_quiet")]
    final_quiet_seconds: f64,
    #[serde(default = "default_close")]
    close_when_done: bool,
}

fn default_duration() -> f64 {
    3600.0
}
fn default_cycles() -> usize {
    30
}
fn default_quiet() -> f64 {
    120.0
}
fn default_close() -> bool {
    false
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        if self.plot_config_path.is_empty()
            || self.output_jsonl.is_empty()
            || !self.duration_seconds.is_finite()
            || self.duration_seconds < 10.0
            || !(1..=10000).contains(&self.cycles)
            || self.duration_seconds / (self.cycles as f64) < 5.0
            || !self.final_quiet_seconds.is_finite()
            || self.final_quiet_seconds < 0.0
        {
            return Err("need paths, finite duration >=10s, 1..10000 cycles with >=5s per cycle, and finite nonnegative quiet duration".into());
        }
        Ok(())
    }
}

pub(crate) fn run(path: &str) -> Result<(), String> {
    let config: Config = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    config.validate()?;
    // Never truncate previous evidence, including when a launch is retried.
    let output = open_output(&config.output_jsonl)?;
    let outcome = Arc::new(AtomicU8::new(0));
    let result = Arc::clone(&outcome);
    eframe::run_native(
        "PanelY native soak validation",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            install_system_cjk_font_fallback(&cc.egui_ctx);
            Ok(Box::new(NativeSoakApp::new(config, output, result)))
        }),
    )
    .map_err(|e| e.to_string())?;
    match outcome.load(Ordering::Relaxed) {
        1 => Ok(()),
        2 => Err("a validation assertion failed; see JSONL".into()),
        _ => Err("window exited before validation completed".into()),
    }
}

fn open_output(path: &str) -> Result<File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Start,
    Loading,
    Statistics,
    Fft,
    Active,
    Removing,
    Empty,
    Restoring,
    RestoredHold,
    FinalRemoving,
    FinalEmpty,
    Done,
    Failed,
}

struct NativeSoakApp {
    app: PanelYApp,
    config: Config,
    output: File,
    outcome: Arc<AtomicU8>,
    started: Instant,
    session_started: Option<Instant>,
    stage_started: Instant,
    stage: Stage,
    completed: usize,
    frames: u64,
    last_tick_frame: Option<u64>,
    next_heartbeat: Instant,
    cpu_frames: u64,
    cpu_sum_ms: f64,
    cpu_max_ms: f64,
    next_navigation: Instant,
    navigation_steps: u64,
    layout: Option<Value>,
    statistics: Option<Value>,
    fft: Option<Value>,
    preset_range: Option<(f64, f64)>,
    after_restore: bool,
    restore_kind: &'static str,
    weak_times: Vec<Weak<Vec<f64>>>,
    weak_values: Vec<Weak<Vec<f32>>>,
}

impl NativeSoakApp {
    fn new(config: Config, output: File, outcome: Arc<AtomicU8>) -> Self {
        let now = Instant::now();
        Self {
            app: PanelYApp::default(),
            config,
            output,
            outcome,
            started: now,
            session_started: None,
            stage_started: now,
            stage: Stage::Start,
            completed: 0,
            frames: 0,
            last_tick_frame: None,
            next_heartbeat: now + Duration::from_secs(30),
            cpu_frames: 0,
            cpu_sum_ms: 0.0,
            cpu_max_ms: 0.0,
            next_navigation: now,
            navigation_steps: 0,
            layout: None,
            statistics: None,
            fft: None,
            preset_range: None,
            after_restore: false,
            restore_kind: "initial",
            weak_times: Vec::new(),
            weak_values: Vec::new(),
        }
    }

    fn elapsed(&self) -> f64 {
        self.session_started
            .map_or(0.0, |s| s.elapsed().as_secs_f64())
    }
    fn transition(&mut self, stage: Stage) {
        self.stage = stage;
        self.stage_started = Instant::now();
    }

    fn record(&mut self, event: &str, detail: Value) -> Result<(), String> {
        let c = self.app.workspace.cache_stats();
        let row = json!({"event":event, "elapsed_seconds":self.started.elapsed().as_secs_f64(),
            "timestamp_unix_seconds":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d|d.as_secs_f64()).ok(),
            "session_elapsed_seconds":self.elapsed(), "completed_cycles":self.completed,
            "cycle":if event=="cycle_complete" {self.completed} else {(self.completed+1).min(self.config.cycles)},
            "pid":std::process::id(), "native_frames":self.frames, "rss_mib":process_rss_mib(),
            "cache":{"sources":c.source_count,"loaded_channels":c.loaded_channel_count,
                "raw_bytes":c.total_raw_memory_bytes(),"envelopes":c.envelope_count,
                "line_tiles":c.line_tile_cache_count,"line_tile_bytes":c.line_tile_memory_bytes,
                "step_edges":c.step_edge_cache_count,"step_edge_bytes":c.step_edge_memory_bytes,
                "step_fallback_hints":c.step_fallback_hint_count},
            "ui_cpu":{"ui_passes":self.cpu_frames,"mean_ms":if self.cpu_frames>0 {self.cpu_sum_ms/self.cpu_frames as f64}else{0.0},"max_ms":self.cpu_max_ms},
            "navigation_steps":self.navigation_steps,"detail":detail});
        serde_json::to_writer(&mut self.output, &row).map_err(|e| e.to_string())?;
        self.output
            .write_all(b"\n")
            .and_then(|_| self.output.flush())
            .map_err(|e| e.to_string())?;
        self.cpu_frames = 0;
        self.cpu_sum_ms = 0.0;
        self.cpu_max_ms = 0.0;
        Ok(())
    }

    fn verify_loaded(&self) -> Result<(), String> {
        if let Some(e) = &self.app.load.error {
            return Err(format!("load error: {e}"));
        }
        let c = self.app.workspace.cache_stats();
        if c.source_count != 3
            || c.loaded_channel_count != 12
            || self.app.view.rows.len() != 3
            || self
                .app
                .view
                .rows
                .iter()
                .map(|r| r.channels.len())
                .sum::<usize>()
                != 12
        {
            return Err(format!("expected 3 sources / 3 rows / 12 channels: {c:?}"));
        }
        if let Some(layout) = &self.layout {
            if *layout != layout_snapshot(&self.app) {
                return Err("restored layout/styles/source paths differ".into());
            }
        }
        Ok(())
    }

    fn begin_analysis(&mut self) {
        self.app.analysis.clear_cursors();
        self.app.analysis.set_cursor_mode(true);
        self.app.analysis.place_cursor(0.04);
        self.app.analysis.place_cursor(0.09);
        self.app.analysis.info_panel_visible = true;
        self.app.analysis.info_panel_collapsed = false;
        self.app.analysis.pane = AnalysisPane::Statistics;
        self.transition(Stage::Statistics);
    }

    fn capture_weak(&mut self) {
        self.weak_times.clear();
        self.weak_values.clear();
        for source in self.app.workspace.sources.values() {
            if let Some(time) = &source.shared_time {
                self.weak_times.push(Arc::downgrade(&time.time));
            }
            for channel in source.loaded_channels.raw_by_channel.values() {
                self.weak_values.push(Arc::downgrade(&channel.values));
            }
        }
    }

    fn empty_ready(&self) -> bool {
        !self.app.load.is_busy()
            && !self.app.analysis.fft.is_running()
            && !self.app.analysis.interval_stats_worker.has_running_job()
            && self.weak_times.iter().all(|w| w.strong_count() == 0)
            && self.weak_values.iter().all(|w| w.strong_count() == 0)
    }

    fn verify_empty(&self) -> Result<(), String> {
        let c = self.app.workspace.cache_stats();
        if c != WorkspaceCacheStats::default()
            || !self.empty_ready()
            || self.app.analysis.cursor_a.is_some()
            || self.app.analysis.cursor_b.is_some()
            || !self.app.analysis.interval_stats_cache.stats.is_empty()
            || !self.app.analysis.fft.results.is_empty()
        {
            return Err(format!(
                "empty workspace retains cache/analysis/buffers: {c:?}"
            ));
        }
        Ok(())
    }

    fn navigate(&mut self) {
        if Instant::now() < self.next_navigation {
            return;
        }
        self.next_navigation = Instant::now() + Duration::from_millis(250);
        let Some(full) = self.app.workspace.display_time_range() else {
            return;
        };
        let min_span = self.app.workspace.display_min_view_span().unwrap_or(1e-6);
        let range = self.app.view.x_range.unwrap_or(full);
        let step = self.navigation_steps % 40;
        let next = if step == 0 {
            full
        } else if step < 10 {
            zoom_view_range(range, full, 0.4, 0.85, min_span)
        } else if step < 25 {
            pan_view_range(range, full, (full.1 - full.0) * 0.025, min_span)
        } else {
            zoom_view_range(range, full, 0.6, 1.12, min_span)
        };
        self.app.view.x_range = Some(next);
        self.app.view.hover_x = Some(next.0 + (next.1 - next.0) * ((step % 10) as f64 / 10.0));
        self.app.analysis.pane = match (self.navigation_steps / 8) % 3 {
            0 => AnalysisPane::Statistics,
            1 => AnalysisPane::Fft,
            _ => AnalysisPane::Split,
        };
        self.navigation_steps += 1;
    }

    fn tick(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if Instant::now() >= self.next_heartbeat
            && !matches!(self.stage, Stage::Done | Stage::Failed)
        {
            self.next_heartbeat = Instant::now() + Duration::from_secs(30);
            self.record("heartbeat", json!({"stage":format!("{:?}",self.stage)}))?;
        }
        if matches!(
            self.stage,
            Stage::Loading
                | Stage::Statistics
                | Stage::Fft
                | Stage::Removing
                | Stage::Empty
                | Stage::Restoring
                | Stage::FinalRemoving
        ) && self.stage_started.elapsed() > Duration::from_secs(60)
        {
            return Err(format!("timeout in {:?}", self.stage));
        }
        match self.stage {
            Stage::Start => {
                self.record("initialization", json!({"duration_seconds":self.config.duration_seconds,"cycles":self.config.cycles,
                    "final_quiet_seconds":self.config.final_quiet_seconds,"plot_config_path":self.config.plot_config_path,
                    "malloc_large_cache":std::env::var("MallocLargeCache").ok(),
                    "action_mode":"scripted application methods with native eframe rendering; not OS input",
                    "numeric_comparison":"exact deterministic statistics and FNV-1a hashes over all FFT f64 bins/amplitudes"}))?;
                self.app.plot_config_path = self.config.plot_config_path.clone();
                self.app.load_plot_config(ctx);
                self.transition(Stage::Loading);
            }
            Stage::Loading if !self.app.load.is_busy() => {
                self.verify_loaded()?;
                self.layout = Some(layout_snapshot(&self.app));
                self.session_started = Some(Instant::now());
                self.app.view.x_range = Some((0.025, 0.125));
                self.preset_range = self.app.view.x_range;
                self.app.save_session_preset();
                if self.app.session_preset.is_none() {
                    return Err("session preset was not saved".into());
                }
                self.record(
                    "loaded",
                    json!({"layout":self.layout,"preset_x_range":self.preset_range}),
                )?;
                self.begin_analysis();
            }
            Stage::Statistics => {
                if let Some(error) = &self.app.analysis.interval_stats_worker.error {
                    return Err(error.clone());
                }
                let expected = interval_stats_cache_key(
                    &self.app.analysis,
                    &self.app.workspace,
                    &self.app.view,
                );
                if expected.is_some()
                    && self.app.analysis.interval_stats_cache.key == expected
                    && !self.app.analysis.interval_stats_worker.has_running_job()
                {
                    let snapshot = statistics_snapshot(&self.app)?;
                    if snapshot.as_array().map_or(0, Vec::len) != 12 {
                        return Err("expected 12 statistics rows".into());
                    }
                    check_snapshot(&mut self.statistics, &snapshot, "statistics")?;
                    self.record(
                        "statistics",
                        json!({"values":snapshot,"after_restore":self.after_restore}),
                    )?;
                    self.app.analysis.fft.enabled = true;
                    self.app.analysis.fft.section_collapsed = false;
                    self.app.analysis.fft.target_initialized = true;
                    self.app.analysis.fft.target_channels = self
                        .app
                        .workspace
                        .source_order
                        .iter()
                        .map(|id| ChannelKey::new(*id, "Ia_phys".to_owned()))
                        .collect();
                    self.app.analysis.fft.freq_range.max_hz = Some(200.0);
                    self.app.analysis.pane = AnalysisPane::Fft;
                    self.app.start_fft_jobs(ctx);
                    self.transition(Stage::Fft);
                }
            }
            Stage::Fft if !self.app.analysis.fft.is_running() => {
                if self.app.analysis.fft.error.is_some()
                    || !self.app.analysis.fft.source_errors.is_empty()
                    || self.app.analysis.fft.results.len() != 3
                {
                    return Err(format!(
                        "FFT failed: {:?} {:?}",
                        self.app.analysis.fft.error, self.app.analysis.fft.source_errors
                    ));
                }
                for result in self.app.analysis.fft.results.values() {
                    if !result.dt.is_finite()
                        || result.dt <= 0.0
                        || result.frequency_hz.is_empty()
                        || result.frequency_hz.iter().any(|v| !v.is_finite())
                        || result.spectra.len() != 1
                        || result.spectra.iter().any(|s| {
                            s.channel_path != "Ia_phys"
                                || s.amplitude.len() != result.frequency_hz.len()
                                || s.amplitude.iter().any(|v| !v.is_finite())
                                || !s.peak_freq_hz.is_some_and(f64::is_finite)
                                || !s.peak_amp.is_some_and(f64::is_finite)
                        })
                    {
                        return Err("FFT has invalid bins, amplitudes, or target channels".into());
                    }
                }
                let snapshot = fft_snapshot(&self.app);
                check_snapshot(&mut self.fft, &snapshot, "FFT")?;
                self.record(
                    "fft",
                    json!({"values":snapshot,"after_restore":self.after_restore}),
                )?;
                self.transition(if self.after_restore {
                    Stage::RestoredHold
                } else {
                    Stage::Active
                });
            }
            Stage::Active => {
                self.navigate();
                let period = self.config.duration_seconds / self.config.cycles as f64;
                let deadline = (self.completed + 1) as f64 * period;
                if self.elapsed() >= deadline - (period * 0.25).min(10.0) {
                    self.capture_weak();
                    self.record("before_remove",json!({"weak_time_buffers":self.weak_times.len(),"weak_channel_buffers":self.weak_values.len()}))?;
                    self.transition(Stage::Removing);
                }
            }
            Stage::Removing | Stage::FinalRemoving => {
                if let Some(id) = self.app.workspace.source_order.first().copied() {
                    self.app.remove_source_from_ui(id);
                } else {
                    self.transition(if self.stage == Stage::FinalRemoving {
                        Stage::FinalEmpty
                    } else {
                        Stage::Empty
                    });
                }
            }
            Stage::Empty
                if self.stage_started.elapsed().as_secs_f64() >= 2.0 && self.empty_ready() =>
            {
                self.verify_empty()?;
                self.record("empty",json!({"weak_time_alive":0,"weak_channel_alive":0,"settled_seconds":self.stage_started.elapsed().as_secs_f64()}))?;
                self.restore_kind = if self.completed % 2 == 0 {
                    "session_preset"
                } else {
                    "plot_config"
                };
                if self.restore_kind == "session_preset" {
                    self.app.apply_session_preset(ctx);
                } else {
                    self.app.plot_config_path = self.config.plot_config_path.clone();
                    self.app.load_plot_config(ctx);
                }
                self.transition(Stage::Restoring);
            }
            Stage::Restoring if !self.app.load.is_busy() => {
                self.verify_loaded()?;
                let expected = if self.restore_kind == "session_preset" {
                    self.preset_range
                } else {
                    self.app.workspace.display_time_range()
                };
                if self.app.view.x_range != expected {
                    return Err(format!(
                        "{} X range differs: {:?} != {:?}",
                        self.restore_kind, self.app.view.x_range, expected
                    ));
                }
                if self.app.analysis.cursor_a.is_some()
                    || self.app.analysis.cursor_b.is_some()
                    || !self.app.analysis.fft.results.is_empty()
                {
                    return Err("restoration did not clear analysis".into());
                }
                self.record("restored",json!({"kind":self.restore_kind,"x_range":self.app.view.x_range,"layout_matches":true,"analysis_reset":true}))?;
                self.after_restore = true;
                self.begin_analysis();
            }
            Stage::RestoredHold => {
                self.navigate();
                if self.elapsed()
                    >= (self.completed + 1) as f64 * self.config.duration_seconds
                        / self.config.cycles as f64
                {
                    self.completed += 1;
                    self.record("cycle_complete", json!({"restore_kind":self.restore_kind}))?;
                    if self.completed == self.config.cycles {
                        self.capture_weak();
                        self.transition(Stage::FinalRemoving);
                    } else {
                        self.transition(Stage::Active);
                    }
                }
            }
            Stage::FinalEmpty => {
                if !self.empty_ready() {
                    if self.stage_started.elapsed() > Duration::from_secs(60) {
                        return Err("final buffer release timeout".into());
                    }
                } else {
                    self.verify_empty()?;
                    if self.stage_started.elapsed().as_secs_f64() >= 2.0
                        && !self.weak_times.is_empty()
                    {
                        self.record(
                            "final_empty",
                            json!({"weak_time_alive":0,"weak_channel_alive":0}),
                        )?;
                        self.weak_times.clear();
                        self.weak_values.clear();
                        // Start the final quiet period only after release has been verified.
                        self.stage_started = Instant::now();
                    }
                    if self.weak_times.is_empty()
                        && self.stage_started.elapsed().as_secs_f64()
                            >= self.config.final_quiet_seconds
                    {
                        self.record("complete",json!({"cycles":self.completed,"all_numeric_comparisons_passed":true,
                            "quiet_seconds":self.stage_started.elapsed().as_secs_f64(),"empty_cache_and_raw_lifetime_passed":true}))?;
                        self.outcome.store(1, Ordering::Relaxed);
                        self.transition(Stage::Done);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl eframe::App for NativeSoakApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let native_frame = ui.ctx().cumulative_frame_nr();
        self.frames = native_frame + 1;
        // egui may repeat a sizing pass in one native frame. Do not execute an
        // action twice, or remove multiple sources without an intervening frame.
        if self.last_tick_frame != Some(native_frame) {
            self.last_tick_frame = Some(native_frame);
            if let Err(error) = self.tick(ui.ctx()) {
                let _ = self.record(
                    "failed",
                    json!({"stage":format!("{:?}",self.stage),"error":error}),
                );
                eprintln!("native soak validation: {error}");
                self.outcome.store(2, Ordering::Relaxed);
                self.transition(Stage::Failed);
            }
        }
        let started = Instant::now();
        self.app.draw_ui(ui);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        self.cpu_frames += 1;
        self.cpu_sum_ms += ms;
        self.cpu_max_ms = self.cpu_max_ms.max(ms);
        if matches!(self.stage, Stage::Done | Stage::Failed) {
            if self.config.close_when_done {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
    }
}

impl Drop for NativeSoakApp {
    fn drop(&mut self) {
        if !matches!(self.stage, Stage::Done | Stage::Failed) {
            let _ = self.record("aborted", json!({"stage":format!("{:?}",self.stage)}));
        }
    }
}

fn source_path(app: &PanelYApp, id: SourceId) -> &str {
    app.workspace
        .source(id)
        .map(|s| s.parquet_path.as_str())
        .unwrap_or("<missing>")
}

fn layout_snapshot(app: &PanelYApp) -> Value {
    json!({"sources":app.workspace.source_order.iter().map(|id| {
        let s=app.workspace.source(*id).expect("ordered source");
        json!({"path":s.parquet_path,"label":s.label,"offset":s.time_transform.offset_seconds})
    }).collect::<Vec<_>>(),
    "rows":app.view.rows.iter().map(|r|json!({"y_mode":format!("{:?}",r.y_range.mode),
        "y_min":r.y_range.min,"y_max":r.y_range.max,
        "channels":r.channels.iter().map(|c|json!({"source_path":source_path(app,c.key.source_id),
            "path":c.key.channel_path,"color_index":c.color_index,"color":c.color_override.map(|c|c.to_array()),
            "draw_mode":format!("{:?}",c.draw_mode),"visible":c.visible,"line_width":c.line_width})).collect::<Vec<_>>()
    })).collect::<Vec<_>>(),"theme":format!("{:?}",app.view.theme),"visual_mode":format!("{:?}",app.view.visual_mode),"scroll_zoom":app.view.scroll_zoom})
}

fn statistics_snapshot(app: &PanelYApp) -> Result<Value, String> {
    let mut rows = Vec::new();
    for s in &app.analysis.interval_stats_cache.stats {
        let values = [
            s.value_a,
            s.value_b,
            s.delta,
            s.mean,
            s.min,
            s.max,
            s.peak_to_peak,
            s.rms,
            s.slope_per_second,
        ];
        if values.iter().any(|v| !v.is_some_and(f64::is_finite)) {
            return Err("statistics contain missing/nonfinite values".into());
        }
        let source = app
            .workspace
            .source(s.key.source_id)
            .ok_or("statistics source missing")?;
        let time = &source
            .shared_time
            .as_ref()
            .ok_or("statistics time missing")?
            .time;
        let channel = source
            .loaded_channels
            .channel(&s.key.channel_path)
            .ok_or("statistics channel missing")?;
        let count = time.len().min(channel.values.len());
        let a = nearest_sample_index_in_range(time, source.time_transform.to_raw(0.04), count)
            .ok_or("cursor A outside data")?;
        let b = nearest_sample_index_in_range(time, source.time_transform.to_raw(0.09), count)
            .ok_or("cursor B outside data")?;
        rows.push(json!({
        "source_path":source_path(app,s.key.source_id),"channel":s.key.channel_path,
        "index_start":a.min(b),"index_end_exclusive":a.max(b)+1,"sample_count":a.abs_diff(b)+1,
        "value_a":s.value_a,"value_b":s.value_b,"delta":s.delta,"mean":s.mean,"min":s.min,"max":s.max,
        "peak_to_peak":s.peak_to_peak,"rms":s.rms,"slope_per_second":s.slope_per_second
        }));
    }
    rows.sort_by_key(|v| format!("{}:{}", v["source_path"], v["channel"]));
    Ok(Value::Array(rows))
}

fn hash_f64(values: &[f64]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("{hash:016x}")
}

fn fft_snapshot(app: &PanelYApp) -> Value {
    let mut rows=app.analysis.fft.results.iter().map(|(id,r)|json!({
        "source_path":source_path(app,*id),"cursor_a":r.cursor_a,"cursor_b":r.cursor_b,
        "index_start":r.index_start,"index_end_exclusive":r.index_end_exclusive,"sample_count":r.sample_count,"dt":r.dt,
        "frequencies_count":r.frequency_hz.len(),"frequencies_fnv64":hash_f64(&r.frequency_hz),
        "spectra":r.spectra.iter().map(|s|json!({"channel":s.channel_path,"amplitude_count":s.amplitude.len(),
            "amplitude_fnv64":hash_f64(&s.amplitude),"peak_frequency_hz":s.peak_freq_hz,"peak_amplitude":s.peak_amp,"peak_db":s.peak_db})).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    rows.sort_by_key(|v| v["source_path"].to_string());
    Value::Array(rows)
}

fn check_snapshot(baseline: &mut Option<Value>, actual: &Value, label: &str) -> Result<(), String> {
    if let Some(expected) = baseline {
        if expected != actual {
            return Err(format!(
                "{label} numerical results changed from initial baseline"
            ));
        }
    } else {
        *baseline = Some(actual.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_numeric_changes_and_preserves_small_baseline() {
        let mut baseline = None;
        check_snapshot(&mut baseline, &json!({"mean":1.0}), "stats").unwrap();
        check_snapshot(&mut baseline, &json!({"mean":1.0}), "stats").unwrap();
        assert!(check_snapshot(&mut baseline, &json!({"mean":1.001}), "stats").is_err());
        assert_ne!(hash_f64(&[1.0, 2.0]), hash_f64(&[1.0, 2.001]));
    }
    #[test]
    fn rejects_invalid_soak_schedule() {
        let mut c = Config {
            plot_config_path: "config".into(),
            output_jsonl: "out".into(),
            duration_seconds: 3600.0,
            cycles: 30,
            final_quiet_seconds: 120.0,
            close_when_done: true,
        };
        assert!(c.validate().is_ok());
        c.cycles = 0;
        assert!(c.validate().is_err());
        c.cycles = 30;
        c.duration_seconds = f64::NAN;
        assert!(c.validate().is_err());
    }
    #[test]
    fn refuses_to_overwrite_existing_evidence() {
        let path = std::env::temp_dir().join(format!(
            "panely-soak-create-new-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path_str = path.to_str().unwrap();
        let mut file = open_output(path_str).unwrap();
        file.write_all(b"existing evidence").unwrap();
        drop(file);
        assert!(open_output(path_str).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"existing evidence");
        std::fs::remove_file(path).unwrap();
    }
}
