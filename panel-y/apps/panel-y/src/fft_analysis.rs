use crate::{
    ChannelKey, ControlButtonRole, SourceId, ViewState, VisualPalette, WorkspaceDataState,
    add_control_button, add_control_button_enabled, add_control_button_enabled_with_role,
    add_control_button_with_role, add_control_checkbox, add_control_checkbox_enabled,
    analysis_visible_loaded_channels, channel_label_or_fallback, channel_tooltip_or_fallback,
    nearest_sample_index_in_range,
};
use eframe::egui;
use rustfft::{FftPlanner, num_complex::Complex};
use std::collections::BTreeMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_FFT_TARGET_CHANNELS: usize = 3;
const FFT_SOFT_WARNING_SAMPLES_PER_CHANNEL: usize = 1_000_000;
const FFT_HARD_LIMIT_SAMPLES_PER_CHANNEL: usize = 5_000_000;
const FFT_HARD_LIMIT_COPY_BYTES: usize = 384 * 1024 * 1024;
const MAX_FFT_DRAW_POINTS_PER_TRACE: usize = 2_048;
const MAX_FFT_HOVER_READOUT_TRACES: usize = 4;
const FFT_PLOT_HEIGHT: f32 = 180.0;
const FFT_PLOT_TOP_MARGIN: f32 = 22.0;
const FFT_PLOT_MIN_ZOOM_PIXEL_WIDTH: f32 = 3.0;
const FFT_PLOT_WHEEL_ZOOM_SENSITIVITY: f64 = 0.001;
const DB_FLOOR_AMPLITUDE: f64 = 1.0e-12;

#[derive(Clone, Debug)]
pub(crate) struct FftAnalysisState {
    pub(crate) enabled: bool,
    pub(crate) section_collapsed: bool,
    pub(crate) target_channels: Vec<ChannelKey>,
    pub(crate) target_initialized: bool,
    pub(crate) window: FftWindowKind,
    pub(crate) y_scale: FftYScale,
    pub(crate) freq_range: FftFrequencyRange,
    pub(crate) hover_frequency: Option<f64>,
    plot_zoom_drag: Option<FftPlotZoomDrag>,
    pub(crate) results: BTreeMap<SourceId, FftResult>,
    pub(crate) source_errors: BTreeMap<SourceId, String>,
    pub(crate) error: Option<String>,
    pub(crate) jobs: BTreeMap<SourceId, FftJobState>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FftWindowKind {
    #[default]
    Hanning,
    Hamming,
    Rectangular,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FftYScale {
    #[default]
    Amplitude,
    Db,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FftFrequencyRange {
    pub(crate) min_hz: Option<f64>,
    pub(crate) max_hz: Option<f64>,
}

#[derive(Clone, Debug)]
pub(crate) enum FftJobState {
    Idle,
    Running {
        job_id: u64,
        key: FftRequestKey,
        sample_count: usize,
        channel_count: usize,
        started: Instant,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FftResult {
    pub(crate) key: FftRequestKey,
    pub(crate) cursor_a: f64,
    pub(crate) cursor_b: f64,
    pub(crate) index_start: usize,
    pub(crate) index_end_exclusive: usize,
    pub(crate) sample_count: usize,
    pub(crate) dt: f64,
    pub(crate) frequency_hz: Vec<f64>,
    pub(crate) spectra: Vec<FftChannelSpectrum>,
    pub(crate) elapsed: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FftChannelSpectrum {
    pub(crate) key: ChannelKey,
    pub(crate) channel_name: String,
    pub(crate) channel_path: String,
    pub(crate) display_label: String,
    pub(crate) tooltip: String,
    pub(crate) amplitude: Vec<f64>,
    pub(crate) peak_freq_hz: Option<f64>,
    pub(crate) peak_amp: Option<f64>,
    pub(crate) peak_db: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FftPlotZoomDrag {
    plot_rect: egui::Rect,
    start_pos: egui::Pos2,
    current_pos: egui::Pos2,
    frequency_range: (f64, f64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FftRequestKey {
    source_id: SourceId,
    source_generation: u64,
    time_offset_bits: u64,
    cursor_a_bits: u64,
    cursor_b_bits: u64,
    time_sample_count: usize,
    time_start_bits: u64,
    time_end_bits: u64,
    window: FftWindowKind,
    index_start: usize,
    index_end_exclusive: usize,
    channels: Vec<FftRequestChannelKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FftRequestChannelKey {
    key: ChannelKey,
    sample_count: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct FftJobInput {
    key: FftRequestKey,
    cursor_a: f64,
    cursor_b: f64,
    index_start: usize,
    index_end_exclusive: usize,
    dt: f64,
    time: Vec<f64>,
    window: FftWindowKind,
    channels: Vec<FftJobChannelInput>,
}

#[derive(Clone, Debug)]
struct FftJobChannelInput {
    key: ChannelKey,
    channel_name: String,
    channel_path: String,
    display_label: String,
    tooltip: String,
    values: Vec<f64>,
}

#[derive(Debug)]
pub(crate) struct FftJobResult {
    pub(crate) job_id: u64,
    pub(crate) key: FftRequestKey,
    pub(crate) result: Result<FftResult, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FftUiAction {
    Run,
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FftChannelChoice {
    key: ChannelKey,
    channel_name: String,
    channel_path: String,
    display_label: String,
    tooltip: String,
    sample_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FftRequestPreview {
    sample_count: usize,
    channel_count: usize,
    copy_bytes: usize,
}

impl Default for FftAnalysisState {
    fn default() -> Self {
        Self {
            enabled: true,
            section_collapsed: false,
            target_channels: Vec::new(),
            target_initialized: false,
            window: FftWindowKind::Hanning,
            y_scale: FftYScale::Amplitude,
            freq_range: FftFrequencyRange::default(),
            hover_frequency: None,
            plot_zoom_drag: None,
            results: BTreeMap::new(),
            source_errors: BTreeMap::new(),
            error: None,
            jobs: BTreeMap::new(),
        }
    }
}

impl Default for FftJobState {
    fn default() -> Self {
        Self::Idle
    }
}

impl FftAnalysisState {
    pub(crate) fn invalidate_result(&mut self) {
        self.hover_frequency = None;
        self.plot_zoom_drag = None;
        self.results.clear();
        self.source_errors.clear();
        self.error = None;
        self.jobs.clear();
    }

    pub(crate) fn reset_for_dataset(&mut self) {
        self.target_channels.clear();
        self.target_initialized = false;
        self.invalidate_result();
    }

    pub(crate) fn cleanup_source(&mut self, source_id: SourceId) {
        let targets_changed = self
            .target_channels
            .iter()
            .any(|key| key.source_id == source_id);
        self.target_channels
            .retain(|key| key.source_id != source_id);
        let result_affected =
            self.results.contains_key(&source_id) || self.source_errors.contains_key(&source_id);
        let job_affected = self.jobs.contains_key(&source_id);
        if targets_changed || result_affected || job_affected {
            self.invalidate_result();
        }
    }

    pub(crate) fn accepts_job_result(
        &self,
        job_id: u64,
        key: &FftRequestKey,
        workspace: &WorkspaceDataState,
    ) -> bool {
        matches!(
            self.jobs.get(&key.source_id),
            Some(FftJobState::Running {
                job_id: running_id,
                key: running_key,
                ..
            }) if *running_id == job_id && running_key == key
        ) && key.matches_workspace(workspace)
    }

    pub(crate) fn register_job(&mut self, job_id: u64, input: &FftJobInput) {
        let source_id = input.source_id();
        self.results.remove(&source_id);
        self.source_errors.remove(&source_id);
        self.jobs.insert(
            source_id,
            FftJobState::Running {
                job_id,
                key: input.key.clone(),
                sample_count: input.sample_count(),
                channel_count: input.channel_count(),
                started: Instant::now(),
            },
        );
    }

    pub(crate) fn apply_job_result(
        &mut self,
        result: FftJobResult,
        workspace: &WorkspaceDataState,
    ) -> bool {
        if !self.accepts_job_result(result.job_id, &result.key, workspace) {
            // An obsolete delivery cannot retire a newer job for the same source.
            if matches!(self.jobs.get(&result.key.source_id),
                Some(FftJobState::Running { job_id, key, .. })
                    if *job_id == result.job_id && *key == result.key)
            {
                self.jobs.remove(&result.key.source_id);
                self.results.remove(&result.key.source_id);
                self.source_errors.insert(
                    result.key.source_id,
                    "Source changed during FFT; run FFT again.".to_owned(),
                );
            }
            return false;
        }
        let source_id = result.key.source_id;
        self.jobs.remove(&source_id);
        match result.result {
            Ok(value) if value.key == result.key => {
                self.source_errors.remove(&source_id);
                self.results.insert(source_id, value);
            }
            Ok(_) => {
                self.results.remove(&source_id);
                self.source_errors
                    .insert(source_id, "FFT result identity mismatch.".to_owned());
            }
            Err(error) => {
                self.results.remove(&source_id);
                self.source_errors.insert(source_id, error);
            }
        }
        true
    }

    pub(crate) fn cancel_jobs(&mut self) {
        if self.is_running() {
            self.jobs.clear();
            self.error = Some("FFT jobs cancelled; completed results retained.".to_owned());
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        self.jobs
            .values()
            .any(|job| matches!(job, FftJobState::Running { .. }))
    }
}

impl FftRequestKey {
    fn matches_workspace(&self, workspace: &WorkspaceDataState) -> bool {
        let Some(source) = workspace.source(self.source_id) else {
            return false;
        };
        if source.generation != self.source_generation
            || source.time_transform.offset_seconds.to_bits() != self.time_offset_bits
        {
            return false;
        }
        let Some(shared_time) = source.shared_time.as_ref() else {
            return false;
        };
        if shared_time.time.len() != self.time_sample_count
            || shared_time
                .time
                .first()
                .copied()
                .unwrap_or(f64::NAN)
                .to_bits()
                != self.time_start_bits
            || shared_time
                .time
                .last()
                .copied()
                .unwrap_or(f64::NAN)
                .to_bits()
                != self.time_end_bits
        {
            return false;
        }

        self.channels.iter().all(|request_channel| {
            request_channel.key.source_id == self.source_id
                && workspace
                    .channel(&request_channel.key)
                    .is_some_and(|channel| channel.values.len() == request_channel.sample_count)
        })
    }
}

impl FftWindowKind {
    const ALL: [Self; 3] = [Self::Hanning, Self::Hamming, Self::Rectangular];

    fn as_str(self) -> &'static str {
        match self {
            Self::Hanning => "Hanning",
            Self::Hamming => "Hamming",
            Self::Rectangular => "Rectangular",
        }
    }
}

impl FftYScale {
    const ALL: [Self; 2] = [Self::Amplitude, Self::Db];

    fn as_str(self) -> &'static str {
        match self {
            Self::Amplitude => "Amplitude",
            Self::Db => "dB",
        }
    }
}

impl FftFrequencyRange {
    fn normalized(self) -> Option<(Option<f64>, Option<f64>)> {
        let min = self
            .min_hz
            .filter(|value| value.is_finite())
            .map(|value| value.max(0.0));
        let max = self
            .max_hz
            .filter(|value| value.is_finite())
            .map(|value| value.max(0.0));

        if let (Some(min), Some(max)) = (min, max)
            && min >= max
        {
            return None;
        }

        Some((min, max))
    }

    fn bounds_for(self, frequencies: &[f64]) -> Option<(f64, f64)> {
        let first = frequencies.first().copied()?;
        let last = frequencies.last().copied()?;
        let (min, max) = self.normalized()?;
        let min = min.unwrap_or(first).max(first);
        let max = max.unwrap_or(last).min(last);
        (min < max).then_some((min, max))
    }
}

impl FftJobInput {
    pub(crate) fn source_id(&self) -> SourceId {
        self.key.source_id
    }

    pub(crate) fn key(&self) -> &FftRequestKey {
        &self.key
    }

    pub(crate) fn sample_count(&self) -> usize {
        self.index_end_exclusive.saturating_sub(self.index_start)
    }

    pub(crate) fn channel_count(&self) -> usize {
        self.channels.len()
    }
}

pub(crate) fn draw_fft_section(
    ui: &mut egui::Ui,
    fft: &mut FftAnalysisState,
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Option<FftUiAction> {
    let available_channels = available_fft_channels(workspace, view);
    let palette = view.visual_palette();
    if sync_fft_targets(fft, &available_channels) {
        fft.invalidate_result();
    }
    let cursors_ready = cursor_a.is_some_and(f64::is_finite)
        && cursor_b.is_some_and(f64::is_finite)
        && cursor_a != cursor_b;
    let request_previews = fft_request_previews(cursor_a, cursor_b, fft, workspace, view);
    let pane_rect = ui.available_rect_before_wrap();
    ui.set_clip_rect(ui.clip_rect().intersect(pane_rect));

    let mut action = None;
    // Keep execution controls outside the scrolling settings/results body.
    ui.horizontal_wrapped(|ui| {
        let collapse_label = if fft.section_collapsed {
            "[+] FFT"
        } else {
            "[-] FFT"
        };
        if add_control_button(ui, palette, egui::Button::new(collapse_label)).clicked() {
            fft.section_collapsed = !fft.section_collapsed;
        }
        let running = fft.is_running();
        if add_control_button_enabled(
            ui,
            palette,
            fft.enabled && cursors_ready && !fft.target_channels.is_empty() && !running,
            egui::Button::new("Run FFT"),
        )
        .clicked()
        {
            action = Some(FftUiAction::Run);
        }
        if running {
            ui.spinner();
            if add_control_button_enabled_with_role(
                ui,
                palette,
                fft.enabled,
                ControlButtonRole::Destructive,
                egui::Button::new("Cancel"),
            )
            .on_hover_text("Ignore unfinished FFT jobs; keep completed source results.")
            .clicked()
            {
                action = Some(FftUiAction::Cancel);
            }
        }
        ui.label(format!("{} target(s)", fft.target_channels.len()))
            .on_hover_text(format!(
                "Source spectra overlay, up to {MAX_FFT_TARGET_CHANNELS} total channels"
            ));
        if request_previews
            .iter()
            .any(|(_, preview)| preview.sample_count >= FFT_SOFT_WARNING_SAMPLES_PER_CHANNEL)
        {
            ui.colored_label(ui.visuals().warn_fg_color, "Large FFT interval")
                .on_hover_text("Computation runs in the background. See Computation details for sample counts and copy memory.");
        }
    });

    if fft.section_collapsed {
        fft.hover_frequency = None;
        fft.plot_zoom_drag = None;
        return action;
    }

    let body_height = ui.available_height().max(1.0);
    egui::ScrollArea::vertical()
        .id_salt("fft_settings_results_y")
        .auto_shrink([false, false])
        .min_scrolled_height(1.0)
        .max_height(body_height)
        .show(ui, |ui| {
            ui.add_enabled_ui(fft.enabled, |ui| {
                if available_channels.is_empty() {
                    fft.hover_frequency = None;
                    ui.label("Load and show channels before running FFT.");
                    return;
                }
                if !cursors_ready {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Place Cursor A and B to enable FFT.",
                    );
                }

                egui::CollapsingHeader::new(format!(
                    "Targets {}/{}",
                    fft.target_channels.len(),
                    MAX_FFT_TARGET_CHANNELS
                ))
                .id_salt("fft_targets_section")
                .default_open(false)
                .show(ui, |ui| {
                    if draw_fft_target_controls(ui, fft, &available_channels, palette) {
                        fft.invalidate_result();
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    let old_window = fft.window;
                    ui.label("Window");
                    egui::ComboBox::from_id_salt("fft_window_kind")
                        .selected_text(fft.window.as_str())
                        .show_ui(ui, |ui| {
                            for kind in FftWindowKind::ALL {
                                ui.selectable_value(&mut fft.window, kind, kind.as_str());
                            }
                        });
                    if fft.window != old_window {
                        fft.invalidate_result();
                    }

                    ui.separator();
                    ui.label("Y");
                    egui::ComboBox::from_id_salt("fft_y_scale")
                        .selected_text(fft.y_scale.as_str())
                        .show_ui(ui, |ui| {
                            for scale in FftYScale::ALL {
                                ui.selectable_value(&mut fft.y_scale, scale, scale.as_str());
                            }
                        });
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Frequency range");
                    let _ = optional_frequency_drag(ui, palette, "Min", &mut fft.freq_range.min_hz);
                    let _ = optional_frequency_drag(ui, palette, "Max", &mut fft.freq_range.max_hz);
                });
                if fft.freq_range.normalized().is_none() {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Frequency Min must be lower than Max.",
                    );
                }

                if let Some(error) = &fft.error {
                    ui.colored_label(ui.visuals().warn_fg_color, error);
                }
                for (source_id, error) in &fft.source_errors {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!("{}: {error}", fft_source_label(workspace, *source_id)),
                    );
                }

                if !fft.results.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for trace in fft_overlay_traces(&fft.results, &fft.target_channels) {
                            ui.colored_label(
                                crate::channel_color(trace.color_index, ui.visuals().dark_mode),
                                format!("━ {}", compact_label(&trace.spectrum.display_label, 26)),
                            )
                            .on_hover_text(&trace.spectrum.tooltip);
                        }
                    });
                    let y_scale = fft.y_scale;
                    draw_fft_plot(
                        ui,
                        &fft.results,
                        &fft.target_channels,
                        y_scale,
                        view.scroll_zoom,
                        &mut fft.freq_range,
                        &mut fft.hover_frequency,
                        &mut fft.plot_zoom_drag,
                    );
                    egui::CollapsingHeader::new("Result values and sample grids")
                        .id_salt("fft_result_details")
                        .default_open(false)
                        .show(ui, |ui| {
                            draw_fft_results_summary(
                                ui,
                                &fft.results,
                                &fft.target_channels,
                                workspace,
                            );
                        });
                } else {
                    fft.hover_frequency = None;
                    fft.plot_zoom_drag = None;
                }

                egui::CollapsingHeader::new("Computation details")
                    .id_salt("fft_computation_details")
                    .default_open(false)
                    .show(ui, |ui| {
                        for (source_id, preview) in &request_previews {
                            ui.monospace(format!(
                                "{}: {} samples/ch, {} channel(s), copy {:.1} MiB",
                                fft_source_label(workspace, *source_id),
                                preview.sample_count,
                                preview.channel_count,
                                bytes_to_mib(preview.copy_bytes)
                            ));
                        }
                        for (source_id, job) in &fft.jobs {
                            if let FftJobState::Running {
                                sample_count,
                                channel_count,
                                started,
                                ..
                            } = job
                            {
                                ui.label(format!(
                                    "{}: Running... {} samples/ch, {} channel(s), {:.0} ms",
                                    fft_source_label(workspace, *source_id),
                                    sample_count,
                                    channel_count,
                                    duration_ms(started.elapsed())
                                ));
                            }
                        }
                    });
            });
        });
    action
}

#[derive(Debug)]
pub(crate) struct FftJobBatch {
    pub(crate) inputs: Vec<FftJobInput>,
    pub(crate) errors: BTreeMap<SourceId, String>,
}

struct FftInputPlan<'a> {
    key: FftRequestKey,
    cursor_a: f64,
    cursor_b: f64,
    dt: f64,
    time: &'a [f64],
    channels: Vec<(ChannelKey, &'a crate::parquet_waveform::ChannelData)>,
}

impl FftInputPlan<'_> {
    fn copy_bytes(&self) -> usize {
        estimated_fft_copy_bytes(self.time.len(), self.channels.len())
    }

    fn into_input(self, workspace: &WorkspaceDataState) -> FftJobInput {
        let index_start = self.key.index_start;
        let index_end_exclusive = self.key.index_end_exclusive;
        let channels = self
            .channels
            .into_iter()
            .map(|(key, channel)| FftJobChannelInput {
                display_label: channel_label_or_fallback(workspace, &key),
                tooltip: channel_tooltip_or_fallback(workspace, &key),
                key,
                channel_name: channel.channel_name.clone(),
                channel_path: channel.channel_path.clone(),
                values: channel.values[index_start..index_end_exclusive]
                    .iter()
                    .copied()
                    .map(f64::from)
                    .collect(),
            })
            .collect();
        FftJobInput {
            window: self.key.window,
            key: self.key,
            cursor_a: self.cursor_a,
            cursor_b: self.cursor_b,
            index_start,
            index_end_exclusive,
            dt: self.dt,
            time: self.time.to_vec(),
            channels,
        }
    }
}

fn validated_cursors(cursor_a: Option<f64>, cursor_b: Option<f64>) -> Result<(f64, f64), String> {
    let cursor_a = cursor_a.ok_or_else(|| "Cursor A is not set.".to_owned())?;
    let cursor_b = cursor_b.ok_or_else(|| "Cursor B is not set.".to_owned())?;
    if !cursor_a.is_finite() || !cursor_b.is_finite() || cursor_a == cursor_b {
        return Err("Cursor A/B interval is invalid.".to_owned());
    }

    Ok((cursor_a, cursor_b))
}

pub(crate) fn build_fft_job_inputs(
    fft: &FftAnalysisState,
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Result<FftJobBatch, String> {
    let (cursor_a, cursor_b) = validated_cursors(cursor_a, cursor_b)?;
    if fft.target_initialized && fft.target_channels.len() > MAX_FFT_TARGET_CHANNELS {
        return Err(format!(
            "FFT supports at most {MAX_FFT_TARGET_CHANNELS} total target channels."
        ));
    }
    let selected = selected_loaded_channels(fft, workspace, view);
    if selected.is_empty() {
        return Err("Select at least one visible loaded channel for FFT.".to_owned());
    }
    let mut groups = BTreeMap::new();
    for (key, channel) in selected {
        groups
            .entry(key.source_id)
            .or_insert_with(Vec::new)
            .push((key, channel));
    }
    let mut errors = BTreeMap::new();
    let mut plans = Vec::new();
    for (source_id, channels) in groups {
        match prepare_fft_input(fft.window, cursor_a, cursor_b, workspace, channels) {
            Ok(plan) => plans.push(plan),
            Err(error) => {
                errors.insert(source_id, error);
            }
        }
    }
    let copy_bytes = plans.iter().fold(0usize, |total, plan| {
        total.saturating_add(plan.copy_bytes())
    });
    validate_fft_copy_budget(copy_bytes)?;
    Ok(FftJobBatch {
        inputs: plans
            .into_iter()
            .map(|plan| plan.into_input(workspace))
            .collect(),
        errors,
    })
}

#[cfg(test)]
pub(crate) fn build_fft_job_input(
    fft: &FftAnalysisState,
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Result<FftJobInput, String> {
    let (cursor_a, cursor_b) = validated_cursors(cursor_a, cursor_b)?;
    let selected = selected_loaded_channels(fft, workspace, view);
    let plan = prepare_fft_input(fft.window, cursor_a, cursor_b, workspace, selected)?;
    validate_fft_copy_budget(plan.copy_bytes())?;
    Ok(plan.into_input(workspace))
}

fn validate_fft_copy_budget(copy_bytes: usize) -> Result<(), String> {
    if copy_bytes > FFT_HARD_LIMIT_COPY_BYTES {
        return Err(format!(
            "FFT input is too large: estimated total copy {:.1} MiB exceeds the {:.1} MiB limit.",
            bytes_to_mib(copy_bytes),
            bytes_to_mib(FFT_HARD_LIMIT_COPY_BYTES)
        ));
    }
    Ok(())
}

fn prepare_fft_input<'a>(
    window: FftWindowKind,
    cursor_a: f64,
    cursor_b: f64,
    workspace: &'a WorkspaceDataState,
    selected_channels: Vec<(ChannelKey, &'a crate::parquet_waveform::ChannelData)>,
) -> Result<FftInputPlan<'a>, String> {
    if selected_channels.is_empty() {
        return Err("Select at least one visible loaded channel for FFT.".to_owned());
    }
    let source_id = single_fft_source_id(&selected_channels)?;
    let source = workspace
        .source(source_id)
        .ok_or_else(|| "The FFT data source no longer exists.".to_owned())?;
    let shared_time = source
        .shared_time
        .as_ref()
        .ok_or_else(|| "Time data is not loaded for the FFT source.".to_owned())?;
    let raw_cursor_a = source.time_transform.to_raw(cursor_a);
    let raw_cursor_b = source.time_transform.to_raw(cursor_b);

    let sample_count = selected_channels
        .iter()
        .fold(shared_time.time.len(), |count, (_, channel)| {
            count.min(channel.values.len())
        });
    let index_a = nearest_sample_index_in_range(&shared_time.time, raw_cursor_a, sample_count)
        .ok_or_else(|| "Cursor A is outside the loaded time range.".to_owned())?;
    let index_b = nearest_sample_index_in_range(&shared_time.time, raw_cursor_b, sample_count)
        .ok_or_else(|| "Cursor B is outside the loaded time range.".to_owned())?;
    let index_start = index_a.min(index_b);
    let index_end_exclusive = index_a.max(index_b).saturating_add(1).min(sample_count);
    let segment_sample_count = index_end_exclusive.saturating_sub(index_start);
    if segment_sample_count < 4 {
        return Err("FFT requires at least 4 samples in the Cursor A/B interval.".to_owned());
    }
    if segment_sample_count > FFT_HARD_LIMIT_SAMPLES_PER_CHANNEL {
        return Err(format!(
            "FFT interval is too large: {} samples/ch exceeds the {} samples/ch limit.",
            segment_sample_count, FFT_HARD_LIMIT_SAMPLES_PER_CHANNEL
        ));
    }

    let dt = shared_time.time[index_start + 1] - shared_time.time[index_start];
    if !dt.is_finite() || dt <= 0.0 {
        return Err("FFT requires a positive finite sample interval.".to_owned());
    }

    let key = FftRequestKey {
        source_id,
        source_generation: source.generation,
        time_offset_bits: source.time_transform.offset_seconds.to_bits(),
        cursor_a_bits: cursor_a.to_bits(),
        cursor_b_bits: cursor_b.to_bits(),
        time_sample_count: shared_time.time.len(),
        time_start_bits: shared_time
            .time
            .first()
            .copied()
            .unwrap_or(f64::NAN)
            .to_bits(),
        time_end_bits: shared_time
            .time
            .last()
            .copied()
            .unwrap_or(f64::NAN)
            .to_bits(),
        window,
        index_start,
        index_end_exclusive,
        channels: selected_channels
            .iter()
            .map(|(key, channel)| FftRequestChannelKey {
                key: key.clone(),
                sample_count: channel.values.len(),
            })
            .collect(),
    };

    Ok(FftInputPlan {
        key,
        cursor_a,
        cursor_b,
        dt,
        time: &shared_time.time[index_start..index_end_exclusive],
        channels: selected_channels,
    })
}

pub(crate) fn spawn_fft_job(
    tx: mpsc::Sender<FftJobResult>,
    ctx: egui::Context,
    job_id: u64,
    input: FftJobInput,
) {
    std::thread::spawn(move || {
        let key = input.key().clone();
        let result = compute_fft(input);
        let _ = tx.send(FftJobResult {
            job_id,
            key,
            result,
        });
        ctx.request_repaint();
    });
}

pub(crate) fn compute_fft(input: FftJobInput) -> Result<FftResult, String> {
    let started = Instant::now();
    let sample_count = input.sample_count();
    if sample_count < 4 || input.time.len() != sample_count {
        return Err("FFT input has an invalid sample count.".to_owned());
    }
    validate_uniform_time_step(&input.time, input.dt)?;

    let frequency_count = sample_count / 2 + 1;
    let frequency_hz = (0..frequency_count)
        .map(|bin| bin as f64 / (sample_count as f64 * input.dt))
        .collect::<Vec<_>>();
    let window = fft_window_values(input.window, sample_count);
    let normalization = 2.0 / sample_count as f64;
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(sample_count);
    let mut spectra = Vec::with_capacity(input.channels.len());

    for channel in input.channels {
        if channel.values.len() != sample_count {
            return Err(format!(
                "FFT input length mismatch for {}.",
                channel.display_label
            ));
        }
        if channel.values.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "FFT input contains NaN/Inf in {}.",
                channel.display_label
            ));
        }

        let mean = channel.values.iter().sum::<f64>() / sample_count as f64;
        let mut buffer = channel
            .values
            .iter()
            .zip(window.iter())
            .map(|(value, window)| Complex {
                re: (value - mean) * window,
                im: 0.0,
            })
            .collect::<Vec<_>>();
        fft.process(&mut buffer);

        let amplitude = buffer
            .iter()
            .take(frequency_count)
            .map(|bin| bin.norm() * normalization)
            .collect::<Vec<_>>();
        let (peak_freq_hz, peak_amp) = peak_frequency_and_amplitude(&frequency_hz, &amplitude);
        let peak_db = peak_amp.map(amplitude_to_db);
        spectra.push(FftChannelSpectrum {
            key: channel.key,
            channel_name: channel.channel_name,
            channel_path: channel.channel_path,
            display_label: channel.display_label,
            tooltip: channel.tooltip,
            amplitude,
            peak_freq_hz,
            peak_amp,
            peak_db,
        });
    }

    Ok(FftResult {
        key: input.key,
        cursor_a: input.cursor_a,
        cursor_b: input.cursor_b,
        index_start: input.index_start,
        index_end_exclusive: input.index_end_exclusive,
        sample_count,
        dt: input.dt,
        frequency_hz,
        spectra,
        elapsed: started.elapsed(),
    })
}

fn available_fft_channels(
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Vec<FftChannelChoice> {
    analysis_visible_loaded_channels(workspace, view)
        .into_iter()
        .map(|(key, channel)| FftChannelChoice {
            display_label: channel_label_or_fallback(workspace, &key),
            tooltip: channel_tooltip_or_fallback(workspace, &key),
            key,
            channel_name: channel.channel_name.clone(),
            channel_path: channel.channel_path.clone(),
            sample_count: channel.values.len(),
        })
        .collect()
}

fn sync_fft_targets(fft: &mut FftAnalysisState, available_channels: &[FftChannelChoice]) -> bool {
    if available_channels.is_empty() {
        let changed = !fft.target_channels.is_empty() || fft.target_initialized;
        fft.target_channels.clear();
        fft.target_initialized = false;
        return changed;
    }

    let before = fft.target_channels.clone();
    fft.target_channels
        .retain(|key| available_channels.iter().any(|choice| choice.key == *key));
    if fft.target_channels.len() > MAX_FFT_TARGET_CHANNELS {
        fft.target_channels.truncate(MAX_FFT_TARGET_CHANNELS);
    }
    if !fft.target_initialized {
        fft.target_channels = available_channels
            .iter()
            .take(MAX_FFT_TARGET_CHANNELS)
            .map(|choice| choice.key.clone())
            .collect();
        fft.target_initialized = true;
    }

    before != fft.target_channels
}

fn draw_fft_target_controls(
    ui: &mut egui::Ui,
    fft: &mut FftAnalysisState,
    available_channels: &[FftChannelChoice],
    palette: VisualPalette,
) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "Targets {}/{}",
            fft.target_channels.len().min(MAX_FFT_TARGET_CHANNELS),
            MAX_FFT_TARGET_CHANNELS
        ));
        if add_control_button(ui, palette, egui::Button::new("First visible")).clicked() {
            fft.target_channels = available_channels
                .iter()
                .take(MAX_FFT_TARGET_CHANNELS)
                .map(|choice| choice.key.clone())
                .collect();
            fft.target_initialized = true;
            changed = true;
        }
        if add_control_button_with_role(
            ui,
            palette,
            ControlButtonRole::Destructive,
            egui::Button::new("Clear targets"),
        )
        .clicked()
        {
            fft.target_channels.clear();
            fft.target_initialized = true;
            changed = true;
        }
    });

    for choice in available_channels {
        let selected = fft.target_channels.contains(&choice.key);
        let limit_reached = !selected && fft.target_channels.len() >= MAX_FFT_TARGET_CHANNELS;
        let mut next_selected = selected;
        let response = add_control_checkbox_enabled(
            ui,
            palette,
            !limit_reached,
            &mut next_selected,
            &choice.display_label,
        );
        response.on_hover_text(&choice.tooltip);

        if next_selected != selected {
            fft.target_initialized = true;
            changed = true;
            if next_selected {
                if fft.target_channels.len() < MAX_FFT_TARGET_CHANNELS {
                    fft.target_channels.push(choice.key.clone());
                }
            } else {
                fft.target_channels.retain(|key| key != &choice.key);
            }
        }
    }

    changed
}

fn optional_frequency_drag(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    label: &str,
    value: &mut Option<f64>,
) -> bool {
    let mut changed = false;
    let mut enabled = value.is_some();
    if add_control_checkbox(ui, palette, &mut enabled, label).changed() {
        changed = true;
        *value = enabled.then_some(value.unwrap_or(0.0).max(0.0));
    }
    if enabled {
        let mut hz = value.unwrap_or(0.0);
        if ui
            .add(
                egui::DragValue::new(&mut hz)
                    .speed(1.0)
                    .suffix(" Hz")
                    .max_decimals(3),
            )
            .changed()
        {
            *value = Some(hz.max(0.0));
            changed = true;
        }
    }

    changed
}

fn fft_source_label(workspace: &WorkspaceDataState, source_id: SourceId) -> String {
    crate::source_presentation(workspace, source_id)
        .map(|presentation| presentation.short_label)
        .unwrap_or_else(|| format!("Source {}", source_id.value()))
}

fn fft_request_previews(
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    fft: &FftAnalysisState,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Vec<(SourceId, FftRequestPreview)> {
    let Ok((cursor_a, cursor_b)) = validated_cursors(cursor_a, cursor_b) else {
        return Vec::new();
    };
    let mut groups = BTreeMap::new();
    for (key, channel) in selected_loaded_channels(fft, workspace, view) {
        groups
            .entry(key.source_id)
            .or_insert_with(Vec::new)
            .push((key, channel));
    }
    groups
        .into_iter()
        .filter_map(|(source_id, channels)| {
            let plan =
                prepare_fft_input(fft.window, cursor_a, cursor_b, workspace, channels).ok()?;
            Some((
                source_id,
                FftRequestPreview {
                    sample_count: plan.time.len(),
                    channel_count: plan.channels.len(),
                    copy_bytes: plan.copy_bytes(),
                },
            ))
        })
        .collect()
}

fn single_fft_source_id(
    selected_channels: &[(ChannelKey, &crate::parquet_waveform::ChannelData)],
) -> Result<SourceId, String> {
    let Some(source_id) = selected_channels.first().map(|(key, _)| key.source_id) else {
        return Err("Select at least one visible loaded channel for FFT.".to_owned());
    };
    if selected_channels
        .iter()
        .any(|(key, _)| key.source_id != source_id)
    {
        return Err(
            "FFT targets must belong to exactly one data source; mixed-source FFT was not started."
                .to_owned(),
        );
    }
    Ok(source_id)
}

fn selected_loaded_channels<'a>(
    fft: &FftAnalysisState,
    workspace: &'a WorkspaceDataState,
    view: &ViewState,
) -> Vec<(ChannelKey, &'a crate::parquet_waveform::ChannelData)> {
    let available = analysis_visible_loaded_channels(workspace, view);
    let mut selected = Vec::new();

    if fft.target_initialized {
        for key in fft.target_channels.iter().take(MAX_FFT_TARGET_CHANNELS) {
            if let Some(channel) = available
                .iter()
                .find(|(available_key, _)| available_key == key)
            {
                selected.push(channel.clone());
            }
        }
    } else {
        selected.extend(available.into_iter().take(MAX_FFT_TARGET_CHANNELS));
    }

    selected
}

fn estimated_fft_copy_bytes(sample_count: usize, channel_count: usize) -> usize {
    sample_count
        .saturating_mul(std::mem::size_of::<f64>())
        .saturating_mul(channel_count.saturating_add(1))
}

fn validate_uniform_time_step(time: &[f64], dt: f64) -> Result<(), String> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err("FFT requires a positive finite sample interval.".to_owned());
    }

    let tolerance = (dt.abs() * 1.0e-6).max(1.0e-12);
    for pair in time.windows(2) {
        let step = pair[1] - pair[0];
        if !step.is_finite() || (step - dt).abs() > tolerance {
            return Err(format!(
                "FFT requires uniformly sampled time data; found dt deviation {:.3e} s.",
                (step - dt).abs()
            ));
        }
    }

    Ok(())
}

fn fft_window_values(kind: FftWindowKind, sample_count: usize) -> Vec<f64> {
    (0..sample_count)
        .map(|index| fft_window_value(kind, sample_count, index))
        .collect()
}

pub(crate) fn fft_window_value(kind: FftWindowKind, sample_count: usize, index: usize) -> f64 {
    if sample_count <= 1 {
        return 1.0;
    }

    let phase = 2.0 * std::f64::consts::PI * index as f64 / (sample_count - 1) as f64;
    match kind {
        FftWindowKind::Hanning => 0.5 - 0.5 * phase.cos(),
        FftWindowKind::Hamming => 0.54 - 0.46 * phase.cos(),
        FftWindowKind::Rectangular => 1.0,
    }
}

pub(crate) fn amplitude_to_db(amplitude: f64) -> f64 {
    let amplitude = if amplitude.is_finite() {
        amplitude.max(DB_FLOOR_AMPLITUDE)
    } else {
        DB_FLOOR_AMPLITUDE
    };
    20.0 * amplitude.log10()
}

fn peak_frequency_and_amplitude(
    frequency_hz: &[f64],
    amplitude: &[f64],
) -> (Option<f64>, Option<f64>) {
    amplitude
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, amplitude)| amplitude.is_finite())
        .max_by(|(_, left), (_, right)| {
            left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, amplitude)| (frequency_hz.get(index).copied(), Some(amplitude)))
        .unwrap_or((None, None))
}

struct FftPlotTrace<'a> {
    result: &'a FftResult,
    spectrum: &'a FftChannelSpectrum,
    color_index: usize,
}

fn fft_overlay_traces<'a>(
    results: &'a BTreeMap<SourceId, FftResult>,
    targets: &[ChannelKey],
) -> Vec<FftPlotTrace<'a>> {
    let mut traces = results
        .values()
        .flat_map(|result| {
            result.spectra.iter().map(move |spectrum| FftPlotTrace {
                result,
                spectrum,
                color_index: targets
                    .iter()
                    .position(|key| key == &spectrum.key)
                    .unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    traces.sort_by_key(|trace| trace.color_index);
    traces
}

fn draw_fft_results_summary(
    ui: &mut egui::Ui,
    results: &BTreeMap<SourceId, FftResult>,
    targets: &[ChannelKey],
    workspace: &WorkspaceDataState,
) {
    ui.separator();
    ui.label(format!(
        "FFT results: {} source(s) · independent sample grids",
        results.len()
    ));
    for (source_id, result) in results {
        ui.monospace(format!(
            "{} | N {} | dt {:.9} s | Δf {} | Nyquist {} | {:.0} ms",
            fft_source_label(workspace, *source_id),
            result.sample_count,
            result.dt,
            format_frequency(1.0 / (result.sample_count as f64 * result.dt)),
            format_frequency(0.5 / result.dt),
            duration_ms(result.elapsed)
        ));
    }
    egui::ScrollArea::horizontal()
        .id_salt("fft_result_summary_x")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new("fft_result_summary")
                .num_columns(4)
                .striped(true)
                .min_col_width(92.0)
                .show(ui, |ui| {
                    ui.label("Channel / legend");
                    ui.label("Peak freq");
                    ui.label("Peak amp");
                    ui.label("Peak dB");
                    ui.end_row();
                    for trace in fft_overlay_traces(results, targets) {
                        let spectrum = trace.spectrum;
                        let color = crate::channel_color(trace.color_index, ui.visuals().dark_mode);
                        ui.colored_label(
                            color,
                            format!("━ {}", compact_label(&spectrum.display_label, 26)),
                        )
                        .on_hover_text(&spectrum.tooltip);
                        ui.monospace(format_optional_frequency(spectrum.peak_freq_hz));
                        ui.monospace(format_optional_number(spectrum.peak_amp));
                        ui.monospace(format_optional_db(spectrum.peak_db));
                        ui.end_row();
                    }
                });
        });
}

fn fft_overlay_frequency_range(results: &BTreeMap<SourceId, FftResult>) -> Option<(f64, f64)> {
    results
        .values()
        .filter_map(|result| fft_full_frequency_range(&result.frequency_hz))
        .reduce(|left, right| (left.0.min(right.0), left.1.max(right.1)))
}

fn draw_fft_plot(
    ui: &mut egui::Ui,
    results: &BTreeMap<SourceId, FftResult>,
    targets: &[ChannelKey],
    y_scale: FftYScale,
    scroll_zoom: bool,
    freq_range: &mut FftFrequencyRange,
    hover_frequency: &mut Option<f64>,
    plot_zoom_drag: &mut Option<FftPlotZoomDrag>,
) {
    let desired = egui::vec2(ui.available_width().max(240.0), FFT_PLOT_HEIGHT);
    let (outer_rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    let plot_rect = egui::Rect::from_min_max(
        egui::pos2(
            outer_rect.left() + 56.0,
            outer_rect.top() + FFT_PLOT_TOP_MARGIN,
        ),
        egui::pos2(outer_rect.right() - 14.0, outer_rect.bottom() - 30.0),
    );
    let painter = ui.painter_at(outer_rect);
    painter.rect_filled(outer_rect, 0.0, ui.visuals().extreme_bg_color);
    draw_fft_plot_frame(&painter, plot_rect, ui.visuals());

    let Some(full_frequency_range) = fft_overlay_frequency_range(results) else {
        clear_fft_hover(ui, hover_frequency);
        *plot_zoom_drag = None;
        draw_fft_status_label(&painter, plot_rect, "Invalid FFT frequency range");
        return;
    };
    let Some((freq_min, freq_max)) =
        freq_range.bounds_for(&[full_frequency_range.0, full_frequency_range.1])
    else {
        clear_fft_hover(ui, hover_frequency);
        *plot_zoom_drag = None;
        draw_fft_status_label(&painter, plot_rect, "Invalid frequency range");
        return;
    };
    let min_frequency_span = results
        .values()
        .map(|result| fft_min_frequency_span(&result.frequency_hz, full_frequency_range))
        .fold(f64::INFINITY, f64::min);
    let (freq_min, freq_max) = handle_fft_plot_interaction(
        ui,
        &response,
        plot_rect,
        full_frequency_range,
        (freq_min, freq_max),
        min_frequency_span,
        scroll_zoom,
        freq_range,
        hover_frequency,
        plot_zoom_drag,
    );

    let traces = fft_overlay_traces(results, targets);
    let series = traces
        .iter()
        .map(|trace| {
            fft_display_series(trace.result, trace.spectrum, y_scale, (freq_min, freq_max))
        })
        .collect::<Vec<_>>();
    let Some((value_min, value_max)) = combined_series_value_range(&series) else {
        clear_fft_hover(ui, hover_frequency);
        *plot_zoom_drag = None;
        draw_fft_status_label(&painter, plot_rect, "No FFT bins in range");
        return;
    };
    let (value_min, value_max) = padded_value_range(value_min, value_max);
    let freq_span = freq_max - freq_min;
    let value_span = value_max - value_min;
    if freq_span <= 0.0 || value_span <= 0.0 {
        clear_fft_hover(ui, hover_frequency);
        *plot_zoom_drag = None;
        draw_fft_status_label(&painter, plot_rect, "Invalid FFT plot range");
        return;
    }

    let to_screen = |frequency: f64, value: f64| -> egui::Pos2 {
        let x_t = ((frequency - freq_min) / freq_span) as f32;
        let y_t = ((value - value_min) / value_span) as f32;
        egui::pos2(
            egui::lerp(plot_rect.left()..=plot_rect.right(), x_t),
            egui::lerp(plot_rect.bottom()..=plot_rect.top(), y_t),
        )
    };
    let clipped_painter = painter.with_clip_rect(plot_rect);
    for (index, points) in series.iter().enumerate() {
        if points.len() < 2 {
            continue;
        }
        let color = crate::channel_color(traces[index].color_index, ui.visuals().dark_mode);
        let screen_points = points
            .iter()
            .map(|(frequency, value)| to_screen(*frequency, *value))
            .collect::<Vec<_>>();
        clipped_painter.line(screen_points, egui::Stroke::new(1.4, color));
    }

    draw_fft_axis_labels(
        &painter,
        outer_rect,
        plot_rect,
        (freq_min, freq_max),
        (value_min, value_max),
        y_scale,
        ui.visuals(),
    );
    draw_fft_box_zoom_overlay(&painter, plot_rect, plot_zoom_drag.as_ref(), ui.visuals());
    draw_fft_hover_overlay(
        &painter,
        plot_rect,
        &traces,
        *hover_frequency,
        (freq_min, freq_max),
        y_scale,
        ui.visuals(),
    );
}

fn handle_fft_plot_interaction(
    ui: &mut egui::Ui,
    response: &egui::Response,
    plot_rect: egui::Rect,
    full_range: (f64, f64),
    current_range: (f64, f64),
    min_span: f64,
    scroll_zoom: bool,
    freq_range: &mut FftFrequencyRange,
    hover_frequency: &mut Option<f64>,
    plot_zoom_drag: &mut Option<FftPlotZoomDrag>,
) -> (f64, f64) {
    let visible_plot_rect = plot_rect.intersect(ui.clip_rect());
    if visible_plot_rect.width() <= 0.0 || visible_plot_rect.height() <= 0.0 {
        clear_fft_hover(ui, hover_frequency);
        *plot_zoom_drag = None;
        return current_range;
    }

    let mut next_range = clamp_fft_frequency_range(current_range, full_range, min_span);
    let mut changed = next_range != current_range;
    let (
        pointer_pos,
        press_origin,
        zoom_delta,
        scroll_delta,
        zoom_modifier_pressed,
        shift_pressed,
        primary_down,
        primary_released,
    ) = ui.input(|input| {
        (
            input.pointer.latest_pos(),
            input.pointer.press_origin(),
            input.zoom_delta(),
            input.smooth_scroll_delta(),
            input.modifiers.matches_any(egui::Modifiers::COMMAND),
            input.modifiers.shift,
            input.pointer.button_down(egui::PointerButton::Primary),
            input.pointer.button_released(egui::PointerButton::Primary),
        )
    });
    let pointer_over_plot = pointer_pos.is_some_and(|pos| visible_plot_rect.contains(pos));

    if response.double_clicked() && pointer_over_plot {
        *plot_zoom_drag = None;
        next_range = full_range;
        changed = true;
    } else if response.drag_started_by(egui::PointerButton::Primary)
        && shift_pressed
        && plot_zoom_drag.is_none()
    {
        if let Some(start_pos) = press_origin.or(pointer_pos)
            && visible_plot_rect.contains(start_pos)
        {
            *plot_zoom_drag = Some(FftPlotZoomDrag {
                plot_rect,
                start_pos,
                current_pos: start_pos,
                frequency_range: next_range,
            });
        }
    }

    if plot_zoom_drag.is_some() {
        if let Some(pointer_pos) = pointer_pos
            && let Some(drag) = plot_zoom_drag.as_mut()
        {
            drag.current_pos = pointer_pos;
        }

        let should_finish_zoom = primary_released
            || response.drag_stopped_by(egui::PointerButton::Primary)
            || !primary_down;
        if should_finish_zoom {
            if let Some(drag) = plot_zoom_drag.take() {
                if let Some(zoom_range) = fft_box_zoom_frequency_range(&drag, full_range, min_span)
                {
                    next_range = zoom_range;
                    changed = true;
                }
                set_fft_hover(
                    ui,
                    hover_frequency,
                    frequency_at_fft_plot_x(drag.current_pos.x, drag.plot_rect, next_range),
                );
            }
        } else {
            if let Some(drag) = plot_zoom_drag.as_ref() {
                set_fft_hover(
                    ui,
                    hover_frequency,
                    frequency_at_fft_plot_x(drag.current_pos.x, drag.plot_rect, next_range),
                );
            }
            ui.ctx().request_repaint();
        }

        if changed {
            set_fft_frequency_range(freq_range, next_range, full_range);
            ui.ctx().request_repaint();
        }
        return next_range;
    }

    let Some(pointer_pos) = pointer_pos.filter(|_| pointer_over_plot) else {
        clear_fft_hover(ui, hover_frequency);
        if changed {
            set_fft_frequency_range(freq_range, next_range, full_range);
            ui.ctx().request_repaint();
        }
        return next_range;
    };

    if !shift_pressed
        && response.dragged_by(egui::PointerButton::Primary)
        && plot_rect.width() > 1.0
    {
        let delta_x = f64::from(response.drag_delta().x);
        if delta_x != 0.0 {
            let span = next_range.1 - next_range.0;
            let shift = -delta_x / f64::from(plot_rect.width()) * span;
            next_range = pan_fft_frequency_range(next_range, full_range, shift, min_span);
            changed = true;
        }
    }

    if plot_rect.width() > 1.0 {
        if scroll_delta.x != 0.0 {
            let span = next_range.1 - next_range.0;
            let shift = -f64::from(scroll_delta.x) / f64::from(plot_rect.width()) * span;
            next_range = pan_fft_frequency_range(next_range, full_range, shift, min_span);
            changed = true;
            ui.input_mut(|input| {
                input.smooth_scroll_delta.x = 0.0;
            });
        }

        let wheel_zoom_enabled = scroll_zoom || zoom_modifier_pressed;
        let zoom_factor = fft_x_zoom_factor(zoom_delta, scroll_delta.y, wheel_zoom_enabled);
        if zoom_delta == 1.0 && wheel_zoom_enabled && scroll_delta.y != 0.0 {
            ui.input_mut(|input| {
                input.smooth_scroll_delta.y = 0.0;
            });
        }

        if zoom_factor.is_finite() && (zoom_factor - 1.0).abs() > f64::EPSILON {
            let anchor_ratio = ((pointer_pos.x - plot_rect.left()) / plot_rect.width()) as f64;
            next_range = zoom_fft_frequency_range(
                next_range,
                full_range,
                anchor_ratio,
                zoom_factor,
                min_span,
            );
            changed = true;
        }
    }

    if changed {
        set_fft_frequency_range(freq_range, next_range, full_range);
        ui.ctx().request_repaint();
    }

    set_fft_hover(
        ui,
        hover_frequency,
        frequency_at_fft_plot_x(pointer_pos.x, plot_rect, next_range),
    );

    next_range
}

fn clear_fft_hover(ui: &mut egui::Ui, hover_frequency: &mut Option<f64>) {
    set_fft_hover(ui, hover_frequency, None);
}

fn set_fft_hover(ui: &mut egui::Ui, hover_frequency: &mut Option<f64>, next_hover: Option<f64>) {
    if *hover_frequency != next_hover {
        *hover_frequency = next_hover;
        ui.ctx().request_repaint();
    }
}

fn frequency_at_fft_plot_x(x: f32, rect: egui::Rect, frequency_range: (f64, f64)) -> Option<f64> {
    let (start, end) = frequency_range;
    if !start.is_finite() || !end.is_finite() || end <= start || rect.width() <= 1.0 {
        return None;
    }

    let ratio = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
    Some(start + (end - start) * ratio)
}

fn fft_plot_x_for_frequency(
    frequency_hz: f64,
    rect: egui::Rect,
    frequency_range: (f64, f64),
) -> Option<f32> {
    let (start, end) = frequency_range;
    if !frequency_hz.is_finite() || !start.is_finite() || !end.is_finite() || end <= start {
        return None;
    }
    if rect.width() <= 1.0 {
        return None;
    }

    let ratio = ((frequency_hz - start) / (end - start)) as f32;
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        return None;
    }

    Some(egui::lerp(rect.left()..=rect.right(), ratio))
}

fn fft_full_frequency_range(frequencies: &[f64]) -> Option<(f64, f64)> {
    let start = frequencies
        .iter()
        .copied()
        .find(|value| value.is_finite())?;
    let end = frequencies
        .iter()
        .rev()
        .copied()
        .find(|value| value.is_finite())?;
    (end > start).then_some((start.max(0.0), end.max(0.0)))
}

fn fft_min_frequency_span(frequencies: &[f64], full_range: (f64, f64)) -> f64 {
    let full_span = full_range.1 - full_range.0;
    if !full_span.is_finite() || full_span <= 0.0 {
        return 0.0;
    }

    let min_bin_span = frequencies
        .windows(2)
        .filter_map(|pair| {
            let span = pair[1] - pair[0];
            (span.is_finite() && span > 0.0).then_some(span)
        })
        .fold(f64::INFINITY, f64::min);
    let min_bin_span = if min_bin_span.is_finite() {
        min_bin_span
    } else {
        full_span * 0.001
    };

    min_bin_span.max(full_span * 1.0e-9).min(full_span)
}

fn set_fft_frequency_range(
    freq_range: &mut FftFrequencyRange,
    range: (f64, f64),
    full_range: (f64, f64),
) {
    if fft_ranges_nearly_equal(range, full_range) {
        *freq_range = FftFrequencyRange::default();
    } else {
        freq_range.min_hz = Some(range.0.max(0.0));
        freq_range.max_hz = Some(range.1.max(0.0));
    }
}

fn fft_ranges_nearly_equal(left: (f64, f64), right: (f64, f64)) -> bool {
    let span = (right.1 - right.0).abs();
    let tolerance = (span * 1.0e-12).max(1.0e-9);
    (left.0 - right.0).abs() <= tolerance && (left.1 - right.1).abs() <= tolerance
}

fn clamp_fft_frequency_range(
    range: (f64, f64),
    full_range: (f64, f64),
    min_span: f64,
) -> (f64, f64) {
    let (full_start, full_end) = full_range;
    let full_span = full_end - full_start;
    if !full_start.is_finite()
        || !full_end.is_finite()
        || full_end <= full_start
        || !full_span.is_finite()
    {
        return full_range;
    }

    let min_span = min_span.max(f64::EPSILON).min(full_span);
    let (start, end) = if range.0 <= range.1 {
        (range.0, range.1)
    } else {
        (range.1, range.0)
    };
    if !start.is_finite() || !end.is_finite() {
        return full_range;
    }

    let span = (end - start).clamp(min_span, full_span);
    let center = (start + end) * 0.5;
    fft_frequency_range_with_start(center - span * 0.5, span, full_range)
}

fn pan_fft_frequency_range(
    range: (f64, f64),
    full_range: (f64, f64),
    shift: f64,
    min_span: f64,
) -> (f64, f64) {
    let (start, end) = clamp_fft_frequency_range(range, full_range, min_span);
    let span = end - start;
    fft_frequency_range_with_start(start + shift, span, full_range)
}

fn zoom_fft_frequency_range(
    range: (f64, f64),
    full_range: (f64, f64),
    anchor_ratio: f64,
    zoom_factor: f64,
    min_span: f64,
) -> (f64, f64) {
    let (full_start, full_end) = full_range;
    let full_span = full_end - full_start;
    if !full_span.is_finite() || full_span <= 0.0 {
        return full_range;
    }

    let (start, end) = clamp_fft_frequency_range(range, full_range, min_span);
    let span = end - start;
    let anchor_ratio = anchor_ratio.clamp(0.0, 1.0);
    let anchor_frequency = start + span * anchor_ratio;
    let min_span = min_span.max(f64::EPSILON).min(full_span);
    let next_span = (span * zoom_factor).clamp(min_span, full_span);
    let next_start = anchor_frequency - next_span * anchor_ratio;

    fft_frequency_range_with_start(next_start, next_span, (full_start, full_end))
}

fn fft_frequency_range_with_start(start: f64, span: f64, full_range: (f64, f64)) -> (f64, f64) {
    let (full_start, full_end) = full_range;
    let full_span = full_end - full_start;
    if span >= full_span {
        return full_range;
    }

    let span = span.max(f64::EPSILON).min(full_span);
    let start = start.clamp(full_start, full_end - span);
    (start, start + span)
}

fn fft_x_zoom_factor(zoom_delta: f32, scroll_delta_y: f32, wheel_zoom_enabled: bool) -> f64 {
    if zoom_delta != 1.0 {
        1.0 / f64::from(zoom_delta)
    } else if wheel_zoom_enabled && scroll_delta_y != 0.0 {
        (-f64::from(scroll_delta_y) * FFT_PLOT_WHEEL_ZOOM_SENSITIVITY).exp()
    } else {
        1.0
    }
}

fn fft_box_zoom_screen_rect(zoom_drag: &FftPlotZoomDrag) -> Option<egui::Rect> {
    if zoom_drag.plot_rect.width() <= 1.0 {
        return None;
    }

    let start_x = zoom_drag
        .start_pos
        .x
        .clamp(zoom_drag.plot_rect.left(), zoom_drag.plot_rect.right());
    let current_x = zoom_drag
        .current_pos
        .x
        .clamp(zoom_drag.plot_rect.left(), zoom_drag.plot_rect.right());
    if (current_x - start_x).abs() <= f32::EPSILON {
        return None;
    }

    Some(egui::Rect::from_min_max(
        egui::pos2(start_x.min(current_x), zoom_drag.plot_rect.top()),
        egui::pos2(start_x.max(current_x), zoom_drag.plot_rect.bottom()),
    ))
}

fn fft_box_zoom_frequency_range(
    zoom_drag: &FftPlotZoomDrag,
    full_range: (f64, f64),
    min_span: f64,
) -> Option<(f64, f64)> {
    let selection_rect = fft_box_zoom_screen_rect(zoom_drag)?;
    if selection_rect.width() < FFT_PLOT_MIN_ZOOM_PIXEL_WIDTH {
        return None;
    }

    let start = frequency_at_fft_plot_x(
        selection_rect.left(),
        zoom_drag.plot_rect,
        zoom_drag.frequency_range,
    )?;
    let end = frequency_at_fft_plot_x(
        selection_rect.right(),
        zoom_drag.plot_rect,
        zoom_drag.frequency_range,
    )?;

    Some(clamp_fft_frequency_range(
        (start, end),
        full_range,
        min_span,
    ))
}

fn draw_fft_box_zoom_overlay(
    painter: &egui::Painter,
    rect: egui::Rect,
    zoom_drag: Option<&FftPlotZoomDrag>,
    visuals: &egui::Visuals,
) {
    let Some(zoom_drag) = zoom_drag else {
        return;
    };
    if zoom_drag.plot_rect != rect {
        return;
    }
    let Some(selection_rect) = fft_box_zoom_screen_rect(zoom_drag) else {
        return;
    };

    painter.rect_filled(
        selection_rect,
        0.0,
        visuals.selection.bg_fill.linear_multiply(0.35),
    );
    painter.rect_stroke(
        selection_rect,
        0.0,
        visuals.selection.stroke,
        egui::StrokeKind::Inside,
    );
}

fn draw_fft_hover_overlay(
    painter: &egui::Painter,
    plot_rect: egui::Rect,
    traces: &[FftPlotTrace<'_>],
    hover_frequency: Option<f64>,
    frequency_range: (f64, f64),
    y_scale: FftYScale,
    visuals: &egui::Visuals,
) {
    let Some(hover_frequency) = hover_frequency else {
        return;
    };
    let Some(hover_line_x) = fft_plot_x_for_frequency(hover_frequency, plot_rect, frequency_range)
    else {
        return;
    };

    let hover_stroke = egui::Stroke::new(1.0, visuals.selection.stroke.color);
    painter.with_clip_rect(plot_rect).line_segment(
        [
            egui::pos2(hover_line_x, plot_rect.top()),
            egui::pos2(hover_line_x, plot_rect.bottom()),
        ],
        hover_stroke,
    );
    draw_fft_hover_readout(
        painter,
        plot_rect,
        traces,
        hover_frequency,
        hover_line_x,
        y_scale,
        visuals,
    );
}

struct FftHoverReadoutValue {
    color: egui::Color32,
    label: String,
    value: String,
}

fn draw_fft_hover_readout(
    painter: &egui::Painter,
    rect: egui::Rect,
    traces: &[FftPlotTrace<'_>],
    hover_frequency: f64,
    hover_line_x: f32,
    y_scale: FftYScale,
    visuals: &egui::Visuals,
) {
    let readouts = traces
        .iter()
        .filter_map(|trace| {
            let (bin_frequency, amplitude) =
                fft_hover_sample(trace.result, trace.spectrum, hover_frequency)?;
            Some(FftHoverReadoutValue {
                color: crate::channel_color(trace.color_index, visuals.dark_mode),
                label: compact_label(&trace.spectrum.display_label, 22),
                value: format!(
                    "{} @ {}",
                    format_fft_hover_value(amplitude, y_scale),
                    format_frequency(bin_frequency)
                ),
            })
        })
        .collect::<Vec<_>>();
    if readouts.is_empty() {
        return;
    }

    let visible_count = readouts.len().min(MAX_FFT_HOVER_READOUT_TRACES);
    let extra_count = readouts.len().saturating_sub(visible_count);
    let mut lines = Vec::with_capacity(visible_count + 4);
    lines.push(format!("f {}", format_frequency(hover_frequency)));
    lines.push(format!(
        "T {}",
        format_period_for_frequency(hover_frequency)
    ));
    for readout in readouts.iter().take(visible_count) {
        lines.push(format!("{}  {}", readout.label, readout.value));
    }
    if extra_count > 0 {
        lines.push(format!("+{extra_count} more"));
    }

    let font = egui::FontId::monospace(11.0);
    let line_height = 15.0;
    let max_panel_width = (rect.width() - 8.0).max(64.0).min(480.0);
    let panel_width = lines
        .iter()
        .map(|line| line.chars().count() as f32)
        .fold(0.0, f32::max)
        .mul_add(7.2, 18.0)
        .clamp(64.0, max_panel_width);
    let panel_height = lines.len() as f32 * line_height + 12.0;
    let panel_left = if hover_line_x < rect.center().x {
        rect.right() - panel_width - 8.0
    } else {
        rect.left() + 8.0
    }
    .max(rect.left() + 4.0)
    .min(rect.right() - panel_width - 4.0);
    let panel_top = rect.top() + 8.0;
    let panel_rect = egui::Rect::from_min_size(
        egui::pos2(panel_left, panel_top),
        egui::vec2(panel_width, panel_height),
    );
    let fill = if visuals.dark_mode {
        egui::Color32::from_black_alpha(190)
    } else {
        egui::Color32::from_white_alpha(220)
    };
    painter.rect_filled(panel_rect, 3.0, fill);
    painter.rect_stroke(
        panel_rect,
        3.0,
        egui::Stroke::new(1.0, visuals.widgets.noninteractive.fg_stroke.color),
        egui::StrokeKind::Inside,
    );

    let text_color = visuals.text_color();
    let mut y = panel_rect.top() + 6.0;
    for line in lines.iter().take(2) {
        painter.text(
            egui::pos2(panel_rect.left() + 8.0, y),
            egui::Align2::LEFT_TOP,
            line,
            font.clone(),
            text_color,
        );
        y += line_height;
    }

    for (readout, line) in readouts
        .iter()
        .take(visible_count)
        .zip(lines.iter().skip(2))
    {
        let swatch_y = y + line_height * 0.5;
        painter.line_segment(
            [
                egui::pos2(panel_rect.left() + 8.0, swatch_y),
                egui::pos2(panel_rect.left() + 18.0, swatch_y),
            ],
            egui::Stroke::new(2.0, readout.color),
        );
        painter.text(
            egui::pos2(panel_rect.left() + 24.0, y),
            egui::Align2::LEFT_TOP,
            line,
            font.clone(),
            text_color,
        );
        y += line_height;
    }

    if extra_count > 0
        && let Some(extra_line) = lines.last()
    {
        painter.text(
            egui::pos2(panel_rect.left() + 24.0, y),
            egui::Align2::LEFT_TOP,
            extra_line,
            font,
            visuals.weak_text_color(),
        );
    }
}

fn fft_display_series(
    result: &FftResult,
    spectrum: &FftChannelSpectrum,
    y_scale: FftYScale,
    frequency_range: (f64, f64),
) -> Vec<(f64, f64)> {
    let (freq_min, freq_max) = frequency_range;
    let frequencies = &result.frequency_hz;
    let start = frequencies.partition_point(|frequency| *frequency < freq_min);
    let end = frequencies.partition_point(|frequency| *frequency <= freq_max);
    if start >= end || start >= spectrum.amplitude.len() {
        return Vec::new();
    }
    let end = end.min(spectrum.amplitude.len());
    let count = end - start;

    let value_at = |index: usize| -> f64 {
        let amplitude = spectrum.amplitude[index];
        match y_scale {
            FftYScale::Amplitude => amplitude,
            FftYScale::Db => amplitude_to_db(amplitude),
        }
    };

    if count <= MAX_FFT_DRAW_POINTS_PER_TRACE {
        return (start..end)
            .map(|index| (frequencies[index], value_at(index)))
            .collect();
    }

    let bucket_count = (MAX_FFT_DRAW_POINTS_PER_TRACE / 2).max(1);
    let mut points = Vec::with_capacity(bucket_count * 2);
    for bucket in 0..bucket_count {
        let bucket_start = start + bucket * count / bucket_count;
        let bucket_end = start + (bucket + 1) * count / bucket_count;
        if bucket_start >= bucket_end {
            continue;
        }

        let mut min_index = bucket_start;
        let mut max_index = bucket_start;
        let mut min_value = value_at(bucket_start);
        let mut max_value = min_value;
        for index in bucket_start + 1..bucket_end {
            let value = value_at(index);
            if value < min_value {
                min_value = value;
                min_index = index;
            }
            if value > max_value {
                max_value = value;
                max_index = index;
            }
        }

        if min_index <= max_index {
            points.push((frequencies[min_index], min_value));
            if max_index != min_index {
                points.push((frequencies[max_index], max_value));
            }
        } else {
            points.push((frequencies[max_index], max_value));
            points.push((frequencies[min_index], min_value));
        }
    }

    points
}

fn combined_series_value_range(series: &[Vec<(f64, f64)>]) -> Option<(f64, f64)> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut found = false;

    for value in series
        .iter()
        .flat_map(|points| points.iter().map(|(_, value)| *value))
        .filter(|value| value.is_finite())
    {
        min = min.min(value);
        max = max.max(value);
        found = true;
    }

    found.then_some((min, max))
}

fn fft_hover_sample(
    result: &FftResult,
    spectrum: &FftChannelSpectrum,
    frequency: f64,
) -> Option<(f64, f64)> {
    let first = *result.frequency_hz.first()?;
    let last = *result.frequency_hz.last()?;
    if !frequency.is_finite() || frequency < first || frequency > last {
        return None;
    }
    let index = nearest_fft_bin_index(&result.frequency_hz, frequency)?;
    let amplitude = *spectrum.amplitude.get(index)?;
    amplitude
        .is_finite()
        .then_some((result.frequency_hz[index], amplitude))
}

fn nearest_fft_bin_index(frequencies: &[f64], target_frequency_hz: f64) -> Option<usize> {
    if !target_frequency_hz.is_finite() || frequencies.is_empty() {
        return None;
    }

    let insertion_index = frequencies.partition_point(|frequency| *frequency < target_frequency_hz);
    let candidates = [
        insertion_index.checked_sub(1),
        (insertion_index < frequencies.len()).then_some(insertion_index),
    ];
    let mut best = None;

    for index in candidates.into_iter().flatten() {
        let frequency = frequencies[index];
        if !frequency.is_finite() {
            continue;
        }

        let distance = (frequency - target_frequency_hz).abs();
        match best {
            Some((best_distance, _)) if best_distance <= distance => {}
            _ => best = Some((distance, index)),
        }
    }

    best.map(|(_, index)| index)
}

fn padded_value_range(min: f64, max: f64) -> (f64, f64) {
    if min == max {
        let pad = min.abs().max(1.0) * 0.05;
        return (min - pad, max + pad);
    }

    let pad = (max - min).abs() * 0.05;
    (min - pad, max + pad)
}

fn draw_fft_plot_frame(painter: &egui::Painter, rect: egui::Rect, visuals: &egui::Visuals) {
    let stroke = egui::Stroke::new(1.0, visuals.widgets.noninteractive.fg_stroke.color);
    let grid_stroke = egui::Stroke::new(1.0, visuals.faint_bg_color);

    for i in 1..5 {
        let t = i as f32 / 5.0;
        let x = egui::lerp(rect.left()..=rect.right(), t);
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            grid_stroke,
        );
        let y = egui::lerp(rect.top()..=rect.bottom(), t);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            grid_stroke,
        );
    }

    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
}

fn draw_fft_status_label(painter: &egui::Painter, rect: egui::Rect, label: &str) {
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(14.0),
        egui::Color32::GRAY,
    );
}

fn draw_fft_axis_labels(
    painter: &egui::Painter,
    outer_rect: egui::Rect,
    plot_rect: egui::Rect,
    frequency_range: (f64, f64),
    value_range: (f64, f64),
    y_scale: FftYScale,
    visuals: &egui::Visuals,
) {
    let color = visuals.widgets.noninteractive.fg_stroke.color;
    painter.text(
        egui::pos2(plot_rect.left(), outer_rect.bottom() - 15.0),
        egui::Align2::LEFT_CENTER,
        format_frequency(frequency_range.0),
        egui::FontId::monospace(11.0),
        color,
    );
    painter.text(
        egui::pos2(plot_rect.right(), outer_rect.bottom() - 15.0),
        egui::Align2::RIGHT_CENTER,
        format_frequency(frequency_range.1),
        egui::FontId::monospace(11.0),
        color,
    );
    painter.text(
        egui::pos2(plot_rect.center().x, outer_rect.bottom() - 15.0),
        egui::Align2::CENTER_CENTER,
        "Frequency [Hz]",
        egui::FontId::proportional(11.0),
        color,
    );
    painter.text(
        egui::pos2(plot_rect.left(), outer_rect.top() + 4.0),
        egui::Align2::LEFT_TOP,
        match y_scale {
            FftYScale::Amplitude => "Amplitude",
            FftYScale::Db => "Magnitude [dB]",
        },
        egui::FontId::proportional(11.0),
        color,
    );
    painter.text(
        egui::pos2(plot_rect.left() - 4.0, plot_rect.top()),
        egui::Align2::RIGHT_TOP,
        format_fft_axis_value(value_range.1, y_scale),
        egui::FontId::monospace(11.0),
        color,
    );
    painter.text(
        egui::pos2(plot_rect.left() - 4.0, plot_rect.bottom()),
        egui::Align2::RIGHT_BOTTOM,
        format_fft_axis_value(value_range.0, y_scale),
        egui::FontId::monospace(11.0),
        color,
    );
}

fn compact_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_owned();
    }
    // Source identity must survive truncation of long, otherwise identical names.
    let suffix = label
        .rsplit_once(" #")
        .filter(|(_, number)| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
        .map(|(_, number)| format!(" #{number}"))
        .unwrap_or_default();
    let keep = max_chars.saturating_sub(3 + suffix.chars().count()).max(1);
    let mut compact = label.chars().take(keep).collect::<String>();
    compact.push_str("...");
    compact.push_str(&suffix);
    compact
}

fn format_optional_frequency(value: Option<f64>) -> String {
    value
        .map(format_frequency)
        .unwrap_or_else(|| "-".to_owned())
}

fn format_optional_number(value: Option<f64>) -> String {
    value.map(format_number).unwrap_or_else(|| "-".to_owned())
}

fn format_optional_db(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3} dB"))
        .unwrap_or_else(|| "-".to_owned())
}

fn format_fft_hover_value(amplitude: f64, y_scale: FftYScale) -> String {
    match y_scale {
        FftYScale::Amplitude => format_number(amplitude),
        FftYScale::Db => format!("{:.3} dB", amplitude_to_db(amplitude)),
    }
}

fn format_fft_axis_value(value: f64, y_scale: FftYScale) -> String {
    if !value.is_finite() {
        return "-".to_owned();
    }

    match y_scale {
        FftYScale::Amplitude => format_compact_axis_number(value),
        FftYScale::Db => trim_decimal_zeros(format!("{value:.1}")),
    }
}

fn format_compact_axis_number(value: f64) -> String {
    let abs = value.abs();
    if abs <= f64::EPSILON {
        return "0".to_owned();
    }

    if !(1.0e-3..1.0e4).contains(&abs) {
        return format!("{value:.2e}");
    }

    let formatted = if abs >= 1_000.0 {
        format!("{value:.0}")
    } else if abs >= 100.0 {
        format!("{value:.1}")
    } else if abs >= 10.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.3}")
    };

    trim_decimal_zeros(formatted)
}

fn trim_decimal_zeros(mut value: String) -> String {
    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
    }

    value
}

fn format_frequency(frequency_hz: f64) -> String {
    if !frequency_hz.is_finite() {
        return "-".to_owned();
    }
    let frequency_hz = frequency_hz.abs();
    if frequency_hz >= 1_000_000.0 {
        format!("{:.3} MHz", frequency_hz / 1_000_000.0)
    } else if frequency_hz >= 1_000.0 {
        format!("{:.3} kHz", frequency_hz / 1_000.0)
    } else {
        format!("{frequency_hz:.3} Hz")
    }
}

fn format_period_for_frequency(frequency_hz: f64) -> String {
    let frequency_hz = frequency_hz.abs();
    if !frequency_hz.is_finite() || frequency_hz <= f64::EPSILON {
        return "-".to_owned();
    }

    format_period_seconds(1.0 / frequency_hz)
}

fn format_period_seconds(period_seconds: f64) -> String {
    if !period_seconds.is_finite() {
        return "-".to_owned();
    }

    if period_seconds >= 1.0 {
        format!("{period_seconds:.6} s")
    } else if period_seconds >= 1.0e-3 {
        format!("{:.3} ms", period_seconds * 1.0e3)
    } else if period_seconds >= 1.0e-6 {
        format!("{:.3} us", period_seconds * 1.0e6)
    } else if period_seconds >= 1.0e-9 {
        format!("{:.3} ns", period_seconds * 1.0e9)
    } else {
        format!("{period_seconds:.3e} s")
    }
}

fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_owned();
    }

    let abs = value.abs();
    if value == 0.0 || (1.0e-3..1.0e5).contains(&abs) {
        format!("{value:.6}")
    } else {
        format!("{value:.6e}")
    }
}

fn bytes_to_mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }

    fn tiny_key(window: FftWindowKind, index_start: usize) -> FftRequestKey {
        FftRequestKey {
            source_id: crate::DEFAULT_SOURCE_ID,
            source_generation: 0,
            time_offset_bits: 0.0f64.to_bits(),
            cursor_a_bits: 0.1f64.to_bits(),
            cursor_b_bits: 0.9f64.to_bits(),
            time_sample_count: 8,
            time_start_bits: 0.0f64.to_bits(),
            time_end_bits: (7.0f64 * 0.1).to_bits(),
            window,
            index_start,
            index_end_exclusive: 8,
            channels: vec![FftRequestChannelKey {
                key: ChannelKey::new(crate::DEFAULT_SOURCE_ID, "sine_50Hz"),
                sample_count: 8,
            }],
        }
    }

    #[test]
    fn fft_request_identity_includes_channel_source() {
        let source_one_key = tiny_key(FftWindowKind::Hanning, 0);
        let mut source_two_key = source_one_key.clone();
        source_two_key.channels[0].key =
            ChannelKey::new(crate::workspace_data::SourceId::new(2), "sine_50Hz");

        assert_ne!(source_one_key, source_two_key);
    }

    #[test]
    fn window_functions_match_numpy_definitions() {
        let hann = (0..4)
            .map(|index| fft_window_value(FftWindowKind::Hanning, 4, index))
            .collect::<Vec<_>>();
        assert_close(hann[0], 0.0, 1.0e-12);
        assert_close(hann[1], 0.75, 1.0e-12);
        assert_close(hann[2], 0.75, 1.0e-12);
        assert_close(hann[3], 0.0, 1.0e-12);

        let hamming = (0..4)
            .map(|index| fft_window_value(FftWindowKind::Hamming, 4, index))
            .collect::<Vec<_>>();
        assert_close(hamming[0], 0.08, 1.0e-12);
        assert_close(hamming[1], 0.77, 1.0e-12);
        assert_close(hamming[2], 0.77, 1.0e-12);
        assert_close(hamming[3], 0.08, 1.0e-12);

        for index in 0..4 {
            assert_close(
                fft_window_value(FftWindowKind::Rectangular, 4, index),
                1.0,
                1.0e-12,
            );
        }
    }

    #[test]
    fn amplitude_db_uses_floor() {
        assert_close(amplitude_to_db(10.0), 20.0, 1.0e-12);
        assert_close(amplitude_to_db(0.0), -240.0, 1.0e-12);
        assert_close(amplitude_to_db(f64::NAN), -240.0, 1.0e-12);
    }

    #[test]
    fn fft_axis_value_formatter_keeps_labels_compact() {
        assert_eq!(format_fft_axis_value(0.0, FftYScale::Amplitude), "0");
        assert_eq!(
            format_fft_axis_value(0.000_001_234, FftYScale::Amplitude),
            "1.23e-6"
        );
        assert_eq!(
            format_fft_axis_value(0.012_345, FftYScale::Amplitude),
            "0.012"
        );
        assert_eq!(format_fft_axis_value(12.345, FftYScale::Amplitude), "12.35");
        assert_eq!(format_fft_axis_value(1234.56, FftYScale::Amplitude), "1235");
        assert_eq!(format_fft_axis_value(-239.987, FftYScale::Db), "-240");
        assert_eq!(format_fft_axis_value(13.979, FftYScale::Db), "14");
    }

    #[test]
    fn nearest_fft_bin_index_uses_closest_frequency() {
        let frequencies = [0.0, 10.0, 20.0, 40.0];

        assert_eq!(nearest_fft_bin_index(&frequencies, -2.0), Some(0));
        assert_eq!(nearest_fft_bin_index(&frequencies, 14.9), Some(1));
        assert_eq!(nearest_fft_bin_index(&frequencies, 15.0), Some(1));
        assert_eq!(nearest_fft_bin_index(&frequencies, 15.1), Some(2));
        assert_eq!(nearest_fft_bin_index(&frequencies, 100.0), Some(3));
    }

    #[test]
    fn nearest_fft_bin_index_rejects_empty_or_invalid_target() {
        assert_eq!(nearest_fft_bin_index(&[], 10.0), None);
        assert_eq!(nearest_fft_bin_index(&[0.0, 10.0], f64::NAN), None);
        assert_eq!(nearest_fft_bin_index(&[0.0, 10.0], f64::INFINITY), None);
    }

    #[test]
    fn fft_zoom_range_anchors_and_respects_min_span() {
        let zoomed = zoom_fft_frequency_range((0.0, 100.0), (0.0, 100.0), 0.25, 0.5, 1.0);
        assert_close(zoomed.0, 12.5, 1.0e-12);
        assert_close(zoomed.1, 62.5, 1.0e-12);

        let clamped = zoom_fft_frequency_range((90.0, 100.0), (0.0, 100.0), 1.0, 0.5, 1.0);
        assert_close(clamped.0, 95.0, 1.0e-12);
        assert_close(clamped.1, 100.0, 1.0e-12);

        let min_span = zoom_fft_frequency_range((10.0, 11.0), (0.0, 100.0), 0.5, 0.01, 2.0);
        assert_close(min_span.0, 9.5, 1.0e-12);
        assert_close(min_span.1, 11.5, 1.0e-12);
    }

    #[test]
    fn fft_box_zoom_range_accepts_reversed_drag() {
        let drag = FftPlotZoomDrag {
            plot_rect: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 100.0)),
            start_pos: egui::pos2(150.0, 50.0),
            current_pos: egui::pos2(50.0, 50.0),
            frequency_range: (0.0, 100.0),
        };

        let range = fft_box_zoom_frequency_range(&drag, (0.0, 100.0), 1.0).expect("box zoom range");

        assert_close(range.0, 25.0, 1.0e-12);
        assert_close(range.1, 75.0, 1.0e-12);
    }

    #[test]
    fn fft_box_zoom_range_ignores_tiny_drags() {
        let drag = FftPlotZoomDrag {
            plot_rect: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 100.0)),
            start_pos: egui::pos2(20.0, 50.0),
            current_pos: egui::pos2(21.0, 50.0),
            frequency_range: (0.0, 100.0),
        };

        assert_eq!(fft_box_zoom_frequency_range(&drag, (0.0, 100.0), 1.0), None);
    }

    #[test]
    fn fft_frequency_range_writeback_clears_full_range() {
        let mut range = FftFrequencyRange {
            min_hz: Some(1.0),
            max_hz: Some(99.0),
        };

        set_fft_frequency_range(&mut range, (0.0, 100.0), (0.0, 100.0));
        assert_eq!(range, FftFrequencyRange::default());

        set_fft_frequency_range(&mut range, (10.0, 20.0), (0.0, 100.0));
        assert_eq!(
            range,
            FftFrequencyRange {
                min_hz: Some(10.0),
                max_hz: Some(20.0)
            }
        );
    }

    #[test]
    fn stale_fft_job_results_are_rejected() {
        let key = tiny_key(FftWindowKind::Hanning, 0);
        let stale_key = tiny_key(FftWindowKind::Hamming, 0);
        let mut workspace = tiny_workspace();
        let mut fft = FftAnalysisState::default();
        fft.jobs.insert(
            key.source_id,
            FftJobState::Running {
                job_id: 7,
                key: key.clone(),
                sample_count: 8,
                channel_count: 1,
                started: Instant::now(),
            },
        );

        assert!(fft.accepts_job_result(7, &key, &workspace));
        assert!(!fft.accepts_job_result(8, &key, &workspace));
        assert!(!fft.accepts_job_result(7, &stale_key, &workspace));

        workspace
            .source_mut(crate::DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .generation += 1;
        assert!(!fft.accepts_job_result(7, &key, &workspace));
    }

    #[test]
    fn mixed_source_fft_is_rejected_before_input_copy() {
        let mut workspace = tiny_workspace();
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        install_tiny_fft_source(&mut workspace, source_two, 10.0, &["sine_50Hz"]);
        let mut view = crate::ViewState::default();
        view.rows[0].channels = vec![
            crate::RowChannel::new(ChannelKey::new(crate::DEFAULT_SOURCE_ID, "sine_50Hz"), 0),
            crate::RowChannel::new(ChannelKey::new(source_two, "sine_50Hz"), 1),
        ];
        let fft = FftAnalysisState {
            target_channels: vec![
                ChannelKey::new(crate::DEFAULT_SOURCE_ID, "sine_50Hz"),
                ChannelKey::new(source_two, "sine_50Hz"),
            ],
            target_initialized: true,
            ..FftAnalysisState::default()
        };

        let error = build_fft_job_input(&fft, Some(0.1), Some(0.6), &workspace, &view)
            .expect_err("mixed-source FFT must fail");

        assert!(error.contains("mixed-source FFT was not started"));
    }

    #[test]
    fn same_source_fft_uses_raw_cursor_times_without_changing_dt() {
        let mut workspace = crate::WorkspaceDataState::default();
        install_tiny_fft_source(
            &mut workspace,
            crate::DEFAULT_SOURCE_ID,
            10.0,
            &["sine_50Hz", "cosine_50Hz"],
        );
        workspace
            .source_mut(crate::DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .time_transform = crate::TimeTransform {
            offset_seconds: -10.0,
        };
        let mut view = crate::ViewState::default();
        view.rows[0].channels = vec![
            crate::RowChannel::new(ChannelKey::new(crate::DEFAULT_SOURCE_ID, "sine_50Hz"), 0),
            crate::RowChannel::new(ChannelKey::new(crate::DEFAULT_SOURCE_ID, "cosine_50Hz"), 1),
        ];
        let fft = FftAnalysisState {
            target_channels: vec![
                ChannelKey::new(crate::DEFAULT_SOURCE_ID, "sine_50Hz"),
                ChannelKey::new(crate::DEFAULT_SOURCE_ID, "cosine_50Hz"),
            ],
            target_initialized: true,
            ..FftAnalysisState::default()
        };

        let input = build_fft_job_input(&fft, Some(0.1), Some(0.6), &workspace, &view)
            .expect("same-source FFT input");

        assert_eq!(input.channel_count(), 2);
        assert_eq!(input.sample_count(), 6);
        assert!((input.dt - 0.1).abs() < 1.0e-12);
        assert!((input.time[0] - 10.1).abs() < 1.0e-12);
        assert!((input.time[5] - 10.6).abs() < 1.0e-12);
    }

    #[test]
    fn source_cleanup_removes_targets_and_invalidates_running_job() {
        let source_two = SourceId::new(2);
        let source_one_key = ChannelKey::new(crate::DEFAULT_SOURCE_ID, "iu");
        let source_two_key = ChannelKey::new(source_two, "iu");
        let mut request_key = tiny_key(FftWindowKind::Hanning, 0);
        request_key.channels[0].key = source_two_key.clone();
        let mut fft = FftAnalysisState::default();
        fft.target_channels = vec![source_one_key.clone(), source_two_key];
        fft.target_initialized = true;
        fft.results.insert(
            source_two,
            FftResult {
                key: request_key.clone(),
                cursor_a: 0.0,
                cursor_b: 1.0,
                index_start: 0,
                index_end_exclusive: 8,
                sample_count: 8,
                dt: 0.125,
                frequency_hz: vec![0.0],
                spectra: vec![FftChannelSpectrum {
                    key: ChannelKey::new(source_two, "iu"),
                    channel_name: "iu".to_owned(),
                    channel_path: "iu".to_owned(),
                    display_label: "iu #2".to_owned(),
                    tooltip: "source_2.parquet / iu".to_owned(),
                    amplitude: vec![1.0],
                    peak_freq_hz: Some(0.0),
                    peak_amp: Some(1.0),
                    peak_db: Some(0.0),
                }],
                elapsed: Duration::ZERO,
            },
        );
        fft.jobs.insert(
            source_two,
            FftJobState::Running {
                job_id: 9,
                key: request_key,
                sample_count: 8,
                channel_count: 1,
                started: Instant::now(),
            },
        );
        fft.error = Some("old".to_owned());

        fft.cleanup_source(source_two);

        assert_eq!(fft.target_channels, vec![source_one_key]);
        assert!(fft.jobs.is_empty());
        assert!(fft.results.is_empty());
        assert!(fft.error.is_none());
    }

    #[test]
    fn compute_fft_matches_phase0_reference_peaks() {
        let Some(input) = reference_fft_input() else {
            return;
        };
        let result = compute_fft(input).expect("fft result");
        assert_eq!(result.sample_count, 80_001);

        let sine = result
            .spectra
            .iter()
            .find(|spectrum| spectrum.channel_path == "sine_50Hz")
            .expect("sine spectrum");
        assert_close(
            sine.peak_freq_hz.expect("sine peak freq"),
            49.99937500783179,
            0.1,
        );
        assert_close(
            sine.peak_amp.expect("sine peak amp"),
            4.9999366897603945,
            1.0e-4,
        );
        assert_close(
            sine.peak_db.expect("sine peak db"),
            13.979290104873243,
            1.0e-3,
        );

        let pwm = result
            .spectra
            .iter()
            .find(|spectrum| spectrum.channel_path == "pwm_1kHz")
            .expect("pwm spectrum");
        assert_close(
            pwm.peak_freq_hz.expect("pwm peak freq"),
            999.9875001566357,
            0.1,
        );
        assert_close(
            pwm.peak_amp.expect("pwm peak amp"),
            1.2867577545787694,
            1.0e-4,
        );
        assert_close(
            pwm.peak_db.expect("pwm peak db"),
            2.1899358836646003,
            1.0e-3,
        );

        let chirp = result
            .spectra
            .iter()
            .find(|spectrum| spectrum.channel_path == "chirp_1_500Hz")
            .expect("chirp spectrum");
        assert_close(
            chirp.peak_freq_hz.expect("chirp peak freq"),
            249.99687503915894,
            0.1,
        );
        assert_close(
            chirp.peak_amp.expect("chirp peak amp"),
            0.2797771990412082,
            1.0e-4,
        );
        assert_close(
            chirp.peak_db.expect("chirp peak db"),
            -11.063753640634989,
            1.0e-3,
        );
    }

    fn reference_fft_input() -> Option<FftJobInput> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/test_100k.parquet");
        if !path.exists() {
            return None;
        }

        let summary = crate::parquet_schema::read_schema_summary(&path).expect("read schema");
        let time_data =
            crate::parquet_waveform::read_time_column(&path, &summary).expect("read time column");
        let channel_data = ["sine_50Hz", "pwm_1kHz", "chirp_1_500Hz"]
            .into_iter()
            .map(|channel| {
                crate::parquet_waveform::read_channel_values(&path, &summary, channel)
                    .expect("read channel values")
            })
            .collect::<Vec<_>>();

        let time = &time_data.time;
        let sample_count = time.len();
        let index_start = 10_000usize;
        let index_end_exclusive = 90_001usize;
        let cursor_a = 0.1f64;
        let cursor_b = 0.9f64;
        let dt = time[index_start + 1] - time[index_start];
        let key = FftRequestKey {
            source_id: crate::DEFAULT_SOURCE_ID,
            source_generation: 0,
            time_offset_bits: 0.0f64.to_bits(),
            cursor_a_bits: cursor_a.to_bits(),
            cursor_b_bits: cursor_b.to_bits(),
            time_sample_count: sample_count,
            time_start_bits: time[0].to_bits(),
            time_end_bits: time[sample_count - 1].to_bits(),
            window: FftWindowKind::Hanning,
            index_start,
            index_end_exclusive,
            channels: channel_data
                .iter()
                .map(|channel| FftRequestChannelKey {
                    key: ChannelKey::new(crate::DEFAULT_SOURCE_ID, channel.channel_path.clone()),
                    sample_count: channel.values.len(),
                })
                .collect(),
        };

        let channels = channel_data
            .into_iter()
            .map(|channel| FftJobChannelInput {
                key: ChannelKey::new(crate::DEFAULT_SOURCE_ID, channel.channel_path.clone()),
                display_label: format!("{} #1", channel.channel_name),
                tooltip: format!("phase0_fft_reference.parquet / {}", channel.channel_path),
                channel_name: channel.channel_name,
                channel_path: channel.channel_path,
                values: channel.values[index_start..index_end_exclusive]
                    .iter()
                    .copied()
                    .map(f64::from)
                    .collect(),
            })
            .collect();

        Some(FftJobInput {
            key,
            cursor_a,
            cursor_b,
            index_start,
            index_end_exclusive,
            dt,
            time: time[index_start..index_end_exclusive].to_vec(),
            window: FftWindowKind::Hanning,
            channels,
        })
    }

    fn overlay_fixture() -> (WorkspaceDataState, ViewState, FftAnalysisState, SourceId) {
        let mut workspace = WorkspaceDataState::default();
        let second = workspace.add_source("/tmp/source_2.parquet".to_owned());
        for (source_id, count, raw_start, frequency) in [
            (crate::DEFAULT_SOURCE_ID, 1001, 0.0, 50.0),
            (second, 2001, 10.0, 120.0),
        ] {
            install_tiny_fft_source(&mut workspace, source_id, raw_start, &["signal"]);
            let source = workspace.source_mut(source_id).unwrap();
            source.time_transform.offset_seconds = -raw_start;
            source.shared_time.as_mut().unwrap().time = (0..count)
                .map(|index| raw_start + index as f64 / (count - 1) as f64)
                .collect::<Vec<_>>()
                .into();
            source
                .loaded_channels
                .insert_channel(crate::parquet_waveform::ChannelData {
                    path: std::path::PathBuf::new(),
                    channel_name: "signal".to_owned(),
                    channel_path: "signal".to_owned(),
                    projected_column_index: 1,
                    elapsed: Duration::ZERO,
                    values: (0..count)
                        .map(|index| {
                            (2.0 * std::f64::consts::PI * frequency * index as f64
                                / (count - 1) as f64)
                                .sin() as f32
                        })
                        .collect::<Vec<_>>()
                        .into(),
                });
        }
        let targets = vec![
            ChannelKey::new(second, "signal"),
            ChannelKey::new(crate::DEFAULT_SOURCE_ID, "signal"),
        ];
        let mut view = ViewState::default();
        view.rows[0].channels = targets
            .iter()
            .enumerate()
            .map(|(index, key)| crate::RowChannel::new(key.clone(), index))
            .collect();
        let fft = FftAnalysisState {
            target_channels: targets,
            target_initialized: true,
            window: FftWindowKind::Rectangular,
            ..FftAnalysisState::default()
        };
        (workspace, view, fft, second)
    }

    fn render_compact_fft_ui(
        ctx: &egui::Context,
        fft: &mut FftAnalysisState,
        workspace: &WorkspaceDataState,
        view: &ViewState,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Option<FftUiAction>, egui::Id) {
        let mut action = None;
        let mut scroll_id = egui::Id::NULL;
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 112.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                scroll_id = ui.make_persistent_id(egui::Id::new("fft_settings_results_y"));
                action = draw_fft_section(ui, fft, Some(0.0), Some(1.0), workspace, view);
            },
        );
        (output, action, scroll_id)
    }

    fn visible_fft_text_center(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                let center = text.pos + text.galley.size() * 0.5;
                shape.clip_rect.contains(center).then_some(center)
            }
            _ => None,
        })
    }

    fn click_compact_fft_ui(
        ctx: &egui::Context,
        fft: &mut FftAnalysisState,
        workspace: &WorkspaceDataState,
        view: &ViewState,
        pos: egui::Pos2,
    ) -> Option<FftUiAction> {
        let mut action = None;
        for pressed in [true, false] {
            action = render_compact_fft_ui(
                ctx,
                fft,
                workspace,
                view,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            )
            .1;
        }
        action
    }

    #[test]
    fn compact_fft_shows_large_interval_warning_with_details_closed() {
        let mut workspace = tiny_workspace();
        let sample_count = FFT_SOFT_WARNING_SAMPLES_PER_CHANNEL;
        let source = workspace.source_mut(crate::DEFAULT_SOURCE_ID).unwrap();
        source.shared_time.as_mut().unwrap().time = (0..sample_count)
            .map(|index| index as f64 / (sample_count - 1) as f64)
            .collect::<Vec<_>>()
            .into();
        source
            .loaded_channels
            .insert_channel(crate::parquet_waveform::ChannelData {
                path: std::path::PathBuf::new(),
                channel_name: "sine_50Hz".to_owned(),
                channel_path: "sine_50Hz".to_owned(),
                values: vec![0.0; sample_count].into(),
                projected_column_index: 1,
                elapsed: Duration::ZERO,
            });
        let mut view = ViewState::default();
        view.rows[0].channels = vec![crate::RowChannel::new(
            ChannelKey::new(crate::DEFAULT_SOURCE_ID, "sine_50Hz"),
            0,
        )];
        let mut fft = FftAnalysisState::default();
        let ctx = egui::Context::default();
        let _ = render_compact_fft_ui(&ctx, &mut fft, &workspace, &view, Vec::new());
        let (output, _, _) = render_compact_fft_ui(&ctx, &mut fft, &workspace, &view, Vec::new());
        assert!(visible_fft_text_center(&output, "Large FFT interval").is_some());
        assert!(visible_fft_text_center(&output, "Run FFT").is_some());
        assert!(
            !output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Text(text) if text.galley.text().contains("samples/ch")
            )),
            "computation details stay collapsed"
        );
    }

    #[test]
    fn compact_fft_keeps_run_and_cancel_clickable_after_results_scroll() {
        let (workspace, view, mut fft, _) = overlay_fixture();
        for input in build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view)
            .unwrap()
            .inputs
        {
            fft.results
                .insert(input.source_id(), compute_fft(input).unwrap());
        }
        let results = fft.results.clone();
        let ctx = egui::Context::default();
        let _ = render_compact_fft_ui(&ctx, &mut fft, &workspace, &view, Vec::new());
        let (initial, _, scroll_id) =
            render_compact_fft_ui(&ctx, &mut fft, &workspace, &view, Vec::new());
        let run_pos = visible_fft_text_center(&initial, "Run FFT").expect("Run is visible");
        assert!(visible_fft_text_center(&initial, "Window").is_some());

        let _ = render_compact_fft_ui(
            &ctx,
            &mut fft,
            &workspace,
            &view,
            vec![
                egui::Event::PointerMoved(egui::pos2(500.0, 80.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -400.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        for _ in 0..12 {
            let _ = render_compact_fft_ui(&ctx, &mut fft, &workspace, &view, Vec::new());
        }
        let state = egui::scroll_area::State::load(&ctx, scroll_id).expect("scroll state");
        assert!(state.offset.y > 80.0, "the results body actually scrolled");
        let (scrolled, _, _) = render_compact_fft_ui(&ctx, &mut fft, &workspace, &view, Vec::new());
        assert_eq!(visible_fft_text_center(&scrolled, "Run FFT"), Some(run_pos));
        assert!(visible_fft_text_center(&scrolled, "Window").is_none());
        assert_eq!(
            fft.results, results,
            "scrolling preserves completed results"
        );
        assert_eq!(
            click_compact_fft_ui(&ctx, &mut fft, &workspace, &view, run_pos),
            Some(FftUiAction::Run)
        );

        for (index, input) in build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view)
            .unwrap()
            .inputs
            .iter()
            .enumerate()
        {
            fft.register_job(index as u64 + 1, input);
        }
        let (running, _, _) = render_compact_fft_ui(&ctx, &mut fft, &workspace, &view, Vec::new());
        let cancel_pos = visible_fft_text_center(&running, "Cancel").expect("Cancel is visible");
        assert_eq!(
            click_compact_fft_ui(&ctx, &mut fft, &workspace, &view, cancel_pos),
            Some(FftUiAction::Cancel)
        );
    }

    #[test]
    fn fft_overlay_groups_sources_and_preserves_native_grids_and_offsets() {
        assert_eq!(
            compact_label("very_long_identical_channel #1", 15),
            "very_long... #1"
        );
        assert!(compact_label("very_long_identical_channel #2", 15).ends_with(" #2"));
        let (workspace, view, fft, second) = overlay_fixture();
        let batch = build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view).unwrap();
        assert!(batch.errors.is_empty());
        assert_eq!(batch.inputs.len(), 2);
        let mut results = BTreeMap::new();
        for input in batch.inputs {
            assert_eq!(input.channel_count(), 1);
            if input.source_id() == second {
                assert_close(input.time[0], 10.0, 1.0e-12);
                assert_close(input.dt, 0.0005, 1.0e-12);
                assert_eq!(input.sample_count(), 2001);
            } else {
                assert_close(input.dt, 0.001, 1.0e-12);
                assert_eq!(input.sample_count(), 1001);
            }
            results.insert(input.source_id(), compute_fft(input).unwrap());
        }
        let first = &results[&crate::DEFAULT_SOURCE_ID];
        let last = &results[&second];
        assert_ne!(first.frequency_hz, last.frequency_hz);
        assert_close(first.spectra[0].peak_freq_hz.unwrap(), 50.0, 0.1);
        assert_close(last.spectra[0].peak_freq_hz.unwrap(), 120.0, 0.1);
        assert!(first.spectra[0].display_label.contains("#1"));
        assert!(last.spectra[0].display_label.contains("#2"));
        assert_eq!(
            fft_overlay_frequency_range(&results),
            Some((0.0, *last.frequency_hz.last().unwrap()))
        );
        let traces = fft_overlay_traces(&results, &fft.target_channels);
        assert_eq!(traces[0].spectrum.key.source_id, second);
        assert_eq!(traces[0].color_index, 0);
        assert_eq!(traces[1].color_index, 1);
        let second_only = BTreeMap::from([(second, last.clone())]);
        assert_eq!(
            fft_overlay_traces(&second_only, &fft.target_channels)[0].color_index,
            0
        );
    }

    #[test]
    fn fft_overlay_hover_uses_each_native_bin_and_omits_outside_sources() {
        let (workspace, view, fft, second) = overlay_fixture();
        let results = build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view)
            .unwrap()
            .inputs
            .into_iter()
            .map(|input| (input.source_id(), compute_fft(input).unwrap()))
            .collect::<BTreeMap<_, _>>();
        let first = &results[&crate::DEFAULT_SOURCE_ID];
        let last = &results[&second];
        let a = fft_hover_sample(first, &first.spectra[0], 50.0).unwrap();
        let b = fft_hover_sample(last, &last.spectra[0], 50.0).unwrap();
        assert_ne!(a.0, b.0);
        assert!(first.frequency_hz.contains(&a.0));
        assert!(last.frequency_hz.contains(&b.0));
        assert!(fft_hover_sample(first, &first.spectra[0], 750.0).is_none());
        assert!(fft_hover_sample(last, &last.spectra[0], 750.0).is_some());
        assert!(fft_hover_sample(first, &first.spectra[0], -0.1).is_none());
        assert!(fft_hover_sample(last, &last.spectra[0], f64::NAN).is_none());
        let points = fft_display_series(first, &first.spectra[0], FftYScale::Db, (0.0, 1000.0));
        assert!(
            points
                .iter()
                .all(|(frequency, _)| first.frequency_hz.contains(frequency))
        );
        assert!(
            points
                .iter()
                .all(|(frequency, _)| *frequency <= *first.frequency_hz.last().unwrap())
        );
    }

    #[test]
    fn fft_batch_keeps_valid_source_when_other_cursor_is_outside() {
        let (mut workspace, view, fft, second) = overlay_fixture();
        workspace
            .source_mut(second)
            .unwrap()
            .time_transform
            .offset_seconds = 0.0;
        let batch = build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view).unwrap();
        assert_eq!(batch.inputs.len(), 1);
        assert_eq!(batch.inputs[0].source_id(), crate::DEFAULT_SOURCE_ID);
        assert!(batch.errors[&second].contains("outside"));
        assert!(batch.inputs[0].time[0] == 0.0);
    }

    #[test]
    fn fft_batch_validates_global_targets_cursors_and_copy_budget() {
        let (workspace, view, mut fft, _) = overlay_fixture();
        assert!(build_fft_job_inputs(&fft, Some(0.0), Some(0.0), &workspace, &view).is_err());
        fft.target_channels.extend(fft.target_channels.clone());
        assert!(
            build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view)
                .unwrap_err()
                .contains("3 total")
        );
        assert!(validate_fft_copy_budget(FFT_HARD_LIMIT_COPY_BYTES).is_ok());
        assert!(validate_fft_copy_budget(FFT_HARD_LIMIT_COPY_BYTES.saturating_add(1)).is_err());
        assert_eq!(
            estimated_fft_copy_bytes(10, 2) + estimated_fft_copy_bytes(20, 1),
            560
        );
    }

    #[test]
    fn fft_jobs_complete_independently_and_cancel_keeps_completed_results() {
        let (workspace, view, mut fft, second) = overlay_fixture();
        let mut inputs = build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view)
            .unwrap()
            .inputs;
        let first_input = inputs.remove(0);
        let second_input = inputs.remove(0);
        fft.register_job(11, &first_input);
        fft.register_job(12, &second_input);
        let key = first_input.key.clone();
        assert!(fft.apply_job_result(
            FftJobResult {
                job_id: 11,
                key,
                result: compute_fft(first_input)
            },
            &workspace
        ));
        assert_eq!(fft.results.len(), 1);
        assert!(fft.is_running());
        let old_key = second_input.key.clone();
        fft.cancel_jobs();
        assert!(!fft.is_running());
        assert_eq!(fft.results.len(), 1);
        assert!(!fft.apply_job_result(
            FftJobResult {
                job_id: 12,
                key: old_key,
                result: compute_fft(second_input)
            },
            &workspace
        ));
        assert!(!fft.results.contains_key(&second));
    }

    #[test]
    fn fft_source_failure_keeps_other_result_and_rejects_replaced_jobs() {
        let (mut workspace, view, mut fft, second) = overlay_fixture();
        let inputs = build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view)
            .unwrap()
            .inputs;
        for (index, input) in inputs.into_iter().enumerate() {
            let job_id = index as u64 + 1;
            fft.register_job(job_id, &input);
            if input.source_id() == second {
                let stale_key = input.key.clone();
                fft.register_job(99, &input);
                assert!(!fft.apply_job_result(
                    FftJobResult {
                        job_id,
                        key: stale_key.clone(),
                        result: Err("old".to_owned())
                    },
                    &workspace
                ));
                assert!(fft.apply_job_result(
                    FftJobResult {
                        job_id: 99,
                        key: stale_key,
                        result: Err("non-uniform time".to_owned())
                    },
                    &workspace
                ));
            } else {
                let key = input.key.clone();
                assert!(fft.apply_job_result(
                    FftJobResult {
                        job_id,
                        key,
                        result: compute_fft(input)
                    },
                    &workspace
                ));
            }
        }
        assert_eq!(fft.results.len(), 1);
        assert!(fft.source_errors[&second].contains("non-uniform"));
        assert!(!fft.is_running());
        let input = build_fft_job_inputs(&fft, Some(0.0), Some(1.0), &workspace, &view)
            .unwrap()
            .inputs
            .remove(0);
        fft.register_job(100, &input);
        workspace
            .source_mut(input.source_id())
            .unwrap()
            .time_transform
            .offset_seconds = 1.0;
        let key = input.key.clone();
        assert!(!fft.apply_job_result(
            FftJobResult {
                job_id: 100,
                key,
                result: compute_fft(input)
            },
            &workspace
        ));
        assert!(!fft.is_running());
        fft.invalidate_result();
        assert!(fft.results.is_empty() && fft.jobs.is_empty() && fft.source_errors.is_empty());
    }

    fn tiny_workspace() -> crate::WorkspaceDataState {
        let mut workspace = crate::WorkspaceDataState::default();
        install_tiny_fft_source(
            &mut workspace,
            crate::DEFAULT_SOURCE_ID,
            0.0,
            &["sine_50Hz"],
        );
        workspace
    }

    fn install_tiny_fft_source(
        workspace: &mut crate::WorkspaceDataState,
        source_id: SourceId,
        time_start: f64,
        channel_paths: &[&str],
    ) {
        let source = workspace.source_mut(source_id).expect("FFT source");
        source.shared_time = Some(crate::parquet_waveform::TimeData {
            path: std::path::PathBuf::new(),
            column_name: "time".to_owned(),
            column_path: "time".to_owned(),
            time: (0..8)
                .map(|index| time_start + index as f64 * 0.1)
                .collect::<Vec<_>>()
                .into(),
            projected_column_index: 0,
            elapsed: Duration::ZERO,
        });
        for channel_path in channel_paths {
            source
                .loaded_channels
                .insert_channel(crate::parquet_waveform::ChannelData {
                    path: std::path::PathBuf::new(),
                    channel_name: (*channel_path).to_owned(),
                    channel_path: (*channel_path).to_owned(),
                    values: (0..8).map(|index| index as f32).collect::<Vec<_>>().into(),
                    projected_column_index: 1,
                    elapsed: Duration::ZERO,
                });
        }
    }
}
