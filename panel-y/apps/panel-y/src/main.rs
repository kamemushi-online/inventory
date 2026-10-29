use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

mod cold_render_bench;
mod fft_analysis;
mod interval_stats;
mod interval_stats_bench;
mod interval_stats_multi_bench;
mod parquet_schema;
mod parquet_waveform;
mod plot_config;
#[cfg(feature = "soak-validation")]
mod soak_validation;
mod workspace_data;

use workspace_data::{ChannelKey, SourceId, TimeTransform};

const FULL_RANGE_ENVELOPE_BUCKETS: usize = 4_096;
const MIN_VISIBLE_ENVELOPE_BUCKETS: usize = 128;
const MAX_VISIBLE_ENVELOPE_BUCKETS: usize = 8_192;
const MIN_WHEEL_ZOOM_SENSITIVITY: f64 = 0.0005;
const MAX_WHEEL_ZOOM_SENSITIVITY: f64 = 0.0015;
const MIN_WHEEL_ZOOM_SAMPLE_COUNT: usize = 20_000;
const MAX_WHEEL_ZOOM_SAMPLE_COUNT: usize = 10_000_000;
const BENCH_VISIBLE_ENVELOPE_BUCKETS: usize = 1_200;
const BENCH_RANGE_RUNS: usize = 24;
const BENCH_HOVER_RUNS: usize = 1_000;
const STRESS_RANGE_RUNS: usize = 1_000;
const STRESS_REPORT_BLOCKS: usize = 5;
const MIN_WAVEFORM_ROW_HEIGHT: f32 = 180.0;
const WAVEFORM_ROW_GAP: f32 = 6.0;
const PLOT_TOP_MARGIN: f32 = 14.0;
const PLOT_BOTTOM_MARGIN: f32 = 20.0;
const MAX_EXACT_STEP_SAMPLES: usize = 12_000;
const MAX_STEP_CHANGE_POINTS: usize = 12_000;
const INTERACTION_PREVIEW_SETTLE: Duration = Duration::from_millis(140);
const BOX_ZOOM_MIN_PIXEL_WIDTH: f32 = 3.0;
const PREVIEW_MIN_DATASET_SAMPLES: usize = 8_000_000;
const PREVIEW_MIN_VISIBLE_LINE_SAMPLE_WORK: usize = 32_000_000;
const PREVIEW_MIN_VISIBLE_STEP_SAMPLES: usize = 1_000_000;
const PREVIEW_BUCKET_DIVISOR: usize = 4;
const MAX_PREVIEW_ENVELOPE_BUCKETS: usize = 512;
const ENVELOPE_OVERSCAN_RATIO: f64 = 0.5;
const ENVELOPE_CONTEXT_SPAN_TOLERANCE: f64 = 0.05;
const MAX_CACHED_ENVELOPE_BUCKETS: usize = MAX_VISIBLE_ENVELOPE_BUCKETS * 3;
const LINE_TILE_SAMPLE_WIDTH: usize = 256;
const LINE_TILE_MIN_SOURCE_SAMPLES: usize = 500_000;
const LINE_TILE_MIN_BUCKET_SIZE: usize = LINE_TILE_SAMPLE_WIDTH * 2;
const LINE_TILE_BATCH_MIN_SAMPLES: usize = 8_000_000;
const LINE_TILE_MAX_WORKERS: usize = 4;
const DEFAULT_TRACE_LINE_WIDTH: f32 = 1.25;
const MIN_TRACE_LINE_WIDTH: f32 = 0.5;
const MAX_TRACE_LINE_WIDTH: f32 = 6.0;
const MAX_HOVER_READOUT_CHANNELS: usize = 4;
const MAX_RECENT_PARQUET_PATHS: usize = 8;
const LEFT_PANEL_DEFAULT_WIDTH: f32 = 320.0;
const LEFT_PANEL_MIN_WIDTH: f32 = 240.0;
const LEFT_PANEL_MAX_WIDTH: f32 = 520.0;
const LEFT_PANEL_CHANNEL_LABEL_CHARS: usize = 28;
const LEFT_PANEL_DETAIL_LABEL_CHARS: usize = 36;
const CURSOR_INFO_COLLAPSED_HEIGHT: f32 = 42.0;
const CURSOR_INFO_DEFAULT_HEIGHT: f32 = 360.0;
const CURSOR_INFO_MIN_HEIGHT: f32 = 96.0;
const CURSOR_INFO_MAX_HEIGHT: f32 = 640.0;
const CURSOR_INFO_MAX_HEIGHT_FRACTION: f32 = 0.75;
const CURSOR_INFO_RESIZE_HANDLE_HEIGHT: f32 = 8.0;
const ANALYSIS_SPLIT_DEFAULT_HEIGHT: f32 = 140.0;
const ANALYSIS_SPLIT_MIN_HEIGHT: f32 = 72.0;
const ANALYSIS_SPLIT_HANDLE_HEIGHT: f32 = 8.0;
const FFT_SECTION_MIN_HEIGHT: f32 = 120.0;
const MIN_WAVEFORM_VIEWPORT_HEIGHT: f32 = 120.0;
const APP_NAME: &str = "Panel-y";
const APP_VERSION: &str = "v3.0";
const APP_WINDOW_TITLE: &str = "Panel-y v3.0";
const ABOUT_PLATE_IMAGE_BYTES: &[u8] = include_bytes!("../assets/panel-y_blue2_LOGO.png");
const ABOUT_PLATE_MAX_WIDTH: f32 = 820.0;
const ABOUT_PLATE_MAX_SCREEN_WIDTH_FRACTION: f32 = 0.8;
const ABOUT_PLATE_MAX_SCREEN_HEIGHT_FRACTION: f32 = 0.75;
const SYSTEM_CJK_FONT_NAME: &str = "system_cjk_fallback";
const SYSTEM_CJK_FONT_PATHS: &[&str] = &[
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
    "/System/Library/Fonts/AppleSDGothicNeo.ttc",
    "C:\\Windows\\Fonts\\meiryo.ttc",
    "C:\\Windows\\Fonts\\YuGothM.ttc",
    "C:\\Windows\\Fonts\\msgothic.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
];
const DEFAULT_SOURCE_ID: SourceId = SourceId::new(1);

fn main() -> eframe::Result {
    #[cfg(feature = "soak-validation")]
    {
        let config_path = if std::env::args().nth(1).as_deref() == Some("--native-soak") {
            Some(
                std::env::args()
                    .nth(2)
                    .ok_or_else(|| "--native-soak requires a JSON configuration path".to_owned()),
            )
        } else {
            std::env::var_os("PANEL_Y_NATIVE_SOAK_CONFIG").map(|path| {
                path.into_string()
                    .map_err(|_| "PANEL_Y_NATIVE_SOAK_CONFIG must be valid Unicode".to_owned())
            })
        };
        if let Some(config_path) = config_path {
            let result = config_path.and_then(|path| {
                if path.trim().is_empty() {
                    Err("native soak configuration path must not be empty".to_owned())
                } else {
                    soak_validation::run(&path)
                }
            });
            if let Err(error) = result {
                eprintln!("native soak validation failed: {error}");
                std::process::exit(1);
            }
            return Ok(());
        }
    }
    if let Some(command) = cli_command_arg() {
        let result = match command {
            CliCommand::Schema { path } => parquet_schema::read_schema_summary(path)
                .map(|summary| summary.to_report()),
            CliCommand::LoadChannel { path, channel } => {
                parquet_waveform::read_selected_channel(path, &channel).map(|data| {
                    let envelope = data.min_max_envelope(FULL_RANGE_ENVELOPE_BUCKETS);
                    let time_range = format_range(envelope.time_range);
                    let value_range = format_range(envelope.value_range);
                    format!(
                        "file: {}\nchannel: {}\nchannel path: {}\nsamples: {}\ntime range: {time_range}\nvalue range: {value_range}\nprojected columns: {:?}\nread time: {:.3}s\nenvelope: {} buckets requested, {} buckets built, bucket size {}, draw points {}, {:.3}s\nmemory: {:.1} MiB",
                        data.path.display(),
                        data.channel_name,
                        data.channel_path,
                        data.sample_count(),
                        data.projected_column_indices,
                        data.elapsed.as_secs_f64(),
                        envelope.requested_bucket_count,
                        envelope.bucket_count(),
                        envelope.bucket_size,
                        envelope.draw_point_count(),
                        envelope.elapsed.as_secs_f64(),
                        bytes_to_mib(data.memory_bytes()),
                    )
                })
            }
            CliCommand::BenchChannel { path, channel } => benchmark_channel(path, &channel),
            CliCommand::StressChannel {
                path,
                channel,
                runs,
            } => stress_channel(path, &channel, runs),
            CliCommand::BenchMultiChannel { path, channels } => {
                benchmark_multi_channel(path, channels)
            }
            CliCommand::BenchPhase2 { path, channels } => benchmark_phase2_view(path, channels),
            CliCommand::BenchIntervalStats { path, channels } => interval_stats_bench::run(path, channels),
            CliCommand::BenchColdRender { config_path } => cold_render_bench::run(config_path),
            CliCommand::BenchIntervalStatsMulti { config_path } => {
                interval_stats_multi_bench::run(config_path)
            }
            CliCommand::BenchMultiSource {
                source_one_path,
                source_two_path,
            } => benchmark_multi_source(source_one_path, source_two_path),
        };

        match result {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("read failed: {error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_WINDOW_TITLE)
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        APP_WINDOW_TITLE,
        options,
        Box::new(|cc| {
            install_system_cjk_font_fallback(&cc.egui_ctx);
            Ok(Box::new(PanelYApp::default()))
        }),
    )
}

fn install_system_cjk_font_fallback(ctx: &egui::Context) {
    let Some(font_bytes) = SYSTEM_CJK_FONT_PATHS
        .iter()
        .find_map(|path| std::fs::read(path).ok())
    else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        SYSTEM_CJK_FONT_NAME.to_owned(),
        Arc::new(egui::FontData::from_owned(font_bytes)),
    );

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(font_names) = fonts.families.get_mut(&family) {
            font_names.push(SYSTEM_CJK_FONT_NAME.to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

enum CliCommand {
    Schema {
        path: String,
    },
    LoadChannel {
        path: String,
        channel: String,
    },
    BenchChannel {
        path: String,
        channel: String,
    },
    StressChannel {
        path: String,
        channel: String,
        runs: usize,
    },
    BenchMultiChannel {
        path: String,
        channels: Vec<String>,
    },
    BenchPhase2 {
        path: String,
        channels: Vec<String>,
    },
    BenchIntervalStats {
        path: String,
        channels: Vec<String>,
    },
    BenchColdRender {
        config_path: String,
    },
    BenchIntervalStatsMulti {
        config_path: String,
    },
    BenchMultiSource {
        source_one_path: String,
        source_two_path: String,
    },
}

fn cli_command_arg() -> Option<CliCommand> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--bench-cold-render") => args
            .next()
            .map(|config_path| CliCommand::BenchColdRender { config_path }),
        Some("--bench-interval-stats-multi") => args
            .next()
            .map(|config_path| CliCommand::BenchIntervalStatsMulti { config_path }),
        Some("--bench-interval-stats") => {
            let path = args.next()?;
            Some(CliCommand::BenchIntervalStats {
                path,
                channels: args.collect(),
            })
        }
        Some("--schema") => args.next().map(|path| CliCommand::Schema { path }),
        Some("--load-channel") => {
            let path = args.next()?;
            let channel = args.next()?;
            Some(CliCommand::LoadChannel { path, channel })
        }
        Some("--bench-channel") => {
            let path = args.next()?;
            let channel = args.next()?;
            Some(CliCommand::BenchChannel { path, channel })
        }
        Some("--stress-channel") => {
            let path = args.next()?;
            let channel = args.next()?;
            let runs = args
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(STRESS_RANGE_RUNS);
            Some(CliCommand::StressChannel {
                path,
                channel,
                runs,
            })
        }
        Some("--bench-multi-channel") => {
            let path = args.next()?;
            let channels = args.collect();
            Some(CliCommand::BenchMultiChannel { path, channels })
        }
        Some("--bench-phase2") => {
            let path = args.next()?;
            let channels = args.collect();
            Some(CliCommand::BenchPhase2 { path, channels })
        }
        Some("--bench-multi-source") => {
            let source_one_path = args.next()?;
            let source_two_path = args.next()?;
            Some(CliCommand::BenchMultiSource {
                source_one_path,
                source_two_path,
            })
        }
        _ => None,
    }
}

struct PanelYApp {
    workspace: WorkspaceDataState,
    view: ViewState,
    analysis: AnalysisState,
    load: LoadState,
    interaction: InteractionState,
    perf: PerfStats,
    session_preset: Option<SessionPreset>,
    session_preset_status: Option<String>,
    plot_config_path: String,
    plot_config_report: Option<plot_config::PlotConfigApplyReport>,
    fit_plot_config_after_load: bool,
    pending_session_x_range: Option<Option<(f64, f64)>>,
    load_result_tx: mpsc::Sender<LoadJobResult>,
    load_result_rx: mpsc::Receiver<LoadJobResult>,
    fft_result_tx: mpsc::Sender<fft_analysis::FftJobResult>,
    fft_result_rx: mpsc::Receiver<fft_analysis::FftJobResult>,
    next_load_job_id: u64,
    next_fft_job_id: u64,
    show_about: bool,
    about_plate_texture: Option<egui::TextureHandle>,
}

#[derive(Debug)]
struct DataSourceState {
    id: SourceId,
    label: String,
    parquet_path: String,
    schema: Option<parquet_schema::SchemaSummary>,
    shared_time: Option<parquet_waveform::TimeData>,
    loaded_channels: ChannelStore,
    time_transform: TimeTransform,
    generation: u64,
}

#[derive(Debug)]
struct WorkspaceDataState {
    tile_batch_wall: Duration,
    tile_batch_workers: usize,
    sources: BTreeMap<SourceId, DataSourceState>,
    source_order: Vec<SourceId>,
    selected_source_id: Option<SourceId>,
    next_source_id: u64,
    recent_parquet_paths: Vec<String>,
}

type DatasetState = DataSourceState;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WorkspaceCacheStats {
    source_count: usize,
    loaded_channel_count: usize,
    raw_time_memory_bytes: usize,
    raw_channel_memory_bytes: usize,
    envelope_count: usize,
    line_tile_cache_count: usize,
    line_tile_memory_bytes: usize,
    step_edge_cache_count: usize,
    step_edge_memory_bytes: usize,
    step_fallback_hint_count: usize,
}

impl WorkspaceCacheStats {
    fn total_raw_memory_bytes(self) -> usize {
        self.raw_time_memory_bytes
            .saturating_add(self.raw_channel_memory_bytes)
    }
}

#[derive(Debug)]
struct ViewState {
    selected_channel: Option<ChannelKey>,
    channel_filter: String,
    channel_state_filter: ChannelStateFilter,
    channel_sort: ChannelSort,
    selected_row_id: Option<u64>,
    next_row_id: u64,
    x_range: Option<(f64, f64)>,
    hover_x: Option<f64>,
    large_preview_enabled: bool,
    theme: ThemeMode,
    visual_mode: VisualMode,
    scroll_zoom: bool,
    rows: Vec<PlotRow>,
}

#[derive(Debug)]
struct AnalysisState {
    cursor_mode: bool,
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    next_cursor: CursorSlot,
    info_panel_visible: bool,
    info_panel_collapsed: bool,
    info_panel_height: f32,
    pane: AnalysisPane,
    analysis_split_height: f32,
    interval_stats_cache: IntervalStatsCache,
    interval_stats_worker: interval_stats::IntervalStatsWorker,
    fft: fft_analysis::FftAnalysisState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum AnalysisPane {
    #[default]
    Statistics,
    Fft,
    Split,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CursorSlot {
    #[default]
    A,
    B,
}

#[derive(Debug)]
struct LoadState {
    status: String,
    pending_jobs: usize,
    progress: Option<String>,
    error: Option<String>,
    active_jobs: BTreeMap<u64, ActiveLoadJob>,
}

#[derive(Clone, Debug)]
struct InteractionState {
    last_range_change: Option<Instant>,
    box_zoom: Option<BoxZoomDrag>,
}

#[derive(Clone, Copy, Debug)]
struct BoxZoomDrag {
    plot_rect: egui::Rect,
    start_pos: egui::Pos2,
    current_pos: egui::Pos2,
    view_range: (f64, f64),
}

#[derive(Clone, Debug)]
struct PerfStats {
    show: bool,
    frame_ms: f64,
    interaction_ms: f64,
    visible_ms: f64,
    draw_ms: f64,
    rows: usize,
    channels: usize,
    draw_points: usize,
    requested_buckets: usize,
    effective_buckets: usize,
    preview: bool,
    envelope_cache_hits: usize,
    envelope_cache_misses: usize,
    envelope_context_hits: usize,
    envelope_context_misses: usize,
    line_tile_hits: usize,
    line_tile_builds: usize,
    step_edge_hits: usize,
    step_edge_builds: usize,
}

#[derive(Clone, Debug)]
struct ActiveLoadJob {
    source_id: SourceId,
    source_generation: u64,
    channel_key: ChannelKey,
    target_row_id: u64,
    target_row_label: String,
    add_to_row: bool,
    time_was_cached: bool,
    channel_was_cached: bool,
    started: std::time::Instant,
}

#[derive(Clone, Debug)]
struct SessionPresetSource {
    id: SourceId,
    parquet_path: String,
    label: String,
    offset_seconds: f64,
}

#[derive(Clone, Debug)]
struct SessionPreset {
    sources: Vec<SessionPresetSource>,
    // Stable saved identities map to their latest successfully restored sources.
    runtime_source_ids: BTreeMap<SourceId, SourceId>,
    selected_source_id: Option<SourceId>,
    selected_channel: Option<ChannelKey>,
    selected_row_index: Option<usize>,
    x_range: Option<(f64, f64)>,
    large_preview_enabled: bool,
    rows: Vec<SessionPresetRow>,
}

#[derive(Clone, Debug, PartialEq)]
struct SessionPresetRow {
    y_range: RowYRange,
    channels: Vec<SessionPresetChannel>,
}

#[derive(Clone, Debug, PartialEq)]
struct SessionPresetChannel {
    key: ChannelKey,
    color_index: usize,
    draw_mode: DrawMode,
    visible: bool,
    color_override: Option<egui::Color32>,
    line_width: f32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SessionPresetApplyReport {
    restored_rows: usize,
    restored_channels: usize,
    missing_channels: Vec<String>,
}

#[derive(Debug)]
struct LoadJobResult {
    job_id: u64,
    source_id: SourceId,
    source_generation: u64,
    channel_path: String,
    result: Result<LoadedChannelData, String>,
}

#[derive(Debug)]
struct LoadedChannelData {
    time: Option<parquet_waveform::TimeData>,
    channel: Option<parquet_waveform::ChannelData>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SchemaControlsResponse {
    selection_changed: bool,
    load_channel: Option<ChannelKey>,
    add_matching_channels: Option<ChannelKey>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CacheControlsResponse {
    action: Option<CacheControlAction>,
}

#[derive(Clone, Debug, PartialEq)]
enum DataSourceUiAction {
    EditPath(SourceId, String),
    SelectRecentPath(SourceId, String),
    Browse(SourceId),
    LoadSchema(SourceId),
    SetTimeOffset(SourceId, f64),
    StartAtZero(SourceId),
    StartAllAtZero,
    Remove(SourceId),
    Add,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CacheControlAction {
    AddToSelectedRow(ChannelKey),
    UnloadChannel(ChannelKey),
    UnloadAllChannels(SourceId),
    ClearDrawCaches(SourceId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourcePresentation {
    number: usize,
    short_label: String,
    file_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChannelPresentation {
    short_label: String,
    source_label: String,
    tooltip: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MatchingChannelOutcome {
    Match(ChannelKey),
    MissingSchema(SourceId),
    NoMatch(SourceId),
    Ambiguous(SourceId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlotConfigApplyMode {
    LoadConfigDataPath,
    ApplyToCurrentData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PersistenceAction {
    PlotConfigLoad,
    PlotConfigApplyToCurrentData,
    PlotConfigSave,
    PlotConfigSaveAs,
    SessionPresetSave,
    SessionPresetApply,
}

impl PersistenceAction {
    fn label(self) -> &'static str {
        match self {
            Self::PlotConfigLoad => "Plot config Load",
            Self::PlotConfigApplyToCurrentData => "Plot config Apply to Current Data",
            Self::PlotConfigSave => "Plot config Save",
            Self::PlotConfigSaveAs => "Plot config Save As",
            Self::SessionPresetSave => "Session preset Save",
            Self::SessionPresetApply => "Session preset Apply",
        }
    }
}

fn persistence_action_block_reason(
    workspace: &WorkspaceDataState,
    action: PersistenceAction,
) -> Option<String> {
    if matches!(
        action,
        PersistenceAction::PlotConfigLoad | PersistenceAction::SessionPresetApply
    ) || !workspace.source_order.is_empty()
    {
        None
    } else {
        Some(format!("{} requires a data source", action.label()))
    }
}

fn plot_config_load_workspace_notice(workspace: &WorkspaceDataState) -> Option<String> {
    let source_count = workspace.source_order.len();
    (source_count > 1).then(|| {
        format!(
            "Load replaces the current {source_count}-source workspace; v1 restores one source, v2 restores its saved sources"
        )
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParquetPathValidationError {
    Empty,
    Missing { path: String },
    NotAccessible { path: String, error: String },
    NotFile { path: String },
    WrongExtension { path: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChannelLoadState {
    Loaded,
    Loading,
    Unloaded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChannelStateFilter {
    All,
    Loaded,
    Loading,
    Unloaded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChannelSort {
    SchemaOrder,
    Name,
    LoadedFirst,
}

#[derive(Clone, Debug)]
struct PlotRow {
    id: u64,
    y_range: RowYRange,
    channels: Vec<RowChannel>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RowYRange {
    mode: YRangeMode,
    min: Option<f64>,
    max: Option<f64>,
    last_auto: Option<(f64, f64)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum YRangeMode {
    Auto,
    Manual,
}

#[derive(Clone, Debug, PartialEq)]
struct RowChannel {
    key: ChannelKey,
    color_index: usize,
    draw_mode: DrawMode,
    visible: bool,
    color_override: Option<egui::Color32>,
    line_width: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DrawMode {
    Line,
    Step,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ThemeMode {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum VisualMode {
    #[default]
    Plain,
    Rock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisualSkin {
    Standard,
    Industrial,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlButtonTreatment {
    Standard,
    PlainDarkMachinery,
    RockMachinery,
}

impl ControlButtonTreatment {
    fn uses_machinery(self) -> bool {
        matches!(
            self,
            ControlButtonTreatment::PlainDarkMachinery | ControlButtonTreatment::RockMachinery
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlButtonRole {
    Default,
    Primary,
    Destructive,
    Utility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControlButtonAccent {
    edge: egui::Color32,
    glow: egui::Color32,
    highlight: egui::Color32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlToggleTreatment {
    Standard,
    RockMachinery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControlToggleAccent {
    edge: egui::Color32,
    glow: egui::Color32,
    highlight: egui::Color32,
    off_edge: egui::Color32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlSectionHeaderTreatment {
    Standard,
    RockMachineryRack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControlSectionHeaderAccent {
    line: egui::Color32,
    glow: egui::Color32,
    highlight: egui::Color32,
    screw_edge: egui::Color32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlStatusIndicatorTreatment {
    Standard,
    RockMachineryLed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlTextInputTreatment {
    Standard,
    RockInstrumentDisplay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlSurfaceTextureTreatment {
    Standard,
    RockSubtleGrime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlStatusLevel {
    Good,
    Info,
    Busy,
    Warning,
    Danger,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControlStatusIndicatorAccent {
    core: egui::Color32,
    glow: egui::Color32,
    edge: egui::Color32,
    highlight: egui::Color32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControlTextInputAccent {
    background: egui::Color32,
    edge: egui::Color32,
    glow: egui::Color32,
    highlight: egui::Color32,
    shadow: egui::Color32,
    selection_bg: egui::Color32,
    selection_stroke: egui::Color32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ControlSurfaceTextureAccent {
    scanline: egui::Color32,
    scratch: egui::Color32,
    grime: egui::Color32,
    separator_shadow: egui::Color32,
    separator_edge: egui::Color32,
    separator_glow: egui::Color32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DisplayModeChoice {
    PlainDark,
    PlainLight,
    Rock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VisualPalette {
    visual_mode: VisualMode,
    skin: VisualSkin,
    control_button_treatment: ControlButtonTreatment,
    control_toggle_treatment: ControlToggleTreatment,
    control_section_header_treatment: ControlSectionHeaderTreatment,
    control_status_indicator_treatment: ControlStatusIndicatorTreatment,
    control_text_input_treatment: ControlTextInputTreatment,
    control_surface_texture_treatment: ControlSurfaceTextureTreatment,
    app_bg: egui::Color32,
    panel_bg: egui::Color32,
    panel_bg_alt: egui::Color32,
    input_bg: egui::Color32,
    plot_bg: egui::Color32,
    text_primary: egui::Color32,
    text_secondary: egui::Color32,
    accent_blue: egui::Color32,
    accent_blue_deep: egui::Color32,
    accent_blue_dim: egui::Color32,
    grid_minor: egui::Color32,
    grid_major: egui::Color32,
    selected_frame: egui::Color32,
    warning: egui::Color32,
    danger: egui::Color32,
}

fn control_button_accent(palette: VisualPalette, role: ControlButtonRole) -> ControlButtonAccent {
    let role = match palette.control_button_treatment {
        ControlButtonTreatment::RockMachinery => role,
        ControlButtonTreatment::Standard | ControlButtonTreatment::PlainDarkMachinery => {
            ControlButtonRole::Default
        }
    };

    match role {
        ControlButtonRole::Default | ControlButtonRole::Primary => ControlButtonAccent {
            edge: palette.accent_blue,
            glow: egui::Color32::from_rgb(69, 216, 255),
            highlight: egui::Color32::from_rgb(150, 244, 255),
        },
        ControlButtonRole::Destructive => ControlButtonAccent {
            edge: egui::Color32::from_rgb(255, 98, 42),
            glow: egui::Color32::from_rgb(255, 69, 24),
            highlight: egui::Color32::from_rgb(255, 176, 86),
        },
        ControlButtonRole::Utility => ControlButtonAccent {
            edge: egui::Color32::from_rgb(126, 158, 176),
            glow: egui::Color32::from_rgb(82, 119, 143),
            highlight: egui::Color32::from_rgb(177, 213, 228),
        },
    }
}

fn control_button_color_with_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn control_toggle_accent(palette: VisualPalette) -> Option<ControlToggleAccent> {
    match palette.control_toggle_treatment {
        ControlToggleTreatment::Standard => None,
        ControlToggleTreatment::RockMachinery => Some(ControlToggleAccent {
            edge: palette.accent_blue,
            glow: egui::Color32::from_rgb(69, 216, 255),
            highlight: egui::Color32::from_rgb(150, 244, 255),
            off_edge: egui::Color32::from_rgb(75, 100, 111),
        }),
    }
}

fn control_section_header_accent(palette: VisualPalette) -> Option<ControlSectionHeaderAccent> {
    match palette.control_section_header_treatment {
        ControlSectionHeaderTreatment::Standard => None,
        ControlSectionHeaderTreatment::RockMachineryRack => Some(ControlSectionHeaderAccent {
            line: palette.accent_blue,
            glow: egui::Color32::from_rgb(69, 216, 255),
            highlight: egui::Color32::from_rgb(150, 244, 255),
            screw_edge: egui::Color32::from_rgb(87, 114, 126),
        }),
    }
}

fn control_status_indicator_accent(
    palette: VisualPalette,
    level: ControlStatusLevel,
) -> Option<ControlStatusIndicatorAccent> {
    match palette.control_status_indicator_treatment {
        ControlStatusIndicatorTreatment::Standard => None,
        ControlStatusIndicatorTreatment::RockMachineryLed => {
            let (core, glow, edge, highlight) = match level {
                ControlStatusLevel::Good => (
                    egui::Color32::from_rgb(89, 255, 130),
                    egui::Color32::from_rgb(54, 238, 98),
                    egui::Color32::from_rgb(71, 168, 96),
                    egui::Color32::from_rgb(186, 255, 205),
                ),
                ControlStatusLevel::Info => (
                    palette.accent_blue,
                    egui::Color32::from_rgb(69, 216, 255),
                    egui::Color32::from_rgb(58, 154, 183),
                    egui::Color32::from_rgb(184, 248, 255),
                ),
                ControlStatusLevel::Busy => (
                    egui::Color32::from_rgb(255, 192, 74),
                    egui::Color32::from_rgb(255, 158, 39),
                    egui::Color32::from_rgb(184, 124, 44),
                    egui::Color32::from_rgb(255, 228, 148),
                ),
                ControlStatusLevel::Warning => (
                    egui::Color32::from_rgb(255, 143, 62),
                    egui::Color32::from_rgb(255, 111, 35),
                    egui::Color32::from_rgb(179, 92, 49),
                    egui::Color32::from_rgb(255, 205, 126),
                ),
                ControlStatusLevel::Danger => (
                    egui::Color32::from_rgb(255, 82, 82),
                    egui::Color32::from_rgb(255, 45, 28),
                    egui::Color32::from_rgb(194, 68, 59),
                    egui::Color32::from_rgb(255, 184, 150),
                ),
            };
            Some(ControlStatusIndicatorAccent {
                core,
                glow,
                edge,
                highlight,
            })
        }
    }
}

fn control_text_input_accent(palette: VisualPalette) -> Option<ControlTextInputAccent> {
    match palette.control_text_input_treatment {
        ControlTextInputTreatment::Standard => None,
        ControlTextInputTreatment::RockInstrumentDisplay => Some(ControlTextInputAccent {
            background: egui::Color32::from_rgb(2, 5, 7),
            edge: palette.accent_blue,
            glow: egui::Color32::from_rgb(69, 216, 255),
            highlight: egui::Color32::from_rgb(150, 244, 255),
            shadow: egui::Color32::from_rgb(0, 0, 0),
            selection_bg: egui::Color32::from_rgba_unmultiplied(24, 183, 255, 96),
            selection_stroke: palette.accent_blue,
        }),
    }
}

fn control_surface_texture_accent(palette: VisualPalette) -> Option<ControlSurfaceTextureAccent> {
    match palette.control_surface_texture_treatment {
        ControlSurfaceTextureTreatment::Standard => None,
        ControlSurfaceTextureTreatment::RockSubtleGrime => Some(ControlSurfaceTextureAccent {
            scanline: egui::Color32::from_rgba_unmultiplied(69, 216, 255, 10),
            scratch: egui::Color32::from_rgba_unmultiplied(184, 222, 230, 24),
            grime: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 18),
            separator_shadow: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 74),
            separator_edge: egui::Color32::from_rgba_unmultiplied(69, 216, 255, 34),
            separator_glow: egui::Color32::from_rgba_unmultiplied(69, 216, 255, 18),
        }),
    }
}

fn load_status_indicator_level(load: &LoadState) -> ControlStatusLevel {
    if load.error.is_some() {
        ControlStatusLevel::Danger
    } else if load.is_busy() || load.pending_jobs > 0 || load.progress.is_some() {
        ControlStatusLevel::Busy
    } else if load.status == "Ready" {
        ControlStatusLevel::Good
    } else {
        ControlStatusLevel::Info
    }
}

fn channel_load_status_indicator_level(load_state: ChannelLoadState) -> ControlStatusLevel {
    match load_state {
        ChannelLoadState::Loaded => ControlStatusLevel::Good,
        ChannelLoadState::Loading => ControlStatusLevel::Busy,
        ChannelLoadState::Unloaded => ControlStatusLevel::Warning,
    }
}

fn cache_count_status_indicator_level(count: usize) -> ControlStatusLevel {
    if count == 0 {
        ControlStatusLevel::Warning
    } else {
        ControlStatusLevel::Info
    }
}

#[derive(Debug, Default)]
struct ChannelStore {
    raw_by_channel: BTreeMap<String, parquet_waveform::ChannelData>,
    line_tile_cache: BTreeMap<String, parquet_waveform::LineTileCache>,
    step_edge_cache: BTreeMap<String, StepEdgeCache>,
    envelope_cache: BTreeMap<EnvelopeKey, parquet_waveform::MinMaxEnvelope>,
    envelope_context: Option<EnvelopeContext>,
    envelope_plan: Option<parquet_waveform::MinMaxEnvelopePlan>,
    last_envelope_stats: EnvelopeCacheStats,
    step_fallback_hints: BTreeMap<String, StepFallbackHint>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StepFallbackHint {
    min_span: f64,
    min_source_sample_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EnvelopeContext {
    cache_range_start_bits: u64,
    cache_range_end_bits: u64,
    view_span_bits: u64,
    requested_view_bucket_count: usize,
    cache_bucket_count: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct EnvelopeCacheStats {
    context_hits: usize,
    context_misses: usize,
    hits: usize,
    misses: usize,
    clipped: usize,
    tile_hits: usize,
    tile_builds: usize,
    tile_buckets: usize,
    step_edge_hits: usize,
    step_edge_builds: usize,
    step_edge_scan_elapsed: Duration,
    step_edge_aborts: usize,
    tile_build_wall: Duration,
    tile_build_workers: usize,
}

impl EnvelopeCacheStats {
    fn accumulate(&mut self, other: Self) {
        self.context_hits = self.context_hits.saturating_add(other.context_hits);
        self.context_misses = self.context_misses.saturating_add(other.context_misses);
        self.hits = self.hits.saturating_add(other.hits);
        self.misses = self.misses.saturating_add(other.misses);
        self.clipped = self.clipped.saturating_add(other.clipped);
        self.tile_hits = self.tile_hits.saturating_add(other.tile_hits);
        self.tile_builds = self.tile_builds.saturating_add(other.tile_builds);
        self.tile_build_wall += other.tile_build_wall;
        self.tile_build_workers = self.tile_build_workers.max(other.tile_build_workers);
        self.tile_buckets = self.tile_buckets.saturating_add(other.tile_buckets);
        self.step_edge_hits = self.step_edge_hits.saturating_add(other.step_edge_hits);
        self.step_edge_builds = self.step_edge_builds.saturating_add(other.step_edge_builds);
        self.step_edge_scan_elapsed += other.step_edge_scan_elapsed;
        self.step_edge_aborts = self.step_edge_aborts.saturating_add(other.step_edge_aborts);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct EnvelopeKey {
    channel_path: String,
    range_start_bits: u64,
    range_end_bits: u64,
    requested_bucket_count: usize,
}

#[derive(Clone, Debug)]
struct VisibleTrace {
    key: ChannelKey,
    channel_path: String,
    display_label: String,
    time_transform: TimeTransform,
    sample_count: usize,
    color: egui::Color32,
    line_width: f32,
    draw_mode: DrawMode,
    hover_value: Option<f32>,
    data: VisibleTraceData,
}

#[derive(Clone, Debug)]
enum VisibleTraceData {
    Envelope(parquet_waveform::MinMaxEnvelope),
    DenseStepEnvelope(parquet_waveform::MinMaxEnvelope),
    RawStep(RawStepTrace),
}

#[derive(Clone, Debug, PartialEq)]
struct RawStepTrace {
    samples: Vec<StepSample>,
    source_sample_count: usize,
    time_range: Option<(f64, f64)>,
    value_range: Option<(f64, f64)>,
    kind: StepTraceKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepTraceKind {
    RawSamples,
    ChangePoints,
}

#[derive(Clone, Debug, PartialEq)]
enum StepTraceBuildResult {
    Trace(RawStepTrace),
    TooManyChangePoints { source_sample_count: usize },
    Empty,
}

#[derive(Clone, Debug, PartialEq)]
struct StepEdgeCache {
    sample_count: usize,
    cached_time_range: Option<(f64, f64)>,
    edges: Vec<StepEdge>,
    elapsed: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StepEdge {
    index: usize,
    time: f64,
    value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StepSample {
    time: f64,
    value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct VisibleSampleRange {
    range_start: f64,
    range_end: f64,
    start: usize,
    end: usize,
    context_start: usize,
    source_sample_count: usize,
    draw_sample_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct SourceRenderContext {
    time_transform: TimeTransform,
    raw_view_range: (f64, f64),
    raw_full_range: (f64, f64),
}

#[derive(Debug)]
struct VisibleRowTrace {
    row_id: u64,
    row_index: usize,
    row_channel_count: usize,
    loading_channel_count: usize,
    unloaded_channel_count: usize,
    out_of_range_channel_count: usize,
    y_range: RowYRange,
    traces: Vec<VisibleTrace>,
}

#[derive(Clone, Debug, PartialEq)]
struct CursorChannelReadout {
    key: ChannelKey,
    channel_name: String,
    channel_path: String,
    display_label: String,
    tooltip: String,
    value_a: Option<f32>,
    value_b: Option<f32>,
    delta: Option<f32>,
}

#[derive(Clone, Debug, Default)]
struct IntervalStatsCache {
    key: Option<IntervalStatsCacheKey>,
    stats: Vec<IntervalChannelStats>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IntervalStatsCacheKey {
    cursor_a_bits: u64,
    cursor_b_bits: u64,
    channels: Vec<IntervalStatsCacheChannel>,
}

#[derive(Clone, Debug)]
struct IntervalStatsCacheChannel {
    key: ChannelKey,
    source_generation: u64,
    time_offset_bits: u64,
    time_sample_count: usize,
    time_start_bits: u64,
    time_end_bits: u64,
    sample_count: usize,
    time: Arc<Vec<f64>>,
    values: Arc<Vec<f32>>,
}

impl PartialEq for IntervalStatsCacheChannel {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.source_generation == other.source_generation
            && self.time_offset_bits == other.time_offset_bits
            && self.time_sample_count == other.time_sample_count
            && self.time_start_bits == other.time_start_bits
            && self.time_end_bits == other.time_end_bits
            && self.sample_count == other.sample_count
            // Buffer identity also catches unload/reload with identical lengths and timestamps.
            // Retaining the Arcs in the key prevents allocator-address reuse (ABA).
            && Arc::ptr_eq(&self.time, &other.time)
            && Arc::ptr_eq(&self.values, &other.values)
    }
}

impl Eq for IntervalStatsCacheChannel {}

#[derive(Clone, Debug, PartialEq)]
struct IntervalChannelStats {
    key: ChannelKey,
    channel_name: String,
    channel_path: String,
    display_label: String,
    tooltip: String,
    value_a: Option<f64>,
    value_b: Option<f64>,
    delta: Option<f64>,
    mean: Option<f64>,
    max: Option<f64>,
    min: Option<f64>,
    peak_to_peak: Option<f64>,
    rms: Option<f64>,
    slope_per_second: Option<f64>,
}

impl Default for PanelYApp {
    fn default() -> Self {
        let (load_result_tx, load_result_rx) = mpsc::channel();
        let (fft_result_tx, fft_result_rx) = mpsc::channel();
        let default_parquet_path = default_parquet_path();
        Self {
            workspace: WorkspaceDataState::with_parquet_path(default_parquet_path.clone()),
            view: ViewState::default(),
            analysis: AnalysisState::default(),
            load: LoadState::default(),
            interaction: InteractionState::default(),
            perf: PerfStats::default(),
            session_preset: None,
            session_preset_status: None,
            plot_config_path: default_plot_config_path(&default_parquet_path),
            plot_config_report: None,
            fit_plot_config_after_load: false,
            pending_session_x_range: None,
            load_result_tx,
            load_result_rx,
            fft_result_tx,
            fft_result_rx,
            next_load_job_id: 1,
            next_fft_job_id: 1,
            show_about: false,
            about_plate_texture: None,
        }
    }
}

impl Default for WorkspaceDataState {
    fn default() -> Self {
        Self::with_parquet_path(default_parquet_path())
    }
}

impl DataSourceState {
    fn new(id: SourceId, parquet_path: String) -> Self {
        let label = source_label(&parquet_path);
        Self {
            id,
            label,
            parquet_path,
            schema: None,
            shared_time: None,
            loaded_channels: ChannelStore::default(),
            time_transform: TimeTransform::default(),
            generation: 0,
        }
    }

    fn set_parquet_path(&mut self, parquet_path: String) -> bool {
        if self.parquet_path == parquet_path {
            return false;
        }
        self.label = source_label(&parquet_path);
        self.parquet_path = parquet_path;
        self.reset();
        true
    }

    fn reset(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("source generation space exhausted");
        self.schema = None;
        self.shared_time = None;
        self.loaded_channels.clear_all();
    }
}

fn source_label(parquet_path: &str) -> String {
    let trimmed = parquet_path.trim();
    if trimmed.is_empty() {
        return "(no file)".to_owned();
    }
    Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(trimmed)
        .to_owned()
}

fn source_presentation(
    workspace: &WorkspaceDataState,
    source_id: SourceId,
) -> Option<SourcePresentation> {
    let source = workspace.source(source_id)?;
    let number = workspace.source_number(source_id)?;
    let file_name = source_label(&source.parquet_path);
    Some(SourcePresentation {
        number,
        short_label: format!("#{number} {file_name}"),
        file_name,
    })
}

fn channel_presentation(
    workspace: &WorkspaceDataState,
    key: &ChannelKey,
) -> Option<ChannelPresentation> {
    let source = workspace.source(key.source_id)?;
    let source_presentation = source_presentation(workspace, key.source_id)?;
    let display_name = source
        .schema
        .as_ref()
        .and_then(|schema| {
            schema
                .channels
                .iter()
                .find(|channel| channel.path == key.channel_path)
        })
        .map(|channel| channel.name.as_str())
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            source
                .loaded_channels
                .channel(&key.channel_path)
                .map(|channel| channel.channel_name.as_str())
                .filter(|name| !name.trim().is_empty())
        })
        .unwrap_or(key.channel_path.as_str());
    Some(ChannelPresentation {
        short_label: format!("{display_name} #{}", source_presentation.number),
        source_label: source_presentation.short_label.clone(),
        tooltip: format!("{} / {}", source_presentation.file_name, key.channel_path),
    })
}

fn channel_label_or_fallback(workspace: &WorkspaceDataState, key: &ChannelKey) -> String {
    channel_presentation(workspace, key)
        .map(|presentation| presentation.short_label)
        .unwrap_or_else(|| key.channel_path.clone())
}

fn channel_tooltip_or_fallback(workspace: &WorkspaceDataState, key: &ChannelKey) -> String {
    channel_presentation(workspace, key)
        .map(|presentation| presentation.tooltip)
        .unwrap_or_else(|| key.channel_path.clone())
}

fn channel_matches_browser_filter(
    channel: &parquet_schema::ColumnInfo,
    presentation: &ChannelPresentation,
    filter: &str,
) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }

    let filter = filter.to_ascii_lowercase();
    [
        channel.name.as_str(),
        channel.path.as_str(),
        channel.display_name(),
        presentation.short_label.as_str(),
        presentation.source_label.as_str(),
        presentation.tooltip.as_str(),
    ]
    .into_iter()
    .any(|value| value.to_ascii_lowercase().contains(&filter))
}

fn matching_channel_outcomes(
    workspace: &WorkspaceDataState,
    selected_key: &ChannelKey,
) -> Vec<MatchingChannelOutcome> {
    let selected_display_name = workspace
        .source(selected_key.source_id)
        .and_then(|source| source.schema.as_ref())
        .and_then(|schema| {
            schema
                .channels
                .iter()
                .find(|channel| channel.path == selected_key.channel_path)
        })
        .map(|channel| channel.name.as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(selected_key.channel_path.as_str());

    workspace
        .source_order
        .iter()
        .copied()
        .map(|source_id| {
            let Some(schema) = workspace
                .source(source_id)
                .and_then(|source| source.schema.as_ref())
            else {
                return MatchingChannelOutcome::MissingSchema(source_id);
            };
            if let Some(channel) = schema
                .channels
                .iter()
                .find(|channel| channel.path == selected_key.channel_path)
            {
                return MatchingChannelOutcome::Match(ChannelKey::new(
                    source_id,
                    channel.path.clone(),
                ));
            }

            let matching_names = schema
                .channels
                .iter()
                .filter(|channel| channel.name == selected_display_name)
                .collect::<Vec<_>>();
            match matching_names.as_slice() {
                [channel] => {
                    MatchingChannelOutcome::Match(ChannelKey::new(source_id, channel.path.clone()))
                }
                [] => MatchingChannelOutcome::NoMatch(source_id),
                _ => MatchingChannelOutcome::Ambiguous(source_id),
            }
        })
        .collect()
}

impl Default for DataSourceState {
    fn default() -> Self {
        Self::new(DEFAULT_SOURCE_ID, default_parquet_path())
    }
}

impl WorkspaceDataState {
    fn with_parquet_path(parquet_path: String) -> Self {
        let source = DataSourceState::new(DEFAULT_SOURCE_ID, parquet_path.clone());
        let mut sources = BTreeMap::new();
        sources.insert(source.id, source);
        Self {
            tile_batch_wall: Duration::ZERO,
            tile_batch_workers: 0,
            sources,
            source_order: vec![DEFAULT_SOURCE_ID],
            selected_source_id: Some(DEFAULT_SOURCE_ID),
            next_source_id: 2,
            recent_parquet_paths: vec![parquet_path],
        }
    }

    fn replace_with_plot_config_v1_source(
        &mut self,
        parquet_path: String,
        schema: parquet_schema::SchemaSummary,
    ) {
        let previous_recent_paths = self.recent_parquet_paths.clone();
        let mut replacement = Self::with_parquet_path(parquet_path.clone());
        replacement
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Plot config v1 replacement always creates Source #1")
            .schema = Some(schema);

        for previous_path in previous_recent_paths.into_iter().rev() {
            remember_recent_parquet_path(&mut replacement.recent_parquet_paths, previous_path);
        }
        remember_recent_parquet_path(&mut replacement.recent_parquet_paths, parquet_path);
        *self = replacement;
    }

    fn source(&self, source_id: SourceId) -> Option<&DataSourceState> {
        self.sources.get(&source_id)
    }

    fn source_mut(&mut self, source_id: SourceId) -> Option<&mut DataSourceState> {
        self.sources.get_mut(&source_id)
    }

    fn selected_source_id(&self) -> Option<SourceId> {
        self.selected_source_id
    }

    fn selected_source(&self) -> Option<&DataSourceState> {
        self.selected_source_id
            .and_then(|source_id| self.source(source_id))
    }

    fn selected_source_mut(&mut self) -> Option<&mut DataSourceState> {
        let source_id = self.selected_source_id?;
        self.source_mut(source_id)
    }

    fn configured_schema_source(&self) -> Option<&DataSourceState> {
        self.source_order.iter().find_map(|source_id| {
            self.source(*source_id)
                .filter(|source| source.schema.is_some() && !source.parquet_path.trim().is_empty())
        })
    }

    fn reserve_source_id(&mut self) -> SourceId {
        let source_id = SourceId::new(self.next_source_id);
        self.next_source_id = self
            .next_source_id
            .checked_add(1)
            .expect("source id space exhausted");
        source_id
    }

    fn add_source(&mut self, parquet_path: String) -> SourceId {
        let source_id = self.reserve_source_id();
        self.sources.insert(
            source_id,
            DataSourceState::new(source_id, parquet_path.clone()),
        );
        self.source_order.push(source_id);
        if self.selected_source_id.is_none() {
            self.selected_source_id = Some(source_id);
        }
        remember_recent_parquet_path(&mut self.recent_parquet_paths, parquet_path);
        source_id
    }

    fn set_source_parquet_path(&mut self, source_id: SourceId, parquet_path: String) -> bool {
        self.source_mut(source_id)
            .is_some_and(|source| source.set_parquet_path(parquet_path))
    }

    fn set_source_time_offset(&mut self, source_id: SourceId, offset_seconds: f64) -> Option<bool> {
        let offset_seconds = canonical_time_offset_seconds(offset_seconds)?;
        let source = self.source_mut(source_id)?;
        if source.time_transform.offset_seconds.to_bits() == offset_seconds.to_bits() {
            return Some(false);
        }
        source.time_transform.offset_seconds = offset_seconds;
        Some(true)
    }

    fn reload_source_parquet_path(&mut self, source_id: SourceId, parquet_path: String) -> bool {
        let Some(source) = self.source_mut(source_id) else {
            return false;
        };
        if !source.set_parquet_path(parquet_path) {
            source.reset();
        }
        true
    }

    fn reset_source(&mut self, source_id: SourceId) -> bool {
        let Some(source) = self.source_mut(source_id) else {
            return false;
        };
        source.reset();
        true
    }

    fn remove_source(&mut self, source_id: SourceId) -> Option<DataSourceState> {
        let removed = self.sources.remove(&source_id)?;
        self.source_order
            .retain(|candidate| *candidate != source_id);
        if self.selected_source_id == Some(source_id) {
            self.selected_source_id = self.source_order.first().copied();
        }
        Some(removed)
    }

    fn channel(&self, key: &ChannelKey) -> Option<&parquet_waveform::ChannelData> {
        self.source(key.source_id)?
            .loaded_channels
            .channel(&key.channel_path)
    }

    fn has_channel(&self, key: &ChannelKey) -> bool {
        self.channel(key).is_some()
    }

    fn source_number(&self, source_id: SourceId) -> Option<usize> {
        self.source_order
            .iter()
            .position(|candidate| *candidate == source_id)
            .map(|index| index + 1)
    }

    fn unload_channel(&mut self, key: &ChannelKey) -> bool {
        self.source_mut(key.source_id)
            .is_some_and(|source| source.loaded_channels.unload_channel(&key.channel_path))
    }

    fn unload_all_channels_for_source(&mut self, source_id: SourceId) -> bool {
        let Some(source) = self.source_mut(source_id) else {
            return false;
        };
        let had_channels = !source.loaded_channels.raw_by_channel.is_empty();
        source.loaded_channels.clear_all();
        had_channels
    }

    fn clear_draw_caches_for_source(&mut self, source_id: SourceId) -> bool {
        self.source_mut(source_id)
            .is_some_and(|source| source.loaded_channels.clear_draw_caches())
    }

    fn cache_stats(&self) -> WorkspaceCacheStats {
        let mut stats = WorkspaceCacheStats {
            source_count: self.sources.len(),
            ..WorkspaceCacheStats::default()
        };
        for source in self.sources.values() {
            stats.loaded_channel_count = stats
                .loaded_channel_count
                .saturating_add(source.loaded_channels.raw_by_channel.len());
            stats.raw_time_memory_bytes = stats.raw_time_memory_bytes.saturating_add(
                source
                    .shared_time
                    .as_ref()
                    .map_or(0, parquet_waveform::TimeData::memory_bytes),
            );
            stats.raw_channel_memory_bytes = stats
                .raw_channel_memory_bytes
                .saturating_add(source.loaded_channels.raw_memory_bytes());
            stats.envelope_count = stats
                .envelope_count
                .saturating_add(source.loaded_channels.envelope_cache.len());
            stats.line_tile_cache_count = stats
                .line_tile_cache_count
                .saturating_add(source.loaded_channels.line_tile_cache.len());
            stats.line_tile_memory_bytes = stats
                .line_tile_memory_bytes
                .saturating_add(source.loaded_channels.line_tile_memory_bytes());
            stats.step_edge_cache_count = stats
                .step_edge_cache_count
                .saturating_add(source.loaded_channels.step_edge_cache.len());
            stats.step_edge_memory_bytes = stats
                .step_edge_memory_bytes
                .saturating_add(source.loaded_channels.step_edge_memory_bytes());
            stats.step_fallback_hint_count = stats
                .step_fallback_hint_count
                .saturating_add(source.loaded_channels.step_fallback_hints.len());
        }
        stats
    }

    fn display_time_range(&self) -> Option<(f64, f64)> {
        let mut combined = None;
        for source_id in &self.source_order {
            let Some(display_range) = self
                .source(*source_id)
                .and_then(data_source_display_time_range)
            else {
                continue;
            };
            combined = extend_range(combined, display_range.0);
            combined = extend_range(combined, display_range.1);
        }
        combined.and_then(normalized_range)
    }

    fn display_overlap_time_range(&self) -> Option<(f64, f64)> {
        let mut sources = self
            .source_order
            .iter()
            .filter_map(|source_id| self.source(*source_id))
            .filter(|source| source.shared_time.is_some());
        let mut overlap = data_source_display_time_range(sources.next()?)?;
        for source in sources {
            overlap = range_intersection(overlap, data_source_display_time_range(source)?)?;
        }
        Some(overlap)
    }

    fn display_min_view_span(&self) -> Option<f64> {
        let sample_span = self
            .source_order
            .iter()
            .filter_map(|source_id| {
                let source = self.source(*source_id)?;
                let raw_range = source.shared_time.as_ref()?.time_range()?;
                let display_range =
                    normalized_range(source.time_transform.display_range(raw_range))?;
                Some(min_view_span(
                    display_range,
                    source.shared_time.as_ref()?.sample_count(),
                ))
            })
            .filter(|span| span.is_finite() && *span > 0.0)
            .reduce(f64::min)?;
        // Keep Fit Overlap inside the common interval even when it is shorter
        // than one sample interval. Rendering and interaction use this limit too.
        Some(
            self.display_overlap_time_range()
                .map_or(sample_span, |(start, end)| sample_span.min(end - start)),
        )
    }

    fn total_time_sample_count(&self) -> usize {
        self.source_order
            .iter()
            .filter_map(|source_id| {
                self.source(*source_id)
                    .and_then(|source| source.shared_time.as_ref())
                    .map(parquet_waveform::TimeData::sample_count)
            })
            .fold(0usize, usize::saturating_add)
    }

    fn max_time_sample_count(&self) -> usize {
        self.source_order
            .iter()
            .filter_map(|source_id| {
                self.source(*source_id)
                    .and_then(|source| source.shared_time.as_ref())
                    .map(parquet_waveform::TimeData::sample_count)
            })
            .max()
            .unwrap_or_default()
    }

    fn clear_envelope_caches(&mut self) {
        for source in self.sources.values_mut() {
            source.loaded_channels.clear_envelope_cache();
        }
    }

    fn pending_line_tile_keys(
        &self,
        rows: &[PlotRow],
        contexts: &BTreeMap<SourceId, SourceRenderContext>,
    ) -> Vec<ChannelKey> {
        rows.iter()
            .flat_map(|row| &row.channels)
            .filter(|channel| channel.visible && channel.draw_mode == DrawMode::Line)
            .filter(|channel| {
                contexts.contains_key(&channel.key.source_id)
                    && self.source(channel.key.source_id).is_some_and(|source| {
                        let store = &source.loaded_channels;
                        store.has_channel(&channel.key.channel_path)
                            && !store
                                .line_tile_cache
                                .contains_key(&channel.key.channel_path)
                            && store
                                .envelope_plan
                                .as_ref()
                                .is_some_and(should_use_line_tile_lod)
                    })
            })
            .map(|channel| channel.key.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn prepare_visible_line_tiles(
        &mut self,
        rows: &[PlotRow],
        contexts: &BTreeMap<SourceId, SourceRenderContext>,
    ) {
        let keys = self.pending_line_tile_keys(rows, contexts);
        if keys.len() < 2 {
            return;
        }
        let values = keys
            .iter()
            .map(|key| {
                self.channel(key)
                    .expect("selected loaded channel")
                    .values
                    .as_slice()
            })
            .collect::<Vec<_>>();
        let total_samples = values
            .iter()
            .fold(0usize, |n, values| n.saturating_add(values.len()));
        if total_samples < LINE_TILE_BATCH_MIN_SAMPLES {
            return;
        }
        let workers = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(LINE_TILE_MAX_WORKERS)
            .min(keys.len());
        if workers < 2 {
            return;
        }
        // Scoped workers borrow immutable raw slices. All joins finish before
        // publishing caches, so no raw copies or pending results outlive a frame.
        let batch =
            parquet_waveform::build_line_tile_caches(&values, LINE_TILE_SAMPLE_WIDTH, workers);
        self.tile_batch_wall += batch.elapsed;
        self.tile_batch_workers = self.tile_batch_workers.max(batch.workers);
        for (key, cache) in keys.into_iter().zip(batch.caches) {
            let store = &mut self
                .source_mut(key.source_id)
                .expect("selected source")
                .loaded_channels;
            store.line_tile_cache.insert(key.channel_path, cache);
            store.last_envelope_stats.tile_builds += 1;
        }
    }

    fn envelope_stats(&self) -> EnvelopeCacheStats {
        let mut combined = EnvelopeCacheStats {
            tile_build_wall: self.tile_batch_wall,
            tile_build_workers: self.tile_batch_workers,
            ..Default::default()
        };
        for source in self.sources.values() {
            combined.accumulate(source.loaded_channels.last_envelope_stats);
        }
        combined
    }
}

fn data_source_display_time_range(source: &DataSourceState) -> Option<(f64, f64)> {
    let raw_range = source.shared_time.as_ref()?.time_range()?;
    normalized_range(source.time_transform.display_range(raw_range))
}

fn canonical_time_offset_seconds(offset_seconds: f64) -> Option<f64> {
    if !offset_seconds.is_finite() {
        return None;
    }
    Some(if offset_seconds == 0.0 {
        0.0
    } else {
        offset_seconds
    })
}

fn source_start_at_zero_offset(source: &DataSourceState) -> Option<f64> {
    let raw_start = source.shared_time.as_ref()?.time.first().copied()?;
    canonical_time_offset_seconds(-raw_start)
}

impl std::ops::Deref for WorkspaceDataState {
    type Target = DataSourceState;

    fn deref(&self) -> &Self::Target {
        self.selected_source()
            .expect("M1 workspace always contains the selected Source #1")
    }
}

impl std::ops::DerefMut for WorkspaceDataState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.selected_source_mut()
            .expect("M1 workspace always contains the selected Source #1")
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            selected_channel: None,
            channel_filter: String::new(),
            channel_state_filter: ChannelStateFilter::All,
            channel_sort: ChannelSort::SchemaOrder,
            selected_row_id: Some(0),
            next_row_id: 1,
            x_range: None,
            hover_x: None,
            large_preview_enabled: false,
            theme: ThemeMode::Dark,
            visual_mode: VisualMode::Plain,
            scroll_zoom: false,
            rows: vec![PlotRow::new(0)],
        }
    }
}

impl Default for AnalysisState {
    fn default() -> Self {
        Self {
            cursor_mode: false,
            cursor_a: None,
            cursor_b: None,
            next_cursor: CursorSlot::A,
            info_panel_visible: true,
            info_panel_collapsed: false,
            info_panel_height: CURSOR_INFO_DEFAULT_HEIGHT,
            pane: AnalysisPane::default(),
            analysis_split_height: ANALYSIS_SPLIT_DEFAULT_HEIGHT,
            interval_stats_cache: IntervalStatsCache::default(),
            interval_stats_worker: interval_stats::IntervalStatsWorker::default(),
            fft: fft_analysis::FftAnalysisState::default(),
        }
    }
}

impl Default for LoadState {
    fn default() -> Self {
        Self {
            status: "Ready".to_owned(),
            pending_jobs: 0,
            progress: None,
            error: None,
            active_jobs: BTreeMap::new(),
        }
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            last_range_change: None,
            box_zoom: None,
        }
    }
}

impl Default for PerfStats {
    fn default() -> Self {
        Self {
            show: true,
            frame_ms: 0.0,
            interaction_ms: 0.0,
            visible_ms: 0.0,
            draw_ms: 0.0,
            rows: 0,
            channels: 0,
            draw_points: 0,
            requested_buckets: 0,
            effective_buckets: 0,
            preview: false,
            envelope_cache_hits: 0,
            envelope_cache_misses: 0,
            envelope_context_hits: 0,
            envelope_context_misses: 0,
            line_tile_hits: 0,
            line_tile_builds: 0,
            step_edge_hits: 0,
            step_edge_builds: 0,
        }
    }
}

impl LoadState {
    fn is_busy(&self) -> bool {
        !self.active_jobs.is_empty()
    }

    fn is_channel_loading(&self, key: &ChannelKey) -> bool {
        self.active_jobs.values().any(|job| job.channel_key == *key)
    }

    fn loading_channel_keys(&self) -> BTreeSet<ChannelKey> {
        self.active_jobs
            .values()
            .map(|job| job.channel_key.clone())
            .collect()
    }

    fn is_source_loading(&self, source_id: SourceId) -> bool {
        self.active_jobs
            .values()
            .any(|job| job.source_id == source_id)
    }

    #[allow(dead_code)]
    fn cancel_source_jobs(&mut self, source_id: SourceId, workspace: &WorkspaceDataState) -> usize {
        let before = self.active_jobs.len();
        self.active_jobs.retain(|_, job| job.source_id != source_id);
        let removed = before - self.active_jobs.len();
        self.refresh_progress(workspace);
        removed
    }

    fn refresh_progress(&mut self, workspace: &WorkspaceDataState) {
        self.pending_jobs = self.active_jobs.len();
        self.progress = if self.active_jobs.is_empty() {
            None
        } else {
            let labels = self
                .active_jobs
                .values()
                .map(|job| {
                    format!(
                        "{} -> {}",
                        channel_label_or_fallback(workspace, &job.channel_key),
                        job.target_row_label
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("loading: {labels}"))
        };
    }
}

impl AnalysisState {
    fn set_cursor_mode(&mut self, enabled: bool) {
        if self.cursor_mode == enabled {
            return;
        }

        self.cursor_mode = enabled;
        if !enabled {
            self.clear_cursors();
        }
    }

    fn clear_cursors(&mut self) {
        self.cursor_a = None;
        self.cursor_b = None;
        self.next_cursor = CursorSlot::A;
        self.invalidate_interval_statistics();
        self.fft.invalidate_result();
    }

    fn clear_all(&mut self) {
        self.cursor_mode = false;
        self.clear_cursors();
        self.fft.reset_for_dataset();
    }

    fn cleanup_source(&mut self, source_id: SourceId) {
        self.clear_cursors();
        self.fft.cleanup_source(source_id);
    }

    fn invalidate_time_transform_dependents(&mut self) {
        self.invalidate_interval_statistics();
        self.fft.invalidate_result();
    }

    fn place_cursor(&mut self, time: f64) -> bool {
        if !time.is_finite() {
            return false;
        }

        match self.next_cursor {
            CursorSlot::A => {
                self.cursor_a = Some(time);
                self.cursor_b = None;
                self.next_cursor = CursorSlot::B;
            }
            CursorSlot::B => {
                self.cursor_b = Some(time);
                self.next_cursor = CursorSlot::A;
            }
        }
        self.invalidate_interval_statistics();
        self.fft.invalidate_result();
        true
    }

    #[cfg(test)]
    fn cursor_interval(&self) -> Option<(f64, f64)> {
        let a = self.cursor_a?;
        let b = self.cursor_b?;
        normalized_range((a, b))
    }

    fn invalidate_interval_statistics(&mut self) {
        self.interval_stats_cache = IntervalStatsCache::default();
        self.interval_stats_worker.invalidate();
    }

    fn interval_channel_statistics(
        &mut self,
        workspace: &WorkspaceDataState,
        view: &ViewState,
        ctx: &egui::Context,
    ) -> Vec<IntervalChannelStats> {
        let key = interval_stats_cache_key(self, workspace, view);
        self.interval_stats_worker
            .update(key, workspace, ctx, &mut self.interval_stats_cache);
        self.interval_stats_cache.stats.clone()
    }
}

impl CursorSlot {
    fn as_str(self) -> &'static str {
        match self {
            CursorSlot::A => "A",
            CursorSlot::B => "B",
        }
    }
}

impl InteractionState {
    fn begin_box_zoom(
        &mut self,
        plot_rect: egui::Rect,
        start_pos: egui::Pos2,
        view_range: (f64, f64),
    ) {
        self.box_zoom = Some(BoxZoomDrag {
            plot_rect,
            start_pos,
            current_pos: start_pos,
            view_range,
        });
    }

    fn update_box_zoom(&mut self, current_pos: egui::Pos2) {
        if let Some(box_zoom) = self.box_zoom.as_mut() {
            box_zoom.current_pos = current_pos;
        }
    }

    fn finish_box_zoom(&mut self) -> Option<BoxZoomDrag> {
        self.box_zoom.take()
    }

    fn mark_range_changed(&mut self) {
        self.last_range_change = Some(Instant::now());
    }

    fn preview_active(&self) -> bool {
        self.last_range_change
            .is_some_and(|changed_at| changed_at.elapsed() < INTERACTION_PREVIEW_SETTLE)
    }
}

impl PerfStats {
    fn update(
        &mut self,
        timing: FrameTiming,
        visible_rows: &[VisibleRowTrace],
        requested_buckets: usize,
        effective_buckets: usize,
        preview: bool,
        envelope_stats: EnvelopeCacheStats,
    ) {
        self.frame_ms = duration_ms(timing.frame);
        self.interaction_ms = duration_ms(timing.interaction);
        self.visible_ms = duration_ms(timing.visible);
        self.draw_ms = duration_ms(timing.draw);
        self.rows = visible_rows.len();
        self.channels = visible_rows
            .iter()
            .map(|row| row.traces.len())
            .sum::<usize>();
        self.draw_points = visible_rows
            .iter()
            .flat_map(|row| row.traces.iter())
            .map(trace_draw_point_count)
            .sum::<usize>();
        self.requested_buckets = requested_buckets;
        self.effective_buckets = effective_buckets;
        self.preview = preview;
        self.envelope_cache_hits = envelope_stats.hits;
        self.envelope_cache_misses = envelope_stats.misses;
        self.envelope_context_hits = envelope_stats.context_hits;
        self.envelope_context_misses = envelope_stats.context_misses;
        self.line_tile_hits = envelope_stats.tile_hits;
        self.line_tile_builds = envelope_stats.tile_builds;
        self.step_edge_hits = envelope_stats.step_edge_hits;
        self.step_edge_builds = envelope_stats.step_edge_builds;
    }

    fn summary(&self) -> String {
        let mode = if self.preview { "preview" } else { "full" };
        format!(
            "{mode} frame {:.1}ms visible {:.1}ms draw {:.1}ms input {:.1}ms rows {} ch {} points {} buckets {}/{} env h/m {}/{} ctx {}/{} tile h/b {}/{} edge h/b {}/{}",
            self.frame_ms,
            self.visible_ms,
            self.draw_ms,
            self.interaction_ms,
            self.rows,
            self.channels,
            self.draw_points,
            self.effective_buckets,
            self.requested_buckets,
            self.envelope_cache_hits,
            self.envelope_cache_misses,
            self.envelope_context_hits,
            self.envelope_context_misses,
            self.line_tile_hits,
            self.line_tile_builds,
            self.step_edge_hits,
            self.step_edge_builds
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct FrameTiming {
    frame: Duration,
    interaction: Duration,
    visible: Duration,
    draw: Duration,
}

impl DrawMode {
    const ALL: [Self; 2] = [Self::Line, Self::Step];

    fn as_str(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Step => "Step",
        }
    }
}

impl ThemeMode {
    fn visuals(self) -> egui::Visuals {
        match self {
            Self::Dark => default_dark_visuals(),
            Self::Light => egui::Visuals::light(),
        }
    }
}

impl DisplayModeChoice {
    const ALL: [Self; 3] = [Self::PlainDark, Self::PlainLight, Self::Rock];

    fn from_view(view: &ViewState) -> Self {
        match view.visual_mode {
            VisualMode::Plain => match view.theme {
                ThemeMode::Dark => Self::PlainDark,
                ThemeMode::Light => Self::PlainLight,
            },
            VisualMode::Rock => Self::Rock,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::PlainDark => "Plain Dark",
            Self::PlainLight => "Plain Light",
            Self::Rock => "Rock",
        }
    }

    fn state(self) -> (ThemeMode, VisualMode) {
        match self {
            Self::PlainDark => (ThemeMode::Dark, VisualMode::Plain),
            Self::PlainLight => (ThemeMode::Light, VisualMode::Plain),
            Self::Rock => (ThemeMode::Dark, VisualMode::Rock),
        }
    }

    fn apply_to_view(self, view: &mut ViewState) {
        let (theme, visual_mode) = self.state();
        view.theme = theme;
        view.visual_mode = visual_mode;
    }
}

fn visual_palette(theme: ThemeMode, visual_mode: VisualMode) -> VisualPalette {
    match visual_mode {
        VisualMode::Plain => plain_palette(theme),
        VisualMode::Rock => rock_palette(),
    }
}

fn plain_palette(theme: ThemeMode) -> VisualPalette {
    match theme {
        ThemeMode::Dark => default_dark_palette(),
        ThemeMode::Light => plain_light_palette(),
    }
}

fn plain_light_palette() -> VisualPalette {
    let visuals = egui::Visuals::light();
    let accent_blue = egui::Color32::from_rgb(0, 112, 190);

    VisualPalette {
        visual_mode: VisualMode::Plain,
        skin: VisualSkin::Standard,
        control_button_treatment: ControlButtonTreatment::Standard,
        control_toggle_treatment: ControlToggleTreatment::Standard,
        control_section_header_treatment: ControlSectionHeaderTreatment::Standard,
        control_status_indicator_treatment: ControlStatusIndicatorTreatment::Standard,
        control_text_input_treatment: ControlTextInputTreatment::Standard,
        control_surface_texture_treatment: ControlSurfaceTextureTreatment::Standard,
        app_bg: visuals.extreme_bg_color,
        panel_bg: visuals.panel_fill,
        panel_bg_alt: visuals.widgets.noninteractive.weak_bg_fill,
        input_bg: visuals.extreme_bg_color,
        plot_bg: visuals.extreme_bg_color,
        text_primary: visuals.widgets.noninteractive.fg_stroke.color,
        text_secondary: visuals.weak_text_color.unwrap_or_else(|| {
            visuals
                .widgets
                .noninteractive
                .fg_stroke
                .color
                .gamma_multiply(visuals.weak_text_alpha)
        }),
        accent_blue,
        accent_blue_deep: accent_blue,
        accent_blue_dim: visuals.selection.stroke.color,
        grid_minor: visuals.faint_bg_color,
        grid_major: visuals.faint_bg_color,
        selected_frame: visuals.selection.stroke.color,
        warning: visuals.warn_fg_color,
        danger: visuals.error_fg_color,
    }
}

fn default_dark_palette() -> VisualPalette {
    VisualPalette {
        visual_mode: VisualMode::Plain,
        skin: VisualSkin::Industrial,
        control_button_treatment: ControlButtonTreatment::PlainDarkMachinery,
        control_toggle_treatment: ControlToggleTreatment::Standard,
        control_section_header_treatment: ControlSectionHeaderTreatment::Standard,
        control_status_indicator_treatment: ControlStatusIndicatorTreatment::Standard,
        control_text_input_treatment: ControlTextInputTreatment::Standard,
        control_surface_texture_treatment: ControlSurfaceTextureTreatment::Standard,
        app_bg: egui::Color32::from_rgb(5, 9, 13),
        panel_bg: egui::Color32::from_rgb(16, 22, 27),
        panel_bg_alt: egui::Color32::from_rgb(21, 28, 34),
        input_bg: egui::Color32::from_rgb(7, 11, 15),
        plot_bg: egui::Color32::from_rgb(3, 16, 24),
        text_primary: egui::Color32::from_rgb(214, 227, 234),
        text_secondary: egui::Color32::from_rgb(142, 164, 175),
        accent_blue: egui::Color32::from_rgb(69, 216, 255),
        accent_blue_deep: egui::Color32::from_rgb(24, 183, 255),
        accent_blue_dim: egui::Color32::from_rgb(11, 90, 117),
        grid_minor: egui::Color32::from_rgba_unmultiplied(27, 109, 128, 42),
        grid_major: egui::Color32::from_rgba_unmultiplied(69, 216, 255, 74),
        selected_frame: egui::Color32::from_rgba_unmultiplied(69, 216, 255, 180),
        warning: egui::Color32::from_rgb(255, 184, 74),
        danger: egui::Color32::from_rgb(255, 92, 122),
    }
}

fn rock_palette() -> VisualPalette {
    VisualPalette {
        visual_mode: VisualMode::Rock,
        skin: VisualSkin::Industrial,
        control_button_treatment: ControlButtonTreatment::RockMachinery,
        control_toggle_treatment: ControlToggleTreatment::RockMachinery,
        control_section_header_treatment: ControlSectionHeaderTreatment::RockMachineryRack,
        control_status_indicator_treatment: ControlStatusIndicatorTreatment::RockMachineryLed,
        control_text_input_treatment: ControlTextInputTreatment::RockInstrumentDisplay,
        control_surface_texture_treatment: ControlSurfaceTextureTreatment::RockSubtleGrime,
        app_bg: egui::Color32::from_rgb(5, 9, 13),
        panel_bg: egui::Color32::from_rgb(16, 22, 27),
        panel_bg_alt: egui::Color32::from_rgb(21, 28, 34),
        input_bg: egui::Color32::from_rgb(7, 11, 15),
        plot_bg: egui::Color32::from_rgb(3, 16, 24),
        text_primary: egui::Color32::from_rgb(214, 227, 234),
        text_secondary: egui::Color32::from_rgb(142, 164, 175),
        accent_blue: egui::Color32::from_rgb(69, 216, 255),
        accent_blue_deep: egui::Color32::from_rgb(24, 183, 255),
        accent_blue_dim: egui::Color32::from_rgb(11, 90, 117),
        grid_minor: egui::Color32::from_rgba_unmultiplied(27, 109, 128, 42),
        grid_major: egui::Color32::from_rgba_unmultiplied(69, 216, 255, 74),
        selected_frame: egui::Color32::from_rgba_unmultiplied(69, 216, 255, 180),
        warning: egui::Color32::from_rgb(255, 184, 74),
        danger: egui::Color32::from_rgb(255, 92, 122),
    }
}

fn view_visuals(theme: ThemeMode, visual_mode: VisualMode) -> egui::Visuals {
    match visual_mode {
        VisualMode::Plain => theme.visuals(),
        VisualMode::Rock => rock_visuals(),
    }
}

fn default_dark_visuals() -> egui::Visuals {
    let palette = default_dark_palette();
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(palette.text_primary);
    visuals.weak_text_color = Some(palette.text_secondary);
    visuals.panel_fill = palette.panel_bg;
    visuals.window_fill = palette.panel_bg;
    visuals.extreme_bg_color = palette.app_bg;
    visuals.text_edit_bg_color = Some(palette.input_bg);
    visuals.code_bg_color = palette.input_bg;
    visuals.faint_bg_color = egui::Color32::from_rgba_unmultiplied(27, 109, 128, 28);
    visuals.hyperlink_color = palette.accent_blue_deep;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;
    visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(24, 183, 255, 54);
    visuals.selection.stroke = egui::Stroke::new(1.0, palette.accent_blue);
    visuals.window_stroke = egui::Stroke::new(1.0, palette.accent_blue_dim);
    visuals.window_corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.noninteractive.bg_fill = palette.panel_bg;
    visuals.widgets.noninteractive.weak_bg_fill = palette.panel_bg_alt;
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(24, 57, 67));
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.inactive.bg_fill = palette.input_bg;
    visuals.widgets.inactive.weak_bg_fill = palette.panel_bg_alt;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.accent_blue_dim);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(12, 31, 40);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(18, 42, 52);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.accent_blue);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.active.bg_fill = egui::Color32::from_rgba_unmultiplied(24, 183, 255, 48);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(16, 46, 58);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.accent_blue);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.open = visuals.widgets.hovered;
    visuals
}

fn rock_visuals() -> egui::Visuals {
    let palette = rock_palette();
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(palette.text_primary);
    visuals.weak_text_color = Some(palette.text_secondary);
    visuals.panel_fill = palette.panel_bg;
    visuals.window_fill = palette.panel_bg;
    visuals.extreme_bg_color = palette.app_bg;
    visuals.text_edit_bg_color = Some(palette.input_bg);
    visuals.code_bg_color = palette.input_bg;
    visuals.faint_bg_color = egui::Color32::from_rgba_unmultiplied(27, 109, 128, 28);
    visuals.hyperlink_color = palette.accent_blue_deep;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;
    visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(24, 183, 255, 54);
    visuals.selection.stroke = egui::Stroke::new(1.0, palette.accent_blue);
    visuals.window_stroke = egui::Stroke::new(1.0, palette.accent_blue_dim);
    visuals.window_corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.noninteractive.bg_fill = palette.panel_bg;
    visuals.widgets.noninteractive.weak_bg_fill = palette.panel_bg_alt;
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(24, 57, 67));
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.inactive.bg_fill = palette.input_bg;
    visuals.widgets.inactive.weak_bg_fill = palette.panel_bg_alt;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.accent_blue_dim);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(12, 31, 40);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(18, 42, 52);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.accent_blue);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.active.bg_fill = egui::Color32::from_rgba_unmultiplied(24, 183, 255, 48);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(16, 46, 58);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.accent_blue);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.open = visuals.widgets.hovered;
    visuals
}

fn apply_view_visuals(ctx: &egui::Context, view: &ViewState) {
    ctx.set_visuals(view_visuals(view.theme, view.visual_mode));
}

fn add_control_text_edit_enabled<'a>(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    enabled: bool,
    text_edit: egui::TextEdit<'a>,
) -> egui::Response {
    match palette.control_text_input_treatment {
        ControlTextInputTreatment::Standard => ui.add_enabled(enabled, text_edit),
        ControlTextInputTreatment::RockInstrumentDisplay => {
            let Some(accent) = control_text_input_accent(palette) else {
                return ui.add_enabled(enabled, text_edit);
            };
            let frame = egui::Frame::new()
                .fill(accent.background)
                .stroke(egui::Stroke::new(1.0, accent.edge.linear_multiply(0.42)))
                .inner_margin(egui::Margin::symmetric(6, 2))
                .corner_radius(egui::CornerRadius::same(3));

            let response = ui
                .scope(|ui| {
                    let visuals = ui.visuals_mut();
                    visuals.text_edit_bg_color = Some(accent.background);
                    visuals.selection.bg_fill = accent.selection_bg;
                    visuals.selection.stroke = egui::Stroke::new(1.0, accent.selection_stroke);
                    visuals.widgets.inactive.bg_fill = accent.background;
                    visuals.widgets.inactive.bg_stroke =
                        egui::Stroke::new(1.0, accent.edge.linear_multiply(0.48));
                    visuals.widgets.hovered.bg_fill = accent.background;
                    visuals.widgets.hovered.bg_stroke =
                        egui::Stroke::new(1.0, accent.edge.linear_multiply(0.86));
                    visuals.widgets.active.bg_fill = accent.background;
                    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.1, accent.edge);
                    ui.add_enabled(enabled, text_edit.frame(frame))
                })
                .inner;

            paint_control_text_input_treatment(ui, accent, &response);
            response
        }
    }
}

fn paint_control_text_input_treatment(
    ui: &mut egui::Ui,
    accent: ControlTextInputAccent,
    response: &egui::Response,
) {
    if !ui.is_rect_visible(response.rect) {
        return;
    }

    let enabled = response.enabled();
    let focused = response.has_focus();
    let lit = response.hovered() || focused;
    let (edge_alpha, inner_glow_alpha, outer_glow_alpha, highlight_alpha, shadow_alpha) =
        if !enabled {
            (54, 10, 0, 26, 120)
        } else if focused {
            (238, 92, 46, 122, 190)
        } else if lit {
            (188, 58, 24, 88, 178)
        } else {
            (108, 24, 0, 48, 164)
        };

    let rect = response.rect.shrink(0.5);
    let corner = egui::CornerRadius::same(3);
    let painter = ui.painter();

    if outer_glow_alpha > 0 {
        painter.rect_stroke(
            rect.expand(3.0),
            corner,
            egui::Stroke::new(
                1.2,
                control_button_color_with_alpha(accent.glow, outer_glow_alpha),
            ),
            egui::StrokeKind::Outside,
        );
    }
    painter.rect_stroke(
        rect.expand(1.5),
        corner,
        egui::Stroke::new(
            1.0,
            control_button_color_with_alpha(accent.glow, inner_glow_alpha),
        ),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        rect,
        corner,
        egui::Stroke::new(1.0, accent.edge.linear_multiply(edge_alpha as f32 / 255.0)),
        egui::StrokeKind::Inside,
    );

    let inner = rect.shrink(1.0);
    painter.line_segment(
        [
            egui::pos2(inner.left() + 3.0, inner.top()),
            egui::pos2(inner.right() - 3.0, inner.top()),
        ],
        egui::Stroke::new(
            1.0,
            control_button_color_with_alpha(accent.shadow, shadow_alpha),
        ),
    );
    painter.line_segment(
        [
            egui::pos2(inner.left(), inner.top() + 3.0),
            egui::pos2(inner.left(), inner.bottom() - 3.0),
        ],
        egui::Stroke::new(
            1.0,
            control_button_color_with_alpha(accent.shadow, shadow_alpha.saturating_sub(34)),
        ),
    );
    painter.line_segment(
        [
            egui::pos2(inner.left() + 4.0, inner.bottom()),
            egui::pos2(inner.right() - 4.0, inner.bottom()),
        ],
        egui::Stroke::new(
            1.0,
            accent
                .highlight
                .linear_multiply(highlight_alpha as f32 / 255.0),
        ),
    );
}

fn add_control_separator(ui: &mut egui::Ui, palette: VisualPalette) -> egui::Response {
    let response = ui.separator();
    paint_control_separator_treatment(ui, palette, response.rect);
    response
}

fn paint_control_separator_treatment(ui: &egui::Ui, palette: VisualPalette, rect: egui::Rect) {
    let Some(accent) = control_surface_texture_accent(palette) else {
        return;
    };
    if rect.width() <= 0.0 || rect.height() <= 0.0 || !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter();
    let shadow = egui::Stroke::new(1.0, accent.separator_shadow);
    let edge = egui::Stroke::new(1.0, accent.separator_edge);
    let glow = egui::Stroke::new(1.0, accent.separator_glow);

    if rect.width() <= rect.height() {
        let x = rect.center().x;
        painter.line_segment(
            [
                egui::pos2(x - 1.0, rect.top()),
                egui::pos2(x - 1.0, rect.bottom()),
            ],
            glow,
        );
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            shadow,
        );
        painter.line_segment(
            [
                egui::pos2(x + 1.0, rect.top()),
                egui::pos2(x + 1.0, rect.bottom()),
            ],
            edge,
        );
    } else {
        let y = rect.center().y;
        painter.line_segment(
            [
                egui::pos2(rect.left(), y - 1.0),
                egui::pos2(rect.right(), y - 1.0),
            ],
            glow,
        );
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            shadow,
        );
        painter.line_segment(
            [
                egui::pos2(rect.left(), y + 1.0),
                egui::pos2(rect.right(), y + 1.0),
            ],
            edge,
        );
    }
}

fn paint_control_surface_texture(ui: &egui::Ui, palette: VisualPalette, rect: egui::Rect) {
    paint_control_surface_texture_rect(ui.painter(), palette, rect);
}

fn paint_waveform_surface_texture(
    painter: &egui::Painter,
    palette: VisualPalette,
    rect: egui::Rect,
    row_rects: &[egui::Rect],
    plot_rects: &[egui::Rect],
) {
    if control_surface_texture_accent(palette).is_none() {
        return;
    }
    if row_rects.is_empty() || plot_rects.is_empty() {
        paint_control_surface_texture_rect(painter, palette, rect);
        return;
    }

    let mut previous_bottom = rect.top();
    for row_rect in row_rects {
        paint_control_surface_texture_rect(
            painter,
            palette,
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), previous_bottom),
                egui::pos2(rect.right(), row_rect.top()),
            ),
        );
        previous_bottom = row_rect.bottom();
    }
    paint_control_surface_texture_rect(
        painter,
        palette,
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), previous_bottom),
            egui::pos2(rect.right(), rect.bottom()),
        ),
    );

    for (row_rect, plot_rect) in row_rects.iter().zip(plot_rects) {
        paint_control_surface_texture_rect(
            painter,
            palette,
            egui::Rect::from_min_max(
                row_rect.left_top(),
                egui::pos2(row_rect.right(), plot_rect.top()),
            ),
        );
        paint_control_surface_texture_rect(
            painter,
            palette,
            egui::Rect::from_min_max(
                egui::pos2(row_rect.left(), plot_rect.bottom()),
                row_rect.right_bottom(),
            ),
        );
        paint_control_surface_texture_rect(
            painter,
            palette,
            egui::Rect::from_min_max(
                egui::pos2(row_rect.left(), plot_rect.top()),
                egui::pos2(plot_rect.left(), plot_rect.bottom()),
            ),
        );
        paint_control_surface_texture_rect(
            painter,
            palette,
            egui::Rect::from_min_max(
                egui::pos2(plot_rect.right(), plot_rect.top()),
                egui::pos2(row_rect.right(), plot_rect.bottom()),
            ),
        );
    }
}

fn paint_control_surface_texture_rect(
    painter: &egui::Painter,
    palette: VisualPalette,
    rect: egui::Rect,
) {
    let Some(accent) = control_surface_texture_accent(palette) else {
        return;
    };
    let rect = rect.intersect(painter.clip_rect());
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }

    paint_surface_scanlines(painter, rect, accent.scanline);
    paint_surface_scratches(painter, rect, accent.scratch);
    paint_surface_grime(painter, rect, accent.grime);
}

fn paint_surface_scanlines(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let interval = 7.0;
    let first = (rect.top() / interval).floor() as i32;
    let last = (rect.bottom() / interval).ceil() as i32;

    for row in first..=last {
        let y = row as f32 * interval + 0.5;
        if y < rect.top() || y > rect.bottom() {
            continue;
        }
        let alpha = color
            .a()
            .saturating_add((surface_hash(row as u32) & 0x03) as u8);
        painter.hline(
            rect.x_range(),
            y,
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha),
            ),
        );
    }
}

fn paint_surface_scratches(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let cell_w = 96.0;
    let cell_h = 52.0;
    let first_col = (rect.left() / cell_w).floor() as i32;
    let last_col = (rect.right() / cell_w).ceil() as i32;
    let first_row = (rect.top() / cell_h).floor() as i32;
    let last_row = (rect.bottom() / cell_h).ceil() as i32;

    for row in first_row..=last_row {
        for col in first_col..=last_col {
            let seed = surface_hash(
                (row as u32)
                    .wrapping_mul(0x9e37_79b9)
                    .wrapping_add(col as u32)
                    ^ 0x67d1_4a2f,
            );
            if seed % 11 > 1 {
                continue;
            }

            let x_jitter = ((seed >> 8) & 0x03ff) as f32 / 1023.0;
            let y_jitter = ((seed >> 18) & 0x03ff) as f32 / 1023.0;
            let start = egui::pos2(
                col as f32 * cell_w + 6.0 + x_jitter * (cell_w - 18.0),
                row as f32 * cell_h + 5.0 + y_jitter * (cell_h - 10.0),
            );
            if !rect.contains(start) {
                continue;
            }

            let length = 10.0 + ((seed >> 3) & 0x1f) as f32;
            let slope = if seed & 0x01 == 0 { 1.6 } else { -1.2 };
            let end = egui::pos2(
                (start.x + length).min(rect.right()),
                (start.y + slope).clamp(rect.top(), rect.bottom()),
            );
            painter.line_segment([start, end], egui::Stroke::new(1.0, color));
        }
    }
}

fn paint_surface_grime(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let cell = 22.0;
    let first_col = (rect.left() / cell).floor() as i32;
    let last_col = (rect.right() / cell).ceil() as i32;
    let first_row = (rect.top() / cell).floor() as i32;
    let last_row = (rect.bottom() / cell).ceil() as i32;

    for row in first_row..=last_row {
        for col in first_col..=last_col {
            let seed = surface_hash(
                (row as u32)
                    .wrapping_mul(0x85eb_ca6b)
                    .wrapping_add((col as u32).wrapping_mul(0xc2b2_ae35))
                    ^ 0xa53f_9e17,
            );
            if seed % 17 != 0 {
                continue;
            }

            let x_jitter = ((seed >> 9) & 0x01ff) as f32 / 511.0;
            let y_jitter = ((seed >> 18) & 0x01ff) as f32 / 511.0;
            let center = egui::pos2(
                col as f32 * cell + 2.0 + x_jitter * (cell - 4.0),
                row as f32 * cell + 2.0 + y_jitter * (cell - 4.0),
            );
            if rect.contains(center) {
                let radius = 0.7 + ((seed >> 27) & 0x03) as f32 * 0.18;
                painter.circle_filled(center, radius, color);
            }
        }
    }
}

fn surface_hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

fn add_control_button<'a>(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    button: egui::Button<'a>,
) -> egui::Response {
    add_control_button_enabled_with_role(ui, palette, true, ControlButtonRole::Default, button)
}

fn add_control_button_enabled<'a>(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    enabled: bool,
    button: egui::Button<'a>,
) -> egui::Response {
    add_control_button_enabled_with_role(ui, palette, enabled, ControlButtonRole::Default, button)
}

fn add_control_button_with_role<'a>(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    role: ControlButtonRole,
    button: egui::Button<'a>,
) -> egui::Response {
    add_control_button_enabled_with_role(ui, palette, true, role, button)
}

fn add_control_button_enabled_with_role<'a>(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    enabled: bool,
    role: ControlButtonRole,
    button: egui::Button<'a>,
) -> egui::Response {
    let response = ui.add_enabled(enabled, styled_control_button(palette, button));
    paint_control_button_treatment(ui, palette, role, &response);
    response
}

fn styled_control_button<'a>(palette: VisualPalette, button: egui::Button<'a>) -> egui::Button<'a> {
    match palette.control_button_treatment {
        ControlButtonTreatment::Standard => button,
        ControlButtonTreatment::PlainDarkMachinery | ControlButtonTreatment::RockMachinery => {
            button.corner_radius(egui::CornerRadius::same(3))
        }
    }
}

fn paint_control_button_treatment(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    role: ControlButtonRole,
    response: &egui::Response,
) {
    if !palette.control_button_treatment.uses_machinery() || !ui.is_rect_visible(response.rect) {
        return;
    }

    let rect = response.rect.shrink(0.5);
    let enabled = response.enabled();
    let active = response.is_pointer_button_down_on();
    let lit = response.hovered() || response.has_focus();
    let (edge_alpha, inner_glow_alpha, outer_glow_alpha, bevel_alpha) = if !enabled {
        (54, 12, 0, 42)
    } else if active {
        (240, 84, 42, 130)
    } else if lit {
        (226, 108, 66, 158)
    } else {
        (150, 44, 24, 96)
    };

    let painter = ui.painter();
    let corner = egui::CornerRadius::same(4);
    let accent = control_button_accent(palette, role);
    let shadow = egui::Color32::from_rgba_unmultiplied(0, 0, 0, if active { 140 } else { 92 });

    if outer_glow_alpha > 0 {
        painter.rect_stroke(
            rect.expand(4.0),
            corner,
            egui::Stroke::new(
                1.2,
                control_button_color_with_alpha(accent.glow, outer_glow_alpha),
            ),
            egui::StrokeKind::Outside,
        );
    }
    painter.rect_stroke(
        rect.expand(2.0),
        corner,
        egui::Stroke::new(
            1.0,
            control_button_color_with_alpha(accent.glow, inner_glow_alpha),
        ),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        rect,
        corner,
        egui::Stroke::new(1.1, accent.edge.linear_multiply(edge_alpha as f32 / 255.0)),
        egui::StrokeKind::Inside,
    );

    let top_y = rect.top() + 1.0;
    let bottom_y = rect.bottom() - 1.0;
    painter.line_segment(
        [
            egui::pos2(rect.left() + 4.0, top_y),
            egui::pos2(rect.right() - 4.0, top_y),
        ],
        egui::Stroke::new(
            1.0,
            accent.highlight.linear_multiply(bevel_alpha as f32 / 255.0),
        ),
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + 4.0, bottom_y),
            egui::pos2(rect.right() - 4.0, bottom_y),
        ],
        egui::Stroke::new(1.0, shadow),
    );

    if lit && enabled {
        let corner_len = 7.0_f32.min(rect.width() * 0.28).min(rect.height() * 0.45);
        let spark = egui::Stroke::new(1.2, accent.highlight.linear_multiply(0.86));
        painter.line_segment(
            [
                egui::pos2(rect.left() + 1.5, rect.top() + 1.5),
                egui::pos2(rect.left() + 1.5 + corner_len, rect.top() + 1.5),
            ],
            spark,
        );
        painter.line_segment(
            [
                egui::pos2(rect.right() - 1.5 - corner_len, rect.bottom() - 1.5),
                egui::pos2(rect.right() - 1.5, rect.bottom() - 1.5),
            ],
            spark,
        );
    }
}

fn add_section_header(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    title: impl Into<egui::RichText>,
) -> egui::Response {
    let title = title.into().heading();
    match palette.control_section_header_treatment {
        ControlSectionHeaderTreatment::Standard => ui.add(egui::Label::new(title)),
        ControlSectionHeaderTreatment::RockMachineryRack => ui.add(MachinerySectionHeader {
            title: title.into(),
            palette,
        }),
    }
}

struct MachinerySectionHeader {
    title: egui::WidgetText,
    palette: VisualPalette,
}

impl egui::Widget for MachinerySectionHeader {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self { title, palette } = self;

        let title_text = title.text().to_owned();
        let galley = title.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        );
        let galley_size = galley.size();
        let is_horizontal = matches!(
            ui.layout().main_dir,
            egui::Direction::LeftToRight | egui::Direction::RightToLeft
        );
        let compact_width = galley_size.x + 44.0;
        let desired_width = if is_horizontal {
            compact_width
        } else {
            ui.available_width().max(compact_width)
        };
        let desired_size = egui::vec2(desired_width, 26.0_f32.max(galley_size.y + 8.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Label,
                ui.is_enabled(),
                title_text.as_str(),
            )
        });

        if ui.is_rect_visible(rect) {
            let Some(accent) = control_section_header_accent(palette) else {
                return response;
            };
            let painter = ui.painter();
            let body = rect.shrink2(egui::vec2(0.0, 1.0));
            let corner = egui::CornerRadius::same(2);

            painter.rect_filled(body, corner, palette.panel_bg_alt);
            painter.rect_stroke(
                body,
                corner,
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(70, 94, 106, 116)),
                egui::StrokeKind::Inside,
            );

            painter.line_segment(
                [
                    egui::pos2(body.left() + 4.0, body.top() + 1.0),
                    egui::pos2(body.right() - 4.0, body.top() + 1.0),
                ],
                egui::Stroke::new(1.0, accent.highlight.linear_multiply(0.26)),
            );
            painter.line_segment(
                [
                    egui::pos2(body.left() + 4.0, body.bottom() - 1.0),
                    egui::pos2(body.right() - 4.0, body.bottom() - 1.0),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 118)),
            );

            let line_y = body.bottom() - 4.0;
            painter.line_segment(
                [
                    egui::pos2(body.left() + 18.0, line_y),
                    egui::pos2(body.right() - 18.0, line_y),
                ],
                egui::Stroke::new(2.0, control_button_color_with_alpha(accent.glow, 34)),
            );
            painter.line_segment(
                [
                    egui::pos2(body.left() + 18.0, line_y),
                    egui::pos2(body.right() - 18.0, line_y),
                ],
                egui::Stroke::new(1.0, control_button_color_with_alpha(accent.line, 166)),
            );

            let screw_y = body.center().y;
            for screw_x in [body.left() + 8.0, body.right() - 8.0] {
                if screw_x <= body.left() + 3.0 || screw_x >= body.right() - 3.0 {
                    continue;
                }
                let center = egui::pos2(screw_x, screw_y);
                painter.circle_filled(center, 3.2, egui::Color32::from_rgb(8, 13, 17));
                painter.circle_stroke(
                    center,
                    3.2,
                    egui::Stroke::new(1.0, accent.screw_edge.linear_multiply(0.86)),
                );
                painter.circle_filled(
                    center + egui::vec2(-0.8, -0.8),
                    0.75,
                    accent.highlight.linear_multiply(0.42),
                );
            }

            let text_pos = egui::pos2(
                body.left() + 20.0,
                body.center().y - galley_size.y * 0.5 - 0.5,
            );
            painter.galley(text_pos, galley, palette.text_primary);
        }

        response
    }
}

fn add_status_indicator(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    level: ControlStatusLevel,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    add_status_indicator_impl(ui, palette, level, text, false)
}

fn add_status_indicator_wrapped(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    level: ControlStatusLevel,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    add_status_indicator_impl(ui, palette, level, text, true)
}

fn add_status_indicator_monospace(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    level: ControlStatusLevel,
    text: impl Into<String>,
) -> egui::Response {
    add_status_indicator(
        ui,
        palette,
        level,
        egui::RichText::new(text.into()).monospace(),
    )
}

fn add_compact_status_indicator(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    level: ControlStatusLevel,
    text: &str,
    max_chars: usize,
) -> egui::Response {
    let compact = compact_channel_label(text, max_chars);
    add_status_indicator(ui, palette, level, compact)
}

fn add_status_indicator_impl(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    level: ControlStatusLevel,
    text: impl Into<egui::WidgetText>,
    wrap: bool,
) -> egui::Response {
    let text = text.into();
    match palette.control_status_indicator_treatment {
        ControlStatusIndicatorTreatment::Standard => {
            let label = egui::Label::new(text);
            if wrap {
                ui.add(label.wrap())
            } else {
                ui.add(label)
            }
        }
        ControlStatusIndicatorTreatment::RockMachineryLed => ui.add(MachineryStatusIndicator {
            text,
            palette,
            level,
            wrap,
        }),
    }
}

struct MachineryStatusIndicator {
    text: egui::WidgetText,
    palette: VisualPalette,
    level: ControlStatusLevel,
    wrap: bool,
}

impl egui::Widget for MachineryStatusIndicator {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            text,
            palette,
            level,
            wrap,
        } = self;

        let status_text = text.text().to_owned();
        let led_slot_width = 13.0;
        let text_gap = 5.0;
        let text_width = if wrap {
            (ui.available_width() - led_slot_width - text_gap).max(1.0)
        } else {
            f32::INFINITY
        };
        let wrap_mode = if wrap {
            egui::TextWrapMode::Wrap
        } else {
            egui::TextWrapMode::Extend
        };
        let galley = text.into_galley(
            ui,
            Some(wrap_mode),
            text_width,
            egui::FontSelection::Default,
        );
        let galley_size = galley.size();
        let desired_width = if wrap {
            ui.available_width()
                .max(led_slot_width + text_gap + galley_size.x)
        } else {
            led_slot_width + text_gap + galley_size.x
        };
        let desired_height = ui.spacing().interact_size.y.max(galley_size.y).max(16.0);
        let desired_size = egui::vec2(desired_width, desired_height);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Label,
                ui.is_enabled(),
                status_text.as_str(),
            )
        });

        if ui.is_rect_visible(rect) {
            if let Some(accent) = control_status_indicator_accent(palette, level) {
                paint_status_led(ui, rect, accent, ui.is_enabled());
            }

            let text_pos = egui::pos2(
                rect.left() + led_slot_width + text_gap,
                rect.center().y - galley_size.y * 0.5,
            );
            ui.painter().galley(text_pos, galley, palette.text_primary);
        }

        response
    }
}

fn paint_status_led(
    ui: &egui::Ui,
    rect: egui::Rect,
    accent: ControlStatusIndicatorAccent,
    enabled: bool,
) {
    let painter = ui.painter();
    let center = egui::pos2(rect.left() + 6.0, rect.center().y);
    let power = if enabled { 1.0 } else { 0.35 };

    painter.circle_filled(
        center,
        7.1,
        control_button_color_with_alpha(accent.glow, (34.0 * power) as u8),
    );
    painter.circle_filled(center, 5.7, egui::Color32::from_rgb(3, 7, 9));
    painter.circle_stroke(
        center,
        5.7,
        egui::Stroke::new(1.0, accent.edge.linear_multiply(0.88 * power)),
    );
    painter.circle_filled(center, 3.8, accent.core.linear_multiply(0.82 * power));
    painter.circle_filled(
        center + egui::vec2(-1.1, -1.2),
        1.15,
        accent.highlight.linear_multiply(0.86 * power),
    );
    painter.circle_stroke(
        center,
        8.4,
        egui::Stroke::new(
            1.0,
            control_button_color_with_alpha(accent.glow, (24.0 * power) as u8),
        ),
    );
}

fn add_control_checkbox(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    checked: &mut bool,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    add_control_checkbox_enabled(ui, palette, true, checked, label)
}

fn add_control_checkbox_enabled(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    enabled: bool,
    checked: &mut bool,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    let label = label.into();
    match palette.control_toggle_treatment {
        ControlToggleTreatment::Standard => {
            ui.add_enabled(enabled, egui::Checkbox::new(checked, label))
        }
        ControlToggleTreatment::RockMachinery => ui.add_enabled(
            enabled,
            MachineryCheckbox {
                checked,
                label,
                palette,
            },
        ),
    }
}

struct MachineryCheckbox<'a> {
    checked: &'a mut bool,
    label: egui::WidgetText,
    palette: VisualPalette,
}

impl egui::Widget for MachineryCheckbox<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            checked,
            label,
            palette,
        } = self;

        let label_text = label.text().to_owned();
        let has_label = !label.is_empty();
        let switch_size = egui::vec2(36.0, 18.0);
        let label_gap = if has_label {
            ui.spacing().icon_spacing
        } else {
            0.0
        };
        let label_width = (ui.available_width() - switch_size.x - label_gap).max(0.0);
        let label_galley = has_label.then(|| {
            label.into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                label_width,
                egui::FontSelection::Default,
            )
        });
        let label_size = label_galley
            .as_ref()
            .map_or(egui::Vec2::ZERO, |galley| galley.size());
        let desired_size = egui::vec2(
            switch_size.x + label_gap + label_size.x,
            ui.spacing()
                .interact_size
                .y
                .max(switch_size.y)
                .max(label_size.y),
        );
        let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if response.clicked() {
            *checked = !*checked;
            response.mark_changed();
        }

        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                ui.is_enabled(),
                *checked,
                label_text.as_str(),
            )
        });

        if ui.is_rect_visible(rect) {
            let switch_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.center().y - switch_size.y * 0.5),
                switch_size,
            );
            paint_control_toggle_machinery(ui, palette, &response, switch_rect, *checked);

            if let Some(galley) = label_galley {
                let text_color = ui.style().interact(&response).text_color();
                let label_pos = egui::pos2(
                    switch_rect.right() + label_gap,
                    rect.center().y - label_size.y * 0.5,
                );
                ui.painter().galley(label_pos, galley, text_color);
            }

            if let Some(cursor) = ui.visuals().interact_cursor
                && response.hovered()
            {
                ui.ctx().set_cursor_icon(cursor);
            }
        }

        response
    }
}

fn paint_control_toggle_machinery(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    response: &egui::Response,
    switch_rect: egui::Rect,
    checked: bool,
) {
    let Some(accent) = control_toggle_accent(palette) else {
        return;
    };

    let enabled = response.enabled();
    let active = response.is_pointer_button_down_on();
    let lit = response.hovered() || response.has_focus();
    let painter = ui.painter();
    let track_corner = egui::CornerRadius::same(9);
    let glow_alpha = if !enabled {
        if checked { 22 } else { 0 }
    } else if lit {
        if checked { 104 } else { 76 }
    } else if checked {
        62
    } else {
        0
    };
    let edge_alpha = if !enabled {
        54
    } else if active {
        230
    } else if lit {
        206
    } else if checked {
        164
    } else {
        112
    };
    let bevel_alpha = if !enabled {
        32
    } else if checked {
        126
    } else {
        66
    };
    let track_fill = if checked {
        egui::Color32::from_rgb(2, 31, 38)
    } else {
        egui::Color32::from_rgb(3, 6, 9)
    };
    let edge = if checked {
        accent.edge
    } else {
        accent.off_edge
    };

    if glow_alpha > 0 {
        painter.rect_stroke(
            switch_rect.expand(3.5),
            track_corner,
            egui::Stroke::new(
                1.15,
                control_button_color_with_alpha(accent.glow, glow_alpha),
            ),
            egui::StrokeKind::Outside,
        );
    }
    painter.rect_filled(switch_rect, track_corner, track_fill);
    if checked {
        painter.rect_filled(
            switch_rect.shrink(2.0),
            egui::CornerRadius::same(7),
            control_button_color_with_alpha(accent.glow, if enabled { 28 } else { 10 }),
        );
    }
    painter.rect_stroke(
        switch_rect,
        track_corner,
        egui::Stroke::new(1.1, edge.linear_multiply(edge_alpha as f32 / 255.0)),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            egui::pos2(switch_rect.left() + 8.0, switch_rect.top() + 2.0),
            egui::pos2(switch_rect.right() - 8.0, switch_rect.top() + 2.0),
        ],
        egui::Stroke::new(
            1.0,
            accent.highlight.linear_multiply(bevel_alpha as f32 / 255.0),
        ),
    );
    painter.line_segment(
        [
            egui::pos2(switch_rect.left() + 8.0, switch_rect.bottom() - 2.0),
            egui::pos2(switch_rect.right() - 8.0, switch_rect.bottom() - 2.0),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 130)),
    );

    let knob_radius = 7.0;
    let knob_center = egui::pos2(
        if checked {
            switch_rect.right() - 9.0
        } else {
            switch_rect.left() + 9.0
        },
        switch_rect.center().y,
    );
    if checked {
        painter.circle_stroke(
            knob_center,
            knob_radius + 3.0,
            egui::Stroke::new(
                1.0,
                control_button_color_with_alpha(accent.glow, if enabled { 70 } else { 22 }),
            ),
        );
    }
    let knob_fill = if checked {
        egui::Color32::from_rgb(20, 80, 96)
    } else {
        egui::Color32::from_rgb(21, 27, 31)
    };
    painter.circle_filled(knob_center, knob_radius, knob_fill);
    painter.circle_stroke(
        knob_center,
        knob_radius,
        egui::Stroke::new(
            1.0,
            if checked {
                accent
                    .edge
                    .linear_multiply(if enabled { 0.82 } else { 0.32 })
            } else {
                accent
                    .off_edge
                    .linear_multiply(if enabled { 0.68 } else { 0.28 })
            },
        ),
    );
    painter.circle_filled(
        knob_center + egui::vec2(-2.0, -2.0),
        2.0,
        if checked {
            accent
                .highlight
                .linear_multiply(if enabled { 0.64 } else { 0.24 })
        } else {
            egui::Color32::from_rgb(58, 70, 76).linear_multiply(if enabled { 0.62 } else { 0.26 })
        },
    );
}

impl YRangeMode {
    const ALL: [Self; 2] = [Self::Auto, Self::Manual];

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Manual => "Manual",
        }
    }
}

impl Default for RowYRange {
    fn default() -> Self {
        Self {
            mode: YRangeMode::Auto,
            min: None,
            max: None,
            last_auto: None,
        }
    }
}

impl RowYRange {
    fn set_last_auto(&mut self, range: (f64, f64)) {
        if normalized_y_range(range.0, range.1).is_some() {
            self.last_auto = Some(range);
        }
    }

    fn manual_seed_range(&self) -> (f64, f64) {
        self.last_auto.unwrap_or((-1.0, 1.0))
    }

    fn set_manual_from_last_auto(&mut self) {
        let (min, max) = self.manual_seed_range();
        self.mode = YRangeMode::Manual;
        self.min = Some(min);
        self.max = Some(max);
    }
}

impl PlotRow {
    fn new(id: u64) -> Self {
        Self {
            id,
            y_range: RowYRange::default(),
            channels: Vec::new(),
        }
    }
}

impl RowChannel {
    fn new(key: ChannelKey, color_index: usize) -> Self {
        Self {
            key,
            color_index,
            draw_mode: DrawMode::Line,
            visible: true,
            color_override: None,
            line_width: DEFAULT_TRACE_LINE_WIDTH,
        }
    }

    fn color(&self, dark_mode: bool) -> egui::Color32 {
        self.color_override
            .unwrap_or_else(|| channel_color(self.color_index, dark_mode))
    }
}

fn default_parquet_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/test_100k.parquet")
        .display()
        .to_string()
}

fn default_plot_config_path(parquet_path: &str) -> String {
    let mut path = PathBuf::from(parquet_path.trim());
    path.set_extension("pyc.json");
    path.display().to_string()
}

impl ParquetPathValidationError {
    fn status(&self) -> &'static str {
        match self {
            Self::Empty => "Parquet path is empty",
            Self::Missing { .. }
            | Self::NotAccessible { .. }
            | Self::NotFile { .. }
            | Self::WrongExtension { .. } => "Invalid Parquet path",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Empty => "Enter a .parquet file path or choose one with Browse.".to_owned(),
            Self::Missing { path } => format!("File does not exist:\n{path}"),
            Self::NotAccessible { path, error } => {
                format!("Cannot check this path:\n{path}\n\nDetails: {error}")
            }
            Self::NotFile { path } => format!("Path is not a file:\n{path}"),
            Self::WrongExtension { path } => {
                format!("Expected a .parquet file, but got:\n{path}")
            }
        }
    }
}

fn validate_parquet_path(path: &str) -> Result<PathBuf, ParquetPathValidationError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(ParquetPathValidationError::Empty);
    }

    let path = PathBuf::from(trimmed);
    let display_path = path.display().to_string();
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(ParquetPathValidationError::Missing { path: display_path });
        }
        Err(error) => {
            return Err(ParquetPathValidationError::NotAccessible {
                path: display_path,
                error: error.to_string(),
            });
        }
    }

    if !path.is_file() {
        return Err(ParquetPathValidationError::NotFile { path: display_path });
    }

    let has_parquet_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("parquet"));
    if !has_parquet_extension {
        return Err(ParquetPathValidationError::WrongExtension { path: display_path });
    }

    Ok(path)
}

fn format_schema_load_error(path: &Path, error: &str) -> String {
    format!(
        "Could not load the Parquet schema.\nPath:\n{}\n\nDetails: {error}\n\nThe file exists and has a .parquet extension, but the metadata could not be read.",
        path.display()
    )
}

fn remember_recent_parquet_path(recent_paths: &mut Vec<String>, path: impl Into<String>) {
    let path = path.into();
    let path = path.trim();
    if path.is_empty() {
        return;
    }

    let path = path.to_owned();
    recent_paths.retain(|current| current != &path);
    recent_paths.insert(0, path);
    recent_paths.truncate(MAX_RECENT_PARQUET_PATHS);
}

fn recent_path_label(path: &str) -> String {
    let path_ref = Path::new(path);
    let file_name = path_ref
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path);
    let parent_name = path_ref
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str());

    match parent_name {
        Some(parent_name) if !parent_name.is_empty() => format!("{file_name} ({parent_name})"),
        _ => file_name.to_owned(),
    }
}

fn parquet_dialog_directory(path: &str) -> Option<PathBuf> {
    let path = Path::new(path.trim());
    if path.is_dir() {
        return Some(path.to_path_buf());
    }

    path.parent()
        .filter(|parent| parent.exists())
        .map(Path::to_path_buf)
}

fn plot_config_dialog_directory(path: &str) -> Option<PathBuf> {
    let path = Path::new(path.trim());
    if path.is_dir() {
        return Some(path.to_path_buf());
    }

    path.parent()
        .filter(|parent| parent.exists())
        .map(Path::to_path_buf)
}

fn validate_plot_config_load_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Enter a .pyc.json or .json plot config path.".to_owned());
    }

    let path = PathBuf::from(trimmed);
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Err(format!("File does not exist:\n{}", path.display())),
        Err(error) => {
            return Err(format!(
                "Cannot check this path:\n{}\n\nDetails: {error}",
                path.display()
            ));
        }
    }
    if !path.is_file() {
        return Err(format!("Path is not a file:\n{}", path.display()));
    }

    let allowed_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if !allowed_extension {
        return Err(format!(
            "Expected a .pyc.json or .json file:\n{}",
            path.display()
        ));
    }

    Ok(path)
}

impl ChannelLoadState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Loaded => "loaded",
            Self::Loading => "loading",
            Self::Unloaded => "unloaded",
        }
    }

    fn action_label(self) -> &'static str {
        match self {
            Self::Loaded => "Add",
            Self::Loading => "Loading",
            Self::Unloaded => "Load",
        }
    }

    fn can_load_or_add(self) -> bool {
        !matches!(self, Self::Loading)
    }
}

impl ChannelStateFilter {
    const ALL: [Self; 4] = [Self::All, Self::Loaded, Self::Loading, Self::Unloaded];

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Loaded => "Loaded",
            Self::Loading => "Loading",
            Self::Unloaded => "Unloaded",
        }
    }

    fn matches(self, load_state: ChannelLoadState) -> bool {
        match self {
            Self::All => true,
            Self::Loaded => load_state == ChannelLoadState::Loaded,
            Self::Loading => load_state == ChannelLoadState::Loading,
            Self::Unloaded => load_state == ChannelLoadState::Unloaded,
        }
    }
}

impl ChannelSort {
    const ALL: [Self; 3] = [Self::SchemaOrder, Self::Name, Self::LoadedFirst];

    fn as_str(self) -> &'static str {
        match self {
            Self::SchemaOrder => "Schema order",
            Self::Name => "Name",
            Self::LoadedFirst => "Loaded first",
        }
    }
}

fn channel_load_state(
    source_id: SourceId,
    channel_path: &str,
    loaded_channels: &ChannelStore,
    loading_channel_keys: &BTreeSet<ChannelKey>,
) -> ChannelLoadState {
    if loaded_channels.has_channel(channel_path) {
        ChannelLoadState::Loaded
    } else if loading_channel_keys.contains(&ChannelKey::new(source_id, channel_path)) {
        ChannelLoadState::Loading
    } else {
        ChannelLoadState::Unloaded
    }
}

fn source_dangerous_actions_enabled(load: &LoadState, source_id: SourceId) -> bool {
    !load.is_source_loading(source_id)
}

fn channel_list_entries<'a>(
    workspace: &WorkspaceDataState,
    source_id: SourceId,
    channels: &'a [parquet_schema::ColumnInfo],
    text_filter: &str,
    state_filter: ChannelStateFilter,
    sort: ChannelSort,
    loaded_channels: &ChannelStore,
    loading_channel_keys: &BTreeSet<ChannelKey>,
) -> Vec<(&'a parquet_schema::ColumnInfo, ChannelLoadState)> {
    let mut entries = channels
        .iter()
        .filter_map(|channel| {
            let key = ChannelKey::new(source_id, channel.path.clone());
            let presentation = channel_presentation(workspace, &key)?;
            let load_state = channel_load_state(
                source_id,
                &channel.path,
                loaded_channels,
                loading_channel_keys,
            );
            (channel_matches_browser_filter(channel, &presentation, text_filter)
                && state_filter.matches(load_state))
            .then_some((channel, load_state))
        })
        .collect::<Vec<_>>();

    match sort {
        ChannelSort::SchemaOrder => {}
        ChannelSort::Name => {
            entries.sort_by(|(left, _), (right, _)| {
                left.display_name()
                    .to_ascii_lowercase()
                    .cmp(&right.display_name().to_ascii_lowercase())
                    .then(left.index.cmp(&right.index))
            });
        }
        ChannelSort::LoadedFirst => {
            entries.sort_by(|(left, left_state), (right, right_state)| {
                channel_loaded_first_rank(*left_state)
                    .cmp(&channel_loaded_first_rank(*right_state))
                    .then(left.index.cmp(&right.index))
            });
        }
    }

    entries
}

fn channel_loaded_first_rank(load_state: ChannelLoadState) -> usize {
    match load_state {
        ChannelLoadState::Loaded => 0,
        ChannelLoadState::Loading => 1,
        ChannelLoadState::Unloaded => 2,
    }
}

#[cfg(test)]
fn channel_matches_filter(channel: &parquet_schema::ColumnInfo, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }
    let filter = filter.to_ascii_lowercase();
    channel.name.to_ascii_lowercase().contains(&filter)
        || channel.path.to_ascii_lowercase().contains(&filter)
        || channel
            .display_name()
            .to_ascii_lowercase()
            .contains(&filter)
}

fn estimated_sample_rate_hz(sample_count: usize, time_range: Option<(f64, f64)>) -> Option<f64> {
    let (start, end) = time_range?;
    let span = (end - start).abs();
    if sample_count < 2 || !span.is_finite() || span <= 0.0 {
        return None;
    }

    Some((sample_count - 1) as f64 / span)
}

fn format_frequency_hz(frequency_hz: f64) -> String {
    let frequency_hz = frequency_hz.abs();
    if !frequency_hz.is_finite() {
        return "-".to_owned();
    }

    if frequency_hz >= 1_000_000.0 {
        format!("{:.3} MHz", frequency_hz / 1_000_000.0)
    } else if frequency_hz >= 1_000.0 {
        format!("{:.3} kHz", frequency_hz / 1_000.0)
    } else {
        format!("{frequency_hz:.3} Hz")
    }
}

impl ViewState {
    fn display_mode_choice(&self) -> DisplayModeChoice {
        DisplayModeChoice::from_view(self)
    }

    fn set_display_mode_choice(&mut self, choice: DisplayModeChoice) {
        choice.apply_to_view(self);
    }

    fn visual_palette(&self) -> VisualPalette {
        visual_palette(self.theme, self.visual_mode)
    }

    fn update_for_source_schema(
        &mut self,
        source_id: SourceId,
        schema: &parquet_schema::SchemaSummary,
    ) {
        if self.selected_channel.is_none() {
            self.selected_channel = schema
                .channels
                .first()
                .map(|channel| ChannelKey::new(source_id, channel.path.clone()));
        }
        self.hover_x = None;
        self.ensure_row_state();
    }

    fn ensure_row_state(&mut self) {
        if self.rows.is_empty() {
            self.rows.push(PlotRow::new(0));
        }

        let next_row_id = self.rows.iter().map(|row| row.id).max().unwrap_or(0) + 1;
        self.next_row_id = self.next_row_id.max(next_row_id);

        let selected_row_exists = self
            .selected_row_id
            .is_some_and(|selected_id| self.rows.iter().any(|row| row.id == selected_id));
        if !selected_row_exists {
            self.selected_row_id = self.rows.first().map(|row| row.id);
        }
    }

    fn cleanup_source(&mut self, source_id: SourceId) -> usize {
        let mut removed = 0usize;
        for row in &mut self.rows {
            let before = row.channels.len();
            row.channels
                .retain(|channel| channel.key.source_id != source_id);
            removed += before - row.channels.len();
        }
        if self
            .selected_channel
            .as_ref()
            .is_some_and(|key| key.source_id == source_id)
        {
            self.selected_channel = None;
        }
        self.hover_x = None;
        removed
    }

    fn add_row(&mut self) -> u64 {
        self.ensure_row_state();
        let id = self.next_row_id;
        self.next_row_id += 1;
        self.rows.push(PlotRow::new(id));
        self.selected_row_id = Some(id);
        id
    }

    fn remove_row(&mut self, row_id: u64) -> bool {
        self.ensure_row_state();
        if self.rows.len() <= 1 {
            return false;
        }

        let Some(remove_index) = self.rows.iter().position(|row| row.id == row_id) else {
            return false;
        };

        self.rows.remove(remove_index);
        if self.selected_row_id == Some(row_id) {
            let next_index = remove_index.min(self.rows.len().saturating_sub(1));
            self.selected_row_id = self.rows.get(next_index).map(|row| row.id);
        }
        self.ensure_row_state();
        true
    }

    fn selected_row_index(&self) -> Option<usize> {
        let selected_row_id = self.selected_row_id?;
        self.rows.iter().position(|row| row.id == selected_row_id)
    }

    fn selected_row_display_name(&self) -> String {
        self.selected_row_index()
            .map(|index| format!("Row {}", index + 1))
            .unwrap_or_else(|| "Row -".to_owned())
    }

    fn row_display_name(&self, row_id: u64) -> String {
        self.rows
            .iter()
            .position(|row| row.id == row_id)
            .map(|index| format!("Row {}", index + 1))
            .unwrap_or_else(|| "Row -".to_owned())
    }

    fn selected_row_id_or_first(&mut self) -> Option<u64> {
        self.ensure_row_state();
        self.selected_row_id
            .or_else(|| self.rows.first().map(|row| row.id))
    }

    fn add_channel_to_row(&mut self, row_id: u64, key: &ChannelKey) -> Option<(bool, u64)> {
        self.ensure_row_state();
        let row = self.rows.iter_mut().find(|row| row.id == row_id)?;
        if row.channels.iter().any(|channel| channel.key == *key) {
            return Some((false, row.id));
        }

        row.channels
            .push(RowChannel::new(key.clone(), row.channels.len()));
        Some((true, row.id))
    }

    #[cfg(test)]
    fn add_channel_to_selected_row(&mut self, key: &ChannelKey) -> (bool, u64) {
        let row_id = self.selected_row_id_or_first().unwrap_or(0);
        self.add_channel_to_row(row_id, key)
            .unwrap_or((false, row_id))
    }

    fn has_visible_channels(&self) -> bool {
        self.rows
            .iter()
            .any(|row| row.channels.iter().any(|channel| channel.visible))
    }
}

impl SessionPreset {
    fn capture(workspace: &WorkspaceDataState, view: &ViewState) -> Self {
        Self {
            sources: workspace
                .source_order
                .iter()
                .filter_map(|id| workspace.source(*id))
                .map(|source| SessionPresetSource {
                    id: source.id,
                    parquet_path: source.parquet_path.clone(),
                    label: source.label.clone(),
                    offset_seconds: source.time_transform.offset_seconds,
                })
                .collect(),
            runtime_source_ids: workspace.source_order.iter().map(|id| (*id, *id)).collect(),
            selected_source_id: workspace.selected_source_id(),
            selected_channel: view.selected_channel.clone(),
            selected_row_index: view
                .selected_row_id
                .and_then(|row_id| view.rows.iter().position(|row| row.id == row_id)),
            x_range: view.x_range,
            large_preview_enabled: view.large_preview_enabled,
            rows: view
                .rows
                .iter()
                .map(|row| SessionPresetRow {
                    y_range: row.y_range,
                    channels: row
                        .channels
                        .iter()
                        .map(SessionPresetChannel::from)
                        .collect(),
                })
                .collect(),
        }
    }

    fn channel_count(&self) -> usize {
        self.rows.iter().map(|row| row.channels.len()).sum()
    }

    fn summary(&self) -> String {
        format!(
            "{} source(s), {} row(s), {} channel(s)",
            self.sources.len(),
            self.rows.len().max(1),
            self.channel_count()
        )
    }

    fn apply_to_view(
        &self,
        view: &mut ViewState,
        workspace: &WorkspaceDataState,
        source_ids: &BTreeMap<SourceId, SourceId>,
    ) -> SessionPresetApplyReport {
        let resolve = |key: &ChannelKey| {
            let id = *source_ids.get(&key.source_id)?;
            let schema = workspace.source(id)?.schema.as_ref()?;
            schema
                .channels
                .iter()
                .any(|channel| channel.path == key.channel_path)
                .then(|| ChannelKey::new(id, key.channel_path.clone()))
        };
        let mut missing_channels = BTreeSet::new();
        let mut restored_channels = 0usize;

        let mut rows = self
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, preset_row)| {
                let channels = preset_row
                    .channels
                    .iter()
                    .filter_map(|channel| {
                        if let Some(key) = resolve(&channel.key) {
                            restored_channels += 1;
                            Some(RowChannel::from_preset(key, channel))
                        } else {
                            missing_channels.insert(format!(
                                "Source {}:{}",
                                channel.key.source_id.value(),
                                channel.key.channel_path
                            ));
                            None
                        }
                    })
                    .collect();

                PlotRow {
                    id: row_index as u64,
                    y_range: preset_row.y_range,
                    channels,
                }
            })
            .collect::<Vec<_>>();

        if rows.is_empty() {
            rows.push(PlotRow::new(0));
        }

        view.rows = rows;
        view.next_row_id = view.rows.len() as u64;
        view.selected_row_id = self
            .selected_row_index
            .and_then(|row_index| view.rows.get(row_index).map(|row| row.id))
            .or_else(|| view.rows.first().map(|row| row.id));
        view.x_range = self.x_range;
        view.hover_x = None;
        view.large_preview_enabled = self.large_preview_enabled;
        view.selected_channel = self
            .selected_channel
            .as_ref()
            .and_then(resolve)
            .or_else(|| {
                view.rows
                    .iter()
                    .flat_map(|row| &row.channels)
                    .map(|channel| channel.key.clone())
                    .next()
            });
        view.ensure_row_state();

        SessionPresetApplyReport {
            restored_rows: view.rows.len(),
            restored_channels,
            missing_channels: missing_channels.into_iter().collect(),
        }
    }
}

impl From<&RowChannel> for SessionPresetChannel {
    fn from(channel: &RowChannel) -> Self {
        Self {
            key: channel.key.clone(),
            color_index: channel.color_index,
            draw_mode: channel.draw_mode,
            visible: channel.visible,
            color_override: channel.color_override,
            line_width: channel.line_width,
        }
    }
}

impl RowChannel {
    fn from_preset(key: ChannelKey, channel: &SessionPresetChannel) -> Self {
        Self {
            key,
            color_index: channel.color_index,
            draw_mode: channel.draw_mode,
            visible: channel.visible,
            color_override: channel.color_override,
            line_width: channel
                .line_width
                .clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH),
        }
    }
}

fn row_channel_load_counts(
    row: &PlotRow,
    workspace: &WorkspaceDataState,
    loading_channel_keys: &BTreeSet<ChannelKey>,
) -> (usize, usize) {
    row.channels.iter().filter(|channel| channel.visible).fold(
        (0usize, 0usize),
        |(loading, unloaded), channel| {
            if workspace
                .source(channel.key.source_id)
                .is_some_and(|source| {
                    source.shared_time.is_some()
                        && source
                            .loaded_channels
                            .has_channel(&channel.key.channel_path)
                })
            {
                (loading, unloaded)
            } else if loading_channel_keys.contains(&channel.key) {
                (loading + 1, unloaded)
            } else {
                (loading, unloaded + 1)
            }
        },
    )
}

fn cursor_channel_readouts(
    analysis: &AnalysisState,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Vec<CursorChannelReadout> {
    let mut readouts = Vec::new();
    for (key, channel) in analysis_visible_loaded_channels(workspace, view) {
        let Some(source) = workspace.source(key.source_id) else {
            continue;
        };
        let Some(shared_time) = source.shared_time.as_ref() else {
            continue;
        };
        let display_label = channel_label_or_fallback(workspace, &key);
        let tooltip = channel_tooltip_or_fallback(workspace, &key);
        let value_a = analysis
            .cursor_a
            .map(|time| source.time_transform.to_raw(time))
            .and_then(|raw_time| {
                nearest_sample_value_in_range(&shared_time.time, &channel.values, raw_time)
            });
        let value_b = analysis
            .cursor_b
            .map(|time| source.time_transform.to_raw(time))
            .and_then(|raw_time| {
                nearest_sample_value_in_range(&shared_time.time, &channel.values, raw_time)
            });
        let delta = value_a.zip(value_b).map(|(a, b)| b - a);
        if value_a.is_none() && value_b.is_none() {
            continue;
        }

        readouts.push(CursorChannelReadout {
            key,
            channel_name: channel.channel_name.clone(),
            channel_path: channel.channel_path.clone(),
            display_label,
            tooltip,
            value_a,
            value_b,
            delta,
        });
    }

    readouts
}

fn unique_visible_loaded_channels<'a>(
    workspace: &'a WorkspaceDataState,
    view: &ViewState,
) -> Vec<(ChannelKey, &'a parquet_waveform::ChannelData)> {
    let mut seen = BTreeSet::new();
    let mut channels = Vec::new();
    for row_channel in view.rows.iter().flat_map(|row| row.channels.iter()) {
        if !row_channel.visible || !seen.insert(row_channel.key.clone()) {
            continue;
        }

        if let Some(channel) = workspace.channel(&row_channel.key) {
            channels.push((row_channel.key.clone(), channel));
        }
    }

    channels
}

fn analysis_visible_loaded_channels<'a>(
    workspace: &'a WorkspaceDataState,
    view: &ViewState,
) -> Vec<(ChannelKey, &'a parquet_waveform::ChannelData)> {
    unique_visible_loaded_channels(workspace, view)
}

fn interval_stats_cache_key(
    analysis: &AnalysisState,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Option<IntervalStatsCacheKey> {
    let cursor_a = analysis.cursor_a?;
    let cursor_b = analysis.cursor_b?;
    if !cursor_a.is_finite() || !cursor_b.is_finite() {
        return None;
    }

    let channels = analysis_visible_loaded_channels(workspace, view)
        .into_iter()
        .filter_map(|(key, channel)| {
            let source = workspace.source(key.source_id)?;
            let shared_time = source.shared_time.as_ref()?;
            Some(IntervalStatsCacheChannel {
                key,
                source_generation: source.generation,
                time_offset_bits: source.time_transform.offset_seconds.to_bits(),
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
                sample_count: channel.sample_count(),
                time: Arc::clone(&shared_time.time),
                values: Arc::clone(&channel.values),
            })
        })
        .collect::<Vec<_>>();
    if channels.is_empty() {
        return None;
    }

    Some(IntervalStatsCacheKey {
        cursor_a_bits: cursor_a.to_bits(),
        cursor_b_bits: cursor_b.to_bits(),
        channels,
    })
}

fn compute_interval_channel_statistics(
    analysis: &AnalysisState,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Vec<IntervalChannelStats> {
    let Some(cursor_a) = analysis.cursor_a else {
        return Vec::new();
    };
    let Some(cursor_b) = analysis.cursor_b else {
        return Vec::new();
    };

    analysis_visible_loaded_channels(workspace, view)
        .into_iter()
        .filter_map(|(key, channel)| {
            let source = workspace.source(key.source_id)?;
            let shared_time = source.shared_time.as_ref()?;
            let display_label = channel_label_or_fallback(workspace, &key);
            let tooltip = channel_tooltip_or_fallback(workspace, &key);
            interval_channel_statistics_for_channel(
                key,
                display_label,
                tooltip,
                &shared_time.time,
                channel,
                source.time_transform.to_raw(cursor_a),
                source.time_transform.to_raw(cursor_b),
                &std::sync::atomic::AtomicBool::new(false),
            )
        })
        .collect()
}

fn interval_channel_statistics_for_channel(
    key: ChannelKey,
    display_label: String,
    tooltip: String,
    time: &[f64],
    channel: &parquet_waveform::ChannelData,
    cursor_a: f64,
    cursor_b: f64,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Option<IntervalChannelStats> {
    let sample_count = time.len().min(channel.values.len());
    let index_a = nearest_sample_index_in_range(time, cursor_a, sample_count)?;
    let index_b = nearest_sample_index_in_range(time, cursor_b, sample_count)?;
    let value_a = sample_value_at_index(&channel.values, index_a);
    let value_b = sample_value_at_index(&channel.values, index_b);
    let delta = value_a.zip(value_b).map(|(a, b)| b - a);
    let slope_per_second = delta.and_then(|delta| {
        let dt = cursor_b - cursor_a;
        (dt.is_finite() && dt.abs() > f64::EPSILON).then_some(delta / dt)
    });

    let start = index_a.min(index_b);
    let end = index_a.max(index_b).saturating_add(1).min(sample_count);
    let (mean, max, min, peak_to_peak, rms) =
        interval_value_statistics(&channel.values[start..end], cancelled)?;

    Some(IntervalChannelStats {
        key,
        channel_name: channel.channel_name.clone(),
        channel_path: channel.channel_path.clone(),
        display_label,
        tooltip,
        value_a,
        value_b,
        delta,
        mean,
        max,
        min,
        peak_to_peak,
        rms,
        slope_per_second,
    })
}

fn interval_value_statistics(
    values: &[f32],
    cancelled: &std::sync::atomic::AtomicBool,
) -> Option<(
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
)> {
    let mut count = 0usize;
    let mut sum = 0.0f64;
    let mut sum_squares = 0.0f64;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    for chunk in values.chunks(interval_stats::CANCELLATION_CHUNK_SAMPLES) {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        for value in chunk.iter().copied().filter(|value| value.is_finite()) {
            let value = f64::from(value);
            count += 1;
            sum += value;
            sum_squares += value * value;
            min = min.min(value);
            max = max.max(value);
        }
    }
    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }

    if count == 0 {
        return Some((None, None, None, None, None));
    }

    let mean = sum / count as f64;
    let rms = (sum_squares / count as f64).sqrt();
    Some((Some(mean), Some(max), Some(min), Some(max - min), Some(rms)))
}

impl ChannelStore {
    fn clear_all(&mut self) {
        self.raw_by_channel.clear();
        self.line_tile_cache.clear();
        self.step_edge_cache.clear();
        self.clear_envelope_cache();
        self.step_fallback_hints.clear();
        self.last_envelope_stats = EnvelopeCacheStats::default();
    }

    fn unload_channel(&mut self, channel_path: &str) -> bool {
        let removed_raw = self.raw_by_channel.remove(channel_path).is_some();
        let removed_line_tiles = self.line_tile_cache.remove(channel_path).is_some();
        let removed_step_edges = self.step_edge_cache.remove(channel_path).is_some();
        let removed_step_hint = self.step_fallback_hints.remove(channel_path).is_some();
        if removed_raw || removed_line_tiles || removed_step_edges || removed_step_hint {
            self.clear_envelope_cache();
        }

        removed_raw
    }

    fn clear_draw_caches(&mut self) -> bool {
        let had_draw_cache = self.has_draw_caches();
        self.line_tile_cache.clear();
        self.step_edge_cache.clear();
        self.step_fallback_hints.clear();
        self.clear_envelope_cache();
        self.last_envelope_stats = EnvelopeCacheStats::default();
        had_draw_cache
    }

    fn clear_envelope_cache(&mut self) {
        self.envelope_cache.clear();
        self.envelope_context = None;
        self.envelope_plan = None;
    }

    fn has_draw_caches(&self) -> bool {
        !self.envelope_cache.is_empty()
            || self.envelope_context.is_some()
            || self.envelope_plan.is_some()
            || !self.line_tile_cache.is_empty()
            || !self.step_edge_cache.is_empty()
            || !self.step_fallback_hints.is_empty()
    }

    fn has_channel(&self, channel_path: &str) -> bool {
        self.raw_by_channel.contains_key(channel_path)
    }

    fn channel(&self, channel_path: &str) -> Option<&parquet_waveform::ChannelData> {
        self.raw_by_channel.get(channel_path)
    }

    fn insert_channel(&mut self, channel: parquet_waveform::ChannelData) {
        self.step_fallback_hints.remove(&channel.channel_path);
        self.line_tile_cache.remove(&channel.channel_path);
        self.step_edge_cache.remove(&channel.channel_path);
        self.raw_by_channel
            .insert(channel.channel_path.clone(), channel);
        self.clear_envelope_cache();
    }

    fn raw_memory_bytes(&self) -> usize {
        self.raw_by_channel
            .values()
            .map(parquet_waveform::ChannelData::memory_bytes)
            .sum()
    }

    fn line_tile_memory_bytes(&self) -> usize {
        self.line_tile_cache
            .values()
            .map(parquet_waveform::LineTileCache::memory_bytes)
            .sum()
    }

    fn line_tile_build_seconds(&self) -> f64 {
        self.line_tile_cache
            .values()
            .map(|cache| cache.elapsed.as_secs_f64())
            .sum()
    }

    fn step_edge_memory_bytes(&self) -> usize {
        self.step_edge_cache
            .values()
            .map(StepEdgeCache::memory_bytes)
            .sum()
    }

    fn step_edge_build_seconds(&self) -> f64 {
        self.step_edge_cache
            .values()
            .map(|cache| cache.elapsed.as_secs_f64())
            .sum()
    }

    fn begin_envelope_frame(&mut self) {
        self.last_envelope_stats = EnvelopeCacheStats::default();
    }

    fn ensure_line_tile_cache(&mut self, channel_path: &str) -> Option<bool> {
        let was_cached = self.line_tile_cache.contains_key(channel_path);
        if !was_cached {
            let channel = self.raw_by_channel.get(channel_path)?;
            let tile_cache =
                parquet_waveform::build_line_tile_cache(&channel.values, LINE_TILE_SAMPLE_WIDTH);
            self.last_envelope_stats.tile_build_wall += tile_cache.elapsed;
            self.last_envelope_stats.tile_build_workers =
                self.last_envelope_stats.tile_build_workers.max(1);
            self.line_tile_cache
                .insert(channel_path.to_owned(), tile_cache);
            self.last_envelope_stats.tile_builds += 1;
        } else {
            self.last_envelope_stats.tile_hits += 1;
        }

        Some(!was_cached)
    }

    fn cached_change_point_step_trace_result(
        &mut self,
        channel_path: &str,
        time: &[f64],
        time_range: (f64, f64),
        max_change_points: usize,
    ) -> Option<StepTraceBuildResult> {
        let Some(range) = normalized_range(time_range) else {
            return Some(StepTraceBuildResult::Empty);
        };
        if self.step_edge_cache.get(channel_path).is_some_and(|cache| {
            cache
                .cached_time_range
                .is_none_or(|cached| range.0 >= cached.0 && range.1 <= cached.1)
        }) {
            self.last_envelope_stats.step_edge_hits += 1;
        } else {
            self.step_edge_cache.remove(channel_path);
            let channel = self.raw_by_channel.get(channel_path)?;
            let Some(range) = visible_sample_range(time, channel.values.len(), time_range) else {
                return Some(StepTraceBuildResult::Empty);
            };
            let started = Instant::now();
            let cache = build_step_edge_cache_with_limit(
                time,
                &channel.values,
                Some((range.range_start, range.range_end, max_change_points)),
            );
            self.last_envelope_stats.step_edge_scan_elapsed += started.elapsed();
            match cache {
                Ok(cache) => {
                    self.step_edge_cache.insert(channel_path.to_owned(), cache);
                    self.last_envelope_stats.step_edge_builds += 1;
                }
                Err(()) => {
                    self.last_envelope_stats.step_edge_aborts += 1;
                    return Some(StepTraceBuildResult::TooManyChangePoints {
                        source_sample_count: range.source_sample_count,
                    });
                }
            }
        }
        self.step_edge_cache.get(channel_path).map(|cache| {
            build_change_point_step_trace_from_cache(time, cache, time_range, max_change_points)
        })
    }

    fn prepare_envelope_context(
        &mut self,
        time: &[f64],
        value_count: usize,
        view_range: (f64, f64),
        full_range: (f64, f64),
        requested_bucket_count: usize,
    ) {
        let Some(context) =
            EnvelopeContext::for_view(view_range, full_range, requested_bucket_count)
        else {
            self.clear_envelope_cache();
            self.last_envelope_stats.context_misses += 1;
            return;
        };

        if self
            .envelope_context
            .is_some_and(|current| current.reuses_for_view(view_range, requested_bucket_count))
        {
            self.last_envelope_stats.context_hits += 1;
            return;
        }

        self.envelope_cache.clear();
        self.envelope_plan = Some(parquet_waveform::min_max_envelope_plan_for_range(
            time,
            value_count,
            context.cache_range(),
            context.cache_bucket_count,
        ));
        self.envelope_context = Some(context);
        self.last_envelope_stats.context_misses += 1;
    }

    fn ensure_envelope(
        &mut self,
        channel_path: &str,
        time: &[f64],
        view_range: (f64, f64),
        requested_bucket_count: usize,
        allow_line_tile_lod: bool,
    ) -> Option<(parquet_waveform::MinMaxEnvelope, bool)> {
        let context = self.envelope_context?;
        let cache_range = context.cache_range();
        let key = EnvelopeKey::new(channel_path, cache_range, context.cache_bucket_count);
        let was_cached = self.envelope_cache.contains_key(&key);
        if !was_cached {
            let envelope = {
                if allow_line_tile_lod
                    && self
                        .envelope_plan
                        .as_ref()
                        .is_some_and(should_use_line_tile_lod)
                {
                    let plan = self.envelope_plan.clone()?;
                    let _tile_was_built = self.ensure_line_tile_cache(channel_path)?;
                    let channel = self.raw_by_channel.get(channel_path)?;
                    let tile_cache = self.line_tile_cache.get(channel_path)?;
                    let envelope = parquet_waveform::min_max_envelope_for_plan_with_tiles(
                        time,
                        &channel.values,
                        &plan,
                        tile_cache,
                    );
                    self.last_envelope_stats.tile_buckets += envelope.bucket_count();
                    envelope
                } else if let Some(plan) = self.envelope_plan.as_ref() {
                    let channel = self.raw_by_channel.get(channel_path)?;
                    parquet_waveform::min_max_envelope_for_plan(&channel.values, plan)
                } else {
                    let channel = self.raw_by_channel.get(channel_path)?;
                    channel.min_max_envelope_for_range(
                        time,
                        cache_range,
                        context.cache_bucket_count,
                    )
                }
            };
            self.envelope_cache.insert(key.clone(), envelope);
            self.last_envelope_stats.misses += 1;
        } else {
            self.last_envelope_stats.hits += 1;
        }

        let cached = self.envelope_cache.get(&key)?;
        if context.cache_range_equals(view_range)
            && context.cache_bucket_count == requested_bucket_count
        {
            return Some((cached.clone(), !was_cached));
        }

        let channel = self.raw_by_channel.get(channel_path)?;
        self.last_envelope_stats.clipped += 1;
        Some((
            parquet_waveform::clip_min_max_envelope_to_range(
                cached,
                time,
                &channel.values,
                view_range,
                requested_bucket_count,
            ),
            !was_cached,
        ))
    }

    fn should_skip_step_change_points(
        &self,
        channel_path: &str,
        time_range: (f64, f64),
        source_sample_count: usize,
    ) -> bool {
        let Some(hint) = self.step_fallback_hints.get(channel_path) else {
            return false;
        };
        let span = (time_range.1 - time_range.0).abs();
        span.is_finite()
            && span >= hint.min_span * 0.95
            && source_sample_count >= hint.min_source_sample_count.saturating_mul(9) / 10
    }

    fn record_step_change_point_fallback(
        &mut self,
        channel_path: &str,
        time_range: (f64, f64),
        source_sample_count: usize,
    ) {
        let span = (time_range.1 - time_range.0).abs();
        if !span.is_finite() || span <= 0.0 || source_sample_count == 0 {
            return;
        }

        self.step_fallback_hints
            .entry(channel_path.to_owned())
            .and_modify(|hint| {
                hint.min_span = hint.min_span.min(span);
                hint.min_source_sample_count =
                    hint.min_source_sample_count.min(source_sample_count);
            })
            .or_insert(StepFallbackHint {
                min_span: span,
                min_source_sample_count: source_sample_count,
            });
    }
}

impl EnvelopeContext {
    fn for_view(
        view_range: (f64, f64),
        full_range: (f64, f64),
        requested_bucket_count: usize,
    ) -> Option<Self> {
        let (view_start, view_end) = normalized_range(view_range)?;
        let (cache_start, cache_end) = overscan_cache_range((view_start, view_end), full_range)?;
        let view_span = view_end - view_start;
        if !view_span.is_finite() || view_span <= 0.0 {
            return None;
        }

        Some(Self {
            cache_range_start_bits: cache_start.to_bits(),
            cache_range_end_bits: cache_end.to_bits(),
            view_span_bits: view_span.to_bits(),
            requested_view_bucket_count: requested_bucket_count,
            cache_bucket_count: cache_bucket_count_for_view(
                (view_start, view_end),
                (cache_start, cache_end),
                requested_bucket_count,
            ),
        })
    }

    fn cache_range(self) -> (f64, f64) {
        (
            f64::from_bits(self.cache_range_start_bits),
            f64::from_bits(self.cache_range_end_bits),
        )
    }

    fn view_span(self) -> f64 {
        f64::from_bits(self.view_span_bits)
    }

    fn reuses_for_view(self, view_range: (f64, f64), requested_bucket_count: usize) -> bool {
        if self.requested_view_bucket_count != requested_bucket_count {
            return false;
        }

        let Some((view_start, view_end)) = normalized_range(view_range) else {
            return false;
        };
        let view_span = view_end - view_start;
        if !similar_span(self.view_span(), view_span, ENVELOPE_CONTEXT_SPAN_TOLERANCE) {
            return false;
        }

        let (cache_start, cache_end) = self.cache_range();
        view_start >= cache_start && view_end <= cache_end
    }

    fn cache_range_equals(self, view_range: (f64, f64)) -> bool {
        let Some((view_start, view_end)) = normalized_range(view_range) else {
            return false;
        };
        self.cache_range_start_bits == view_start.to_bits()
            && self.cache_range_end_bits == view_end.to_bits()
    }
}

impl EnvelopeKey {
    fn new(channel_path: &str, time_range: (f64, f64), requested_bucket_count: usize) -> Self {
        Self {
            channel_path: channel_path.to_owned(),
            range_start_bits: time_range.0.to_bits(),
            range_end_bits: time_range.1.to_bits(),
            requested_bucket_count,
        }
    }
}

impl StepEdgeCache {
    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn memory_bytes(&self) -> usize {
        self.edges.len() * std::mem::size_of::<StepEdge>()
    }
}

fn benchmark_channel(path: String, channel: &str) -> Result<String, String> {
    let data = parquet_waveform::read_selected_channel(&path, channel)?;
    let full_range = data
        .time_range()
        .ok_or_else(|| "loaded waveform has no time range".to_owned())?;
    let full_envelope = data.min_max_envelope(FULL_RANGE_ENVELOPE_BUCKETS);
    let ranges = benchmark_ranges(full_range, BENCH_RANGE_RUNS);
    let mut range_results = Vec::with_capacity(ranges.len());
    let started = std::time::Instant::now();

    for (index, range) in ranges.into_iter().enumerate() {
        let envelope = data.min_max_envelope_for_range(range, BENCH_VISIBLE_ENVELOPE_BUCKETS);
        range_results.push(BenchmarkRangeResult {
            index: index + 1,
            range,
            source_sample_count: envelope.source_sample_count,
            bucket_count: envelope.bucket_count(),
            bucket_size: envelope.bucket_size,
            draw_point_count: envelope.draw_point_count(),
            elapsed_sec: envelope.elapsed.as_secs_f64(),
        });
    }

    let range_total_sec = started.elapsed().as_secs_f64();
    let range_max_sec = range_results
        .iter()
        .map(|result| result.elapsed_sec)
        .fold(0.0, f64::max);
    let range_avg_sec = if range_results.is_empty() {
        0.0
    } else {
        range_results
            .iter()
            .map(|result| result.elapsed_sec)
            .sum::<f64>()
            / range_results.len() as f64
    };

    let mut report = String::new();
    report.push_str(&format!("file: {}\n", data.path.display()));
    report.push_str(&format!("channel: {}\n", data.channel_name));
    report.push_str(&format!("channel path: {}\n", data.channel_path));
    report.push_str(&format!("samples: {}\n", data.sample_count()));
    report.push_str(&format!(
        "time range: {}\n",
        format_range(data.time_range())
    ));
    report.push_str(&format!(
        "projected columns: {:?}\n",
        data.projected_column_indices
    ));
    report.push_str(&format!("read time: {:.3}s\n", data.elapsed.as_secs_f64()));
    report.push_str(&format!(
        "array memory: {:.1} MiB\n",
        bytes_to_mib(data.memory_bytes())
    ));
    report.push_str(&format!(
        "full envelope: {} buckets requested, {} buckets built, bucket size {}, draw points {}, {:.3}s\n",
        full_envelope.requested_bucket_count,
        full_envelope.bucket_count(),
        full_envelope.bucket_size,
        full_envelope.draw_point_count(),
        full_envelope.elapsed.as_secs_f64()
    ));
    report.push_str(&format!(
        "visible envelope benchmark: {} runs, {} buckets requested, total {:.3}s, avg {:.4}s, max {:.4}s\n",
        range_results.len(),
        BENCH_VISIBLE_ENVELOPE_BUCKETS,
        range_total_sec,
        range_avg_sec,
        range_max_sec
    ));
    report.push_str("runs:\n");
    for result in &range_results {
        report.push_str(&format!(
            "  #{:02} {:.6}..{:.6}s samples={} buckets={} bucket={} draw_points={} {:.4}s\n",
            result.index,
            result.range.0,
            result.range.1,
            result.source_sample_count,
            result.bucket_count,
            result.bucket_size,
            result.draw_point_count,
            result.elapsed_sec
        ));
    }

    Ok(report)
}

fn stress_channel(path: String, channel: &str, runs: usize) -> Result<String, String> {
    let runs = runs.max(1);
    let rss_before_load = process_rss_mib();
    let data = parquet_waveform::read_selected_channel(&path, channel)?;
    let rss_after_load = process_rss_mib();
    let full_range = data
        .time_range()
        .ok_or_else(|| "loaded waveform has no time range".to_owned())?;
    let full_envelope = data.min_max_envelope(FULL_RANGE_ENVELOPE_BUCKETS);
    let rss_after_full_envelope = process_rss_mib();
    let ranges = benchmark_ranges(full_range, BENCH_RANGE_RUNS);
    if ranges.is_empty() {
        return Err("no benchmark ranges could be built".to_owned());
    }

    let mut block_results = Vec::with_capacity(STRESS_REPORT_BLOCKS);
    let block_size = runs.div_ceil(STRESS_REPORT_BLOCKS);
    let mut total_elapsed_sec = 0.0;
    let mut max_elapsed_sec: f64 = 0.0;
    let mut draw_points_total = 0usize;
    let mut samples_total = 0usize;

    let started = std::time::Instant::now();
    for index in 0..runs {
        let range = ranges[index % ranges.len()];
        let envelope = data.min_max_envelope_for_range(range, BENCH_VISIBLE_ENVELOPE_BUCKETS);
        let elapsed_sec = envelope.elapsed.as_secs_f64();
        total_elapsed_sec += elapsed_sec;
        max_elapsed_sec = max_elapsed_sec.max(elapsed_sec);
        draw_points_total += envelope.draw_point_count();
        samples_total += envelope.source_sample_count;

        let completed = index + 1;
        if completed % block_size == 0 || completed == runs {
            block_results.push(StressBlockResult {
                completed_runs: completed,
                elapsed_wall_sec: started.elapsed().as_secs_f64(),
                rss_mib: process_rss_mib(),
            });
        }
    }

    let wall_sec = started.elapsed().as_secs_f64();
    let avg_elapsed_sec = total_elapsed_sec / runs as f64;
    let rss_after_stress = process_rss_mib();

    let mut report = String::new();
    report.push_str(&format!("file: {}\n", data.path.display()));
    report.push_str(&format!("channel: {}\n", data.channel_name));
    report.push_str(&format!("samples: {}\n", data.sample_count()));
    report.push_str(&format!(
        "time range: {}\n",
        format_range(data.time_range())
    ));
    report.push_str(&format!("read time: {:.3}s\n", data.elapsed.as_secs_f64()));
    report.push_str(&format!(
        "array memory: {:.1} MiB\n",
        bytes_to_mib(data.memory_bytes())
    ));
    report.push_str(&format!(
        "full envelope: {} buckets requested, {} buckets built, bucket size {}, draw points {}, {:.3}s\n",
        full_envelope.requested_bucket_count,
        full_envelope.bucket_count(),
        full_envelope.bucket_size,
        full_envelope.draw_point_count(),
        full_envelope.elapsed.as_secs_f64()
    ));
    report.push_str(&format!(
        "stress visible envelope: {} runs, {} buckets requested, wall {:.3}s, measured total {:.3}s, avg {:.4}s, max {:.4}s\n",
        runs,
        BENCH_VISIBLE_ENVELOPE_BUCKETS,
        wall_sec,
        total_elapsed_sec,
        avg_elapsed_sec,
        max_elapsed_sec
    ));
    report.push_str(&format!(
        "stress totals: samples={}, draw_points={}\n",
        samples_total, draw_points_total
    ));
    report.push_str(&format!(
        "rss: before_load={}, after_load={}, after_full_envelope={}, after_stress={}\n",
        format_optional_mib(rss_before_load),
        format_optional_mib(rss_after_load),
        format_optional_mib(rss_after_full_envelope),
        format_optional_mib(rss_after_stress)
    ));
    report.push_str("rss blocks:\n");
    for block in &block_results {
        report.push_str(&format!(
            "  runs={} wall={:.3}s rss={}\n",
            block.completed_runs,
            block.elapsed_wall_sec,
            format_optional_mib(block.rss_mib)
        ));
    }

    Ok(report)
}

fn benchmark_multi_channel(path: String, channels: Vec<String>) -> Result<String, String> {
    let rss_before_schema = process_rss_mib();
    let schema = parquet_schema::read_schema_summary(&path)?;
    let rss_after_schema = process_rss_mib();
    if schema.time_column.is_none() {
        return Err("time column not found".to_owned());
    }

    let selected_channels = if channels.is_empty() {
        schema
            .channels
            .iter()
            .map(|channel| channel.path.clone())
            .collect::<Vec<_>>()
    } else {
        channels
    };
    if selected_channels.is_empty() {
        return Err("no channels selected".to_owned());
    }

    let time = parquet_waveform::read_time_column(&path, &schema)?;
    let rss_after_time = process_rss_mib();
    let full_range = time
        .time_range()
        .ok_or_else(|| "loaded time column has no time range".to_owned())?;
    let mut store = ChannelStore::default();
    let mut channel_results = Vec::with_capacity(selected_channels.len());

    for channel_name in &selected_channels {
        let channel = parquet_waveform::read_channel_values(&path, &schema, channel_name)?;
        if channel.sample_count() != time.sample_count() {
            return Err(format!(
                "time/value length mismatch for {}: {} vs {}",
                channel.channel_path,
                time.sample_count(),
                channel.sample_count()
            ));
        }

        let result = MultiChannelLoadResult {
            channel_name: channel.channel_name.clone(),
            channel_path: channel.channel_path.clone(),
            sample_count: channel.sample_count(),
            read_sec: channel.elapsed.as_secs_f64(),
            memory_bytes: channel.memory_bytes(),
            rss_after_load: process_rss_mib(),
        };
        store.insert_channel(channel);
        channel_results.push(result);
    }

    store.begin_envelope_frame();
    store.prepare_envelope_context(
        &time.time,
        time.sample_count(),
        full_range,
        full_range,
        BENCH_VISIBLE_ENVELOPE_BUCKETS,
    );
    let mut envelope_results = Vec::with_capacity(selected_channels.len());
    for channel_path in channel_results
        .iter()
        .map(|result| result.channel_path.as_str())
    {
        let Some((envelope, _was_built)) = store.ensure_envelope(
            channel_path,
            &time.time,
            full_range,
            BENCH_VISIBLE_ENVELOPE_BUCKETS,
            true,
        ) else {
            return Err(format!(
                "loaded channel is missing from cache: {channel_path}"
            ));
        };
        envelope_results.push(MultiChannelEnvelopeResult {
            channel_path: channel_path.to_owned(),
            source_sample_count: envelope.source_sample_count,
            bucket_count: envelope.bucket_count(),
            bucket_size: envelope.bucket_size,
            draw_point_count: envelope.draw_point_count(),
            elapsed_sec: envelope.elapsed.as_secs_f64(),
        });
    }
    let rss_after_envelopes = process_rss_mib();

    let raw_memory_bytes = time.memory_bytes() + store.raw_memory_bytes();
    let channel_memory_bytes = channel_results
        .iter()
        .map(|result| result.memory_bytes)
        .sum::<usize>();
    let envelope_total_sec = envelope_results
        .iter()
        .map(|result| result.elapsed_sec)
        .sum::<f64>();
    let draw_point_total = envelope_results
        .iter()
        .map(|result| result.draw_point_count)
        .sum::<usize>();

    let mut report = String::new();
    report.push_str(&format!("file: {}\n", schema.path.display()));
    report.push_str(&format!("rows: {}\n", schema.row_count));
    report.push_str(&format!("channels selected: {}\n", selected_channels.len()));
    report.push_str(&format!("channel list: {}\n", selected_channels.join(", ")));
    report.push_str(&format!(
        "time: {} samples, {:.1} MiB, read {:.3}s\n",
        time.sample_count(),
        bytes_to_mib(time.memory_bytes()),
        time.elapsed.as_secs_f64()
    ));
    report.push_str(&format!(
        "channel arrays: {:.1} MiB\n",
        bytes_to_mib(channel_memory_bytes)
    ));
    report.push_str(&format!(
        "raw cache memory: {:.1} MiB\n",
        bytes_to_mib(raw_memory_bytes)
    ));
    report.push_str(&format!(
        "envelopes: {} channels, {} buckets requested, draw points {}, total {:.3}s\n",
        envelope_results.len(),
        BENCH_VISIBLE_ENVELOPE_BUCKETS,
        draw_point_total,
        envelope_total_sec
    ));
    report.push_str(&format!(
        "rss: before_schema={}, after_schema={}, after_time={}, after_all_channels={}, after_envelopes={}\n",
        format_optional_mib(rss_before_schema),
        format_optional_mib(rss_after_schema),
        format_optional_mib(rss_after_time),
        format_optional_mib(channel_results.last().and_then(|result| result.rss_after_load)),
        format_optional_mib(rss_after_envelopes)
    ));
    report.push_str("channel loads:\n");
    for (index, result) in channel_results.iter().enumerate() {
        report.push_str(&format!(
            "  #{:02} {} ({}) samples={} array={:.1} MiB read={:.3}s rss={}\n",
            index + 1,
            result.channel_name,
            result.channel_path,
            result.sample_count,
            bytes_to_mib(result.memory_bytes),
            result.read_sec,
            format_optional_mib(result.rss_after_load)
        ));
    }
    report.push_str("envelope builds:\n");
    for (index, result) in envelope_results.iter().enumerate() {
        report.push_str(&format!(
            "  #{:02} {} samples={} buckets={} bucket={} draw_points={} {:.4}s\n",
            index + 1,
            result.channel_path,
            result.source_sample_count,
            result.bucket_count,
            result.bucket_size,
            result.draw_point_count,
            result.elapsed_sec
        ));
    }

    Ok(report)
}

fn benchmark_phase2_view(path: String, channels: Vec<String>) -> Result<String, String> {
    let rss_before_schema = process_rss_mib();
    let schema = parquet_schema::read_schema_summary(&path)?;
    let rss_after_schema = process_rss_mib();
    if schema.time_column.is_none() {
        return Err("time column not found".to_owned());
    }

    let selected_channels = if channels.is_empty() {
        schema
            .channels
            .iter()
            .map(|channel| channel.path.clone())
            .collect::<Vec<_>>()
    } else {
        channels
    };
    if selected_channels.is_empty() {
        return Err("no channels selected".to_owned());
    }

    let time = parquet_waveform::read_time_column(&path, &schema)?;
    let rss_after_time = process_rss_mib();
    let full_range = time
        .time_range()
        .ok_or_else(|| "loaded time column has no time range".to_owned())?;
    let mut store = ChannelStore::default();
    let mut channel_results = Vec::with_capacity(selected_channels.len());

    for channel_name in &selected_channels {
        let channel = parquet_waveform::read_channel_values(&path, &schema, channel_name)?;
        if channel.sample_count() != time.sample_count() {
            return Err(format!(
                "time/value length mismatch for {}: {} vs {}",
                channel.channel_path,
                time.sample_count(),
                channel.sample_count()
            ));
        }

        let result = MultiChannelLoadResult {
            channel_name: channel.channel_name.clone(),
            channel_path: channel.channel_path.clone(),
            sample_count: channel.sample_count(),
            read_sec: channel.elapsed.as_secs_f64(),
            memory_bytes: channel.memory_bytes(),
            rss_after_load: process_rss_mib(),
        };
        store.insert_channel(channel);
        channel_results.push(result);
    }
    let rss_after_all_channels = process_rss_mib();

    let rows = phase2_benchmark_rows(&selected_channels);
    let (line_channel_count, step_channel_count) = phase2_benchmark_mode_counts(&rows);
    let mut app = PanelYApp::default();
    let source = app
        .workspace
        .source_mut(DEFAULT_SOURCE_ID)
        .expect("default source exists");
    source.set_parquet_path(path);
    source.schema = Some(schema);
    source.shared_time = Some(time);
    source.loaded_channels = store;
    app.view.rows = rows;
    app.view.selected_row_id = app.view.rows.first().map(|row| row.id);
    app.view.next_row_id = app.view.rows.iter().map(|row| row.id).max().unwrap_or(0) + 1;
    app.view.x_range = Some(full_range);

    let visible_results = benchmark_visible_traces(&mut app, full_range);
    let pan_cache_result = benchmark_phase2_pan_cache(&mut app, full_range);
    let rss_after_visible_benchmark = process_rss_mib();

    let hover_result = benchmark_workspace_hover(&app, full_range);
    let rss_after_hover_benchmark = process_rss_mib();

    let raw_memory_bytes = app
        .workspace
        .shared_time
        .as_ref()
        .map_or(0, parquet_waveform::TimeData::memory_bytes)
        + app.workspace.loaded_channels.raw_memory_bytes();
    let channel_memory_bytes = channel_results
        .iter()
        .map(|result| result.memory_bytes)
        .sum::<usize>();
    let visible_total_sec = visible_results
        .iter()
        .map(|result| result.elapsed_sec)
        .sum::<f64>();
    let visible_avg_sec = if visible_results.is_empty() {
        0.0
    } else {
        visible_total_sec / visible_results.len() as f64
    };
    let visible_max_sec = visible_results
        .iter()
        .map(|result| result.elapsed_sec)
        .fold(0.0, f64::max);
    let max_draw_points = visible_results
        .iter()
        .map(|result| result.draw_point_count)
        .max()
        .unwrap_or(0);
    let max_raw_step_samples = visible_results
        .iter()
        .map(|result| result.raw_step_sample_count)
        .max()
        .unwrap_or(0);
    let total_envelope_hits = visible_results
        .iter()
        .map(|result| result.envelope_stats.hits)
        .sum::<usize>();
    let total_envelope_misses = visible_results
        .iter()
        .map(|result| result.envelope_stats.misses)
        .sum::<usize>();
    let total_context_hits = visible_results
        .iter()
        .map(|result| result.envelope_stats.context_hits)
        .sum::<usize>();
    let total_context_misses = visible_results
        .iter()
        .map(|result| result.envelope_stats.context_misses)
        .sum::<usize>();
    let total_tile_hits = visible_results
        .iter()
        .map(|result| result.envelope_stats.tile_hits)
        .sum::<usize>();
    let total_tile_builds = visible_results
        .iter()
        .map(|result| result.envelope_stats.tile_builds)
        .sum::<usize>();
    let total_step_edge_hits = visible_results
        .iter()
        .map(|result| result.envelope_stats.step_edge_hits)
        .sum::<usize>();
    let total_step_edge_builds = visible_results
        .iter()
        .map(|result| result.envelope_stats.step_edge_builds)
        .sum::<usize>();
    let step_edge_count = app
        .workspace
        .loaded_channels
        .step_edge_cache
        .values()
        .map(StepEdgeCache::edge_count)
        .sum::<usize>();

    let schema = app.workspace.schema.as_ref().expect("schema is set");
    let shared_time = app.workspace.shared_time.as_ref().expect("time is set");

    let mut report = String::new();
    report.push_str(&format!("file: {}\n", schema.path.display()));
    report.push_str(&format!("rows: {}\n", schema.row_count));
    report.push_str(&format!("channels selected: {}\n", selected_channels.len()));
    report.push_str(&format!("channel list: {}\n", selected_channels.join(", ")));
    report.push_str(&format!(
        "layout: {} rows, line channels {}, step channels {}\n",
        app.view.rows.len(),
        line_channel_count,
        step_channel_count
    ));
    report.push_str(&format!(
        "time: {} samples, {:.1} MiB, read {:.3}s\n",
        shared_time.sample_count(),
        bytes_to_mib(shared_time.memory_bytes()),
        shared_time.elapsed.as_secs_f64()
    ));
    report.push_str(&format!(
        "channel arrays: {:.1} MiB\n",
        bytes_to_mib(channel_memory_bytes)
    ));
    report.push_str(&format!(
        "raw cache memory: {:.1} MiB\n",
        bytes_to_mib(raw_memory_bytes)
    ));
    report.push_str(&format!(
        "visible trace benchmark: {} ranges, {} buckets requested, total {:.3}s, avg {:.4}s, max {:.4}s, max draw points {}, max raw step samples {}\n",
        visible_results.len(),
        BENCH_VISIBLE_ENVELOPE_BUCKETS,
        visible_total_sec,
        visible_avg_sec,
        visible_max_sec,
        max_draw_points,
        max_raw_step_samples
    ));
    report.push_str(&format!(
        "envelope cache: hits {}, misses {}, context hits {}, context misses {}, line tile hits {}, builds {}, tile memory {:.1} MiB, step edge hits {}, builds {}, edges {}, edge memory {:.1} MiB\n",
        total_envelope_hits,
        total_envelope_misses,
        total_context_hits,
        total_context_misses,
        total_tile_hits,
        total_tile_builds,
        bytes_to_mib(app.workspace.loaded_channels.line_tile_memory_bytes()),
        total_step_edge_hits,
        total_step_edge_builds,
        step_edge_count,
        bytes_to_mib(app.workspace.loaded_channels.step_edge_memory_bytes())
    ));
    report.push_str(&format!(
        "pan cache benchmark: {} ranges, total {:.3}s, avg {:.4}s, max {:.4}s, cache hits {}, misses {}, context hits {}, context misses {}, tile hits {}, builds {}, step edge hits {}, builds {}\n",
        pan_cache_result.ranges,
        pan_cache_result.elapsed_sec,
        pan_cache_result.avg_visible_sec(),
        pan_cache_result.max_elapsed_sec,
        pan_cache_result.envelope_hits,
        pan_cache_result.envelope_misses,
        pan_cache_result.context_hits,
        pan_cache_result.context_misses,
        pan_cache_result.tile_hits,
        pan_cache_result.tile_builds,
        pan_cache_result.step_edge_hits,
        pan_cache_result.step_edge_builds
    ));
    report.push_str(&format!(
        "hover benchmark: {} positions, {} lookups, {} hits, total {:.4}s, avg {:.3} us/lookup\n",
        hover_result.positions,
        hover_result.lookups,
        hover_result.hits,
        hover_result.elapsed_sec,
        hover_result.avg_lookup_us()
    ));
    report.push_str(&format!(
        "rss: before_schema={}, after_schema={}, after_time={}, after_all_channels={}, after_visible_benchmark={}, after_hover_benchmark={}\n",
        format_optional_mib(rss_before_schema),
        format_optional_mib(rss_after_schema),
        format_optional_mib(rss_after_time),
        format_optional_mib(rss_after_all_channels),
        format_optional_mib(rss_after_visible_benchmark),
        format_optional_mib(rss_after_hover_benchmark)
    ));
    report.push_str("channel loads:\n");
    for (index, result) in channel_results.iter().enumerate() {
        report.push_str(&format!(
            "  #{:02} {} ({}) samples={} array={:.1} MiB read={:.3}s rss={}\n",
            index + 1,
            result.channel_name,
            result.channel_path,
            result.sample_count,
            bytes_to_mib(result.memory_bytes),
            result.read_sec,
            format_optional_mib(result.rss_after_load)
        ));
    }
    report.push_str("visible runs:\n");
    for result in &visible_results {
        report.push_str(&format!(
            "  #{:02} {:.6}..{:.6}s {:.4}s rows={} ch={} samples={} env={} dense={} step={} edge={} step_samples={} draw_points={} hover_values={} cache_h/m={}/{} ctx={}/{} tile_h/b={}/{} edge_h/b={}/{}\n",
            result.index,
            result.range.0,
            result.range.1,
            result.elapsed_sec,
            result.row_count,
            result.visible_channel_count,
            result.source_sample_count,
            result.envelope_trace_count,
            result.dense_step_trace_count,
            result.raw_step_trace_count,
            result.edge_step_trace_count,
            result.raw_step_sample_count,
            result.draw_point_count,
            result.hover_value_count,
            result.envelope_stats.hits,
            result.envelope_stats.misses,
            result.envelope_stats.context_hits,
            result.envelope_stats.context_misses,
            result.envelope_stats.tile_hits,
            result.envelope_stats.tile_builds,
            result.envelope_stats.step_edge_hits,
            result.envelope_stats.step_edge_builds
        ));
    }

    Ok(report)
}

fn benchmark_multi_source(
    source_one_path: String,
    source_two_path: String,
) -> Result<String, String> {
    let rss_before_load = process_rss_mib();
    let total_load_started = Instant::now();
    let mut app = PanelYApp {
        workspace: WorkspaceDataState::with_parquet_path(source_one_path.clone()),
        ..PanelYApp::default()
    };
    let source_two_id = app.workspace.add_source(source_two_path.clone());
    let source_specs = [
        (DEFAULT_SOURCE_ID, source_one_path),
        (source_two_id, source_two_path),
    ];
    let mut source_results = Vec::with_capacity(source_specs.len());

    for (source_index, (source_id, path)) in source_specs.into_iter().enumerate() {
        let schema_started = Instant::now();
        let schema = parquet_schema::read_schema_summary(&path)?;
        let schema_elapsed_sec = schema_started.elapsed().as_secs_f64();
        if schema.time_column.is_none() {
            return Err(format!("Source #{} has no time column", source_index + 1));
        }
        if schema.channels.is_empty() {
            return Err(format!("Source #{} has no channels", source_index + 1));
        }

        let time = parquet_waveform::read_time_column(&path, &schema)?;
        if time.time_range().is_none() {
            return Err(format!(
                "Source #{} time column has no range",
                source_index + 1
            ));
        }
        let mut store = ChannelStore::default();
        let mut channel_results = Vec::with_capacity(schema.channels.len());
        for channel_info in &schema.channels {
            let channel =
                parquet_waveform::read_channel_values(&path, &schema, &channel_info.path)?;
            if channel.sample_count() != time.sample_count() {
                return Err(format!(
                    "Source #{} time/value length mismatch for {}: {} vs {}",
                    source_index + 1,
                    channel.channel_path,
                    time.sample_count(),
                    channel.sample_count()
                ));
            }
            channel_results.push(MultiChannelLoadResult {
                channel_name: channel.channel_name.clone(),
                channel_path: channel.channel_path.clone(),
                sample_count: channel.sample_count(),
                read_sec: channel.elapsed.as_secs_f64(),
                memory_bytes: channel.memory_bytes(),
                rss_after_load: None,
            });
            store.insert_channel(channel);
        }

        let raw_memory_bytes = time.memory_bytes() + store.raw_memory_bytes();
        let result = MultiSourceLoadResult {
            source_number: source_index + 1,
            path: schema.path.clone(),
            row_count: schema.row_count,
            schema_elapsed_sec,
            time_sample_count: time.sample_count(),
            time_read_sec: time.elapsed.as_secs_f64(),
            raw_memory_bytes,
            channel_results,
        };
        let source = app
            .workspace
            .source_mut(source_id)
            .ok_or_else(|| format!("Source #{} disappeared during load", source_index + 1))?;
        source.schema = Some(schema);
        source.shared_time = Some(time);
        source.loaded_channels = store;
        source_results.push(result);
    }

    let load_wall_sec = total_load_started.elapsed().as_secs_f64();
    let rss_after_load = process_rss_mib();
    let full_range = app
        .workspace
        .display_time_range()
        .ok_or_else(|| "multi-source workspace has no display time range".to_owned())?;
    app.view.rows = multi_source_benchmark_rows(&app.workspace);
    app.view.selected_row_id = app.view.rows.first().map(|row| row.id);
    app.view.next_row_id = app.view.rows.iter().map(|row| row.id).max().unwrap_or(0) + 1;
    app.view.x_range = Some(full_range);
    let (line_channel_count, step_channel_count) = phase2_benchmark_mode_counts(&app.view.rows);

    let visible_results = benchmark_visible_traces(&mut app, full_range);
    let rss_after_visible_benchmark = process_rss_mib();
    let (pan_cache_result, per_source_cache_results) =
        benchmark_workspace_pan_cache(&mut app, full_range);
    let rss_after_pan_benchmark = process_rss_mib();
    let hover_result = benchmark_workspace_hover(&app, full_range);
    let rss_after_hover_benchmark = process_rss_mib();
    let cache_stats = app.workspace.cache_stats();

    let visible_total_sec = visible_results
        .iter()
        .map(|result| result.elapsed_sec)
        .sum::<f64>();
    let visible_avg_sec = if visible_results.is_empty() {
        0.0
    } else {
        visible_total_sec / visible_results.len() as f64
    };
    let visible_max_sec = visible_results
        .iter()
        .map(|result| result.elapsed_sec)
        .fold(0.0, f64::max);
    let max_draw_points = visible_results
        .iter()
        .map(|result| result.draw_point_count)
        .max()
        .unwrap_or(0);
    let max_visible_channels = visible_results
        .iter()
        .map(|result| result.visible_channel_count)
        .max()
        .unwrap_or(0);

    let mut report = String::new();
    report.push_str("multi-source benchmark\n");
    report.push_str(&format!(
        "workspace: {} sources, display range {:.6}..{:.6}s, {} rows, line channels {}, step channels {}\n",
        app.workspace.source_order.len(),
        full_range.0,
        full_range.1,
        app.view.rows.len(),
        line_channel_count,
        step_channel_count
    ));
    report.push_str(&format!(
        "load: wall {:.3}s, raw cache {:.1} MiB, loaded channels {}\n",
        load_wall_sec,
        bytes_to_mib(cache_stats.total_raw_memory_bytes()),
        cache_stats.loaded_channel_count
    ));
    report.push_str(&format!(
        "visible trace benchmark: {} ranges, {} buckets requested, total {:.3}s, avg {:.4}s, max {:.4}s, max channels {}, max draw points {}\n",
        visible_results.len(),
        BENCH_VISIBLE_ENVELOPE_BUCKETS,
        visible_total_sec,
        visible_avg_sec,
        visible_max_sec,
        max_visible_channels,
        max_draw_points
    ));
    report.push_str(&format!(
        "pan cache benchmark: {} ranges, total {:.3}s, avg {:.4}s, max {:.4}s, cache hits {}, misses {}, context hits {}, context misses {}, tile hits {}, builds {}, step edge hits {}, builds {}\n",
        pan_cache_result.ranges,
        pan_cache_result.elapsed_sec,
        pan_cache_result.avg_visible_sec(),
        pan_cache_result.max_elapsed_sec,
        pan_cache_result.envelope_hits,
        pan_cache_result.envelope_misses,
        pan_cache_result.context_hits,
        pan_cache_result.context_misses,
        pan_cache_result.tile_hits,
        pan_cache_result.tile_builds,
        pan_cache_result.step_edge_hits,
        pan_cache_result.step_edge_builds
    ));
    report.push_str(&format!(
        "hover benchmark: {} positions, {} lookups, {} hits, total {:.4}s, avg {:.3} us/lookup\n",
        hover_result.positions,
        hover_result.lookups,
        hover_result.hits,
        hover_result.elapsed_sec,
        hover_result.avg_lookup_us()
    ));
    report.push_str(&format!(
        "draw caches: envelopes {}, line tiles {} ({:.1} MiB), step edges {} ({:.1} MiB), fallback hints {}\n",
        cache_stats.envelope_count,
        cache_stats.line_tile_cache_count,
        bytes_to_mib(cache_stats.line_tile_memory_bytes),
        cache_stats.step_edge_cache_count,
        bytes_to_mib(cache_stats.step_edge_memory_bytes),
        cache_stats.step_fallback_hint_count
    ));
    report.push_str(&format!(
        "rss: before_load={}, after_load={}, after_visible={}, after_pan={}, after_hover={}\n",
        format_optional_mib(rss_before_load),
        format_optional_mib(rss_after_load),
        format_optional_mib(rss_after_visible_benchmark),
        format_optional_mib(rss_after_pan_benchmark),
        format_optional_mib(rss_after_hover_benchmark)
    ));
    report.push_str("source loads:\n");
    for source_result in &source_results {
        let channel_read_sec = source_result
            .channel_results
            .iter()
            .map(|channel| channel.read_sec)
            .sum::<f64>();
        report.push_str(&format!(
            "  Source #{}: {} rows={} time_samples={} channels={} schema={:.3}s time={:.3}s channel_total={:.3}s raw={:.1} MiB\n",
            source_result.source_number,
            source_result.path.display(),
            source_result.row_count,
            source_result.time_sample_count,
            source_result.channel_results.len(),
            source_result.schema_elapsed_sec,
            source_result.time_read_sec,
            channel_read_sec,
            bytes_to_mib(source_result.raw_memory_bytes)
        ));
        for channel in &source_result.channel_results {
            report.push_str(&format!(
                "    {} ({}) samples={} array={:.1} MiB read={:.3}s\n",
                channel.channel_name,
                channel.channel_path,
                channel.sample_count,
                bytes_to_mib(channel.memory_bytes),
                channel.read_sec
            ));
        }
    }
    report.push_str("per-source pan cache:\n");
    for source_id in &app.workspace.source_order {
        let source_number = app.workspace.source_number(*source_id).unwrap_or(0);
        let source_result = per_source_cache_results
            .get(source_id)
            .copied()
            .unwrap_or_default();
        let stats = source_result.envelope_stats;
        report.push_str(&format!(
            "  Source #{}: frames={} cache h/m={}/{} context h/m={}/{} tile h/b={}/{} step edge h/b={}/{}\n",
            source_number,
            source_result.frames,
            stats.hits,
            stats.misses,
            stats.context_hits,
            stats.context_misses,
            stats.tile_hits,
            stats.tile_builds,
            stats.step_edge_hits,
            stats.step_edge_builds
        ));
    }

    Ok(report)
}

fn benchmark_visible_traces(
    app: &mut PanelYApp,
    full_range: (f64, f64),
) -> Vec<Phase2VisibleRunResult> {
    let ranges = benchmark_ranges(full_range, BENCH_RANGE_RUNS);
    let mut visible_results = Vec::with_capacity(ranges.len());
    for (index, range) in ranges.into_iter().enumerate() {
        app.view.x_range = Some(range);
        app.view.hover_x = Some((range.0 + range.1) * 0.5);
        let started = Instant::now();
        let visible_rows = app.visible_row_traces(BENCH_VISIBLE_ENVELOPE_BUCKETS, true, false);
        let elapsed_sec = started.elapsed().as_secs_f64();
        let envelope_stats = app.workspace.envelope_stats();

        let mut visible_channel_count = 0usize;
        let mut envelope_trace_count = 0usize;
        let mut dense_step_trace_count = 0usize;
        let mut raw_step_trace_count = 0usize;
        let mut edge_step_trace_count = 0usize;
        let mut raw_step_sample_count = 0usize;
        let mut draw_point_count = 0usize;
        let mut source_sample_count = 0usize;
        let mut hover_value_count = 0usize;

        for row in &visible_rows {
            for trace in &row.traces {
                visible_channel_count += 1;
                draw_point_count += trace_draw_point_count(trace);
                source_sample_count = source_sample_count.max(trace_source_sample_count(trace));
                if trace.hover_value.is_some() {
                    hover_value_count += 1;
                }

                match &trace.data {
                    VisibleTraceData::Envelope(_) => {
                        envelope_trace_count += 1;
                    }
                    VisibleTraceData::DenseStepEnvelope(_) => {
                        dense_step_trace_count += 1;
                    }
                    VisibleTraceData::RawStep(raw_step) => {
                        raw_step_trace_count += 1;
                        raw_step_sample_count += raw_step.samples.len();
                        if raw_step.kind == StepTraceKind::ChangePoints {
                            edge_step_trace_count += 1;
                        }
                    }
                }
            }
        }

        visible_results.push(Phase2VisibleRunResult {
            index: index + 1,
            range,
            elapsed_sec,
            row_count: visible_rows.len(),
            visible_channel_count,
            envelope_trace_count,
            dense_step_trace_count,
            raw_step_trace_count,
            edge_step_trace_count,
            raw_step_sample_count,
            draw_point_count,
            source_sample_count,
            hover_value_count,
            envelope_stats,
        });
    }

    visible_results
}

fn phase2_benchmark_rows(selected_channels: &[String]) -> Vec<PlotRow> {
    if selected_channels.is_empty() {
        return vec![PlotRow::new(0)];
    }

    selected_channels
        .chunks(3)
        .enumerate()
        .map(|(row_index, channels)| {
            let mut row = PlotRow::new(row_index as u64);
            row.channels = channels
                .iter()
                .enumerate()
                .map(|(channel_index, channel_path)| {
                    let mut row_channel = RowChannel::new(
                        ChannelKey::new(DEFAULT_SOURCE_ID, channel_path.clone()),
                        channel_index,
                    );
                    row_channel.draw_mode = phase2_benchmark_draw_mode(channel_path);
                    row_channel
                })
                .collect();
            row
        })
        .collect()
}

fn multi_source_benchmark_rows(workspace: &WorkspaceDataState) -> Vec<PlotRow> {
    let mut channel_paths = Vec::new();
    let mut seen_paths = BTreeSet::new();
    for source_id in &workspace.source_order {
        let Some(schema) = workspace
            .source(*source_id)
            .and_then(|source| source.schema.as_ref())
        else {
            continue;
        };
        for channel in &schema.channels {
            if seen_paths.insert(channel.path.clone()) {
                channel_paths.push(channel.path.clone());
            }
        }
    }

    let mut rows = channel_paths
        .into_iter()
        .enumerate()
        .filter_map(|(row_index, channel_path)| {
            let channels = workspace
                .source_order
                .iter()
                .enumerate()
                .filter_map(|(source_index, source_id)| {
                    let source = workspace.source(*source_id)?;
                    source.loaded_channels.has_channel(&channel_path).then(|| {
                        let mut row_channel = RowChannel::new(
                            ChannelKey::new(*source_id, channel_path.clone()),
                            source_index,
                        );
                        row_channel.draw_mode = phase2_benchmark_draw_mode(&channel_path);
                        row_channel
                    })
                })
                .collect::<Vec<_>>();
            (!channels.is_empty()).then_some(PlotRow {
                id: row_index as u64,
                y_range: RowYRange::default(),
                channels,
            })
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows.push(PlotRow::new(0));
    }
    rows
}

fn phase2_benchmark_draw_mode(channel_path: &str) -> DrawMode {
    let lower = channel_path.to_ascii_lowercase();
    if lower.contains("pwm") || lower.contains("gate") || lower.contains("step") {
        DrawMode::Step
    } else {
        DrawMode::Line
    }
}

fn phase2_benchmark_mode_counts(rows: &[PlotRow]) -> (usize, usize) {
    rows.iter().flat_map(|row| row.channels.iter()).fold(
        (0usize, 0usize),
        |(line, step), channel| match channel.draw_mode {
            DrawMode::Line => (line + 1, step),
            DrawMode::Step => (line, step + 1),
        },
    )
}

fn benchmark_phase2_pan_cache(
    app: &mut PanelYApp,
    full_range: (f64, f64),
) -> Phase2PanCacheBenchmarkResult {
    benchmark_workspace_pan_cache(app, full_range).0
}

fn benchmark_workspace_pan_cache(
    app: &mut PanelYApp,
    full_range: (f64, f64),
) -> (
    Phase2PanCacheBenchmarkResult,
    BTreeMap<SourceId, SourcePanCacheBenchmarkResult>,
) {
    let ranges = benchmark_pan_ranges(full_range, BENCH_RANGE_RUNS);
    if ranges.is_empty() {
        return (Phase2PanCacheBenchmarkResult::default(), BTreeMap::new());
    }

    app.workspace.clear_envelope_caches();
    let mut result = Phase2PanCacheBenchmarkResult {
        ranges: ranges.len(),
        ..Phase2PanCacheBenchmarkResult::default()
    };
    let mut per_source = app
        .workspace
        .source_order
        .iter()
        .copied()
        .map(|source_id| (source_id, SourcePanCacheBenchmarkResult::default()))
        .collect::<BTreeMap<_, _>>();

    for range in ranges {
        app.view.x_range = Some(range);
        app.view.hover_x = Some((range.0 + range.1) * 0.5);
        let started = std::time::Instant::now();
        let _visible_rows = app.visible_row_traces(BENCH_VISIBLE_ENVELOPE_BUCKETS, true, false);
        let elapsed_sec = started.elapsed().as_secs_f64();
        let envelope_stats = app.workspace.envelope_stats();

        result.elapsed_sec += elapsed_sec;
        result.max_elapsed_sec = result.max_elapsed_sec.max(elapsed_sec);
        result.envelope_hits += envelope_stats.hits;
        result.envelope_misses += envelope_stats.misses;
        result.context_hits += envelope_stats.context_hits;
        result.context_misses += envelope_stats.context_misses;
        result.tile_hits += envelope_stats.tile_hits;
        result.tile_builds += envelope_stats.tile_builds;
        result.step_edge_hits += envelope_stats.step_edge_hits;
        result.step_edge_builds += envelope_stats.step_edge_builds;

        for (source_id, source_result) in &mut per_source {
            let Some(source) = app.workspace.source(*source_id) else {
                continue;
            };
            source_result.frames += 1;
            source_result
                .envelope_stats
                .accumulate(source.loaded_channels.last_envelope_stats);
        }
    }

    (result, per_source)
}

fn benchmark_workspace_hover(
    app: &PanelYApp,
    full_range: (f64, f64),
) -> Phase2HoverBenchmarkResult {
    let Some((range_start, range_end)) = normalized_range(full_range) else {
        return Phase2HoverBenchmarkResult::empty();
    };
    let span = range_end - range_start;
    if span <= 0.0 {
        return Phase2HoverBenchmarkResult::empty();
    }

    let started = std::time::Instant::now();
    let mut lookups = 0usize;
    let mut hits = 0usize;
    for index in 0..BENCH_HOVER_RUNS {
        let ratio = if BENCH_HOVER_RUNS <= 1 {
            0.5
        } else {
            index as f64 / (BENCH_HOVER_RUNS - 1) as f64
        };
        let target_time = range_start + span * ratio;
        for row in &app.view.rows {
            for row_channel in row.channels.iter().filter(|channel| channel.visible) {
                let Some(source) = app.workspace.source(row_channel.key.source_id) else {
                    continue;
                };
                let Some(shared_time) = source.shared_time.as_ref() else {
                    continue;
                };
                let Some(channel) = source
                    .loaded_channels
                    .channel(&row_channel.key.channel_path)
                else {
                    continue;
                };
                lookups += 1;
                if hover_value_at_time(
                    &shared_time.time,
                    &channel.values,
                    source.time_transform.to_raw(target_time),
                    row_channel.draw_mode,
                )
                .is_some()
                {
                    hits += 1;
                }
            }
        }
    }

    Phase2HoverBenchmarkResult {
        positions: BENCH_HOVER_RUNS,
        lookups,
        hits,
        elapsed_sec: started.elapsed().as_secs_f64(),
    }
}

#[derive(Debug)]
struct BenchmarkRangeResult {
    index: usize,
    range: (f64, f64),
    source_sample_count: usize,
    bucket_count: usize,
    bucket_size: usize,
    draw_point_count: usize,
    elapsed_sec: f64,
}

#[derive(Debug)]
struct StressBlockResult {
    completed_runs: usize,
    elapsed_wall_sec: f64,
    rss_mib: Option<f64>,
}

#[derive(Debug)]
struct MultiChannelLoadResult {
    channel_name: String,
    channel_path: String,
    sample_count: usize,
    read_sec: f64,
    memory_bytes: usize,
    rss_after_load: Option<f64>,
}

#[derive(Debug)]
struct MultiSourceLoadResult {
    source_number: usize,
    path: PathBuf,
    row_count: i64,
    schema_elapsed_sec: f64,
    time_sample_count: usize,
    time_read_sec: f64,
    raw_memory_bytes: usize,
    channel_results: Vec<MultiChannelLoadResult>,
}

#[derive(Debug)]
struct MultiChannelEnvelopeResult {
    channel_path: String,
    source_sample_count: usize,
    bucket_count: usize,
    bucket_size: usize,
    draw_point_count: usize,
    elapsed_sec: f64,
}

#[derive(Debug)]
struct Phase2VisibleRunResult {
    index: usize,
    range: (f64, f64),
    elapsed_sec: f64,
    row_count: usize,
    visible_channel_count: usize,
    envelope_trace_count: usize,
    dense_step_trace_count: usize,
    raw_step_trace_count: usize,
    edge_step_trace_count: usize,
    raw_step_sample_count: usize,
    draw_point_count: usize,
    source_sample_count: usize,
    hover_value_count: usize,
    envelope_stats: EnvelopeCacheStats,
}

#[derive(Debug, Default)]
struct Phase2PanCacheBenchmarkResult {
    ranges: usize,
    elapsed_sec: f64,
    max_elapsed_sec: f64,
    envelope_hits: usize,
    envelope_misses: usize,
    context_hits: usize,
    context_misses: usize,
    tile_hits: usize,
    tile_builds: usize,
    step_edge_hits: usize,
    step_edge_builds: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct SourcePanCacheBenchmarkResult {
    frames: usize,
    envelope_stats: EnvelopeCacheStats,
}

impl Phase2PanCacheBenchmarkResult {
    fn avg_visible_sec(&self) -> f64 {
        if self.ranges == 0 {
            0.0
        } else {
            self.elapsed_sec / self.ranges as f64
        }
    }
}

#[derive(Debug)]
struct Phase2HoverBenchmarkResult {
    positions: usize,
    lookups: usize,
    hits: usize,
    elapsed_sec: f64,
}

impl Phase2HoverBenchmarkResult {
    fn empty() -> Self {
        Self {
            positions: 0,
            lookups: 0,
            hits: 0,
            elapsed_sec: 0.0,
        }
    }

    fn avg_lookup_us(&self) -> f64 {
        if self.lookups == 0 {
            0.0
        } else {
            self.elapsed_sec * 1_000_000.0 / self.lookups as f64
        }
    }
}

fn benchmark_ranges(full_range: (f64, f64), run_count: usize) -> Vec<(f64, f64)> {
    let Some((start, end)) = normalized_range(full_range) else {
        return Vec::new();
    };
    let span = end - start;
    if span <= 0.0 || run_count == 0 {
        return Vec::new();
    }

    let mut ranges = Vec::with_capacity(run_count);
    for index in 0..run_count {
        let phase = index as f64 / run_count.max(1) as f64;
        let window_ratio = match index % 4 {
            0 => 0.50,
            1 => 0.20,
            2 => 0.05,
            _ => 0.01,
        };
        let window = (span * window_ratio).max(span * 1.0e-9);
        let available = (span - window).max(0.0);
        let left = start + available * phase;
        ranges.push((left, left + window));
    }

    ranges
}

fn benchmark_pan_ranges(full_range: (f64, f64), run_count: usize) -> Vec<(f64, f64)> {
    let Some((start, end)) = normalized_range(full_range) else {
        return Vec::new();
    };
    let span = end - start;
    if span <= 0.0 || run_count == 0 {
        return Vec::new();
    }

    let window = (span * 0.20).max(span * 1.0e-9).min(span);
    if window >= span || run_count == 1 {
        return vec![(start, start + window)];
    }

    let available = span - window;
    let requested_total_shift = window * 0.10 * run_count.saturating_sub(1) as f64;
    let total_shift = requested_total_shift.min(available);
    let step = total_shift / run_count.saturating_sub(1) as f64;
    let first_left = start + (available - total_shift) * 0.5;

    (0..run_count)
        .map(|index| {
            let left = (first_left + step * index as f64).min(end - window);
            (left, left + window)
        })
        .collect()
}

fn process_rss_mib() -> Option<f64> {
    let pid = std::process::id().to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    let rss_kib = text.trim().parse::<f64>().ok()?;
    Some(rss_kib / 1024.0)
}

fn format_optional_mib(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1} MiB"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn load_channel_data(
    path: &str,
    summary: &parquet_schema::SchemaSummary,
    channel_path: &str,
    needs_time: bool,
    needs_channel: bool,
) -> Result<LoadedChannelData, String> {
    let time = if needs_time {
        Some(parquet_waveform::read_time_column(path, summary)?)
    } else {
        None
    };
    let channel = if needs_channel {
        Some(parquet_waveform::read_channel_values(
            path,
            summary,
            channel_path,
        )?)
    } else {
        None
    };

    if let (Some(time), Some(channel)) = (&time, &channel)
        && time.sample_count() != channel.sample_count()
    {
        return Err(format!(
            "time/value length mismatch: {} vs {}",
            time.sample_count(),
            channel.sample_count()
        ));
    }

    Ok(LoadedChannelData { time, channel })
}

fn spawn_load_channel_job(
    tx: mpsc::Sender<LoadJobResult>,
    ctx: egui::Context,
    job_id: u64,
    source_id: SourceId,
    source_generation: u64,
    path: String,
    summary: parquet_schema::SchemaSummary,
    channel_path: String,
    needs_time: bool,
    needs_channel: bool,
) {
    std::thread::spawn(move || {
        let result = load_channel_data(&path, &summary, &channel_path, needs_time, needs_channel);
        let _ = tx.send(LoadJobResult {
            job_id,
            source_id,
            source_generation,
            channel_path,
            result,
        });
        ctx.request_repaint();
    });
}

fn load_result_matches_active_job(result: &LoadJobResult, job: &ActiveLoadJob) -> bool {
    result.source_id == job.source_id
        && result.source_generation == job.source_generation
        && job.channel_key.source_id == job.source_id
        && result.channel_path == job.channel_key.channel_path
}

impl PanelYApp {
    fn edit_parquet_path_for_source(&mut self, source_id: SourceId, path: String) {
        self.update_parquet_path_for_source(source_id, path, false);
    }

    fn select_parquet_path_for_source(&mut self, source_id: SourceId, path: String) {
        self.update_parquet_path_for_source(source_id, path, true);
    }

    fn update_parquet_path_for_source(
        &mut self,
        source_id: SourceId,
        path: String,
        remember_recent: bool,
    ) {
        if self.load.is_source_loading(source_id) {
            let source_label = source_presentation(&self.workspace, source_id)
                .map(|source| source.short_label)
                .unwrap_or_else(|| "removed source".to_owned());
            self.load.status = format!("Wait for active load jobs before changing {source_label}");
            return;
        }

        let old_parquet_path = self
            .workspace
            .source(source_id)
            .map(|source| source.parquet_path.clone())
            .unwrap_or_default();
        let path = path.trim().to_owned();
        if self
            .workspace
            .set_source_parquet_path(source_id, path.clone())
        {
            if self.workspace.source_number(source_id) == Some(1) {
                self.update_default_plot_config_path_after_parquet_change(&old_parquet_path, &path);
            }
            self.cleanup_source_references(source_id);
            self.reconcile_view_range_to_workspace();
            self.plot_config_report = None;
        }
        if remember_recent && !path.is_empty() {
            remember_recent_parquet_path(&mut self.workspace.recent_parquet_paths, path);
        }
        self.load.error = None;
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "source".to_owned());
        self.load.status = format!("{source_label}: path changed; load schema to continue");
    }

    fn update_default_plot_config_path_after_parquet_change(
        &mut self,
        old_parquet_path: &str,
        new_parquet_path: &str,
    ) {
        let old_default = default_plot_config_path(old_parquet_path);
        if self.plot_config_path.trim().is_empty() || self.plot_config_path == old_default {
            self.plot_config_path = default_plot_config_path(new_parquet_path);
        }
    }

    fn cleanup_source_references(&mut self, source_id: SourceId) -> usize {
        let removed = self.view.cleanup_source(source_id);
        self.analysis.cleanup_source(source_id);
        self.load.refresh_progress(&self.workspace);
        removed
    }

    fn clear_source_after_schema_failure(&mut self, source_id: SourceId) {
        self.workspace.reset_source(source_id);
        self.cleanup_source_references(source_id);
        self.reconcile_view_range_to_workspace();
        self.plot_config_report = None;
        self.load.refresh_progress(&self.workspace);
    }

    fn add_source_from_ui(&mut self) {
        let source_id = self.workspace.add_source(String::new());
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "new source".to_owned());
        self.load.error = None;
        self.load.status = format!("Added {source_label}; choose a Parquet path");
    }

    fn apply_source_time_offset(
        &mut self,
        source_id: SourceId,
        offset_seconds: f64,
    ) -> Option<bool> {
        if !offset_seconds.is_finite() {
            self.load.error = Some("Time offset must be a finite number".to_owned());
            self.load.status = self.load.error.clone().unwrap_or_default();
            return None;
        }
        if self.workspace.source(source_id).is_none() {
            self.load.error = Some("The requested data source no longer exists".to_owned());
            self.load.status = self.load.error.clone().unwrap_or_default();
            return None;
        }
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "source".to_owned());
        let Some(changed) = self
            .workspace
            .set_source_time_offset(source_id, offset_seconds)
        else {
            self.load.error = Some(format!("{source_label}: time offset update failed"));
            self.load.status = self.load.error.clone().unwrap_or_default();
            return None;
        };

        let offset_seconds = self
            .workspace
            .source(source_id)
            .map(|source| source.time_transform.offset_seconds)
            .unwrap_or_default();
        if changed {
            self.analysis.invalidate_time_transform_dependents();
            self.reconcile_view_range_to_workspace();
        }
        let display_range = self
            .workspace
            .source(source_id)
            .and_then(data_source_display_time_range);
        self.load.error = None;
        self.load.status = format!(
            "{source_label}: time offset {} to {offset_seconds:.9} s; display range {} s",
            if changed { "set" } else { "unchanged" },
            format_range(display_range)
        );
        Some(changed)
    }

    fn start_source_at_zero(&mut self, source_id: SourceId) {
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "source".to_owned());
        let Some((raw_start, offset_seconds)) =
            self.workspace.source(source_id).and_then(|source| {
                Some((
                    source.shared_time.as_ref()?.time.first().copied()?,
                    source_start_at_zero_offset(source)?,
                ))
            })
        else {
            self.load.error = Some(format!(
                "{source_label}: load time data before using Start at zero"
            ));
            self.load.status = self.load.error.clone().unwrap_or_default();
            return;
        };

        if self
            .apply_source_time_offset(source_id, offset_seconds)
            .is_some()
        {
            self.load.status =
                format!("{source_label}: raw start {raw_start:.9} s aligned to display time 0 s");
        }
    }

    fn start_all_sources_at_zero(&mut self) {
        let alignments = self
            .workspace
            .source_order
            .iter()
            .filter_map(|source_id| {
                let source = self.workspace.source(*source_id)?;
                Some((*source_id, source_start_at_zero_offset(source)?))
            })
            .collect::<Vec<_>>();
        if alignments.is_empty() {
            self.load.error = Some(
                "Load time data for at least one source before using Start all at zero".to_owned(),
            );
            self.load.status = self.load.error.clone().unwrap_or_default();
            return;
        }

        let mut changed_count = 0usize;
        for (source_id, offset_seconds) in &alignments {
            if self
                .workspace
                .set_source_time_offset(*source_id, *offset_seconds)
                == Some(true)
            {
                changed_count = changed_count.saturating_add(1);
            }
        }
        if changed_count > 0 {
            self.analysis.invalidate_time_transform_dependents();
            self.reconcile_view_range_to_workspace();
        }
        let skipped_count = self
            .workspace
            .source_order
            .len()
            .saturating_sub(alignments.len());
        self.load.error = None;
        self.load.status = format!(
            "Aligned {} loaded source(s) to display time 0 ({} changed, {} without time skipped)",
            alignments.len(),
            changed_count,
            skipped_count
        );
    }

    fn remove_source_from_ui(&mut self, source_id: SourceId) {
        if self.load.is_source_loading(source_id) {
            self.load.status =
                "Wait for this source's active load jobs before removing it".to_owned();
            return;
        }
        let old_selected_source_id = self.workspace.selected_source_id();
        let old_selected_path = self
            .workspace
            .selected_source()
            .map(|source| source.parquet_path.clone());
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "source".to_owned());
        let removed_channels = self.cleanup_source_references(source_id);
        if self.workspace.remove_source(source_id).is_none() {
            self.load.status = "The requested data source no longer exists".to_owned();
            return;
        }
        if self.workspace.selected_source_id() != old_selected_source_id {
            let new_selected_path = self
                .workspace
                .selected_source()
                .map(|source| source.parquet_path.clone());
            self.update_default_plot_config_path_after_selected_source_change(
                old_selected_path.as_deref(),
                new_selected_path.as_deref(),
            );
        }
        self.reconcile_view_range_to_workspace();
        self.load.refresh_progress(&self.workspace);
        self.plot_config_report = None;
        self.load.error = None;
        self.load.status =
            format!("Removed {source_label}; cleaned {removed_channels} row channel reference(s)");
    }

    fn update_default_plot_config_path_after_selected_source_change(
        &mut self,
        old_parquet_path: Option<&str>,
        new_parquet_path: Option<&str>,
    ) {
        let tracked_old_default = old_parquet_path
            .map(default_plot_config_path)
            .is_some_and(|old_default| self.plot_config_path == old_default);
        if self.plot_config_path.trim().is_empty() || tracked_old_default {
            self.plot_config_path = new_parquet_path
                .map(default_plot_config_path)
                .unwrap_or_default();
        }
    }

    fn reconcile_view_range_to_workspace(&mut self) {
        let Some(full_range) = self.workspace.display_time_range() else {
            self.view.x_range = None;
            self.workspace.clear_envelope_caches();
            return;
        };
        let min_span = self
            .workspace
            .display_min_view_span()
            .unwrap_or_else(|| min_view_span(full_range, 1));
        let next_range = clamp_view_range(
            self.view.x_range.unwrap_or(full_range),
            full_range,
            min_span,
        );
        if self.view.x_range != Some(next_range) {
            self.view.x_range = Some(next_range);
            self.workspace.clear_envelope_caches();
        }
    }

    fn save_session_preset(&mut self) {
        if self.load.is_busy() {
            self.load.status = "Wait for active load jobs before saving session preset".to_owned();
            return;
        }
        if !self
            .workspace
            .sources
            .values()
            .any(|source| source.schema.is_some())
        {
            self.load.status = "Load schema before saving session preset".to_owned();
            return;
        }
        self.view.ensure_row_state();
        let preset = SessionPreset::capture(&self.workspace, &self.view);
        self.load.status = format!("Saved session preset: {}", preset.summary());
        self.session_preset = Some(preset);
        self.session_preset_status = None;
        self.load.error = None;
    }

    fn apply_session_preset(&mut self, ctx: &egui::Context) {
        if self.load.is_busy() {
            self.load.status =
                "Wait for active load jobs before applying session preset".to_owned();
            return;
        }
        self.session_preset_status = None;
        let Some(preset) = self.session_preset.clone() else {
            self.load.status = "No session preset saved".to_owned();
            return;
        };
        // Stage new schemas before touching the current workspace. Same runtime
        // identity AND path are required to reuse a source's raw cache.
        let mut prepared = BTreeMap::new();
        let mut source_ids = BTreeMap::new();
        let mut next_id = self.workspace.next_source_id;
        let mut warnings = Vec::new();
        for saved in &preset.sources {
            let reusable_id = preset
                .runtime_source_ids
                .get(&saved.id)
                .copied()
                .filter(|id| {
                    self.workspace.source(*id).is_some_and(|source| {
                        source.parquet_path == saved.parquet_path && source.schema.is_some()
                    })
                });
            if let Some(id) = reusable_id {
                source_ids.insert(saved.id, id);
                continue;
            }
            let result = validate_parquet_path(&saved.parquet_path)
                .map_err(|error| error.message())
                .and_then(|path| {
                    parquet_schema::read_schema_summary(path).map_err(|error| error.to_string())
                })
                .and_then(|schema| {
                    if schema.time_column.is_some() {
                        Ok(schema)
                    } else {
                        Err("Time column is required".to_owned())
                    }
                });
            match result {
                Ok(schema) => {
                    let id = SourceId::new(next_id);
                    next_id = next_id.checked_add(1).expect("source id space exhausted");
                    let mut source = DataSourceState::new(id, saved.parquet_path.clone());
                    source.schema = Some(schema);
                    prepared.insert(id, source);
                    source_ids.insert(saved.id, id);
                }
                Err(error) => warnings.push(format!(
                    "Unavailable source {} ({}): {error}",
                    saved.id.value(),
                    saved.parquet_path
                )),
            }
        }
        if source_ids.is_empty() {
            self.load.status =
                "Session preset apply failed; current workspace preserved".to_owned();
            self.load.error = Some(warnings.join("\n"));
            return;
        }
        let mut order = Vec::new();
        for saved in &preset.sources {
            let Some(&id) = source_ids.get(&saved.id) else {
                continue;
            };
            let mut source = prepared
                .remove(&id)
                .or_else(|| self.workspace.sources.remove(&id))
                .expect("prepared or reused source");
            source.label = saved.label.clone();
            source.time_transform.offset_seconds = saved.offset_seconds;
            source.loaded_channels.clear_draw_caches();
            prepared.insert(id, source);
            order.push(id);
            remember_recent_parquet_path(
                &mut self.workspace.recent_parquet_paths,
                saved.parquet_path.clone(),
            );
        }
        self.workspace.sources = prepared;
        self.workspace.source_order = order;
        self.workspace.next_source_id = next_id;
        self.workspace.selected_source_id = preset
            .selected_source_id
            .and_then(|id| source_ids.get(&id).copied())
            .or_else(|| self.workspace.source_order.first().copied());
        self.session_preset
            .as_mut()
            .expect("saved preset")
            .runtime_source_ids = source_ids.clone();
        let report = preset.apply_to_view(&mut self.view, &self.workspace, &source_ids);
        self.analysis.clear_all();
        self.plot_config_report = None;
        self.fit_plot_config_after_load = false;
        let selected_channel = self.view.selected_channel.clone();
        let selected_source = self.workspace.selected_source_id;
        let selected_row = self.view.selected_row_id;
        let channels = self
            .view
            .rows
            .iter()
            .flat_map(|row| &row.channels)
            .filter(|channel| channel.visible && self.workspace.channel(&channel.key).is_none())
            .map(|channel| channel.key.clone())
            .collect::<BTreeSet<_>>();
        for key in channels {
            if let Some(row_id) = self.first_row_id_containing_channel(&key) {
                self.request_channel_load(ctx, &key, row_id, false);
            }
        }
        self.view.selected_channel = selected_channel;
        self.workspace.selected_source_id = selected_source;
        self.view.selected_row_id = selected_row;
        self.view.x_range = preset.x_range;
        self.pending_session_x_range = self.load.is_busy().then_some(preset.x_range);
        if !report.missing_channels.is_empty() {
            warnings.push(format!(
                "Missing channel(s): {}",
                report.missing_channels.join(", ")
            ));
        }
        self.load.error = (!warnings.is_empty()).then(|| warnings.join("\n"));
        self.load.status = format!(
            "Applied session preset: {} source(s), {} row(s), {} channel(s)",
            self.workspace.source_order.len(),
            report.restored_rows,
            report.restored_channels
        );
        self.session_preset_status = Some(match &self.load.error {
            Some(warnings) => format!("{}\n{warnings}", self.load.status),
            None => self.load.status.clone(),
        });
    }

    fn clear_session_preset(&mut self) {
        self.session_preset = None;
        self.session_preset_status = None;
        self.load.error = None;
        self.load.status = "Cleared session preset".to_owned();
    }

    fn browse_plot_config_path(&mut self) {
        if self.load.is_busy() {
            self.load.status = "Wait for active load jobs before browsing plot config".to_owned();
            return;
        }

        let mut dialog = rfd::FileDialog::new()
            .set_title("Open plot config")
            .add_filter("Plot config", &["pyc.json", "json"]);
        if let Some(directory) = plot_config_dialog_directory(&self.plot_config_path) {
            dialog = dialog.set_directory(directory);
        }

        if let Some(path) = dialog.pick_file() {
            self.plot_config_path = path.display().to_string();
            self.load.error = None;
            self.load.status = "Selected plot config path".to_owned();
        }
    }

    fn save_plot_config_as(&mut self) {
        if let Some(reason) =
            persistence_action_block_reason(&self.workspace, PersistenceAction::PlotConfigSaveAs)
        {
            self.load.status = reason;
            return;
        }
        if self.load.is_busy() {
            self.load.status = "Wait for active load jobs before saving plot config".to_owned();
            return;
        }
        if self.workspace.configured_schema_source().is_none() {
            self.load.status = "Load schema before saving plot config".to_owned();
            return;
        }

        let mut dialog = rfd::FileDialog::new()
            .set_title("Save plot config")
            .add_filter("Plot config", &["pyc.json", "json"])
            .set_file_name("view.pyc.json");
        if let Some(directory) = plot_config_dialog_directory(&self.plot_config_path) {
            dialog = dialog.set_directory(directory);
        }

        if let Some(path) = dialog.save_file() {
            self.plot_config_path = path.display().to_string();
            self.save_plot_config();
        }
    }

    fn save_plot_config(&mut self) {
        if let Some(reason) =
            persistence_action_block_reason(&self.workspace, PersistenceAction::PlotConfigSave)
        {
            self.load.status = reason;
            return;
        }
        if self.load.is_busy() {
            self.load.status = "Wait for active load jobs before saving plot config".to_owned();
            return;
        }
        let Some(source) = self.workspace.configured_schema_source() else {
            self.load.status = "Load schema before saving plot config".to_owned();
            return;
        };

        self.view.ensure_row_state();
        if self.plot_config_path.trim().is_empty() {
            self.plot_config_path = default_plot_config_path(&source.parquet_path);
        }
        let path = PathBuf::from(self.plot_config_path.trim());
        let document = plot_config::capture_plot_config(&self.workspace, &self.view);
        match plot_config::write_plot_config(&path, &document) {
            Ok(()) => {
                self.plot_config_path = path.display().to_string();
                self.plot_config_report = None;
                self.load.error = None;
                self.load.status = format!(
                    "Saved plot config: {} row(s), {} channel(s)",
                    document.rust.rows.len(),
                    document
                        .rust
                        .rows
                        .iter()
                        .map(|row| row.channels.len())
                        .sum::<usize>()
                );
            }
            Err(error) => {
                self.load.status = "Plot config save failed".to_owned();
                self.load.error = Some(error.to_string());
            }
        }
    }

    fn load_plot_config(&mut self, ctx: &egui::Context) {
        self.apply_plot_config(ctx, PlotConfigApplyMode::LoadConfigDataPath);
    }

    fn apply_plot_config_to_current_data(&mut self, ctx: &egui::Context) {
        self.apply_plot_config(ctx, PlotConfigApplyMode::ApplyToCurrentData);
    }

    fn apply_plot_config(&mut self, ctx: &egui::Context, mode: PlotConfigApplyMode) {
        let persistence_action = match mode {
            PlotConfigApplyMode::LoadConfigDataPath => PersistenceAction::PlotConfigLoad,
            PlotConfigApplyMode::ApplyToCurrentData => {
                PersistenceAction::PlotConfigApplyToCurrentData
            }
        };
        if let Some(reason) = persistence_action_block_reason(&self.workspace, persistence_action) {
            self.load.status = reason;
            return;
        }
        if self.load.is_busy() {
            self.load.status = match mode {
                PlotConfigApplyMode::LoadConfigDataPath => {
                    "Wait for active load jobs before loading plot config"
                }
                PlotConfigApplyMode::ApplyToCurrentData => {
                    "Wait for active load jobs before applying plot config"
                }
            }
            .to_owned();
            return;
        }

        let source_count_before_load = self.workspace.source_order.len();
        let config_path = match validate_plot_config_load_path(&self.plot_config_path) {
            Ok(path) => path,
            Err(message) => {
                self.load.status = "Invalid plot config path".to_owned();
                self.load.error = Some(message);
                return;
            }
        };

        let (document, report) = match plot_config::read_plot_config(&config_path) {
            Ok(result) => result,
            Err(error) => {
                self.load.status = "Plot config read failed".to_owned();
                self.load.error = Some(error.to_string());
                return;
            }
        };
        if document.rust.sources.is_some() {
            self.apply_multi_source_plot_config(ctx, document, report, mode, config_path);
            return;
        }
        if mode == PlotConfigApplyMode::ApplyToCurrentData && self.workspace.source_order.len() != 1
        {
            self.load.status =
                "Plot config v1 Apply requires exactly one current source; use Load or a v2 config"
                    .to_owned();
            self.load.error = Some(self.load.status.clone());
            return;
        }
        let config_data_path = document.data_path.clone();

        let (schema, current_data_path) = match mode {
            PlotConfigApplyMode::LoadConfigDataPath => {
                let parquet_path =
                    match validate_parquet_path(&document.data_path.display().to_string()) {
                        Ok(path) => path,
                        Err(error) => {
                            self.load.status = "Plot config data_path is invalid".to_owned();
                            self.load.error = Some(error.message());
                            return;
                        }
                    };

                let schema = match parquet_schema::read_schema_summary(&parquet_path) {
                    Ok(schema) => schema,
                    Err(error) => {
                        self.load.status = "Plot config schema load failed".to_owned();
                        self.load.error = Some(format_schema_load_error(&parquet_path, &error));
                        return;
                    }
                };
                if schema.time_column.is_none() {
                    self.load.status = "Plot config data_path has no time column".to_owned();
                    self.load.error =
                        Some("Time column is required before loading waveform data.".to_owned());
                    return;
                }

                (schema, parquet_path)
            }
            PlotConfigApplyMode::ApplyToCurrentData => {
                if self.workspace.parquet_path.trim().is_empty() {
                    self.load.status = "Current Parquet path is empty".to_owned();
                    self.load.error =
                        Some("Load a Parquet schema before applying a plot config.".to_owned());
                    return;
                }
                let Some(schema) = self.workspace.schema.clone() else {
                    self.load.status =
                        "Load schema before applying plot config to current data".to_owned();
                    self.load.error = Some(
                        "Current data needs a loaded schema before view settings can be applied."
                            .to_owned(),
                    );
                    return;
                };
                if schema.time_column.is_none() {
                    self.load.status = "Current data has no time column".to_owned();
                    self.load.error =
                        Some("Time column is required before loading waveform data.".to_owned());
                    return;
                }

                (schema, PathBuf::from(self.workspace.parquet_path.trim()))
            }
        };

        let source_id = match mode {
            PlotConfigApplyMode::LoadConfigDataPath => DEFAULT_SOURCE_ID,
            PlotConfigApplyMode::ApplyToCurrentData => self
                .workspace
                .selected_source_id()
                .expect("single-source Plot config apply requires a selected source"),
        };
        let mut plan = plot_config::build_apply_plan(document, &schema, source_id, report);
        if mode == PlotConfigApplyMode::ApplyToCurrentData && config_data_path != current_data_path
        {
            plan.report.ignored_fields.push(format!(
                "data_path={} (using current data {})",
                config_data_path.display(),
                current_data_path.display()
            ));
        }

        let plot_config::PlotConfigLoadPlan {
            data_path: plan_data_path,
            theme,
            visual_mode,
            scroll_zoom,
            rows,
            selected_channel,
            channels_to_load,
            report,
        } = plan;

        match mode {
            PlotConfigApplyMode::LoadConfigDataPath => {
                self.workspace.replace_with_plot_config_v1_source(
                    plan_data_path.display().to_string(),
                    schema,
                );
                self.view.x_range = None;
            }
            PlotConfigApplyMode::ApplyToCurrentData => {
                self.workspace.clear_draw_caches_for_source(source_id);
            }
        }

        self.view.rows = rows;
        self.view.next_row_id = self.view.rows.iter().map(|row| row.id).max().unwrap_or(0) + 1;
        self.view.selected_row_id = self.view.rows.first().map(|row| row.id);
        self.view.selected_channel = selected_channel;
        self.view.hover_x = None;
        self.view.theme = theme;
        self.view.visual_mode = visual_mode;
        self.view.scroll_zoom = scroll_zoom;
        self.view.ensure_row_state();
        self.analysis.clear_all();
        apply_view_visuals(ctx, &self.view);

        let report_summary = report.summary();
        let report_details = report.details();
        self.plot_config_report = Some(report.clone());
        self.plot_config_path = config_path.display().to_string();
        self.load.error = report_details.clone();
        self.load.progress = None;

        for key in channels_to_load {
            if let Some(row_id) = self.first_row_id_containing_channel(&key) {
                self.request_channel_load(ctx, &key, row_id, false);
            }
        }

        self.load.error = report_details;
        self.load.refresh_progress(&self.workspace);
        let status_prefix = match mode {
            PlotConfigApplyMode::LoadConfigDataPath if source_count_before_load > 1 => format!(
                "Loaded plot config; replaced {source_count_before_load}-source workspace with Source #1"
            ),
            PlotConfigApplyMode::LoadConfigDataPath => {
                "Loaded plot config into Source #1".to_owned()
            }
            PlotConfigApplyMode::ApplyToCurrentData => {
                "Applied plot config to current data".to_owned()
            }
        };
        self.load.status = if self.load.pending_jobs > 0 {
            format!(
                "{status_prefix}: {report_summary}; loading {} channel(s)",
                self.load.pending_jobs
            )
        } else {
            format!("{status_prefix}: {report_summary}")
        };
    }

    fn apply_multi_source_plot_config(
        &mut self,
        ctx: &egui::Context,
        document: plot_config::PlotConfigDocument,
        mut report: plot_config::PlotConfigApplyReport,
        mode: PlotConfigApplyMode,
        config_path: PathBuf,
    ) {
        let sources = document.rust.sources.as_ref().expect("v2 sources");
        let mut source_ids = BTreeMap::new();
        let mut replacement = WorkspaceDataState {
            sources: BTreeMap::new(),
            source_order: Vec::new(),
            selected_source_id: None,
            next_source_id: 1,
            recent_parquet_paths: self.workspace.recent_parquet_paths.clone(),
            tile_batch_wall: Duration::ZERO,
            tile_batch_workers: 0,
        };
        for saved in sources {
            match mode {
                PlotConfigApplyMode::LoadConfigDataPath => {
                    let schema = validate_parquet_path(&saved.data_path.display().to_string())
                        .map_err(|error| error.message())
                        .and_then(|path| {
                            parquet_schema::read_schema_summary(&path)
                                .map_err(|error| error.to_string())
                        })
                        .and_then(|schema| {
                            if schema.time_column.is_some() {
                                Ok(schema)
                            } else {
                                Err("Time column is required".to_owned())
                            }
                        });
                    match schema {
                        Ok(schema) => {
                            let id = replacement.add_source(saved.data_path.display().to_string());
                            let source = replacement.source_mut(id).expect("new source");
                            source.schema = Some(schema);
                            if !saved.label.is_empty() {
                                source.label = saved.label.clone();
                            }
                            source.time_transform.offset_seconds = saved.time_offset_seconds;
                            source_ids.insert(saved.key.clone(), id);
                        }
                        Err(error) => report.ignored_fields.push(format!(
                            "Unavailable source {} ({}): {error}",
                            saved.key,
                            saved.data_path.display()
                        )),
                    }
                }
                PlotConfigApplyMode::ApplyToCurrentData => {
                    let normalize =
                        |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                    let matches = self
                        .workspace
                        .source_order
                        .iter()
                        .copied()
                        .filter(|id| {
                            self.workspace.source(*id).is_some_and(|source| {
                                source.schema.is_some()
                                    && normalize(Path::new(&source.parquet_path))
                                        == normalize(&saved.data_path)
                            })
                        })
                        .collect::<Vec<_>>();
                    let [id] = matches.as_slice() else {
                        self.load.status = "Plot config apply failed".to_owned();
                        self.load.error = Some(format!(
                            "Source {} must match exactly one current data path: {}. Use Load to restore the saved sources.",
                            saved.key,
                            saved.data_path.display()
                        ));
                        return;
                    };
                    if source_ids.values().any(|existing| existing == id) {
                        self.load.status = "Plot config apply failed".to_owned();
                        self.load.error = Some("Multiple saved sources match the same current source. Use Load to restore them separately.".to_owned());
                        return;
                    }
                    source_ids.insert(saved.key.clone(), *id);
                    if self
                        .workspace
                        .source(*id)
                        .unwrap()
                        .time_transform
                        .offset_seconds
                        != saved.time_offset_seconds
                    {
                        report.ignored_fields.push(format!(
                            "{}.time_offset_seconds (keeping current offset)",
                            saved.key
                        ));
                    }
                }
            }
        }
        if source_ids.is_empty() {
            self.load.status = "Plot config load failed; current workspace preserved".to_owned();
            self.load.error = report
                .details()
                .or_else(|| Some("No usable sources".to_owned()));
            return;
        }
        let plan = plot_config::build_multi_source_apply_plan(
            document,
            if mode == PlotConfigApplyMode::LoadConfigDataPath {
                &replacement
            } else {
                &self.workspace
            },
            &source_ids,
            report,
        );
        if mode == PlotConfigApplyMode::LoadConfigDataPath {
            self.workspace = replacement;
            self.view.x_range = None;
        } else {
            for id in self.workspace.source_order.clone() {
                self.workspace.clear_draw_caches_for_source(id);
            }
        }
        self.view.rows = plan.rows;
        self.view.next_row_id = self.view.rows.iter().map(|row| row.id).max().unwrap_or(0) + 1;
        self.view.selected_row_id = self.view.rows.first().map(|row| row.id);
        self.view.selected_channel = plan.selected_channel;
        self.view.hover_x = None;
        self.view.theme = plan.theme;
        self.view.visual_mode = plan.visual_mode;
        self.view.scroll_zoom = plan.scroll_zoom;
        self.view.ensure_row_state();
        self.analysis.clear_all();
        apply_view_visuals(ctx, &self.view);
        self.plot_config_path = config_path.display().to_string();
        self.load.progress = None;
        for key in plan.channels_to_load {
            if let Some(row_id) = self.first_row_id_containing_channel(&key) {
                self.request_channel_load(ctx, &key, row_id, false);
            }
        }
        self.load.error = plan.report.details();
        self.load.refresh_progress(&self.workspace);
        let action = if mode == PlotConfigApplyMode::LoadConfigDataPath {
            "Loaded"
        } else {
            "Applied"
        };
        self.load.status = format!(
            "{action} plot config v2: {} source(s); {}",
            source_ids.len(),
            plan.report.summary()
        );
        self.fit_plot_config_after_load =
            mode == PlotConfigApplyMode::LoadConfigDataPath && self.load.is_busy();
        self.plot_config_report = Some(plan.report);
    }

    fn first_row_id_containing_channel(&self, key: &ChannelKey) -> Option<u64> {
        self.view
            .rows
            .iter()
            .find(|row| row.channels.iter().any(|channel| channel.key == *key))
            .map(|row| row.id)
    }

    fn add_cached_channel_to_selected_row(&mut self, key: &ChannelKey) {
        let channel_name = channel_label_or_fallback(&self.workspace, key);
        let Some(_channel) = self.workspace.channel(key) else {
            self.load.status = format!("Channel is not cached: {channel_name}");
            return;
        };
        let Some(target_row_id) = self.view.selected_row_id_or_first() else {
            self.load.status = "Select a row before adding cached channel".to_owned();
            return;
        };
        let target_row_label = self.view.row_display_name(target_row_id);
        let (row_added, row_id) = self
            .view
            .add_channel_to_row(target_row_id, key)
            .unwrap_or((false, target_row_id));
        self.view.selected_channel = Some(key.clone());
        if let Some(source) = self.workspace.source_mut(key.source_id) {
            source.loaded_channels.clear_envelope_cache();
        }
        self.load.error = None;
        self.load.status = if row_added {
            format!("Added cached channel: {channel_name} -> {target_row_label} (id {row_id})")
        } else {
            format!(
                "Cached channel already exists in {target_row_label}: {channel_name} (id {row_id})"
            )
        };
    }

    fn unload_cached_channel(&mut self, key: &ChannelKey) {
        let channel_name = channel_label_or_fallback(&self.workspace, key);
        if self.load.is_channel_loading(key) {
            self.load.status = format!("Wait for active load before unloading: {channel_name}");
            return;
        }

        let removed = self.workspace.unload_channel(key);
        if removed {
            self.analysis.fft.invalidate_result();
            self.load.error = None;
            self.load.status = format!(
                "Unloaded cached channel data: {channel_name}; row references remain as unloaded"
            );
        } else {
            self.load.status = format!("Channel is not cached: {channel_name}");
        }
    }

    fn unload_all_cached_channels(&mut self, source_id: SourceId) {
        if self.load.is_source_loading(source_id) {
            self.load.status = "Wait for active load jobs before unloading all channels".to_owned();
            return;
        }

        let (channel_count, raw_memory) = self
            .workspace
            .source(source_id)
            .map(|source| {
                (
                    source.loaded_channels.raw_by_channel.len(),
                    source.loaded_channels.raw_memory_bytes(),
                )
            })
            .unwrap_or_default();
        self.workspace.unload_all_channels_for_source(source_id);
        let aggregate = self.workspace.cache_stats();
        self.analysis.fft.invalidate_result();
        self.load.error = None;
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "source".to_owned());
        self.load.status = format!(
            "Unloaded {source_label} cache: {channel_count} channel(s), {:.1} MiB; workspace cache {} ch, {:.1} MiB raw; row references remain as unloaded",
            bytes_to_mib(raw_memory),
            aggregate.loaded_channel_count,
            bytes_to_mib(aggregate.total_raw_memory_bytes())
        );
    }

    fn clear_draw_caches(&mut self, source_id: SourceId) {
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "source".to_owned());
        if self.load.is_source_loading(source_id) {
            self.load.status =
                format!("Wait for {source_label} load jobs before clearing draw caches");
            return;
        }
        let (envelope_count, line_tile_count, step_edge_count, hint_count) = self
            .workspace
            .source(source_id)
            .map(|source| {
                (
                    source.loaded_channels.envelope_cache.len(),
                    source.loaded_channels.line_tile_cache.len(),
                    source.loaded_channels.step_edge_cache.len(),
                    source.loaded_channels.step_fallback_hints.len(),
                )
            })
            .unwrap_or_default();
        if self.workspace.clear_draw_caches_for_source(source_id) {
            self.load.error = None;
            self.load.status = format!(
                "Cleared {source_label} draw caches: {envelope_count} envelope(s), {line_tile_count} line tile cache(s), {step_edge_count} step edge cache(s), {hint_count} hint(s)"
            );
        } else {
            self.load.status = "No draw caches to clear".to_owned();
        }
    }

    fn handle_cache_action(&mut self, action: CacheControlAction) {
        match action {
            CacheControlAction::AddToSelectedRow(channel_path) => {
                self.add_cached_channel_to_selected_row(&channel_path);
            }
            CacheControlAction::UnloadChannel(channel_path) => {
                self.unload_cached_channel(&channel_path);
            }
            CacheControlAction::UnloadAllChannels(source_id) => {
                self.unload_all_cached_channels(source_id);
            }
            CacheControlAction::ClearDrawCaches(source_id) => {
                self.clear_draw_caches(source_id);
            }
        }
    }

    fn browse_parquet_path_for_source(&mut self, source_id: SourceId) {
        if self.load.is_source_loading(source_id) {
            self.load.status = "Wait for this source's active load jobs before browsing".to_owned();
            return;
        }

        let mut dialog = rfd::FileDialog::new()
            .set_title("Open Parquet file")
            .add_filter("Parquet", &["parquet"]);
        let source_path = self
            .workspace
            .source(source_id)
            .map(|source| source.parquet_path.as_str())
            .unwrap_or_default();
        if let Some(directory) = parquet_dialog_directory(source_path) {
            dialog = dialog.set_directory(directory);
        }

        if let Some(path) = dialog.pick_file() {
            self.select_parquet_path_for_source(source_id, path.display().to_string());
        }
    }

    fn load_schema_for_source(&mut self, source_id: SourceId) {
        let source_label = source_presentation(&self.workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "source".to_owned());
        if self.load.is_source_loading(source_id) {
            self.load.status = format!("Wait for {source_label} load jobs before loading schema");
            return;
        }

        let Some(source_path) = self
            .workspace
            .source(source_id)
            .map(|source| source.parquet_path.clone())
        else {
            self.load.status = "The requested data source no longer exists".to_owned();
            self.load.error = Some(self.load.status.clone());
            return;
        };
        let parquet_path = match validate_parquet_path(&source_path) {
            Ok(path) => path,
            Err(error) => {
                self.clear_source_after_schema_failure(source_id);
                self.load.status = format!("{source_label}: {}", error.status());
                self.load.error = Some(format!("{source_label}: {}", error.message()));
                return;
            }
        };

        match parquet_schema::read_schema_summary(&parquet_path) {
            Ok(summary) => {
                let canonical_path = parquet_path.display().to_string();
                self.cleanup_source_references(source_id);
                if !self
                    .workspace
                    .reload_source_parquet_path(source_id, canonical_path.clone())
                {
                    self.load.status = "The requested data source no longer exists".to_owned();
                    self.load.error = Some(self.load.status.clone());
                    return;
                }
                let source = self
                    .workspace
                    .source_mut(source_id)
                    .expect("source existence checked before applying schema");
                source.schema = Some(summary.clone());
                remember_recent_parquet_path(
                    &mut self.workspace.recent_parquet_paths,
                    canonical_path,
                );
                self.reconcile_view_range_to_workspace();
                let time_status = if summary.time_column.is_some() {
                    "time column detected"
                } else {
                    "time column missing"
                };
                self.view.update_for_source_schema(source_id, &summary);
                self.plot_config_report = None;
                self.load.error = None;
                self.load.refresh_progress(&self.workspace);
                let source_label = source_presentation(&self.workspace, source_id)
                    .map(|source| source.short_label)
                    .unwrap_or(source_label);
                self.load.status = format!(
                    "{source_label}: loaded schema, {} rows, {} channels, {time_status}",
                    summary.row_count,
                    summary.channels.len()
                );
            }
            Err(error) => {
                self.clear_source_after_schema_failure(source_id);
                self.load.status = format!("{source_label}: schema load failed");
                self.load.error = Some(format!(
                    "{source_label}: {}",
                    format_schema_load_error(&parquet_path, &error)
                ));
            }
        }
    }

    fn load_selected_channel(&mut self, ctx: &egui::Context) {
        let Some(selected_channel) = self.view.selected_channel.clone() else {
            self.load.status = "Select a channel before loading waveform data".to_owned();
            return;
        };

        let Some(target_row_id) = self.view.selected_row_id_or_first() else {
            self.load.status = "Select a row before loading waveform data".to_owned();
            return;
        };
        self.request_channel_load(ctx, &selected_channel, target_row_id, true);
    }

    fn add_matching_channels(&mut self, ctx: &egui::Context, selected_key: &ChannelKey) {
        let Some(target_row_id) = self.view.selected_row_id_or_first() else {
            self.load.status = "Select a row before adding matching channels".to_owned();
            return;
        };
        let outcomes = matching_channel_outcomes(&self.workspace, selected_key);
        let matched = outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                MatchingChannelOutcome::Match(key) => Some(key.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let existing = matched
            .iter()
            .filter(|key| {
                self.view.rows.iter().any(|row| {
                    row.id == target_row_id
                        && row.channels.iter().any(|channel| channel.key == **key)
                })
            })
            .count();
        let skipped = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome,
                    MatchingChannelOutcome::MissingSchema(_) | MatchingChannelOutcome::NoMatch(_)
                )
            })
            .count();
        let ambiguous = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, MatchingChannelOutcome::Ambiguous(_)))
            .count();

        for key in &matched {
            self.request_channel_load(ctx, key, target_row_id, true);
        }

        let requested = matched.len().saturating_sub(existing);
        self.load.status = format!(
            "Matching channels -> {}: {requested} load/add request(s), {existing} existing, {skipped} skipped, {ambiguous} ambiguous",
            self.view.row_display_name(target_row_id)
        );
    }

    fn request_channel_load(
        &mut self,
        ctx: &egui::Context,
        key: &ChannelKey,
        target_row_id: u64,
        add_to_row: bool,
    ) {
        let Some((summary, path, source_generation, time_was_cached, channel_was_cached)) =
            self.workspace.source(key.source_id).and_then(|source| {
                source.schema.clone().map(|summary| {
                    (
                        summary,
                        source.parquet_path.clone(),
                        source.generation,
                        source.shared_time.is_some(),
                        source.loaded_channels.has_channel(&key.channel_path),
                    )
                })
            })
        else {
            self.load.status = if self.workspace.source(key.source_id).is_some() {
                "Load schema before loading waveform data".to_owned()
            } else {
                "The selected data source no longer exists".to_owned()
            };
            return;
        };
        if summary.time_column.is_none() {
            self.load.status = "Time column is required before loading waveform data".to_owned();
            return;
        }
        let channel_path = key.channel_path.as_str();
        let channel_name = channel_label_or_fallback(&self.workspace, key);
        let target_row_label = self.view.row_display_name(target_row_id);
        let (row_added, row_id) = if add_to_row {
            self.view
                .add_channel_to_row(target_row_id, key)
                .unwrap_or((false, target_row_id))
        } else {
            (false, target_row_id)
        };
        self.view.selected_channel = Some(key.clone());

        if time_was_cached && channel_was_cached {
            self.finish_cached_channel_add(
                key,
                &target_row_label,
                row_added,
                row_id,
                add_to_row,
                true,
            );
            return;
        }

        if self.load.is_channel_loading(key) {
            let row_note = if row_added {
                format!("queued display in {target_row_label}")
            } else if add_to_row {
                format!("already queued in {target_row_label}")
            } else {
                format!("restore load already pending for {target_row_label}")
            };
            self.load.status = format!("Already loading: {channel_name}, {row_note}");
            return;
        }

        let needs_time = !time_was_cached;
        let needs_channel = !channel_was_cached;
        let job_id = self.next_load_job_id;
        self.next_load_job_id = self.next_load_job_id.saturating_add(1);
        self.load.active_jobs.insert(
            job_id,
            ActiveLoadJob {
                source_id: key.source_id,
                source_generation,
                channel_key: key.clone(),
                target_row_id,
                target_row_label: target_row_label.clone(),
                add_to_row,
                time_was_cached,
                channel_was_cached,
                started: std::time::Instant::now(),
            },
        );
        self.load.error = None;
        self.load.status = format!("Loading: {channel_name} -> {target_row_label}");
        self.load.refresh_progress(&self.workspace);

        spawn_load_channel_job(
            self.load_result_tx.clone(),
            ctx.clone(),
            job_id,
            key.source_id,
            source_generation,
            path,
            summary,
            channel_path.to_owned(),
            needs_time,
            needs_channel,
        );
    }

    fn finish_cached_channel_add(
        &mut self,
        key: &ChannelKey,
        target_row_label: &str,
        row_added: bool,
        row_id: u64,
        add_to_row: bool,
        time_was_cached: bool,
    ) {
        let channel_name = channel_label_or_fallback(&self.workspace, key);
        let Some(source) = self.workspace.source(key.source_id) else {
            self.load.status = "The requested data source no longer exists".to_owned();
            return;
        };
        let Some(channel) = source.loaded_channels.channel(&key.channel_path) else {
            self.load.status = format!("Loaded channel is missing from cache: {channel_name}");
            return;
        };
        let channel_sample_count = channel.sample_count();
        let channel_memory = channel.memory_bytes();
        if self.view.x_range.is_none() {
            self.view.x_range = self.workspace.display_time_range();
        }
        self.load.error = None;
        let row_note = if !add_to_row {
            format!("restored in {target_row_label}")
        } else if row_added {
            format!("added to {target_row_label}")
        } else {
            format!("already in {target_row_label}")
        };
        let time_note = if time_was_cached {
            "time cached"
        } else {
            "time loaded"
        };
        let aggregate = self.workspace.cache_stats();
        self.load.status = format!(
            "reused cached: {channel_name} ({channel_sample_count} samples, {:.1} MiB), {row_note} (id {row_id}); {time_note}; cache {} ch, total {:.1} MiB",
            bytes_to_mib(channel_memory),
            aggregate.loaded_channel_count,
            bytes_to_mib(aggregate.total_raw_memory_bytes())
        );
    }

    fn drain_load_results(&mut self) {
        while let Ok(result) = self.load_result_rx.try_recv() {
            self.apply_load_result(result);
        }
        if self.fit_plot_config_after_load && !self.load.is_busy() {
            self.view.x_range = self.workspace.display_time_range();
            self.workspace.clear_envelope_caches();
            self.fit_plot_config_after_load = false;
        }
        if !self.load.is_busy() {
            if let Some(range) = self.pending_session_x_range.take() {
                self.view.x_range = range;
                self.workspace.clear_envelope_caches();
            }
        }
    }

    fn start_fft_jobs(&mut self, ctx: &egui::Context) {
        if self.analysis.fft.is_running() {
            return;
        }
        self.analysis.fft.invalidate_result();
        let batch = match fft_analysis::build_fft_job_inputs(
            &self.analysis.fft,
            self.analysis.cursor_a,
            self.analysis.cursor_b,
            &self.workspace,
            &self.view,
        ) {
            Ok(batch) => batch,
            Err(error) => {
                self.analysis.fft.error = Some(error);
                return;
            }
        };
        self.analysis.fft.source_errors = batch.errors;
        for input in batch.inputs {
            let job_id = self.next_fft_job_id;
            self.next_fft_job_id = self.next_fft_job_id.saturating_add(1);
            self.analysis.fft.register_job(job_id, &input);
            fft_analysis::spawn_fft_job(self.fft_result_tx.clone(), ctx.clone(), job_id, input);
        }
    }

    fn drain_fft_results(&mut self) {
        while let Ok(result) = self.fft_result_rx.try_recv() {
            self.apply_fft_result(result);
        }
    }

    fn apply_fft_result(&mut self, result: fft_analysis::FftJobResult) {
        self.analysis.fft.apply_job_result(result, &self.workspace);
    }

    fn handle_fft_action(&mut self, action: fft_analysis::FftUiAction, ctx: &egui::Context) {
        match action {
            fft_analysis::FftUiAction::Run => self.start_fft_jobs(ctx),
            fft_analysis::FftUiAction::Cancel => self.analysis.fft.cancel_jobs(),
        }
    }

    fn apply_load_result(&mut self, result: LoadJobResult) {
        let Some(job) = self.load.active_jobs.remove(&result.job_id) else {
            self.load.refresh_progress(&self.workspace);
            return;
        };
        self.load.refresh_progress(&self.workspace);
        let channel_label = channel_label_or_fallback(&self.workspace, &job.channel_key);

        if !load_result_matches_active_job(&result, &job) {
            self.load.status = format!("Ignored mismatched load result for {channel_label}");
            return;
        }
        let Some(source) = self.workspace.source(job.source_id) else {
            self.load.status =
                format!("Ignored stale load result for removed source: {channel_label}");
            return;
        };
        if source.generation != job.source_generation {
            self.load.status = format!(
                "Ignored stale load result for {}: source generation {} is now {}",
                channel_label, job.source_generation, source.generation
            );
            return;
        }

        match result.result {
            Ok(loaded) => self.apply_loaded_channel(job, loaded),
            Err(error) => {
                if self.pending_session_x_range.is_some() {
                    if let Some(status) = &mut self.session_preset_status {
                        status
                            .push_str(&format!("\nWaveform load failed: {channel_label}: {error}"));
                    }
                }
                self.load.error = Some(error.clone());
                self.load.status = format!("Waveform load failed: {channel_label}: {error}");
            }
        }
    }

    fn apply_loaded_channel(&mut self, job: ActiveLoadJob, loaded: LoadedChannelData) {
        let channel_label = channel_label_or_fallback(&self.workspace, &job.channel_key);
        let Some(source) = self.workspace.source(job.source_id) else {
            self.load.status =
                format!("Time data source is no longer available for {channel_label}");
            self.load.error = Some(self.load.status.clone());
            return;
        };
        if source.generation != job.source_generation {
            self.load.status =
                format!("Ignored stale load result after source reset: {channel_label}");
            return;
        }

        let existing_time_sample_count = source
            .shared_time
            .as_ref()
            .map(parquet_waveform::TimeData::sample_count);
        let loaded_time_sample_count = loaded
            .time
            .as_ref()
            .map(parquet_waveform::TimeData::sample_count);
        if let (Some(existing), Some(incoming)) =
            (existing_time_sample_count, loaded_time_sample_count)
            && existing != incoming
        {
            self.load.status =
                format!("Time load failed: sample count changed: {existing} vs {incoming}");
            self.load.error = Some(self.load.status.clone());
            return;
        }
        let Some(time_sample_count) = loaded_time_sample_count.or(existing_time_sample_count)
        else {
            self.load.status = "Time data is not available".to_owned();
            self.load.error = Some(self.load.status.clone());
            return;
        };

        if let Some(channel) = loaded.channel.as_ref()
            && channel.channel_path != job.channel_key.channel_path
        {
            self.load.status = format!(
                "Waveform load failed: expected channel {}, got {}",
                job.channel_key.channel_path, channel.channel_path
            );
            self.load.error = Some(self.load.status.clone());
            return;
        }
        if let Some(channel) = loaded.channel.as_ref()
            && channel.sample_count() != time_sample_count
        {
            self.load.status = format!(
                "Waveform load failed: time/value length mismatch: {} vs {}",
                time_sample_count,
                channel.sample_count()
            );
            self.load.error = Some(self.load.status.clone());
            return;
        }

        let source = self
            .workspace
            .source_mut(job.source_id)
            .expect("source existence checked before applying load result");
        if let Some(time) = loaded.time {
            source.shared_time = Some(time);
        }
        if let Some(channel) = loaded.channel
            && !source.loaded_channels.has_channel(&channel.channel_path)
        {
            source.loaded_channels.insert_channel(channel);
        }

        let Some((time_read_sec, channel)) =
            self.workspace.source(job.source_id).and_then(|source| {
                let shared_time = source.shared_time.as_ref()?;
                let channel = source
                    .loaded_channels
                    .channel(&job.channel_key.channel_path)?;
                Some((shared_time.elapsed.as_secs_f64(), channel))
            })
        else {
            self.load.status =
                format!("Loaded channel is missing from source cache: {channel_label}");
            self.load.error = Some(self.load.status.clone());
            return;
        };

        let channel_name = channel_label_or_fallback(&self.workspace, &job.channel_key);
        let channel_read_sec = channel.elapsed.as_secs_f64();
        let channel_sample_count = channel.sample_count();
        let channel_memory = channel.memory_bytes();
        let (row_added, row_id) = if job.add_to_row {
            self.view
                .add_channel_to_row(job.target_row_id, &job.channel_key)
                .unwrap_or((false, job.target_row_id))
        } else {
            (false, job.target_row_id)
        };
        if self.view.x_range.is_none() {
            self.view.x_range = self.workspace.display_time_range();
        }

        self.load.error = None;
        let cache_note = if job.channel_was_cached {
            "reused cached"
        } else {
            "loaded"
        };
        let row_note = if !job.add_to_row {
            format!("restored in {}", job.target_row_label)
        } else if row_added {
            format!("added to {}", job.target_row_label)
        } else {
            format!("already in {}", job.target_row_label)
        };
        let time_note = if job.time_was_cached {
            "time cached".to_owned()
        } else {
            format!("time loaded {:.3}s", time_read_sec)
        };
        let aggregate = self.workspace.cache_stats();
        let elapsed_sec = job.started.elapsed().as_secs_f64();
        self.load.status = format!(
            "{cache_note}: {channel_name} ({channel_sample_count} samples, {:.1} MiB, read {:.3}s), {row_note} (id {row_id}); {time_note}; workspace cache {} ch, total {:.1} MiB; background {:.3}s",
            bytes_to_mib(channel_memory),
            channel_read_sec,
            aggregate.loaded_channel_count,
            bytes_to_mib(aggregate.total_raw_memory_bytes()),
            elapsed_sec
        );
    }

    fn reset_view_range(&mut self) {
        self.view.x_range = self.workspace.display_time_range();
        self.workspace.clear_envelope_caches();
    }

    fn fit_overlap_view_range(&mut self) {
        if let Some(overlap) = self.workspace.display_overlap_time_range() {
            self.view.x_range = Some(overlap);
            self.workspace.clear_envelope_caches();
        }
    }

    fn should_use_interaction_preview(&self) -> bool {
        if !self.view.large_preview_enabled || !self.interaction.preview_active() {
            return false;
        }

        let Some(full_range) = self.workspace.display_time_range() else {
            return false;
        };
        let min_span = self
            .workspace
            .display_min_view_span()
            .unwrap_or_else(|| min_view_span(full_range, 1));
        let view_range = clamp_view_range(
            self.view.x_range.unwrap_or(full_range),
            full_range,
            min_span,
        );
        let visible_source_sample_count = self
            .workspace
            .source_order
            .iter()
            .filter_map(|source_id| {
                let source = self.workspace.source(*source_id)?;
                let shared_time = source.shared_time.as_ref()?;
                let source_display_range = data_source_display_time_range(source)?;
                let visible_display_range = range_intersection(view_range, source_display_range)?;
                let raw_view_range = source.time_transform.raw_range(visible_display_range);
                visible_sample_range(
                    &shared_time.time,
                    shared_time.sample_count(),
                    raw_view_range,
                )
                .map(|range| range.source_sample_count)
            })
            .fold(0usize, usize::saturating_add);
        let (line_channels, step_channels) = self.visible_loaded_draw_mode_counts();

        preview_needed_for_workload(
            self.workspace.total_time_sample_count(),
            visible_source_sample_count,
            line_channels,
            step_channels,
        )
    }

    fn visible_loaded_draw_mode_counts(&self) -> (usize, usize) {
        self.view
            .rows
            .iter()
            .flat_map(|row| row.channels.iter())
            .filter(|channel| channel.visible && self.workspace.has_channel(&channel.key))
            .fold((0usize, 0usize), |(line, step), channel| {
                match channel.draw_mode {
                    DrawMode::Line => (line + 1, step),
                    DrawMode::Step => (line, step + 1),
                }
            })
    }

    fn visible_row_traces(
        &mut self,
        requested_bucket_count: usize,
        dark_mode: bool,
        preview: bool,
    ) -> Vec<VisibleRowTrace> {
        let rows = self.view.rows.clone();
        let loading_channel_keys = self.load.loading_channel_keys();
        self.workspace.tile_batch_wall = Duration::ZERO;
        self.workspace.tile_batch_workers = 0;
        for source in self.workspace.sources.values_mut() {
            source.loaded_channels.begin_envelope_frame();
        }
        if rows.is_empty() {
            return Vec::new();
        }

        let Some(full_range) = self.workspace.display_time_range() else {
            // Rendering without loaded time must not discard a restored preset range.
            self.workspace.clear_envelope_caches();
            return rows
                .into_iter()
                .enumerate()
                .map(|(row_index, row)| {
                    let (loading_channel_count, unloaded_channel_count) =
                        row_channel_load_counts(&row, &self.workspace, &loading_channel_keys);
                    VisibleRowTrace {
                        row_id: row.id,
                        row_index,
                        row_channel_count: row.channels.len(),
                        loading_channel_count,
                        unloaded_channel_count,
                        out_of_range_channel_count: 0,
                        y_range: row.y_range,
                        traces: Vec::new(),
                    }
                })
                .collect();
        };

        let min_span = self
            .workspace
            .display_min_view_span()
            .unwrap_or_else(|| min_view_span(full_range, 1));
        let view_range = clamp_view_range(
            self.view.x_range.unwrap_or(full_range),
            full_range,
            min_span,
        );
        if self.view.x_range != Some(view_range) {
            self.view.x_range = Some(view_range);
            self.workspace.clear_envelope_caches();
        }

        let source_contexts = self
            .workspace
            .source_order
            .iter()
            .filter_map(|source_id| {
                let source = self.workspace.source(*source_id)?;
                let shared_time = source.shared_time.as_ref()?;
                let raw_full_range = shared_time.time_range()?;
                let display_full_range =
                    normalized_range(source.time_transform.display_range(raw_full_range))?;
                let display_view_range = range_intersection(view_range, display_full_range)?;
                Some((
                    *source_id,
                    SourceRenderContext {
                        time_transform: source.time_transform,
                        raw_view_range: source.time_transform.raw_range(display_view_range),
                        raw_full_range,
                    },
                ))
            })
            .collect::<BTreeMap<_, _>>();
        for (source_id, context) in &source_contexts {
            let Some(source) = self.workspace.source_mut(*source_id) else {
                continue;
            };
            let DataSourceState {
                shared_time,
                loaded_channels,
                ..
            } = source;
            let Some(shared_time) = shared_time.as_ref() else {
                continue;
            };
            loaded_channels.prepare_envelope_context(
                &shared_time.time,
                shared_time.sample_count(),
                context.raw_view_range,
                context.raw_full_range,
                requested_bucket_count,
            );
        }

        self.workspace
            .prepare_visible_line_tiles(&rows, &source_contexts);

        let mut visible_rows = rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                let (loading_channel_count, unloaded_channel_count) =
                    row_channel_load_counts(row, &self.workspace, &loading_channel_keys);
                VisibleRowTrace {
                    row_id: row.id,
                    row_index,
                    row_channel_count: row.channels.len(),
                    loading_channel_count,
                    unloaded_channel_count,
                    out_of_range_channel_count: 0,
                    y_range: row.y_range,
                    traces: Vec::with_capacity(row.channels.len()),
                }
            })
            .collect::<Vec<_>>();
        let mut built_count = 0usize;
        let mut exact_step_count = 0usize;
        let mut edge_step_count = 0usize;
        let mut dense_step_count = 0usize;
        let hover_x = self.view.hover_x;
        let presentations = rows
            .iter()
            .flat_map(|row| row.channels.iter())
            .filter_map(|channel| {
                channel_presentation(&self.workspace, &channel.key)
                    .map(|presentation| (channel.key.clone(), presentation))
            })
            .collect::<BTreeMap<_, _>>();

        for (row_index, row) in rows.into_iter().enumerate() {
            for row_channel in row.channels {
                if !row_channel.visible {
                    continue;
                }
                let Some(context) = source_contexts.get(&row_channel.key.source_id).copied() else {
                    if self
                        .workspace
                        .source(row_channel.key.source_id)
                        .is_some_and(|source| {
                            source.shared_time.is_some()
                                && source
                                    .loaded_channels
                                    .has_channel(&row_channel.key.channel_path)
                        })
                    {
                        visible_rows[row_index].out_of_range_channel_count += 1;
                    }
                    continue;
                };
                let Some(source) = self.workspace.source_mut(row_channel.key.source_id) else {
                    continue;
                };
                let DataSourceState {
                    shared_time,
                    loaded_channels,
                    ..
                } = source;
                let Some(shared_time) = shared_time.as_ref() else {
                    continue;
                };
                let time_values = &shared_time.time;
                let Some((channel_path, sample_count)) = loaded_channels
                    .channel(&row_channel.key.channel_path)
                    .map(|channel| (channel.channel_path.clone(), channel.sample_count()))
                else {
                    continue;
                };

                let data = match row_channel.draw_mode {
                    DrawMode::Line => {
                        let Some((envelope, was_built)) = loaded_channels.ensure_envelope(
                            &channel_path,
                            time_values,
                            context.raw_view_range,
                            requested_bucket_count,
                            true,
                        ) else {
                            continue;
                        };
                        if was_built {
                            built_count += 1;
                        }
                        VisibleTraceData::Envelope(envelope)
                    }
                    DrawMode::Step => {
                        let sample_range =
                            visible_sample_range(time_values, sample_count, context.raw_view_range);
                        let use_preview_envelope = preview
                            && sample_range.is_some_and(|range| {
                                range.draw_sample_count > MAX_EXACT_STEP_SAMPLES
                            });
                        let use_hint_fallback = sample_range.is_some_and(|range| {
                            loaded_channels.should_skip_step_change_points(
                                &channel_path,
                                context.raw_view_range,
                                range.source_sample_count,
                            )
                        });

                        if use_preview_envelope || use_hint_fallback {
                            dense_step_count += 1;
                            let Some((envelope, was_built)) = loaded_channels.ensure_envelope(
                                &channel_path,
                                time_values,
                                context.raw_view_range,
                                requested_bucket_count,
                                true,
                            ) else {
                                continue;
                            };
                            if was_built {
                                built_count += 1;
                            }
                            VisibleTraceData::DenseStepEnvelope(envelope)
                        } else if let Some(raw_step) =
                            loaded_channels.channel(&channel_path).and_then(|channel| {
                                build_raw_step_trace(
                                    time_values,
                                    &channel.values,
                                    context.raw_view_range,
                                    MAX_EXACT_STEP_SAMPLES,
                                )
                            })
                        {
                            exact_step_count += 1;
                            VisibleTraceData::RawStep(raw_step)
                        } else if let Some(step_result) = loaded_channels
                            .cached_change_point_step_trace_result(
                                &channel_path,
                                time_values,
                                context.raw_view_range,
                                MAX_STEP_CHANGE_POINTS,
                            )
                        {
                            match step_result {
                                StepTraceBuildResult::Trace(edge_step) => {
                                    edge_step_count += 1;
                                    VisibleTraceData::RawStep(edge_step)
                                }
                                StepTraceBuildResult::TooManyChangePoints {
                                    source_sample_count,
                                } => {
                                    loaded_channels.record_step_change_point_fallback(
                                        &channel_path,
                                        context.raw_view_range,
                                        source_sample_count,
                                    );
                                    dense_step_count += 1;
                                    let Some((envelope, was_built)) = loaded_channels
                                        .ensure_envelope(
                                            &channel_path,
                                            time_values,
                                            context.raw_view_range,
                                            requested_bucket_count,
                                            true,
                                        )
                                    else {
                                        continue;
                                    };
                                    if was_built {
                                        built_count += 1;
                                    }
                                    VisibleTraceData::DenseStepEnvelope(envelope)
                                }
                                StepTraceBuildResult::Empty => {
                                    dense_step_count += 1;
                                    let Some((envelope, was_built)) = loaded_channels
                                        .ensure_envelope(
                                            &channel_path,
                                            time_values,
                                            context.raw_view_range,
                                            requested_bucket_count,
                                            true,
                                        )
                                    else {
                                        continue;
                                    };
                                    if was_built {
                                        built_count += 1;
                                    }
                                    VisibleTraceData::DenseStepEnvelope(envelope)
                                }
                            }
                        } else {
                            continue;
                        }
                    }
                };

                let hover_value = hover_x
                    .map(|display_hover_x| context.time_transform.to_raw(display_hover_x))
                    .and_then(|raw_hover_x| {
                        loaded_channels.channel(&channel_path).and_then(|channel| {
                            hover_value_at_time(
                                time_values,
                                &channel.values,
                                raw_hover_x,
                                row_channel.draw_mode,
                            )
                        })
                    });

                let presentation =
                    presentations
                        .get(&row_channel.key)
                        .cloned()
                        .unwrap_or(ChannelPresentation {
                            short_label: channel_path.clone(),
                            source_label: "(missing source)".to_owned(),
                            tooltip: channel_path.clone(),
                        });
                visible_rows[row_index].traces.push(VisibleTrace {
                    key: row_channel.key.clone(),
                    channel_path,
                    display_label: presentation.short_label,
                    time_transform: context.time_transform,
                    sample_count,
                    color: row_channel.color(dark_mode),
                    line_width: row_channel
                        .line_width
                        .clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH),
                    draw_mode: row_channel.draw_mode,
                    hover_value,
                    data,
                });
            }
        }

        for visible_row in &visible_rows {
            let Some((min, max)) = combined_trace_value_range(&visible_row.traces) else {
                continue;
            };
            if let Some(row) = self
                .view
                .rows
                .iter_mut()
                .find(|row| row.id == visible_row.row_id)
            {
                row.y_range.set_last_auto(padded_range(min, max));
            }
        }

        if built_count > 0 || exact_step_count > 0 || edge_step_count > 0 || dense_step_count > 0 {
            let visible_channel_count = visible_rows
                .iter()
                .map(|row| row.traces.len())
                .sum::<usize>();
            let source_sample_count = visible_rows
                .iter()
                .flat_map(|row| row.traces.iter())
                .map(trace_source_sample_count)
                .fold(0usize, usize::saturating_add);
            let envelope_stats = self.workspace.envelope_stats();
            let cache_stats = self.workspace.cache_stats();
            let quality = if preview { "preview" } else { "full" };
            self.load.status = format!(
                "{quality} view {:.6}..{:.6}s: {} rows, {} ch, {} visible samples, built {} envelope(s), raw step {}, edge step {}, dense step {}, env h/m {}/{}, ctx {}/{}, clips {}, tile h/b {}/{}, edge h/b {}/{}, cache {}, edge cache {}, hints {}",
                view_range.0,
                view_range.1,
                visible_rows.len(),
                visible_channel_count,
                source_sample_count,
                built_count,
                exact_step_count,
                edge_step_count,
                dense_step_count,
                envelope_stats.hits,
                envelope_stats.misses,
                envelope_stats.context_hits,
                envelope_stats.context_misses,
                envelope_stats.clipped,
                envelope_stats.tile_hits,
                envelope_stats.tile_builds,
                envelope_stats.step_edge_hits,
                envelope_stats.step_edge_builds,
                cache_stats.envelope_count,
                cache_stats.step_edge_cache_count,
                cache_stats.step_fallback_hint_count
            );
        }

        visible_rows
    }

    fn select_clicked_plot_row(
        &mut self,
        response: &egui::Response,
        plot_rects: &[egui::Rect],
        waveform_input_block_rect: Option<egui::Rect>,
    ) {
        if !response.clicked() {
            return;
        }

        let Some(pointer_pos) = response.interact_pointer_pos() else {
            return;
        };
        if pointer_is_over_rect(Some(pointer_pos), waveform_input_block_rect) {
            return;
        }
        let Some(row_id) = plot_row_id_at_position(pointer_pos, plot_rects, &self.view.rows) else {
            return;
        };

        self.view.selected_row_id = Some(row_id);
    }

    fn handle_plot_interaction(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        plot_rects: &[egui::Rect],
        waveform_input_block_rect: Option<egui::Rect>,
    ) {
        let Some(full_range) = self.workspace.display_time_range() else {
            self.view.hover_x = None;
            return;
        };

        let min_span = self
            .workspace
            .display_min_view_span()
            .unwrap_or_else(|| min_view_span(full_range, 1));
        let mut next_range = clamp_view_range(
            self.view.x_range.unwrap_or(full_range),
            full_range,
            min_span,
        );
        let current_range = next_range;
        let mut changed = false;
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

        if pointer_is_over_rect(pointer_pos, waveform_input_block_rect) {
            if !primary_down {
                let _ = self.interaction.finish_box_zoom();
            }
            if self.view.hover_x.take().is_some() {
                ui.ctx().request_repaint();
            }
            return;
        }

        if response.drag_started_by(egui::PointerButton::Primary)
            && shift_pressed
            && self.interaction.box_zoom.is_none()
        {
            if let Some((start_pos, plot_rect)) =
                press_origin.or(pointer_pos).and_then(|start_pos| {
                    plot_rects
                        .iter()
                        .copied()
                        .find(|plot_rect| plot_rect.width() > 1.0 && plot_rect.contains(start_pos))
                        .map(|plot_rect| (start_pos, plot_rect))
                })
            {
                self.interaction
                    .begin_box_zoom(plot_rect, start_pos, next_range);
            }
        }

        if self.interaction.box_zoom.is_some() {
            if let Some(pointer_pos) = pointer_pos {
                self.interaction.update_box_zoom(pointer_pos);
            }

            let should_finish_box_zoom = primary_released
                || response.drag_stopped_by(egui::PointerButton::Primary)
                || !primary_down;
            if should_finish_box_zoom {
                if let Some(box_zoom) = self.interaction.finish_box_zoom() {
                    if let Some(box_range) = box_zoom_view_range(&box_zoom, full_range, min_span) {
                        next_range = box_range;
                        changed = true;
                    }

                    if changed && next_range != current_range {
                        self.view.x_range = Some(next_range);
                        self.interaction.mark_range_changed();
                        ui.ctx().request_repaint_after(INTERACTION_PREVIEW_SETTLE);
                    }

                    let next_hover_x =
                        time_at_plot_x(box_zoom.current_pos.x, box_zoom.plot_rect, next_range);
                    if self.view.hover_x != next_hover_x {
                        self.view.hover_x = next_hover_x;
                        ui.ctx().request_repaint();
                    }
                }
            } else {
                if let Some(box_zoom) = self.interaction.box_zoom {
                    let next_hover_x =
                        time_at_plot_x(box_zoom.current_pos.x, box_zoom.plot_rect, next_range);
                    if self.view.hover_x != next_hover_x {
                        self.view.hover_x = next_hover_x;
                    }
                }
                ui.ctx().request_repaint();
            }
            return;
        }

        let Some((pointer_pos, plot_rect)) = pointer_pos.and_then(|pointer_pos| {
            plot_rects
                .iter()
                .copied()
                .find(|plot_rect| plot_rect.contains(pointer_pos))
                .map(|plot_rect| (pointer_pos, plot_rect))
        }) else {
            if self.view.hover_x.take().is_some() {
                ui.ctx().request_repaint();
            }
            return;
        };

        if response.double_clicked() {
            next_range = full_range;
            changed = true;
        }
        let should_place_cursor = self.analysis.cursor_mode
            && !shift_pressed
            && response.clicked()
            && !response.double_clicked();

        if !shift_pressed
            && response.dragged_by(egui::PointerButton::Primary)
            && plot_rect.width() > 1.0
        {
            let delta_x = f64::from(response.drag_delta().x);
            if delta_x != 0.0 {
                let span = next_range.1 - next_range.0;
                let shift = -delta_x / f64::from(plot_rect.width()) * span;
                next_range = pan_view_range(next_range, full_range, shift, min_span);
                changed = true;
            }
        }

        if plot_rect.width() > 1.0 {
            if scroll_delta.x != 0.0 {
                let span = next_range.1 - next_range.0;
                let shift = -f64::from(scroll_delta.x) / f64::from(plot_rect.width()) * span;
                next_range = pan_view_range(next_range, full_range, shift, min_span);
                changed = true;
                ui.input_mut(|input| {
                    input.smooth_scroll_delta.x = 0.0;
                });
            }

            let wheel_zoom_enabled = self.view.scroll_zoom || zoom_modifier_pressed;
            let zoom_factor = x_zoom_factor(
                zoom_delta,
                scroll_delta.y,
                wheel_zoom_enabled,
                wheel_zoom_sensitivity(self.workspace.max_time_sample_count()),
            );
            if zoom_delta == 1.0 && wheel_zoom_enabled && scroll_delta.y != 0.0 {
                ui.input_mut(|input| {
                    input.smooth_scroll_delta.y = 0.0;
                });
            }

            if zoom_factor.is_finite() && (zoom_factor - 1.0).abs() > f64::EPSILON {
                let anchor_ratio = ((pointer_pos.x - plot_rect.left()) / plot_rect.width()) as f64;
                next_range =
                    zoom_view_range(next_range, full_range, anchor_ratio, zoom_factor, min_span);
                changed = true;
            }
        }

        if changed && next_range != current_range {
            self.view.x_range = Some(next_range);
            self.interaction.mark_range_changed();
            ui.ctx().request_repaint_after(INTERACTION_PREVIEW_SETTLE);
        }

        if should_place_cursor {
            if let Some(cursor_time) = time_at_plot_x(pointer_pos.x, plot_rect, next_range) {
                if self.analysis.place_cursor(cursor_time) {
                    ui.ctx().request_repaint();
                }
            }
        }

        let next_hover_x = time_at_plot_x(pointer_pos.x, plot_rect, next_range);
        if self.view.hover_x != next_hover_x {
            self.view.hover_x = next_hover_x;
            ui.ctx().request_repaint();
        }
    }
}

fn draw_app_brand(ui: &mut egui::Ui, palette: VisualPalette) {
    let dark_mode = ui.visuals().dark_mode;
    let industrial_skin = palette.skin == VisualSkin::Industrial;
    let brand_color = if industrial_skin {
        palette.accent_blue
    } else if dark_mode {
        egui::Color32::from_rgb(56, 189, 248)
    } else {
        egui::Color32::from_rgb(0, 112, 190)
    };
    let version_color = if industrial_skin {
        palette.text_primary
    } else if dark_mode {
        egui::Color32::from_rgb(224, 242, 254)
    } else {
        egui::Color32::from_rgb(12, 74, 110)
    };
    let version_bg = if industrial_skin {
        palette.input_bg
    } else if dark_mode {
        egui::Color32::from_rgb(15, 52, 70)
    } else {
        egui::Color32::from_rgb(219, 234, 254)
    };

    ui.horizontal_centered(|ui| {
        ui.label(
            egui::RichText::new(APP_NAME)
                .size(24.0)
                .strong()
                .color(brand_color),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(APP_VERSION)
                .size(11.0)
                .monospace()
                .color(version_color)
                .background_color(version_bg),
        );
    });
}

impl eframe::App for PanelYApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw_ui(ui);
    }
}

impl PanelYApp {
    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        let frame_started = Instant::now();
        apply_view_visuals(ui.ctx(), &self.view);
        let frame_palette = self.view.visual_palette();
        self.drain_load_results();
        self.drain_fft_results();
        let load_active = self.load.is_busy();
        let loading_channel_keys = self.load.loading_channel_keys();

        egui::Panel::top("top_bar").show_inside(ui, |ui| {
            paint_control_surface_texture(ui, frame_palette, ui.max_rect());
            ui.horizontal(|ui| {
                draw_app_brand(ui, frame_palette);
                if add_control_button(ui, frame_palette, egui::Button::new("About")).clicked() {
                    self.show_about = true;
                }
                add_control_separator(ui, frame_palette);
                add_status_indicator(
                    ui,
                    frame_palette,
                    load_status_indicator_level(&self.load),
                    self.load.status.as_str(),
                );
                if self.load.pending_jobs > 0 {
                    add_control_separator(ui, frame_palette);
                    ui.add(egui::Spinner::new());
                    add_status_indicator(
                        ui,
                        frame_palette,
                        ControlStatusLevel::Busy,
                        format!("jobs: {}", self.load.pending_jobs),
                    );
                }
                if let Some(progress) = &self.load.progress {
                    add_control_separator(ui, frame_palette);
                    add_status_indicator(
                        ui,
                        frame_palette,
                        ControlStatusLevel::Busy,
                        progress.as_str(),
                    );
                }
                if self.perf.show {
                    add_control_separator(ui, frame_palette);
                    ui.monospace(self.perf.summary());
                }
            });
        });

        egui::Panel::left("controls")
            .resizable(true)
            .default_size(LEFT_PANEL_DEFAULT_WIDTH)
            .min_size(LEFT_PANEL_MIN_WIDTH)
            .max_size(LEFT_PANEL_MAX_WIDTH)
            .show_inside(ui, |ui| {
                paint_control_surface_texture(ui, frame_palette, ui.max_rect());
                egui::ScrollArea::vertical()
                    .id_salt("controls_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let data_source_actions =
                            draw_data_sources(ui, &self.workspace, &self.load, frame_palette);
                        for action in data_source_actions {
                            match action {
                                DataSourceUiAction::EditPath(source_id, path) => {
                                    self.edit_parquet_path_for_source(source_id, path);
                                }
                                DataSourceUiAction::SelectRecentPath(source_id, path) => {
                                    self.select_parquet_path_for_source(source_id, path);
                                }
                                DataSourceUiAction::Browse(source_id) => {
                                    self.browse_parquet_path_for_source(source_id);
                                }
                                DataSourceUiAction::LoadSchema(source_id) => {
                                    self.load_schema_for_source(source_id);
                                }
                                DataSourceUiAction::SetTimeOffset(source_id, offset_seconds) => {
                                    self.apply_source_time_offset(source_id, offset_seconds);
                                }
                                DataSourceUiAction::StartAtZero(source_id) => {
                                    self.start_source_at_zero(source_id);
                                }
                                DataSourceUiAction::StartAllAtZero => {
                                    self.start_all_sources_at_zero();
                                }
                                DataSourceUiAction::Remove(source_id) => {
                                    self.remove_source_from_ui(source_id);
                                }
                                DataSourceUiAction::Add => self.add_source_from_ui(),
                            }
                        }

                        ui.add_space(16.0);
                        add_section_header(ui, frame_palette, "Plot config");
                        ui.label(".pyc.json compatible view config.");
                        ui.label("Path");
                        ui.horizontal(|ui| {
                            let browse_width = 72.0;
                            let text_width = (ui.available_width()
                                - browse_width
                                - ui.spacing().item_spacing.x)
                                .max(80.0);
                            add_control_text_edit_enabled(
                                ui,
                                frame_palette,
                                !load_active,
                                egui::TextEdit::singleline(&mut self.plot_config_path)
                                    .desired_width(text_width),
                            );
                            if add_control_button_enabled_with_role(
                                ui,
                                frame_palette,
                                !load_active,
                                ControlButtonRole::Utility,
                                egui::Button::new("Browse").min_size(egui::vec2(
                                    browse_width,
                                    ui.spacing().interact_size.y,
                                )),
                            )
                            .clicked()
                            {
                                self.browse_plot_config_path();
                            }
                        });

                        let mut load_plot_config = false;
                        let mut apply_plot_config_current = false;
                        let mut save_plot_config = false;
                        let mut save_plot_config_as = false;
                        let plot_config_load_notice =
                            plot_config_load_workspace_notice(&self.workspace);
                        let apply_plot_config_block_reason =
                            persistence_action_block_reason(
                                &self.workspace,
                                PersistenceAction::PlotConfigApplyToCurrentData,
                            );
                        let save_plot_config_block_reason =
                            persistence_action_block_reason(
                                &self.workspace,
                                PersistenceAction::PlotConfigSave,
                            );
                        let save_plot_config_as_block_reason =
                            persistence_action_block_reason(
                                &self.workspace,
                                PersistenceAction::PlotConfigSaveAs,
                            );
                        let workspace_schema_ready =
                            self.workspace.configured_schema_source().is_some();
                        ui.horizontal_wrapped(|ui| {
                            let load_response = add_control_button_enabled_with_role(
                                ui,
                                frame_palette,
                                !load_active,
                                ControlButtonRole::Primary,
                                egui::Button::new("Load"),
                            );
                            let load_response =
                                if let Some(notice) = &plot_config_load_notice {
                                    load_response.on_hover_text(notice)
                                } else {
                                    load_response
                                };
                            if load_response.clicked() {
                                load_plot_config = true;
                            }
                            let apply_response = add_control_button_enabled_with_role(
                                ui,
                                frame_palette,
                                !load_active
                                    && apply_plot_config_block_reason.is_none()
                                    && workspace_schema_ready,
                                ControlButtonRole::Primary,
                                egui::Button::new("Apply to Current Data"),
                            );
                            let apply_response =
                                if let Some(reason) = &apply_plot_config_block_reason {
                                    apply_response.on_disabled_hover_text(reason)
                                } else {
                                    apply_response
                                };
                            if apply_response.clicked() {
                                apply_plot_config_current = true;
                            }
                            let save_response = add_control_button_enabled_with_role(
                                ui,
                                frame_palette,
                                !load_active
                                    && save_plot_config_block_reason.is_none()
                                    && workspace_schema_ready,
                                ControlButtonRole::Primary,
                                egui::Button::new("Save"),
                            );
                            let save_response =
                                if let Some(reason) = &save_plot_config_block_reason {
                                    save_response.on_disabled_hover_text(reason)
                                } else {
                                    save_response
                                };
                            if save_response.clicked() {
                                save_plot_config = true;
                            }
                            let save_as_response = add_control_button_enabled_with_role(
                                ui,
                                frame_palette,
                                !load_active
                                    && save_plot_config_as_block_reason.is_none()
                                    && workspace_schema_ready,
                                ControlButtonRole::Primary,
                                egui::Button::new("Save As..."),
                            );
                            let save_as_response =
                                if let Some(reason) = &save_plot_config_as_block_reason {
                                    save_as_response.on_disabled_hover_text(reason)
                                } else {
                                    save_as_response
                                };
                            if save_as_response.clicked() {
                                save_plot_config_as = true;
                            }
                        });
                        if let Some(notice) = &plot_config_load_notice {
                            add_status_indicator_wrapped(
                                ui,
                                frame_palette,
                                ControlStatusLevel::Warning,
                                format!("{notice}."),
                            );
                        }
                        ui.label("Save includes all sources and offsets (v2). Apply keeps current data and offsets; v2 matches file paths.");
                        if let Some(report) = &self.plot_config_report {
                            let report_level = if report.warning_count() == 0 {
                                ControlStatusLevel::Info
                            } else {
                                ControlStatusLevel::Warning
                            };
                            add_status_indicator_monospace(
                                ui,
                                frame_palette,
                                report_level,
                                report.summary(),
                            );
                            if let Some(details) = report.details() {
                                egui::CollapsingHeader::new("Plot config warnings")
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        ui.add(egui::Label::new(details).wrap());
                                    });
                            }
                        }
                        if load_plot_config {
                            self.load_plot_config(ui.ctx());
                        } else if apply_plot_config_current {
                            self.apply_plot_config_to_current_data(ui.ctx());
                        } else if save_plot_config {
                            self.save_plot_config();
                        } else if save_plot_config_as {
                            self.save_plot_config_as();
                        }

                        if let Some(error) = self.load.error.clone() {
                            let mut clear_error = false;
                            ui.add_space(8.0);
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    add_status_indicator(
                                        ui,
                                        frame_palette,
                                        ControlStatusLevel::Danger,
                                        egui::RichText::new("Load issue")
                                            .color(ui.visuals().warn_fg_color),
                                    );
                                    if add_control_button_with_role(
                                        ui,
                                        frame_palette,
                                        ControlButtonRole::Destructive,
                                        egui::Button::new("Clear"),
                                    )
                                    .clicked()
                                    {
                                        clear_error = true;
                                    }
                                });
                                add_status_indicator_wrapped(
                                    ui,
                                    frame_palette,
                                    ControlStatusLevel::Danger,
                                    error,
                                );
                            });
                            if clear_error {
                                self.load.error = None;
                                if !load_active {
                                    self.load.status = "Ready".to_owned();
                                }
                            }
                        }

                        let can_load_waveform = self
                            .view
                            .selected_channel
                            .as_ref()
                            .and_then(|key| self.workspace.source(key.source_id))
                            .and_then(|source| source.schema.as_ref())
                            .is_some_and(|schema| {
                                schema.time_column.is_some() && !schema.channels.is_empty()
                            });
                        let selected_channel_label = self
                            .view
                            .selected_channel
                            .as_ref()
                            .map(|key| channel_label_or_fallback(&self.workspace, key))
                            .unwrap_or_else(|| "channel".to_owned());
                        let add_channel_label = format!(
                            "Load / Add {selected_channel_label} to {}",
                            self.view.selected_row_display_name()
                        );
                        if add_control_button_enabled(
                            ui,
                            frame_palette,
                            can_load_waveform,
                            egui::Button::new(add_channel_label),
                        )
                        .clicked()
                        {
                            self.load_selected_channel(ui.ctx());
                        }

                        ui.add_space(16.0);
                        let schema_response = draw_schema_controls(
                            ui,
                            &mut self.view,
                            &self.workspace,
                            &loading_channel_keys,
                        );
                        if schema_response.selection_changed {
                            if let Some(key) = &self.view.selected_channel {
                                self.load.status = format!(
                                    "Selected channel: {}",
                                    channel_label_or_fallback(&self.workspace, key)
                                );
                            }
                        }
                        if let Some(channel_key) = schema_response.load_channel {
                            self.view.selected_channel = Some(channel_key);
                            self.load_selected_channel(ui.ctx());
                        }
                        if let Some(channel_key) = schema_response.add_matching_channels {
                            self.add_matching_channels(ui.ctx(), &channel_key);
                        }

                        if draw_row_controls(
                            ui,
                            &mut self.view,
                            &self.workspace,
                            &loading_channel_keys,
                        ) {
                            for source_id in self.workspace.source_order.clone() {
                                if let Some(source) = self.workspace.source_mut(source_id) {
                                    source.loaded_channels.clear_envelope_cache();
                                }
                            }
                        }

                        if self.workspace.sources.values().any(|source| source.schema.is_some()) || self.session_preset.is_some() {
                            ui.add_space(16.0);
                            add_section_header(ui, frame_palette, "Session preset");
                            ui.label("In memory until exit. Restores all sources, offsets and view settings.");

                            let mut save_preset = false;
                            let mut apply_preset = false;
                            let mut clear_preset = false;
                            let save_preset_block_reason =
                                persistence_action_block_reason(
                                    &self.workspace,
                                    PersistenceAction::SessionPresetSave,
                                );
                            let apply_preset_block_reason =
                                persistence_action_block_reason(
                                    &self.workspace,
                                    PersistenceAction::SessionPresetApply,
                                );
                            ui.horizontal_wrapped(|ui| {
                                let save_response = add_control_button_enabled_with_role(
                                    ui,
                                    frame_palette,
                                    !load_active
                                        && save_preset_block_reason.is_none()
                                        && self.workspace.sources.values().any(|source| source.schema.is_some()),
                                    ControlButtonRole::Primary,
                                    egui::Button::new("Save"),
                                );
                                let save_response =
                                    if let Some(reason) = &save_preset_block_reason {
                                        save_response.on_disabled_hover_text(reason)
                                    } else {
                                        save_response
                                    };
                                if save_response.clicked() {
                                    save_preset = true;
                                }
                                let apply_response = add_control_button_enabled_with_role(
                                    ui,
                                    frame_palette,
                                    !load_active
                                        && apply_preset_block_reason.is_none()
                                        && self.session_preset.is_some(),
                                    ControlButtonRole::Primary,
                                    egui::Button::new("Apply"),
                                );
                                let apply_response =
                                    if let Some(reason) = &apply_preset_block_reason {
                                        apply_response.on_disabled_hover_text(reason)
                                    } else {
                                        apply_response
                                    };
                                if apply_response.clicked() {
                                    apply_preset = true;
                                }
                                if add_control_button_enabled_with_role(
                                    ui,
                                    frame_palette,
                                    self.session_preset.is_some(),
                                    ControlButtonRole::Destructive,
                                    egui::Button::new("Clear"),
                                )
                                .clicked()
                                {
                                    clear_preset = true;
                                }
                            });

                            if let Some(preset) = &self.session_preset {
                                ui.monospace(preset.summary());
                                if let Some(status) = &self.session_preset_status { ui.label(status); }
                            } else {
                                ui.label("No preset saved");
                            }


                            if save_preset {
                                self.save_session_preset();
                            } else if apply_preset {
                                self.apply_session_preset(ui.ctx());
                            } else if clear_preset {
                                self.clear_session_preset();
                            }
                        }

                        if self.workspace.display_time_range().is_some()
                            || self.view.has_visible_channels()
                        {
                            ui.add_space(16.0);
                            add_section_header(ui, frame_palette, "View");
                            ui.horizontal_wrapped(|ui| {
                                if add_control_button_with_role(
                                    ui,
                                    frame_palette,
                                    ControlButtonRole::Utility,
                                    egui::Button::new("Fit All"),
                                )
                                .clicked()
                                {
                                    self.reset_view_range();
                                }
                                if add_control_button_enabled_with_role(
                                    ui,
                                    frame_palette,
                                    self.workspace.display_overlap_time_range().is_some(),
                                    ControlButtonRole::Utility,
                                    egui::Button::new("Fit Overlap"),
                                )
                                .on_hover_text("Show the common time range of sources with loaded time data, including offsets")
                                .on_disabled_hover_text("No common time interval between sources with loaded time data")
                                .clicked()
                                {
                                    self.fit_overlap_view_range();
                                }
                            });
                            if let Some((start, end)) = self.view.x_range {
                                ui.monospace(format!("x: {start:.6} .. {end:.6} s"));
                            }
                            ui.horizontal(|ui| {
                                ui.label("Mode");
                                let mut display_mode = self.view.display_mode_choice();
                                egui::ComboBox::from_id_salt("view_mode")
                                    .selected_text(display_mode.as_str())
                                    .show_ui(ui, |ui| {
                                        for choice in DisplayModeChoice::ALL {
                                            ui.selectable_value(
                                                &mut display_mode,
                                                choice,
                                                choice.as_str(),
                                            );
                                        }
                                    });
                                if display_mode != self.view.display_mode_choice() {
                                    self.view.set_display_mode_choice(display_mode);
                                    ui.ctx().request_repaint();
                                }
                            });
                            add_control_checkbox(
                                ui,
                                frame_palette,
                                &mut self.view.scroll_zoom,
                                "Plain wheel X zoom",
                            );
                            add_control_checkbox(
                                ui,
                                frame_palette,
                                &mut self.perf.show,
                                "Frame timing",
                            );
                            add_control_checkbox(
                                ui,
                                frame_palette,
                                &mut self.view.large_preview_enabled,
                                "Emergency preview",
                            )
                            .on_hover_text(
                                "Debug option: temporarily reduces quality only while interacting with very large visible ranges",
                            );
                        }

                        if self.workspace.display_time_range().is_some()
                            || self.view.has_visible_channels()
                            || self.analysis.cursor_mode
                            || !self.analysis.info_panel_visible
                        {
                            ui.add_space(16.0);
                            draw_analysis_controls(
                                ui,
                                &mut self.analysis,
                                &self.workspace,
                                frame_palette,
                            );
                        }

                        let cache_response = draw_channel_cache_controls(
                            ui,
                            &self.workspace,
                            &self.view,
                            &loading_channel_keys,
                            &self.load,
                        );
                        if let Some(action) = cache_response.action {
                            self.handle_cache_action(action);
                        }
                    });
            });

        egui::CentralPanel::default_margins().show_inside(ui, |ui| {
            paint_control_surface_texture(ui, frame_palette, ui.max_rect());
            let show_cursor_info_panel =
                self.analysis.info_panel_visible && should_show_cursor_info_panel(&self.analysis);
            let central_available_rect = ui.available_rect_before_wrap();
            let available_size = ui.available_size();
            let cursor_info_height = if show_cursor_info_panel {
                if self.analysis.info_panel_collapsed {
                    CURSOR_INFO_COLLAPSED_HEIGHT
                } else {
                    cursor_info_display_height(self.analysis.info_panel_height, available_size.y)
                }
            } else {
                0.0
            };
            let resize_handle_height =
                if show_cursor_info_panel && !self.analysis.info_panel_collapsed {
                    CURSOR_INFO_RESIZE_HANDLE_HEIGHT
                } else {
                    0.0
                };
            let panel_spacing = if show_cursor_info_panel {
                ui.spacing().item_spacing.y * if resize_handle_height > 0.0 { 2.0 } else { 1.0 }
            } else {
                0.0
            };
            let waveform_height =
                (available_size.y - cursor_info_height - resize_handle_height - panel_spacing)
                    .max(1.0);
            let waveform_size = egui::vec2(available_size.x.max(1.0), waveform_height);
            let waveform_input_block_rect = cursor_info_input_block_rect(
                central_available_rect,
                cursor_info_height + panel_spacing,
                resize_handle_height,
            );

            ui.allocate_ui(waveform_size, |ui| {
                let viewport_size = ui.available_size();
                egui::ScrollArea::vertical()
                    .id_salt("waveform_scroll")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(
                        egui::containers::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .scroll_source(
                        egui::containers::scroll_area::ScrollSource::SCROLL_BAR
                            | egui::containers::scroll_area::ScrollSource::MOUSE_WHEEL,
                    )
                    .show(ui, |ui| {
                        let row_count = self.view.rows.len().max(1);
                        let content_height = waveform_content_height(viewport_size.y, row_count);
                        let content_size =
                            egui::vec2(viewport_size.x.max(1.0), content_height.max(1.0));
                        let (rect, response) =
                            ui.allocate_exact_size(content_size, egui::Sense::click_and_drag());
                        let painter = ui.painter_at(rect);
                        let row_rects = row_outer_rects(rect, row_count);
                        let plot_rects = row_rects
                            .iter()
                            .copied()
                            .map(plot_area_rect)
                            .collect::<Vec<_>>();
                        let requested_buckets = plot_rects
                            .first()
                            .copied()
                            .map(visible_envelope_bucket_count)
                            .unwrap_or(MIN_VISIBLE_ENVELOPE_BUCKETS);

                        let interaction_started = Instant::now();
                        self.select_clicked_plot_row(
                            &response,
                            &plot_rects,
                            waveform_input_block_rect,
                        );
                        self.handle_plot_interaction(
                            ui,
                            &response,
                            &plot_rects,
                            waveform_input_block_rect,
                        );
                        let interaction_elapsed = interaction_started.elapsed();

                        let preview = self.should_use_interaction_preview();
                        let effective_buckets = if preview {
                            preview_envelope_bucket_count(requested_buckets)
                        } else {
                            requested_buckets
                        };
                        let visible_started = Instant::now();
                        let visible_rows = self.visible_row_traces(
                            effective_buckets,
                            ui.visuals().dark_mode,
                            preview,
                        );
                        let visible_elapsed = visible_started.elapsed();

                        let draw_started = Instant::now();
                        let plot_palette = self.view.visual_palette();
                        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
                        paint_waveform_surface_texture(
                            &painter,
                            plot_palette,
                            rect,
                            &row_rects,
                            &plot_rects,
                        );
                        for (index, row_rect) in row_rects.iter().copied().enumerate() {
                            let Some(plot_rect) = plot_rects.get(index).copied() else {
                                continue;
                            };
                            let visible_row = visible_rows.get(index);
                            let row_id = visible_row.map(|row| row.row_id).unwrap_or(index as u64);
                            let row_index = visible_row.map(|row| row.row_index).unwrap_or(index);
                            let selected = self.view.selected_row_id == Some(row_id);

                            draw_row_marker(&painter, row_rect, row_index, selected, ui.visuals());
                            draw_plot_frame(&painter, plot_rect, ui.visuals(), plot_palette);

                            match visible_row {
                                Some(row) if !row.traces.is_empty() => {
                                    if let Some(display_time_range) = self.view.x_range {
                                        draw_waveform_traces(
                                            &painter,
                                            plot_rect,
                                            display_time_range,
                                            ui.visuals(),
                                            &row.traces,
                                            row.y_range,
                                        );
                                    } else {
                                        draw_status_label(
                                            &painter,
                                            plot_rect,
                                            "No time range available",
                                        );
                                    }
                                }
                                Some(row) if row.loading_channel_count > 0 => {
                                    draw_status_label(
                                        &painter,
                                        plot_rect,
                                        "Loading waveform data...",
                                    );
                                }
                                Some(row) if row.unloaded_channel_count > 0 => {
                                    draw_status_label(
                                        &painter,
                                        plot_rect,
                                        "Waveform data is not loaded",
                                    );
                                }
                                Some(row) if row.out_of_range_channel_count > 0 => {
                                    draw_status_label(
                                        &painter,
                                        plot_rect,
                                        "No source data in the current X range",
                                    );
                                }
                                Some(row) if row.row_channel_count > 0 => {
                                    draw_status_label(
                                        &painter,
                                        plot_rect,
                                        "All channels are hidden",
                                    );
                                }
                                _ => {
                                    draw_row_placeholder(
                                        &painter,
                                        plot_rect,
                                        &self.workspace,
                                        self.view.selected_channel.as_ref(),
                                        row_index,
                                    );
                                }
                            }

                            draw_box_zoom_overlay(
                                &painter,
                                plot_rect,
                                self.interaction.box_zoom.as_ref(),
                                ui.visuals(),
                            );
                            draw_cursor_lines(
                                &painter,
                                plot_rect,
                                &self.analysis,
                                self.view.x_range,
                                ui.visuals(),
                            );
                            draw_hover_line(
                                &painter,
                                plot_rect,
                                self.view.hover_x,
                                self.view.x_range,
                                ui.visuals(),
                            );
                            if let Some(row) = visible_row {
                                draw_hover_readout(
                                    &painter,
                                    plot_rect,
                                    self.view.hover_x,
                                    self.view.x_range,
                                    ui.visuals(),
                                    &row.traces,
                                );
                            }
                            draw_selected_plot_frame(
                                &painter,
                                plot_rect,
                                selected,
                                ui.visuals(),
                                plot_palette,
                            );
                        }
                        let draw_elapsed = draw_started.elapsed();
                        let envelope_stats = self.workspace.envelope_stats();
                        self.perf.update(
                            FrameTiming {
                                frame: frame_started.elapsed(),
                                interaction: interaction_elapsed,
                                visible: visible_elapsed,
                                draw: draw_elapsed,
                            },
                            &visible_rows,
                            requested_buckets,
                            effective_buckets,
                            preview,
                            envelope_stats,
                        );
                    });
            });

            if show_cursor_info_panel {
                if !self.analysis.info_panel_collapsed {
                    let max_height = cursor_info_max_height(available_size.y);
                    draw_cursor_info_resize_handle(
                        ui,
                        &mut self.analysis,
                        max_height,
                        frame_palette,
                    );
                }

                let mut fft_action = None;
                ui.allocate_ui(
                    egui::vec2(ui.available_width().max(1.0), cursor_info_height.max(1.0)),
                    |ui| {
                        egui::Frame::side_top_panel(ui.style()).show(ui, |ui| {
                            ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
                            ui.set_min_height(ui.available_height());
                            ui.set_min_width(ui.max_rect().width());
                            paint_control_surface_texture(
                                ui,
                                frame_palette,
                                ui.available_rect_before_wrap(),
                            );
                            fft_action = draw_cursor_info_panel(
                                ui,
                                &mut self.analysis,
                                &self.workspace,
                                &self.view,
                            );
                        });
                    },
                );
                if let Some(action) = fft_action {
                    self.handle_fft_action(action, ui.ctx());
                }
            }
        });
        draw_about_dialog(
            ui.ctx(),
            &mut self.show_about,
            &mut self.about_plate_texture,
        );
        if !self.analysis.info_panel_visible
            || self.analysis.info_panel_collapsed
            || !should_show_cursor_info_panel(&self.analysis)
        {
            self.analysis.interval_stats_worker.update(
                None,
                &self.workspace,
                ui.ctx(),
                &mut self.analysis.interval_stats_cache,
            );
        }
    }
}

fn draw_about_dialog(
    ctx: &egui::Context,
    show_about: &mut bool,
    about_plate_texture: &mut Option<egui::TextureHandle>,
) {
    if !*show_about {
        return;
    }

    if about_plate_texture.is_none() {
        *about_plate_texture = Some(load_about_plate_texture(ctx));
    }

    let content_size = ctx.content_rect().size();
    let max_size = egui::vec2(
        ABOUT_PLATE_MAX_WIDTH
            .min(content_size.x * ABOUT_PLATE_MAX_SCREEN_WIDTH_FRACTION)
            .max(1.0),
        (content_size.y * ABOUT_PLATE_MAX_SCREEN_HEIGHT_FRACTION).max(1.0),
    );

    egui::Window::new("About Panel-y")
        .open(show_about)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            if let Some(texture) = about_plate_texture.as_ref() {
                ui.add(
                    egui::Image::from_texture(texture)
                        .max_size(max_size)
                        .maintain_aspect_ratio(true),
                );
            }
        });
}

fn load_about_plate_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let image = image::load_from_memory(ABOUT_PLATE_IMAGE_BYTES)
        .expect("embedded About plate PNG should decode")
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    ctx.load_texture(
        "about_plate_logo",
        color_image,
        egui::TextureOptions::LINEAR,
    )
}

fn draw_analysis_controls(
    ui: &mut egui::Ui,
    analysis: &mut AnalysisState,
    workspace: &WorkspaceDataState,
    palette: VisualPalette,
) {
    add_section_header(ui, palette, "Analysis");

    let can_place_cursor = workspace.display_time_range().is_some();
    let mut cursor_mode = analysis.cursor_mode;
    if add_control_checkbox_enabled(
        ui,
        palette,
        can_place_cursor,
        &mut cursor_mode,
        "Cursor mode",
    )
    .changed()
    {
        analysis.set_cursor_mode(cursor_mode);
    }
    if !can_place_cursor {
        ui.label("Load waveform data before placing cursors.");
    } else if analysis.cursor_mode {
        ui.monospace(format!("Next cursor: {}", analysis.next_cursor.as_str()));
    }

    if add_control_button_enabled_with_role(
        ui,
        palette,
        analysis.cursor_a.is_some() || analysis.cursor_b.is_some(),
        ControlButtonRole::Destructive,
        egui::Button::new("Clear cursors"),
    )
    .clicked()
    {
        analysis.clear_cursors();
    }

    add_control_checkbox(
        ui,
        palette,
        &mut analysis.info_panel_visible,
        "Show cursor info panel",
    )
    .on_hover_text("Show Cursor A/B values and interval statistics below the plot area.");
    if analysis.info_panel_visible {
        add_control_checkbox(
            ui,
            palette,
            &mut analysis.info_panel_collapsed,
            "Collapse cursor info panel",
        );
    }

    ui.monospace(format!(
        "A: {}",
        format_cursor_time_seconds(analysis.cursor_a)
    ));
    ui.monospace(format!(
        "B: {}",
        format_cursor_time_seconds(analysis.cursor_b)
    ));
    if let (Some(cursor_a), Some(cursor_b)) = (analysis.cursor_a, analysis.cursor_b) {
        let delta = cursor_b - cursor_a;
        ui.monospace(format!("dt(B-A): {delta:.9} s"));
        let reciprocal = if delta.abs() > f64::EPSILON {
            format_frequency_hz(1.0 / delta.abs())
        } else {
            "-".to_owned()
        };
        ui.monospace(format!("1/|dt|: {reciprocal}"));
    }
}

fn should_show_cursor_info_panel(analysis: &AnalysisState) -> bool {
    analysis.cursor_mode || analysis.cursor_a.is_some() || analysis.cursor_b.is_some()
}

fn cursor_info_max_height(available_height: f32) -> f32 {
    let fraction_limit = available_height * CURSOR_INFO_MAX_HEIGHT_FRACTION;
    let viewport_limit =
        available_height - CURSOR_INFO_RESIZE_HANDLE_HEIGHT - MIN_WAVEFORM_VIEWPORT_HEIGHT;
    fraction_limit
        .min(viewport_limit)
        .min(CURSOR_INFO_MAX_HEIGHT)
        .max(CURSOR_INFO_MIN_HEIGHT)
}

fn cursor_info_display_height(desired_height: f32, available_height: f32) -> f32 {
    let desired_height = if desired_height.is_finite() {
        desired_height
    } else {
        CURSOR_INFO_DEFAULT_HEIGHT
    };
    desired_height.clamp(
        CURSOR_INFO_MIN_HEIGHT,
        cursor_info_max_height(available_height),
    )
}

fn cursor_info_input_block_rect(
    available_rect: egui::Rect,
    cursor_info_height: f32,
    resize_handle_height: f32,
) -> Option<egui::Rect> {
    let block_height = cursor_info_height + resize_handle_height;
    if block_height <= 0.0 || !block_height.is_finite() {
        return None;
    }

    let top = (available_rect.bottom() - block_height)
        .clamp(available_rect.top(), available_rect.bottom());
    Some(egui::Rect::from_min_max(
        egui::pos2(available_rect.left(), top),
        available_rect.right_bottom(),
    ))
}

fn pointer_is_over_rect(pointer_pos: Option<egui::Pos2>, rect: Option<egui::Rect>) -> bool {
    pointer_pos
        .zip(rect)
        .is_some_and(|(pointer_pos, rect)| rect.contains(pointer_pos))
}

fn draw_cursor_info_resize_handle(
    ui: &mut egui::Ui,
    analysis: &mut AnalysisState,
    max_height: f32,
    palette: VisualPalette,
) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(
            ui.available_width().max(1.0),
            CURSOR_INFO_RESIZE_HANDLE_HEIGHT,
        ),
        egui::Sense::drag(),
    );
    let response = response.on_hover_text("Resize cursor information panel");

    if response.dragged() {
        analysis.info_panel_height =
            (analysis.info_panel_height.min(max_height) - response.drag_delta().y).clamp(
                CURSOR_INFO_MIN_HEIGHT,
                max_height.max(CURSOR_INFO_MIN_HEIGHT),
            );
        ui.ctx().request_repaint();
    }
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    let stroke = if response.dragged() {
        ui.visuals().widgets.active.fg_stroke
    } else if response.hovered() {
        ui.visuals().widgets.hovered.fg_stroke
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
    paint_control_surface_texture(ui, palette, rect);
    ui.painter().hline(rect.x_range(), rect.center().y, stroke);
    paint_control_separator_treatment(ui, palette, rect);
}

fn draw_cursor_info_panel(
    ui: &mut egui::Ui,
    analysis: &mut AnalysisState,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Option<fft_analysis::FftUiAction> {
    let palette = view.visual_palette();
    ui.horizontal_wrapped(|ui| {
        add_section_header(ui, palette, "Analysis");
        for (pane, label) in [
            (AnalysisPane::Statistics, "Statistics"),
            (AnalysisPane::Fft, "FFT"),
            (AnalysisPane::Split, "Split"),
        ] {
            if ui
                .selectable_value(&mut analysis.pane, pane, label)
                .clicked()
            {
                analysis.info_panel_collapsed = false;
            }
        }
        add_control_separator(ui, palette);
        let collapse_label = if analysis.info_panel_collapsed {
            "Expand"
        } else {
            "Collapse"
        };
        if add_control_button(ui, palette, egui::Button::new(collapse_label)).clicked() {
            analysis.info_panel_collapsed = !analysis.info_panel_collapsed;
        }
        if add_control_button(ui, palette, egui::Button::new("Hide")).clicked() {
            analysis.info_panel_visible = false;
        }
    });
    if analysis.info_panel_collapsed {
        return None;
    }
    ui.horizontal_wrapped(|ui| {
        ui.monospace(format!(
            "A {}",
            format_cursor_time_seconds(analysis.cursor_a)
        ));
        ui.monospace(format!(
            "B {}",
            format_cursor_time_seconds(analysis.cursor_b)
        ));
        if let (Some(cursor_a), Some(cursor_b)) = (analysis.cursor_a, analysis.cursor_b) {
            let delta = cursor_b - cursor_a;
            ui.monospace(format!("dt {delta:.9} s"));
            let reciprocal = if delta.abs() > f64::EPSILON {
                format_frequency_hz(1.0 / delta.abs())
            } else {
                "-".to_owned()
            };
            ui.monospace(format!("1/|dt| {reciprocal}"));
        }
    });
    draw_analysis_split_body(ui, analysis, workspace, view)
}

fn draw_analysis_split_body(
    ui: &mut egui::Ui,
    analysis: &mut AnalysisState,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> Option<fft_analysis::FftUiAction> {
    // Focused views use the full body height; Split keeps the adjustable comparison layout.
    match analysis.pane {
        AnalysisPane::Statistics => {
            draw_analysis_statistics(ui, analysis, workspace, view);
            return None;
        }
        AnalysisPane::Fft => {
            return fft_analysis::draw_fft_section(
                ui,
                &mut analysis.fft,
                analysis.cursor_a,
                analysis.cursor_b,
                workspace,
                view,
            );
        }
        AnalysisPane::Split => {}
    }
    let spacing = ui.spacing().item_spacing.y;
    let body_available_height = (ui.available_height() - 2.0 * spacing).max(1.0);
    let width = ui.available_width().max(1.0);
    let (interval_height, handle_height, fft_height) =
        analysis_split_layout(body_available_height, analysis.analysis_split_height);
    ui.allocate_ui(egui::vec2(width, interval_height), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        ui.set_min_height(interval_height);
        draw_analysis_statistics(ui, analysis, workspace, view);
    });
    if handle_height > 0.0 {
        draw_analysis_split_handle(
            ui,
            analysis,
            body_available_height,
            handle_height,
            interval_height,
            fft_height,
            view.visual_palette(),
        );
    }
    let mut fft_action = None;
    ui.allocate_ui(egui::vec2(width, fft_height), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        ui.set_min_height(fft_height);
        fft_action = fft_analysis::draw_fft_section(
            ui,
            &mut analysis.fft,
            analysis.cursor_a,
            analysis.cursor_b,
            workspace,
            view,
        );
    });
    fft_action
}

fn draw_analysis_statistics(
    ui: &mut egui::Ui,
    analysis: &mut AnalysisState,
    workspace: &WorkspaceDataState,
    view: &ViewState,
) {
    let interval_stats = analysis.interval_channel_statistics(workspace, view, ui.ctx());
    let readouts = if interval_stats.is_empty() {
        cursor_channel_readouts(analysis, workspace, view)
    } else {
        Vec::new()
    };
    if analysis.interval_stats_worker.is_busy() {
        ui.horizontal_wrapped(|ui| {
            ui.spinner();
            ui.label("Calculating interval statistics...");
            if add_control_button(
                ui,
                view.visual_palette(),
                egui::Button::new("Cancel statistics"),
            )
            .clicked()
            {
                analysis.interval_stats_worker.cancel();
            }
        });
    } else if analysis.interval_stats_worker.is_cancelled()
        || analysis.interval_stats_worker.error.is_some()
    {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                analysis
                    .interval_stats_worker
                    .error
                    .as_deref()
                    .unwrap_or("Interval statistics cancelled."),
            );
            if add_control_button(
                ui,
                view.visual_palette(),
                egui::Button::new("Retry statistics"),
            )
            .clicked()
            {
                analysis.interval_stats_worker.retry();
                ui.ctx().request_repaint();
            }
        });
    }
    let table_height = (ui.available_height() - ui.spacing().interact_size.y).max(1.0);
    if !interval_stats.is_empty() {
        draw_interval_statistics_table(ui, &interval_stats, table_height);
    } else if !readouts.is_empty() {
        draw_cursor_values_table(ui, &readouts, table_height);
    } else {
        egui::ScrollArea::vertical()
            .id_salt("cursor_analysis_empty_y")
            .auto_shrink([false, false])
            .max_height(ui.available_height().max(1.0))
            .show(ui, |ui| {
                ui.label("Visible loaded channels will appear here after placing Cursor A/B.");
            });
    }
}

fn analysis_split_layout(
    body_available_height: f32,
    desired_interval_height: f32,
) -> (f32, f32, f32) {
    let body_available_height = body_available_height.max(1.0);
    let handle_height = if body_available_height
        >= ANALYSIS_SPLIT_MIN_HEIGHT + ANALYSIS_SPLIT_HANDLE_HEIGHT + 32.0
    {
        ANALYSIS_SPLIT_HANDLE_HEIGHT
    } else {
        0.0
    };
    let pane_height = (body_available_height - handle_height).max(1.0);

    let interval_height = if pane_height >= ANALYSIS_SPLIT_MIN_HEIGHT + FFT_SECTION_MIN_HEIGHT {
        let max_interval_height =
            (pane_height - FFT_SECTION_MIN_HEIGHT).max(ANALYSIS_SPLIT_MIN_HEIGHT);
        desired_interval_height.clamp(ANALYSIS_SPLIT_MIN_HEIGHT, max_interval_height)
    } else if pane_height > ANALYSIS_SPLIT_MIN_HEIGHT + 1.0 {
        ANALYSIS_SPLIT_MIN_HEIGHT
    } else {
        (pane_height * 0.5).clamp(1.0, pane_height.max(1.0))
    };
    let fft_height = (pane_height - interval_height).max(1.0);

    (interval_height, handle_height, fft_height)
}

fn analysis_split_height_bounds(body_available_height: f32) -> (f32, f32) {
    let body_available_height = body_available_height.max(1.0);
    let pane_height = (body_available_height - ANALYSIS_SPLIT_HANDLE_HEIGHT).max(1.0);
    if pane_height >= ANALYSIS_SPLIT_MIN_HEIGHT + FFT_SECTION_MIN_HEIGHT {
        (
            ANALYSIS_SPLIT_MIN_HEIGHT,
            (pane_height - FFT_SECTION_MIN_HEIGHT).max(ANALYSIS_SPLIT_MIN_HEIGHT),
        )
    } else if pane_height > ANALYSIS_SPLIT_MIN_HEIGHT + 1.0 {
        (ANALYSIS_SPLIT_MIN_HEIGHT, ANALYSIS_SPLIT_MIN_HEIGHT)
    } else {
        let compact = (pane_height * 0.5).max(1.0);
        (compact, compact)
    }
}

fn sanitized_analysis_split_height(height: f32) -> f32 {
    if height.is_finite() {
        height
    } else {
        ANALYSIS_SPLIT_DEFAULT_HEIGHT
    }
}

fn analysis_split_height_after_handle_response(
    current_height: f32,
    body_available_height: f32,
    drag_delta_y: Option<f32>,
) -> f32 {
    let current_height = sanitized_analysis_split_height(current_height);
    let Some(delta_y) = drag_delta_y.filter(|delta_y| delta_y.is_finite()) else {
        return current_height;
    };

    let (min_height, max_height) = analysis_split_height_bounds(body_available_height);
    (current_height + delta_y).clamp(min_height, max_height)
}

fn draw_analysis_split_handle(
    ui: &mut egui::Ui,
    analysis: &mut AnalysisState,
    body_available_height: f32,
    handle_height: f32,
    interval_height: f32,
    fft_height: f32,
    palette: VisualPalette,
) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(1.0), handle_height),
        egui::Sense::drag(),
    );
    let response = response.on_hover_text("Resize Interval statistics / FFT panes");

    if response.dragged() {
        let delta_y = ui.input(|input| input.pointer.delta().y);
        analysis.analysis_split_height = analysis_split_height_after_handle_response(
            analysis.analysis_split_height,
            body_available_height,
            Some(delta_y),
        );
        ui.ctx().request_repaint();
    } else {
        if !analysis.analysis_split_height.is_finite() {
            analysis.analysis_split_height = analysis_split_height_after_handle_response(
                analysis.analysis_split_height,
                body_available_height,
                None,
            );
        }
    }

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    let stroke = if response.dragged() {
        ui.visuals().widgets.active.fg_stroke
    } else if response.hovered() {
        ui.visuals().widgets.hovered.fg_stroke
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
    paint_control_surface_texture(ui, palette, rect);
    let center_y = rect.center().y;
    ui.painter().hline(rect.x_range(), center_y, stroke);
    paint_control_separator_treatment(ui, palette, rect);

    if response.hovered() || response.dragged() {
        let text = format!(
            "Interval {:.0}px / FFT {:.0}px",
            interval_height, fft_height
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::TextStyle::Small.resolve(ui.style()),
            ui.visuals().text_color(),
        );
    }
}

fn draw_interval_statistics_table(
    ui: &mut egui::Ui,
    stats: &[IntervalChannelStats],
    max_height: f32,
) {
    let visible_right = ui
        .available_rect_before_wrap()
        .intersect(ui.clip_rect())
        .right();
    ui.label("Interval statistics");
    egui::ScrollArea::vertical()
        .id_salt("interval_stats_y")
        .auto_shrink([false, false])
        .max_height(max_height)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                egui::Grid::new("interval_stats_channel_col")
                    .num_columns(1)
                    .striped(true)
                    .min_col_width(150.0)
                    .show(ui, |ui| {
                        ui.label("Channel");
                        ui.end_row();
                        for stats in stats {
                            ui.monospace(compact_channel_label(&stats.display_label, 22))
                                .on_hover_text(&stats.tooltip);
                            ui.end_row();
                        }
                    });

                ui.separator();

                egui::ScrollArea::horizontal()
                    .id_salt("interval_stats_x")
                    .max_width((visible_right - ui.available_rect_before_wrap().left()).max(1.0))
                    .min_scrolled_width(1.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(780.0);
                        egui::Grid::new("interval_stats_values")
                            .num_columns(9)
                            .striped(true)
                            .min_col_width(78.0)
                            .show(ui, |ui| {
                                ui.label("A");
                                ui.label("B");
                                ui.label("Delta");
                                ui.label("Mean");
                                ui.label("Max");
                                ui.label("Min");
                                ui.label("P-P");
                                ui.label("RMS");
                                ui.label("Slope/s");
                                ui.end_row();

                                for stats in stats {
                                    ui.monospace(format_optional_stat_value(stats.value_a));
                                    ui.monospace(format_optional_stat_value(stats.value_b));
                                    ui.monospace(format_optional_stat_value(stats.delta));
                                    ui.monospace(format_optional_stat_value(stats.mean));
                                    ui.monospace(format_optional_stat_value(stats.max));
                                    ui.monospace(format_optional_stat_value(stats.min));
                                    ui.monospace(format_optional_stat_value(stats.peak_to_peak));
                                    ui.monospace(format_optional_stat_value(stats.rms));
                                    ui.monospace(format_optional_stat_value(
                                        stats.slope_per_second,
                                    ));
                                    ui.end_row();
                                }
                            });
                    });
            });
        });
}

fn draw_cursor_values_table(ui: &mut egui::Ui, readouts: &[CursorChannelReadout], max_height: f32) {
    let visible_right = ui
        .available_rect_before_wrap()
        .intersect(ui.clip_rect())
        .right();
    ui.label("Cursor values");
    egui::ScrollArea::vertical()
        .id_salt("cursor_values_y")
        .auto_shrink([false, false])
        .max_height(max_height)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                egui::Grid::new("cursor_values_channel_col")
                    .num_columns(1)
                    .striped(true)
                    .min_col_width(150.0)
                    .show(ui, |ui| {
                        ui.label("Channel");
                        ui.end_row();
                        for readout in readouts {
                            ui.monospace(compact_channel_label(&readout.display_label, 22))
                                .on_hover_text(&readout.tooltip);
                            ui.end_row();
                        }
                    });

                ui.separator();

                egui::ScrollArea::horizontal()
                    .id_salt("cursor_values_x")
                    .max_width((visible_right - ui.available_rect_before_wrap().left()).max(1.0))
                    .min_scrolled_width(1.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(260.0);
                        egui::Grid::new("cursor_values")
                            .num_columns(3)
                            .striped(true)
                            .min_col_width(78.0)
                            .show(ui, |ui| {
                                ui.label("A");
                                ui.label("B");
                                ui.label("Delta");
                                ui.end_row();

                                for readout in readouts {
                                    ui.monospace(format_optional_cursor_value(readout.value_a));
                                    ui.monospace(format_optional_cursor_value(readout.value_b));
                                    ui.monospace(format_optional_cursor_value(readout.delta));
                                    ui.end_row();
                                }
                            });
                    });
            });
        });
}

fn draw_data_sources(
    ui: &mut egui::Ui,
    workspace: &WorkspaceDataState,
    load: &LoadState,
    palette: VisualPalette,
) -> Vec<DataSourceUiAction> {
    add_section_header(ui, palette, "Data Sources");
    let mut actions = Vec::new();

    ui.label(format!("{} source(s)", workspace.source_order.len()));
    ui.add_space(8.0);
    let has_loaded_time = workspace.source_order.iter().any(|source_id| {
        workspace
            .source(*source_id)
            .and_then(source_start_at_zero_offset)
            .is_some()
    });
    if add_control_button_enabled_with_role(
        ui,
        palette,
        has_loaded_time,
        ControlButtonRole::Utility,
        egui::Button::new("Start all at zero"),
    )
    .on_hover_text("Align every source with loaded time data to display time 0")
    .clicked()
    {
        actions.push(DataSourceUiAction::StartAllAtZero);
    }

    if add_control_button_enabled_with_role(
        ui,
        palette,
        true,
        ControlButtonRole::Primary,
        egui::Button::new("Add Source"),
    )
    .on_hover_text("Add another independently loaded Parquet source")
    .clicked()
    {
        actions.push(DataSourceUiAction::Add);
    }

    if workspace.source_order.is_empty() {
        add_status_indicator(
            ui,
            palette,
            ControlStatusLevel::Warning,
            "No data sources. Add one to continue.",
        );
    }

    // Bound the source list independently of the outer controls panel. SourceId scopes
    // keep each card's inputs stable when earlier cards are removed or renumbered.
    let list_height = (ui.ctx().content_rect().height() * 0.5).clamp(220.0, 440.0);
    egui::ScrollArea::vertical()
        .id_salt("data_sources_scroll")
        .max_height(list_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for source_id in workspace.source_order.iter().copied() {
                let Some(source) = workspace.source(source_id) else {
                    continue;
                };
                let Some(presentation) = source_presentation(workspace, source_id) else {
                    continue;
                };
                let source_loading = !source_dangerous_actions_enabled(load, source_id);
                ui.push_id(("data_source", source_id), |ui| {
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        add_status_indicator(
                            ui,
                            palette,
                            if source_loading {
                                ControlStatusLevel::Busy
                            } else {
                                ControlStatusLevel::Info
                            },
                            presentation.short_label.clone(),
                        );
                        ui.label("Path");
                        ui.horizontal(|ui| {
                            let browse_width = 72.0;
                            let text_width =
                                (ui.available_width() - browse_width - ui.spacing().item_spacing.x)
                                    .max(80.0);
                            let mut edited_path = source.parquet_path.clone();
                            let response = add_control_text_edit_enabled(
                                ui,
                                palette,
                                !source_loading,
                                egui::TextEdit::singleline(&mut edited_path).desired_width(text_width),
                            );
                            if response.changed() && edited_path != source.parquet_path {
                                actions.push(DataSourceUiAction::EditPath(source_id, edited_path));
                            }
                            if add_control_button_enabled_with_role(
                                ui,
                                palette,
                                !source_loading,
                                ControlButtonRole::Utility,
                                egui::Button::new("Browse")
                                    .min_size(egui::vec2(browse_width, ui.spacing().interact_size.y)),
                            )
                            .clicked()
                            {
                                actions.push(DataSourceUiAction::Browse(source_id));
                            }
                        });

                        match validate_parquet_path(&source.parquet_path) {
                            Ok(path) => {
                                add_status_indicator_wrapped(
                                    ui,
                                    palette,
                                    ControlStatusLevel::Good,
                                    format!("Ready: {}", path.display()),
                                );
                            }
                            Err(error) => {
                                add_status_indicator_wrapped(
                                    ui,
                                    palette,
                                    ControlStatusLevel::Warning,
                                    error.message(),
                                );
                            }
                        }

                        ui.label("Time alignment");
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Offset");
                            let mut offset_seconds = source.time_transform.offset_seconds;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut offset_seconds)
                                        .speed(0.001)
                                        .suffix(" s"),
                                )
                                .on_hover_text(
                                    "Display time = raw time + offset. Raw samples and caches are unchanged.",
                                )
                                .changed()
                            {
                                actions.push(DataSourceUiAction::SetTimeOffset(
                                    source_id,
                                    offset_seconds,
                                ));
                            }
                            if add_control_button_enabled_with_role(
                                ui,
                                palette,
                                source.time_transform.offset_seconds != 0.0,
                                ControlButtonRole::Utility,
                                egui::Button::new("Reset").small(),
                            )
                            .on_hover_text("Restore this source's raw time positions")
                            .clicked()
                            {
                                actions.push(DataSourceUiAction::SetTimeOffset(source_id, 0.0));
                            }
                            if add_control_button_enabled_with_role(
                                ui,
                                palette,
                                source_start_at_zero_offset(source).is_some(),
                                ControlButtonRole::Utility,
                                egui::Button::new("Start at zero").small(),
                            )
                            .on_hover_text("Align this source's first raw timestamp to display time 0")
                            .clicked()
                            {
                                actions.push(DataSourceUiAction::StartAtZero(source_id));
                            }
                        });
                        add_status_indicator_wrapped(
                            ui,
                            palette,
                            ControlStatusLevel::Info,
                            format!(
                                "display time range: {} s",
                                format_range(data_source_display_time_range(source))
                            ),
                        );

                        let schema_state = if source.schema.is_some() {
                            "ready"
                        } else {
                            "not loaded"
                        };
                        let time_state = if source.shared_time.is_some() {
                            "ready"
                        } else {
                            "not loaded"
                        };
                        add_status_indicator_wrapped(
                            ui,
                            palette,
                            if source_loading {
                                ControlStatusLevel::Busy
                            } else {
                                ControlStatusLevel::Info
                            },
                            format!(
                                "schema: {schema_state}, time: {time_state}, channels: {} loaded{}",
                                source.loaded_channels.raw_by_channel.len(),
                                if source_loading { ", loading" } else { "" }
                            ),
                        );

                        ui.horizontal_wrapped(|ui| {
                            let valid_path = validate_parquet_path(&source.parquet_path).is_ok();
                            if add_control_button_enabled_with_role(
                                ui,
                                palette,
                                !source_loading && valid_path,
                                ControlButtonRole::Primary,
                                egui::Button::new("Load Schema"),
                            )
                            .clicked()
                            {
                                actions.push(DataSourceUiAction::LoadSchema(source_id));
                            }
                            if add_control_button_enabled_with_role(
                                ui,
                                palette,
                                !source_loading,
                                ControlButtonRole::Destructive,
                                egui::Button::new("Remove"),
                            )
                            .clicked()
                            {
                                actions.push(DataSourceUiAction::Remove(source_id));
                            }
                        });

                        if !workspace.recent_parquet_paths.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("Recent");
                                egui::ComboBox::from_id_salt("recent_parquet_paths")
                                    .selected_text("Choose...")
                                    .show_ui(ui, |ui| {
                                        for path in &workspace.recent_parquet_paths {
                                            let response = ui.selectable_label(
                                                source.parquet_path == *path,
                                                recent_path_label(path),
                                            );
                                            if response.clicked() {
                                                actions.push(DataSourceUiAction::SelectRecentPath(
                                                    source_id,
                                                    path.clone(),
                                                ));
                                            }
                                            response.on_hover_text(path);
                                        }
                                    });
                            });
                        }
                    });
                });
            }
        });

    actions
}

fn draw_schema_controls(
    ui: &mut egui::Ui,
    view: &mut ViewState,
    workspace: &WorkspaceDataState,
    loading_channel_keys: &BTreeSet<ChannelKey>,
) -> SchemaControlsResponse {
    let palette = view.visual_palette();
    add_section_header(ui, palette, "Channel browser");
    let mut response = SchemaControlsResponse::default();
    draw_target_row_selector(ui, view);
    draw_channel_browser_controls(ui, palette, view);

    if workspace.source_order.is_empty() {
        add_status_indicator(ui, palette, ControlStatusLevel::Warning, "No data sources");
        return response;
    }

    let channel_filter = view.channel_filter.clone();
    let state_filter = view.channel_state_filter;
    let sort = view.channel_sort;
    let target_row_label = view.selected_row_display_name();
    let mut any_schema = false;

    for source_id in workspace.source_order.iter().copied() {
        let Some(dataset) = workspace.source(source_id) else {
            continue;
        };
        let source_label = source_presentation(workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "(missing source)".to_owned());
        ui.push_id(("channel_group", source_id), |ui| {
            ui.add_space(8.0);
            add_status_indicator(
                ui,
                palette,
                if dataset.schema.is_some() {
                    ControlStatusLevel::Info
                } else {
                    ControlStatusLevel::Warning
                },
                source_label,
            );
        });

        let Some(schema) = dataset.schema.as_ref() else {
            add_status_indicator(
                ui,
                palette,
                ControlStatusLevel::Warning,
                "Schema not loaded",
            );
            continue;
        };
        any_schema = true;
        draw_schema_summary(ui, palette, dataset, schema);
        if schema.channels.is_empty() {
            add_status_indicator(
                ui,
                palette,
                ControlStatusLevel::Warning,
                "No numeric channels found",
            );
            continue;
        }

        let entries = channel_list_entries(
            workspace,
            source_id,
            &schema.channels,
            &channel_filter,
            state_filter,
            sort,
            &dataset.loaded_channels,
            loading_channel_keys,
        );
        add_status_indicator(
            ui,
            palette,
            ControlStatusLevel::Info,
            format!(
                "{} / {} channels in this source",
                entries.len(),
                schema.channels.len()
            ),
        );

        egui::ScrollArea::vertical()
            .id_salt(("channel_list", source_id))
            .max_height(220.0)
            .show(ui, |ui| {
                if entries.is_empty() {
                    add_status_indicator(
                        ui,
                        palette,
                        ControlStatusLevel::Warning,
                        "No channels match the filter",
                    );
                }

                for (channel, load_state) in entries {
                    draw_channel_list_row(
                        ui,
                        view,
                        workspace,
                        source_id,
                        channel,
                        load_state,
                        &target_row_label,
                        &mut response,
                    );
                }
            });

        if view
            .selected_channel
            .as_ref()
            .is_some_and(|key| key.source_id == source_id)
        {
            draw_selected_channel_details(
                ui,
                view,
                workspace,
                source_id,
                dataset,
                schema,
                loading_channel_keys,
                &mut response,
            );
        }
        draw_raw_columns(ui, source_id, schema);
    }

    if !any_schema {
        add_status_indicator(
            ui,
            palette,
            ControlStatusLevel::Warning,
            "Load a schema for one or more sources",
        );
    }

    response
}

fn draw_channel_browser_controls(ui: &mut egui::Ui, palette: VisualPalette, view: &mut ViewState) {
    ui.horizontal(|ui| {
        ui.label("Filter");
        add_control_text_edit_enabled(
            ui,
            palette,
            true,
            egui::TextEdit::singleline(&mut view.channel_filter)
                .hint_text("channel name or path")
                .desired_width(ui.available_width().max(80.0)),
        );
    });

    ui.horizontal_wrapped(|ui| {
        ui.label("State");
        egui::ComboBox::from_id_salt("channel_state_filter")
            .selected_text(view.channel_state_filter.as_str())
            .show_ui(ui, |ui| {
                for state_filter in ChannelStateFilter::ALL {
                    ui.selectable_value(
                        &mut view.channel_state_filter,
                        state_filter,
                        state_filter.as_str(),
                    );
                }
            });

        ui.label("Sort");
        egui::ComboBox::from_id_salt("channel_sort")
            .selected_text(view.channel_sort.as_str())
            .show_ui(ui, |ui| {
                for sort in ChannelSort::ALL {
                    ui.selectable_value(&mut view.channel_sort, sort, sort.as_str());
                }
            });
    });
}

fn draw_channel_list_row(
    ui: &mut egui::Ui,
    view: &mut ViewState,
    workspace: &WorkspaceDataState,
    source_id: SourceId,
    channel: &parquet_schema::ColumnInfo,
    load_state: ChannelLoadState,
    target_row_label: &str,
    response: &mut SchemaControlsResponse,
) {
    let key = ChannelKey::new(source_id, channel.path.clone());
    let presentation = channel_presentation(workspace, &key).unwrap_or(ChannelPresentation {
        short_label: channel.path.clone(),
        source_label: "(missing source)".to_owned(),
        tooltip: channel.path.clone(),
    });
    let selected = view.selected_channel.as_ref() == Some(&key);
    let palette = view.visual_palette();
    ui.push_id(("channel", source_id, channel.path.clone()), |ui| {
        ui.horizontal(|ui| {
            if add_control_button_enabled_with_role(
                ui,
                palette,
                load_state.can_load_or_add(),
                ControlButtonRole::Primary,
                egui::Button::new(load_state.action_label())
                    .min_size(egui::vec2(52.0, ui.spacing().interact_size.y)),
            )
            .on_hover_text(format!(
                "{} {} to {}",
                load_state.action_label(),
                presentation.short_label,
                target_row_label
            ))
            .clicked()
            {
                view.selected_channel = Some(key.clone());
                response.selection_changed = true;
                response.load_channel = Some(key.clone());
            }

            add_status_indicator_monospace(
                ui,
                palette,
                channel_load_status_indicator_level(load_state),
                format!("{:<8}", load_state.as_str()),
            );
            ui.monospace(format!("#{:02}", channel.index));

            if ui
                .selectable_label(
                    selected,
                    compact_channel_label(&presentation.short_label, 24),
                )
                .on_hover_text(&presentation.tooltip)
                .clicked()
            {
                view.selected_channel = Some(key);
                response.selection_changed = true;
            }
        });
    });
}

fn draw_selected_channel_details(
    ui: &mut egui::Ui,
    view: &ViewState,
    workspace: &WorkspaceDataState,
    source_id: SourceId,
    dataset: &DatasetState,
    schema: &parquet_schema::SchemaSummary,
    loading_channel_keys: &BTreeSet<ChannelKey>,
    response: &mut SchemaControlsResponse,
) {
    let Some(selected_key) = view.selected_channel.as_ref() else {
        return;
    };
    if selected_key.source_id != source_id {
        return;
    }
    let selected_path = selected_key.channel_path.as_str();
    let presentation =
        channel_presentation(workspace, selected_key).unwrap_or(ChannelPresentation {
            short_label: selected_path.to_owned(),
            source_label: "(missing source)".to_owned(),
            tooltip: selected_path.to_owned(),
        });

    ui.add_space(8.0);
    let Some(channel) = schema
        .channels
        .iter()
        .find(|channel| channel.path == selected_path)
    else {
        let label = format!("Selected: {}", presentation.short_label);
        add_compact_label(ui, &label, LEFT_PANEL_DETAIL_LABEL_CHARS)
            .on_hover_text(&presentation.tooltip);
        return;
    };

    let load_state = channel_load_state(
        source_id,
        &channel.path,
        &dataset.loaded_channels,
        loading_channel_keys,
    );
    let rows = channel_row_membership(view, selected_key);
    let rows = if rows.is_empty() {
        "none".to_owned()
    } else {
        rows.join(", ")
    };
    let logical = channel.logical_type.as_deref().unwrap_or("-");
    let loaded_channel = dataset.loaded_channels.channel(&channel.path);
    let loaded_note = loaded_channel
        .map(|loaded| {
            format!(
                "{} samples, {:.1} MiB, read {:.3}s",
                loaded.sample_count(),
                bytes_to_mib(loaded.memory_bytes()),
                loaded.elapsed.as_secs_f64()
            )
        })
        .unwrap_or_else(|| "not loaded".to_owned());

    ui.group(|ui| {
        ui.label("Selected channel");
        add_compact_monospace_label(ui, &presentation.short_label, LEFT_PANEL_DETAIL_LABEL_CHARS)
            .on_hover_text(&presentation.tooltip);
        ui.label(format!("Source: {}", presentation.source_label));
        add_status_indicator(
            ui,
            view.visual_palette(),
            channel_load_status_indicator_level(load_state),
            format!("State: {}", load_state.as_str()),
        );
        ui.label(format!("Rows: {rows}"));
        ui.label(format!("Index: #{}", channel.index));
        ui.label(format!("Type: {} / {}", channel.physical_type, logical));
        add_status_indicator_wrapped(
            ui,
            view.visual_palette(),
            if loaded_channel.is_some() {
                ControlStatusLevel::Good
            } else {
                ControlStatusLevel::Warning
            },
            format!("Cache: {loaded_note}"),
        );
        let path_label = format!("Path: {}", channel.path);
        add_compact_monospace_label(ui, &path_label, LEFT_PANEL_DETAIL_LABEL_CHARS)
            .on_hover_text(channel.path.as_str());
        if add_control_button_enabled_with_role(
            ui,
            view.visual_palette(),
            true,
            ControlButtonRole::Primary,
            egui::Button::new("Add matching channel from all sources"),
        )
        .on_hover_text(
            "Match exact channel path first, then a unique original display name per source",
        )
        .clicked()
        {
            response.add_matching_channels = Some(selected_key.clone());
        }
    });
}

fn channel_row_membership(view: &ViewState, key: &ChannelKey) -> Vec<String> {
    view.rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.channels.iter().any(|channel| channel.key == *key))
        .map(|(index, _)| format!("Row {}", index + 1))
        .collect()
}

fn draw_raw_columns(
    ui: &mut egui::Ui,
    source_id: SourceId,
    schema: &parquet_schema::SchemaSummary,
) {
    ui.add_space(12.0);
    egui::CollapsingHeader::new("Raw columns")
        .id_salt(("raw_columns", source_id))
        .default_open(false)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    for column in &schema.columns {
                        let role = column.role.as_str();
                        let logical = column.logical_type.as_deref().unwrap_or("-");
                        let numeric = if column.is_numeric { "num" } else { "-" };
                        let full_line = format!(
                            "#{:02} {:<7} {:<3} {:<8} {} ({}, {})",
                            column.index,
                            role,
                            numeric,
                            column.physical_type,
                            column.display_name(),
                            logical,
                            column.converted_type
                        );
                        add_compact_monospace_label(ui, &full_line, LEFT_PANEL_DETAIL_LABEL_CHARS)
                            .on_hover_text(full_line);
                    }
                });
        });
}

fn draw_schema_summary(
    ui: &mut egui::Ui,
    palette: VisualPalette,
    dataset: &DatasetState,
    schema: &parquet_schema::SchemaSummary,
) {
    add_status_indicator(
        ui,
        palette,
        ControlStatusLevel::Info,
        format!("Rows: {}", schema.row_count),
    );
    add_status_indicator(
        ui,
        palette,
        ControlStatusLevel::Info,
        format!("Row groups: {}", schema.row_group_count),
    );
    add_status_indicator(
        ui,
        palette,
        ControlStatusLevel::Info,
        format!("Columns: {}", schema.column_count),
    );
    add_status_indicator(
        ui,
        palette,
        ControlStatusLevel::Info,
        format!("Numeric channels: {}", schema.channels.len()),
    );

    match &schema.time_column {
        Some(time_column) => {
            let label = format!("Time column: {}", time_column.display_name());
            add_compact_status_indicator(
                ui,
                palette,
                ControlStatusLevel::Good,
                &label,
                LEFT_PANEL_DETAIL_LABEL_CHARS,
            )
            .on_hover_text(time_column.path.as_str());
        }
        None => {
            add_status_indicator(
                ui,
                palette,
                ControlStatusLevel::Danger,
                egui::RichText::new("Time column: not found").color(ui.visuals().warn_fg_color),
            );
        }
    }

    if let Some(time) = &dataset.shared_time {
        add_status_indicator(
            ui,
            palette,
            ControlStatusLevel::Good,
            format!("Time range: {} s", format_range(time.time_range())),
        );
        let sample_rate = estimated_sample_rate_hz(time.sample_count(), time.time_range())
            .map(format_frequency_hz)
            .unwrap_or_else(|| "-".to_owned());
        add_status_indicator(
            ui,
            palette,
            ControlStatusLevel::Info,
            format!("Estimated sample rate: {sample_rate}"),
        );
    } else {
        add_status_indicator(
            ui,
            palette,
            ControlStatusLevel::Warning,
            "Time range: not loaded",
        );
        add_status_indicator(
            ui,
            palette,
            ControlStatusLevel::Warning,
            "Estimated sample rate: -",
        );
    }
}

fn draw_target_row_selector(ui: &mut egui::Ui, view: &mut ViewState) {
    view.ensure_row_state();
    let mut selected_row_id = view.selected_row_id;

    ui.horizontal(|ui| {
        ui.label("Target row");
        egui::ComboBox::from_id_salt("channel_list_target_row")
            .selected_text(view.selected_row_display_name())
            .show_ui(ui, |ui| {
                for (row_index, row) in view.rows.iter().enumerate() {
                    ui.selectable_value(
                        &mut selected_row_id,
                        Some(row.id),
                        format!("Row {}", row_index + 1),
                    );
                }
            });
    });

    view.selected_row_id = selected_row_id;
}

fn draw_channel_cache_controls(
    ui: &mut egui::Ui,
    workspace: &WorkspaceDataState,
    view: &ViewState,
    loading_channel_keys: &BTreeSet<ChannelKey>,
    load: &LoadState,
) -> CacheControlsResponse {
    ui.add_space(16.0);
    let palette = view.visual_palette();
    add_section_header(ui, palette, "Cache");
    let mut response = CacheControlsResponse::default();
    let aggregate = workspace.cache_stats();
    add_status_indicator(
        ui,
        palette,
        ControlStatusLevel::Info,
        format!(
            "Workspace: {} source(s), {} channel(s), {:.1} MiB raw",
            aggregate.source_count,
            aggregate.loaded_channel_count,
            bytes_to_mib(aggregate.total_raw_memory_bytes())
        ),
    );
    add_status_indicator(
        ui,
        palette,
        ControlStatusLevel::Info,
        format!(
            "Workspace draw caches: {} envelope(s), {} line tile(s) / {:.1} MiB, {} step edge cache(s) / {:.1} MiB, {} hint(s)",
            aggregate.envelope_count,
            aggregate.line_tile_cache_count,
            bytes_to_mib(aggregate.line_tile_memory_bytes),
            aggregate.step_edge_cache_count,
            bytes_to_mib(aggregate.step_edge_memory_bytes),
            aggregate.step_fallback_hint_count
        ),
    );

    for source_id in workspace.source_order.iter().copied() {
        let Some(dataset) = workspace.source(source_id) else {
            continue;
        };
        let source_label = source_presentation(workspace, source_id)
            .map(|source| source.short_label)
            .unwrap_or_else(|| "(missing source)".to_owned());
        ui.push_id(("source_cache", source_id), |ui| {
            ui.add_space(8.0);
            add_status_indicator(ui, palette, ControlStatusLevel::Info, source_label);

            if let Some(time) = &dataset.shared_time {
                let file_name = time
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("-");
                let time_label = format!(
                    "Time: {} / {} ({} samples, {:.1} MiB, col #{}, file {}, {:.3}s)",
                    time.column_name,
                    time.column_path,
                    time.sample_count(),
                    bytes_to_mib(time.memory_bytes()),
                    time.projected_column_index,
                    file_name,
                    time.elapsed.as_secs_f64()
                );
                add_compact_status_indicator(
                    ui,
                    palette,
                    ControlStatusLevel::Good,
                    &time_label,
                    LEFT_PANEL_DETAIL_LABEL_CHARS,
                )
                .on_hover_text(time_label);
            } else {
                add_status_indicator(ui, palette, ControlStatusLevel::Warning, "Time: not loaded");
            }

            let raw_channel_cache_count = dataset.loaded_channels.raw_by_channel.len();
            add_status_indicator(
                ui,
                palette,
                cache_count_status_indicator_level(raw_channel_cache_count),
                format!(
                    "Source channels: {} cached ({:.1} MiB)",
                    raw_channel_cache_count,
                    bytes_to_mib(dataset.loaded_channels.raw_memory_bytes())
                ),
            );

            if !dataset.loaded_channels.line_tile_cache.is_empty() {
                add_status_indicator(
                    ui,
                    palette,
                    ControlStatusLevel::Info,
                    format!(
                        "Line tiles: {} cached ({:.1} MiB, build {:.3}s)",
                        dataset.loaded_channels.line_tile_cache.len(),
                        bytes_to_mib(dataset.loaded_channels.line_tile_memory_bytes()),
                        dataset.loaded_channels.line_tile_build_seconds()
                    ),
                );
            }
            if !dataset.loaded_channels.step_edge_cache.is_empty() {
                let edge_count = dataset
                    .loaded_channels
                    .step_edge_cache
                    .values()
                    .map(StepEdgeCache::edge_count)
                    .sum::<usize>();
                add_status_indicator(
                    ui,
                    palette,
                    ControlStatusLevel::Info,
                    format!(
                        "Step edges: {} cached, {} edges ({:.1} MiB, build {:.3}s)",
                        dataset.loaded_channels.step_edge_cache.len(),
                        edge_count,
                        bytes_to_mib(dataset.loaded_channels.step_edge_memory_bytes()),
                        dataset.loaded_channels.step_edge_build_seconds()
                    ),
                );
            }

            ui.horizontal_wrapped(|ui| {
                if add_control_button_enabled_with_role(
                    ui,
                    palette,
                    !load.is_source_loading(source_id)
                        && !dataset.loaded_channels.raw_by_channel.is_empty(),
                    ControlButtonRole::Destructive,
                    egui::Button::new("Unload all channels"),
                )
                .on_hover_text("Drop cached channel data while keeping rows and time data")
                .clicked()
                {
                    response.action = Some(CacheControlAction::UnloadAllChannels(source_id));
                }
                if add_control_button_enabled_with_role(
                    ui,
                    palette,
                    !load.is_source_loading(source_id) && dataset.loaded_channels.has_draw_caches(),
                    ControlButtonRole::Destructive,
                    egui::Button::new("Clear draw caches"),
                )
                .on_hover_text(
                    "Drop envelope, line tile, and step edge caches while keeping raw data",
                )
                .clicked()
                {
                    response.action = Some(CacheControlAction::ClearDrawCaches(source_id));
                }
            });

            if dataset.loaded_channels.raw_by_channel.is_empty() {
                add_status_indicator(
                    ui,
                    palette,
                    ControlStatusLevel::Warning,
                    "No cached channels",
                );
                return;
            }

            let target_row_label = view.selected_row_display_name();
            egui::ScrollArea::vertical()
                .id_salt(("loaded_channel_cache_list", source_id))
                .max_height(220.0)
                .show(ui, |ui| {
                    for channel in dataset.loaded_channels.raw_by_channel.values() {
                        let file_name = channel
                            .path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("-");
                        let channel_loading = loading_channel_keys
                            .contains(&ChannelKey::new(source_id, channel.channel_path.clone()));
                        ui.horizontal_wrapped(|ui| {
                            let key = ChannelKey::new(source_id, channel.channel_path.clone());
                            let channel_label = channel_label_or_fallback(workspace, &key);
                            add_compact_monospace_label(
                                ui,
                                &channel_label,
                                LEFT_PANEL_CHANNEL_LABEL_CHARS,
                            )
                            .on_hover_text(channel_tooltip_or_fallback(workspace, &key));
                            let cache_note = format!(
                                "{} samples, {:.1} MiB, column #{}, file {}",
                                channel.sample_count(),
                                bytes_to_mib(channel.memory_bytes()),
                                channel.projected_column_index,
                                file_name
                            );
                            add_compact_status_indicator(
                                ui,
                                palette,
                                ControlStatusLevel::Info,
                                &cache_note,
                                LEFT_PANEL_DETAIL_LABEL_CHARS,
                            )
                            .on_hover_text(cache_note);
                            if add_control_button(
                                ui,
                                palette,
                                egui::Button::new(format!("Add to {target_row_label}")).small(),
                            )
                            .on_hover_text("Reuse cached data in the selected row")
                            .clicked()
                            {
                                response.action = Some(CacheControlAction::AddToSelectedRow(
                                    ChannelKey::new(source_id, channel.channel_path.clone()),
                                ));
                            }
                            if add_control_button_enabled_with_role(
                                ui,
                                palette,
                                !channel_loading,
                                ControlButtonRole::Destructive,
                                egui::Button::new("Unload")
                                    .small()
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            )
                            .on_hover_text("Drop this channel's cached data; row references stay")
                            .clicked()
                            {
                                response.action = Some(CacheControlAction::UnloadChannel(
                                    ChannelKey::new(source_id, channel.channel_path.clone()),
                                ));
                            }
                        });
                    }
                });
        });
    }

    response
}

fn draw_row_controls(
    ui: &mut egui::Ui,
    view: &mut ViewState,
    workspace: &WorkspaceDataState,
    loading_channel_keys: &BTreeSet<ChannelKey>,
) -> bool {
    ui.add_space(16.0);
    let palette = view.visual_palette();
    add_section_header(ui, palette, "Rows");
    view.ensure_row_state();

    let mut changed = false;
    ui.horizontal(|ui| {
        if add_control_button(ui, palette, egui::Button::new("Add Row")).clicked() {
            view.add_row();
            changed = true;
        }
    });

    add_status_indicator(
        ui,
        palette,
        ControlStatusLevel::Info,
        format!("Load target: {}", view.selected_row_display_name()),
    );

    let can_delete_row = view.rows.len() > 1;
    let mut selected_row_id = view.selected_row_id;
    let mut remove_row_id = None;

    for (row_index, row) in view.rows.iter_mut().enumerate() {
        add_control_separator(ui, palette);
        let row_has_loading_channel = row
            .channels
            .iter()
            .any(|channel| loading_channel_keys.contains(&channel.key));
        ui.horizontal(|ui| {
            let row_label = format!("Row {}", row_index + 1);
            if ui
                .selectable_label(selected_row_id == Some(row.id), row_label)
                .clicked()
            {
                selected_row_id = Some(row.id);
            }
            if add_control_button_enabled_with_role(
                ui,
                palette,
                can_delete_row && !row_has_loading_channel,
                ControlButtonRole::Destructive,
                egui::Button::new("Delete"),
            )
            .clicked()
            {
                remove_row_id = Some(row.id);
            }
        });

        if draw_y_range_controls(ui, row.id, &mut row.y_range, palette) {
            changed = true;
        }

        if row.channels.is_empty() {
            add_status_indicator(
                ui,
                palette,
                ControlStatusLevel::Warning,
                "No channels in row",
            );
            continue;
        }

        let mut remove_channel = None;
        for channel in &mut row.channels {
            ui.push_id(("channel_style", row.id, channel.key.clone()), |ui| {
                ui.horizontal_wrapped(|ui| {
                    let channel_loading = loading_channel_keys.contains(&channel.key);
                    if add_control_checkbox(ui, palette, &mut channel.visible, "")
                        .on_hover_text("Visible")
                        .changed()
                    {
                        changed = true;
                    }

                    let mut color = channel.color(ui.visuals().dark_mode);
                    if egui::color_picker::color_edit_button_srgba(
                        ui,
                        &mut color,
                        egui::color_picker::Alpha::Opaque,
                    )
                    .changed()
                    {
                        channel.color_override = Some(color);
                        changed = true;
                    }

                    let label = if channel_loading {
                        format!(
                            "{} (loading)",
                            channel_label_or_fallback(workspace, &channel.key)
                        )
                    } else {
                        channel_label_or_fallback(workspace, &channel.key)
                    };
                    add_compact_label(ui, &label, LEFT_PANEL_CHANNEL_LABEL_CHARS)
                        .on_hover_text(channel_tooltip_or_fallback(workspace, &channel.key));
                    egui::ComboBox::from_id_salt("draw_mode")
                        .selected_text(channel.draw_mode.as_str())
                        .show_ui(ui, |ui| {
                            for draw_mode in DrawMode::ALL {
                                if ui
                                    .selectable_value(
                                        &mut channel.draw_mode,
                                        draw_mode,
                                        draw_mode.as_str(),
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });

                    let mut line_width = channel.line_width;
                    if ui
                        .add(
                            egui::DragValue::new(&mut line_width)
                                .speed(0.1)
                                .range(MIN_TRACE_LINE_WIDTH..=MAX_TRACE_LINE_WIDTH)
                                .prefix("w "),
                        )
                        .on_hover_text("Line width")
                        .changed()
                    {
                        channel.line_width =
                            line_width.clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH);
                        changed = true;
                    }

                    if channel.color_override.is_some()
                        && add_control_button_with_role(
                            ui,
                            palette,
                            ControlButtonRole::Utility,
                            egui::Button::new("Reset color").small(),
                        )
                        .clicked()
                    {
                        channel.color_override = None;
                        changed = true;
                    }

                    if add_control_button_enabled(
                        ui,
                        palette,
                        !channel_loading,
                        egui::Button::new("Remove"),
                    )
                    .clicked()
                    {
                        remove_channel = Some(channel.key.clone());
                    }
                });
            });
        }

        if let Some(key) = remove_channel {
            row.channels.retain(|channel| channel.key != key);
            changed = true;
        }
    }

    view.selected_row_id = selected_row_id;
    if let Some(row_id) = remove_row_id
        && view.remove_row(row_id)
    {
        changed = true;
    }

    changed
}

fn draw_y_range_controls(
    ui: &mut egui::Ui,
    row_id: u64,
    y_range: &mut RowYRange,
    palette: VisualPalette,
) -> bool {
    let mut changed = false;

    ui.push_id(("row_y_range", row_id), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Y");
            let mut selected_mode = y_range.mode;
            egui::ComboBox::from_id_salt("mode")
                .selected_text(selected_mode.as_str())
                .show_ui(ui, |ui| {
                    for mode in YRangeMode::ALL {
                        if ui
                            .selectable_value(&mut selected_mode, mode, mode.as_str())
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
            if selected_mode != y_range.mode {
                match selected_mode {
                    YRangeMode::Auto => y_range.mode = YRangeMode::Auto,
                    YRangeMode::Manual => y_range.set_manual_from_last_auto(),
                }
                changed = true;
            }

            if y_range.mode == YRangeMode::Manual {
                let (seed_min, seed_max) = y_range.manual_seed_range();
                let mut min = y_range.min.unwrap_or(seed_min);
                if ui
                    .add(egui::DragValue::new(&mut min).speed(0.1).prefix("min "))
                    .changed()
                {
                    y_range.min = Some(min);
                    changed = true;
                }

                let mut max = y_range.max.unwrap_or(seed_max);
                if ui
                    .add(egui::DragValue::new(&mut max).speed(0.1).prefix("max "))
                    .changed()
                {
                    y_range.max = Some(max);
                    changed = true;
                }

                if add_control_button_with_role(
                    ui,
                    palette,
                    ControlButtonRole::Utility,
                    egui::Button::new("Reset").small(),
                )
                .clicked()
                {
                    y_range.set_manual_from_last_auto();
                    changed = true;
                }
            }
        });
    });

    changed
}

fn draw_plot_frame(
    painter: &egui::Painter,
    rect: egui::Rect,
    visuals: &egui::Visuals,
    palette: VisualPalette,
) {
    let industrial_skin = palette.skin == VisualSkin::Industrial;
    let stroke = if industrial_skin {
        egui::Stroke::new(1.0, palette.accent_blue_dim)
    } else {
        egui::Stroke::new(1.0, visuals.widgets.noninteractive.fg_stroke.color)
    };
    let grid_stroke = if industrial_skin {
        egui::Stroke::new(1.0, palette.grid_minor)
    } else {
        egui::Stroke::new(1.0, visuals.faint_bg_color)
    };
    let major_grid_stroke = egui::Stroke::new(1.0, palette.grid_major);

    if industrial_skin {
        painter.rect_filled(rect, 0.0, palette.plot_bg);
    }

    for i in 1..10 {
        let t = i as f32 / 10.0;
        let grid_stroke = if industrial_skin && i % 2 == 0 {
            major_grid_stroke
        } else {
            grid_stroke
        };
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

fn draw_selected_plot_frame(
    painter: &egui::Painter,
    rect: egui::Rect,
    selected: bool,
    visuals: &egui::Visuals,
    palette: VisualPalette,
) {
    if !selected {
        return;
    }

    let frame_color = if palette.skin == VisualSkin::Industrial {
        palette.selected_frame
    } else {
        visuals.selection.stroke.color
    };

    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(2.0, frame_color),
        egui::StrokeKind::Inside,
    );
}

fn plot_area_rect(rect: egui::Rect) -> egui::Rect {
    let left = if rect.width() > 180.0 {
        58.0_f32
    } else {
        12.0_f32
    }
    .min(rect.width() * 0.4);
    let bottom = if rect.height() > 120.0 {
        PLOT_BOTTOM_MARGIN
    } else {
        10.0_f32
    }
    .min(rect.height() * 0.3);
    let top = if rect.height() > 120.0 {
        PLOT_TOP_MARGIN
    } else {
        10.0_f32
    }
    .min(rect.height() * 0.3);
    let right = 18.0_f32.min(rect.width() * 0.2);

    egui::Rect::from_min_max(
        egui::pos2(rect.left() + left, rect.top() + top),
        egui::pos2(rect.right() - right, rect.bottom() - bottom),
    )
}

fn waveform_content_height(viewport_height: f32, row_count: usize) -> f32 {
    let row_count = row_count.max(1);
    let total_gap = if row_count > 1 {
        WAVEFORM_ROW_GAP * row_count.saturating_sub(1) as f32
    } else {
        0.0
    };
    let minimum_height = MIN_WAVEFORM_ROW_HEIGHT * row_count as f32 + total_gap;

    viewport_height.max(minimum_height)
}

fn row_outer_rects(rect: egui::Rect, row_count: usize) -> Vec<egui::Rect> {
    if row_count == 0 || rect.height() <= 0.0 || rect.width() <= 0.0 {
        return Vec::new();
    }

    let gap = if row_count > 1 {
        WAVEFORM_ROW_GAP.min(rect.height() / row_count as f32 * 0.2)
    } else {
        0.0
    };
    let total_gap = gap * row_count.saturating_sub(1) as f32;
    let row_height = ((rect.height() - total_gap).max(1.0) / row_count as f32).max(1.0);
    let mut row_rects = Vec::with_capacity(row_count);
    let mut top = rect.top();

    for index in 0..row_count {
        let bottom = if index + 1 == row_count {
            rect.bottom()
        } else {
            (top + row_height).min(rect.bottom())
        };
        row_rects.push(egui::Rect::from_min_max(
            egui::pos2(rect.left(), top),
            egui::pos2(rect.right(), bottom),
        ));
        top = bottom + gap;
    }

    row_rects
}

fn plot_row_id_at_position(
    pointer_pos: egui::Pos2,
    plot_rects: &[egui::Rect],
    rows: &[PlotRow],
) -> Option<u64> {
    plot_rects
        .iter()
        .position(|plot_rect| plot_rect.contains(pointer_pos))
        .and_then(|row_index| rows.get(row_index))
        .map(|row| row.id)
}

fn draw_waveform_traces(
    painter: &egui::Painter,
    rect: egui::Rect,
    display_time_range: (f64, f64),
    visuals: &egui::Visuals,
    visible_traces: &[VisibleTrace],
    y_range: RowYRange,
) {
    if visible_traces.is_empty() {
        draw_status_label(painter, rect, "No channel loaded");
        return;
    }
    let Some((time_min, time_max)) = normalized_range(display_time_range) else {
        draw_status_label(painter, rect, "No time range available");
        return;
    };

    let Some(auto_value_range) = combined_trace_value_range(visible_traces) else {
        draw_status_label(painter, rect, "No finite values available");
        return;
    };

    let time_span = time_max - time_min;
    if !time_span.is_finite() || time_span <= 0.0 {
        draw_status_label(painter, rect, "Invalid time range");
        return;
    }

    let Some((value_min, value_max)) = display_value_range(auto_value_range, y_range) else {
        draw_status_label(painter, rect, "Invalid value range");
        return;
    };
    let value_span = value_max - value_min;
    if !value_span.is_finite() || value_span <= 0.0 {
        draw_status_label(painter, rect, "Invalid value range");
        return;
    }

    let to_screen = |time: f64, value: f64| -> egui::Pos2 {
        let x_t = ((time - time_min) / time_span) as f32;
        let y_t = ((value - value_min) / value_span) as f32;
        egui::pos2(
            egui::lerp(rect.left()..=rect.right(), x_t),
            egui::lerp(rect.bottom()..=rect.top(), y_t),
        )
    };

    let trace_painter = painter.with_clip_rect(rect);
    for visible in visible_traces {
        debug_assert_eq!(visible.key.channel_path, visible.channel_path);
        let line_width = visible
            .line_width
            .clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH);
        let vertical_stroke = egui::Stroke::new(
            (line_width * 0.8).max(1.0),
            visible.color.linear_multiply(0.45),
        );
        let line_stroke = egui::Stroke::new(line_width, visible.color);

        match &visible.data {
            VisibleTraceData::Envelope(envelope)
            | VisibleTraceData::DenseStepEnvelope(envelope) => {
                let mut upper = Vec::with_capacity(envelope.buckets.len());
                let mut lower = Vec::with_capacity(envelope.buckets.len());
                for bucket in &envelope.buckets {
                    let display_time = visible.time_transform.to_display(bucket.time);
                    let min_point = to_screen(display_time, f64::from(bucket.min));
                    let max_point = to_screen(display_time, f64::from(bucket.max));
                    trace_painter.line_segment([min_point, max_point], vertical_stroke);
                    lower.push(min_point);
                    upper.push(max_point);
                }

                if upper.len() >= 2 {
                    trace_painter.line(upper, line_stroke);
                    trace_painter.line(lower, line_stroke);
                }
            }
            VisibleTraceData::RawStep(raw_step) => {
                draw_raw_step_trace(
                    &trace_painter,
                    raw_step,
                    visible.time_transform,
                    line_stroke,
                    to_screen,
                );
            }
        }
    }

    draw_axis_labels(
        painter,
        rect,
        visible_traces,
        (time_min, time_max),
        (value_min, value_max),
        visuals,
    );
}

fn draw_raw_step_trace(
    painter: &egui::Painter,
    raw_step: &RawStepTrace,
    time_transform: TimeTransform,
    stroke: egui::Stroke,
    to_screen: impl Fn(f64, f64) -> egui::Pos2,
) {
    let Some(first) = raw_step.samples.first().copied() else {
        return;
    };
    let Some(raw_time_range) = raw_step.time_range.and_then(normalized_range) else {
        return;
    };

    let mut points = Vec::with_capacity(raw_step.samples.len().saturating_mul(2).max(2));
    points.push(to_screen(
        time_transform.to_display(first.time.max(raw_time_range.0)),
        f64::from(first.value),
    ));

    let mut previous = first;
    for current in raw_step.samples.iter().skip(1).copied() {
        let display_time = time_transform.to_display(current.time);
        points.push(to_screen(display_time, f64::from(previous.value)));
        points.push(to_screen(display_time, f64::from(current.value)));
        previous = current;
    }

    if raw_time_range.1 > previous.time {
        points.push(to_screen(
            time_transform.to_display(raw_time_range.1),
            f64::from(previous.value),
        ));
    }

    if points.len() >= 2 {
        painter.line(points, stroke);
    }
}

fn trace_value_range(trace: &VisibleTrace) -> Option<(f64, f64)> {
    match &trace.data {
        VisibleTraceData::Envelope(envelope) | VisibleTraceData::DenseStepEnvelope(envelope) => {
            envelope.value_range
        }
        VisibleTraceData::RawStep(raw_step) => raw_step.value_range,
    }
}

fn combined_trace_value_range(visible_traces: &[VisibleTrace]) -> Option<(f64, f64)> {
    let mut combined_value_range = None;
    for visible in visible_traces {
        if let Some((min, max)) = trace_value_range(visible) {
            combined_value_range = extend_range(combined_value_range, min);
            combined_value_range = extend_range(combined_value_range, max);
        }
    }

    combined_value_range
}

fn display_value_range(auto_value_range: (f64, f64), y_range: RowYRange) -> Option<(f64, f64)> {
    match y_range.mode {
        YRangeMode::Auto => Some(padded_range(auto_value_range.0, auto_value_range.1)),
        YRangeMode::Manual => {
            let auto_range = padded_range(auto_value_range.0, auto_value_range.1);
            let min = y_range.min.unwrap_or(auto_range.0);
            let max = y_range.max.unwrap_or(auto_range.1);
            normalized_y_range(min, max)
        }
    }
}

fn normalized_y_range(min: f64, max: f64) -> Option<(f64, f64)> {
    if !min.is_finite() || !max.is_finite() {
        return None;
    }

    if min == max {
        return Some(padded_range(min, max));
    }

    Some((min.min(max), min.max(max)))
}

fn trace_source_sample_count(trace: &VisibleTrace) -> usize {
    match &trace.data {
        VisibleTraceData::Envelope(envelope) | VisibleTraceData::DenseStepEnvelope(envelope) => {
            envelope.source_sample_count
        }
        VisibleTraceData::RawStep(raw_step) => raw_step.source_sample_count,
    }
}

fn trace_bucket_count(trace: &VisibleTrace) -> usize {
    match &trace.data {
        VisibleTraceData::Envelope(envelope) | VisibleTraceData::DenseStepEnvelope(envelope) => {
            envelope.bucket_count()
        }
        VisibleTraceData::RawStep(_) => 0,
    }
}

fn trace_draw_point_count(trace: &VisibleTrace) -> usize {
    match &trace.data {
        VisibleTraceData::Envelope(envelope) | VisibleTraceData::DenseStepEnvelope(envelope) => {
            envelope.draw_point_count()
        }
        VisibleTraceData::RawStep(raw_step) => raw_step.samples.len().saturating_mul(2),
    }
}

fn trace_step_kind(trace: &VisibleTrace) -> Option<StepTraceKind> {
    match &trace.data {
        VisibleTraceData::RawStep(raw_step) => Some(raw_step.kind),
        VisibleTraceData::Envelope(_) | VisibleTraceData::DenseStepEnvelope(_) => None,
    }
}

fn visible_envelope_bucket_count(rect: egui::Rect) -> usize {
    (rect.width().round() as usize)
        .clamp(MIN_VISIBLE_ENVELOPE_BUCKETS, MAX_VISIBLE_ENVELOPE_BUCKETS)
}

fn preview_envelope_bucket_count(requested_bucket_count: usize) -> usize {
    (requested_bucket_count / PREVIEW_BUCKET_DIVISOR)
        .max(MIN_VISIBLE_ENVELOPE_BUCKETS)
        .min(MAX_PREVIEW_ENVELOPE_BUCKETS)
}

fn preview_needed_for_workload(
    dataset_sample_count: usize,
    visible_source_sample_count: usize,
    line_channel_count: usize,
    step_channel_count: usize,
) -> bool {
    if dataset_sample_count < PREVIEW_MIN_DATASET_SAMPLES {
        return false;
    }

    let line_sample_work = visible_source_sample_count.saturating_mul(line_channel_count);
    line_sample_work >= PREVIEW_MIN_VISIBLE_LINE_SAMPLE_WORK
        || (step_channel_count > 0
            && visible_source_sample_count >= PREVIEW_MIN_VISIBLE_STEP_SAMPLES)
}

fn should_use_line_tile_lod(plan: &parquet_waveform::MinMaxEnvelopePlan) -> bool {
    plan.source_sample_count >= LINE_TILE_MIN_SOURCE_SAMPLES
        && plan.bucket_size >= LINE_TILE_MIN_BUCKET_SIZE
}

fn channel_color(index: usize, dark_mode: bool) -> egui::Color32 {
    const DARK_COLORS: [(u8, u8, u8); 8] = [
        (80, 190, 255),
        (255, 176, 70),
        (120, 220, 120),
        (255, 110, 150),
        (190, 150, 255),
        (95, 220, 210),
        (240, 220, 90),
        (210, 210, 220),
    ];
    const LIGHT_COLORS: [(u8, u8, u8); 8] = [
        (0, 94, 155),
        (190, 95, 0),
        (32, 130, 60),
        (185, 40, 80),
        (110, 80, 190),
        (0, 130, 130),
        (150, 125, 0),
        (80, 80, 90),
    ];

    let colors = if dark_mode { DARK_COLORS } else { LIGHT_COLORS };
    let (r, g, b) = colors[index % colors.len()];
    egui::Color32::from_rgb(r, g, b)
}

fn draw_axis_labels(
    painter: &egui::Painter,
    rect: egui::Rect,
    visible_traces: &[VisibleTrace],
    time_range: (f64, f64),
    value_range: (f64, f64),
    visuals: &egui::Visuals,
) {
    let text_color = visuals.text_color();
    let weak_color = visuals.weak_text_color();
    let font = egui::FontId::monospace(12.0);
    let channel_label = visible_traces
        .iter()
        .take(3)
        .map(|visible| visible.display_label.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let channel_label = if visible_traces.len() > 3 {
        format!("{channel_label}, ...")
    } else {
        channel_label
    };
    let source_sample_count = visible_traces
        .first()
        .map(trace_source_sample_count)
        .unwrap_or_default();
    let raw_sample_count = visible_traces
        .first()
        .map(|visible| visible.sample_count)
        .unwrap_or_default();
    let bucket_count: usize = visible_traces.iter().map(trace_bucket_count).sum();
    let draw_point_count: usize = visible_traces.iter().map(trace_draw_point_count).sum();
    let step_count = visible_traces
        .iter()
        .filter(|visible| visible.draw_mode == DrawMode::Step)
        .count();
    let edge_step_count = visible_traces
        .iter()
        .filter(|visible| trace_step_kind(visible) == Some(StepTraceKind::ChangePoints))
        .count();
    let dense_step_count = visible_traces
        .iter()
        .filter(|visible| matches!(&visible.data, VisibleTraceData::DenseStepEnvelope(_)))
        .count();

    painter.text(
        rect.left_top() + egui::vec2(0.0, -14.0),
        egui::Align2::LEFT_TOP,
        format!(
            "{}  ch={}  step={}  edge={}  dense={}  visible_samples={}  raw_samples={}  buckets={}  draw_points={}",
            channel_label,
            visible_traces.len(),
            step_count,
            edge_step_count,
            dense_step_count,
            source_sample_count,
            raw_sample_count,
            bucket_count,
            draw_point_count
        ),
        font.clone(),
        text_color,
    );
    painter.text(
        rect.left_bottom() + egui::vec2(0.0, 6.0),
        egui::Align2::LEFT_TOP,
        format!("{:.6}", time_range.0),
        font.clone(),
        weak_color,
    );
    painter.text(
        rect.right_bottom() + egui::vec2(0.0, 6.0),
        egui::Align2::RIGHT_TOP,
        format!("{:.6} s", time_range.1),
        font.clone(),
        weak_color,
    );
    painter.text(
        rect.left_top() + egui::vec2(-8.0, 0.0),
        egui::Align2::RIGHT_TOP,
        format!("{:.3}", value_range.1),
        font.clone(),
        weak_color,
    );
    painter.text(
        rect.left_bottom() + egui::vec2(-8.0, 0.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{:.3}", value_range.0),
        font,
        weak_color,
    );
}

fn draw_hover_line(
    painter: &egui::Painter,
    rect: egui::Rect,
    hover_x: Option<f64>,
    time_range: Option<(f64, f64)>,
    visuals: &egui::Visuals,
) {
    let Some(hover_x) = hover_x else {
        return;
    };
    let Some(time_range) = time_range else {
        return;
    };
    let Some(x) = plot_x_for_time(hover_x, rect, time_range) else {
        return;
    };

    let stroke = egui::Stroke::new(1.25, visuals.selection.stroke.color);
    painter.line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        stroke,
    );
}

fn draw_cursor_lines(
    painter: &egui::Painter,
    rect: egui::Rect,
    analysis: &AnalysisState,
    time_range: Option<(f64, f64)>,
    visuals: &egui::Visuals,
) {
    let Some(time_range) = time_range else {
        return;
    };

    if let Some(cursor_a) = analysis.cursor_a {
        draw_cursor_line(
            painter,
            rect,
            cursor_a,
            time_range,
            "A",
            cursor_a_color(visuals),
        );
    }
    if let Some(cursor_b) = analysis.cursor_b {
        draw_cursor_line(
            painter,
            rect,
            cursor_b,
            time_range,
            "B",
            cursor_b_color(visuals),
        );
    }
}

fn draw_cursor_line(
    painter: &egui::Painter,
    rect: egui::Rect,
    cursor_time: f64,
    time_range: (f64, f64),
    label: &str,
    color: egui::Color32,
) {
    let Some(x) = plot_x_for_time(cursor_time, rect, time_range) else {
        return;
    };

    let stroke = egui::Stroke::new(1.5, color);
    draw_dashed_vertical_line(painter, x, rect, stroke);
    painter.text(
        egui::pos2((x + 4.0).min(rect.right() - 10.0), rect.top() + 4.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(12.0),
        color,
    );
}

fn draw_dashed_vertical_line(
    painter: &egui::Painter,
    x: f32,
    rect: egui::Rect,
    stroke: egui::Stroke,
) {
    let mut y = rect.top();
    let dash = 7.0;
    let gap = 5.0;
    while y < rect.bottom() {
        let next_y = (y + dash).min(rect.bottom());
        painter.line_segment([egui::pos2(x, y), egui::pos2(x, next_y)], stroke);
        y = next_y + gap;
    }
}

fn cursor_a_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_rgb(70, 190, 255)
    } else {
        egui::Color32::from_rgb(0, 105, 185)
    }
}

fn cursor_b_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_rgb(255, 105, 135)
    } else {
        egui::Color32::from_rgb(190, 35, 65)
    }
}

fn draw_box_zoom_overlay(
    painter: &egui::Painter,
    rect: egui::Rect,
    box_zoom: Option<&BoxZoomDrag>,
    visuals: &egui::Visuals,
) {
    let Some(box_zoom) = box_zoom else {
        return;
    };
    if box_zoom.plot_rect != rect {
        return;
    }
    let Some(selection_rect) = box_zoom_screen_rect(box_zoom) else {
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

fn box_zoom_screen_rect(box_zoom: &BoxZoomDrag) -> Option<egui::Rect> {
    if box_zoom.plot_rect.width() <= 1.0 {
        return None;
    }

    let start_x = box_zoom
        .start_pos
        .x
        .clamp(box_zoom.plot_rect.left(), box_zoom.plot_rect.right());
    let current_x = box_zoom
        .current_pos
        .x
        .clamp(box_zoom.plot_rect.left(), box_zoom.plot_rect.right());
    if (current_x - start_x).abs() <= f32::EPSILON {
        return None;
    }

    Some(egui::Rect::from_min_max(
        egui::pos2(start_x.min(current_x), box_zoom.plot_rect.top()),
        egui::pos2(start_x.max(current_x), box_zoom.plot_rect.bottom()),
    ))
}

fn draw_hover_readout(
    painter: &egui::Painter,
    rect: egui::Rect,
    hover_x: Option<f64>,
    display_time_range: Option<(f64, f64)>,
    visuals: &egui::Visuals,
    visible_traces: &[VisibleTrace],
) {
    let Some(hover_x) = hover_x else {
        return;
    };
    let Some(hover_line_x) =
        display_time_range.and_then(|time_range| plot_x_for_time(hover_x, rect, time_range))
    else {
        return;
    };

    let values = visible_traces
        .iter()
        .filter_map(|trace| trace.hover_value.map(|value| (trace, value)))
        .take(MAX_HOVER_READOUT_CHANNELS)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return;
    }

    let extra_count = visible_traces
        .iter()
        .filter(|trace| trace.hover_value.is_some())
        .count()
        .saturating_sub(values.len());
    let mut lines = Vec::with_capacity(values.len() + 2);
    lines.push(format!("x {:.6} s", hover_x));
    for (trace, value) in &values {
        lines.push(format!(
            "{}  {}",
            compact_channel_label(trace_label(trace), 24),
            format_hover_value(*value)
        ));
    }
    if extra_count > 0 {
        lines.push(format!("+{extra_count} more"));
    }

    let font = egui::FontId::monospace(11.0);
    let line_height = 15.0;
    let max_panel_width = (rect.width() - 8.0).max(64.0).min(340.0);
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
    painter.text(
        egui::pos2(panel_rect.left() + 8.0, y),
        egui::Align2::LEFT_TOP,
        &lines[0],
        font.clone(),
        text_color,
    );
    y += line_height;

    for ((trace, _), line) in values.iter().zip(lines.iter().skip(1)) {
        let swatch_y = y + line_height * 0.5;
        painter.line_segment(
            [
                egui::pos2(panel_rect.left() + 8.0, swatch_y),
                egui::pos2(panel_rect.left() + 18.0, swatch_y),
            ],
            egui::Stroke::new(2.0, trace.color),
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

fn draw_row_marker(
    painter: &egui::Painter,
    rect: egui::Rect,
    row_index: usize,
    selected: bool,
    visuals: &egui::Visuals,
) {
    let color = if selected {
        visuals.selection.stroke.color
    } else {
        visuals.weak_text_color()
    };
    painter.text(
        rect.right_top() + egui::vec2(-4.0, 4.0),
        egui::Align2::RIGHT_TOP,
        format!("Row {}", row_index + 1),
        egui::FontId::monospace(12.0),
        color,
    );
}

fn draw_row_placeholder(
    painter: &egui::Painter,
    rect: egui::Rect,
    workspace: &WorkspaceDataState,
    selected_channel: Option<&ChannelKey>,
    row_index: usize,
) {
    let schema = selected_channel
        .and_then(|key| workspace.source(key.source_id))
        .and_then(|source| source.schema.as_ref());
    let label = match schema {
        Some(schema) if schema.time_column.is_some() && selected_channel.is_some() => {
            let key = selected_channel.expect("checked above");
            let channel_label = channel_label_or_fallback(workspace, key);
            format!(
                "Row {} is empty. Load time + {channel_label} into the selected row.",
                row_index + 1
            )
        }
        Some(_) => "Schema loaded. Selectable waveform data is not ready yet.".to_owned(),
        None => "Load a Parquet schema to detect time and channel columns.".to_owned(),
    };

    draw_status_label(painter, rect, &label);
}

fn draw_status_label(painter: &egui::Painter, rect: egui::Rect, label: &str) {
    painter.text(
        rect.left_top() + egui::vec2(16.0, 16.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(14.0),
        egui::Color32::GRAY,
    );
}

fn hover_value_at_time(
    time: &[f64],
    values: &[f32],
    target_time: f64,
    draw_mode: DrawMode,
) -> Option<f32> {
    if !sample_time_contains(time, values.len(), target_time) {
        return None;
    }

    match draw_mode {
        DrawMode::Line => nearest_sample_value(time, values, target_time),
        DrawMode::Step => step_sample_value_at_time(time, values, target_time),
    }
}

fn nearest_sample_index(time: &[f64], target_time: f64, sample_count: usize) -> Option<usize> {
    if !target_time.is_finite() {
        return None;
    }

    let sample_count = time.len().min(sample_count);
    if sample_count == 0 {
        return None;
    }

    let insertion_index =
        time[..sample_count].partition_point(|sample_time| *sample_time < target_time);
    let candidates = [
        insertion_index.checked_sub(1),
        (insertion_index < sample_count).then_some(insertion_index),
    ];
    let mut best = None;

    for index in candidates.into_iter().flatten() {
        let sample_time = time[index];
        if !sample_time.is_finite() {
            continue;
        }

        let distance = (sample_time - target_time).abs();
        match best {
            Some((best_distance, _)) if best_distance <= distance => {}
            _ => best = Some((distance, index)),
        }
    }

    best.map(|(_, index)| index)
}

fn nearest_sample_index_in_range(
    time: &[f64],
    target_time: f64,
    sample_count: usize,
) -> Option<usize> {
    sample_time_contains(time, sample_count, target_time)
        .then(|| nearest_sample_index(time, target_time, sample_count))
        .flatten()
}

fn sample_time_contains(time: &[f64], sample_count: usize, target_time: f64) -> bool {
    if !target_time.is_finite() {
        return false;
    }

    let sample_count = time.len().min(sample_count);
    if sample_count == 0 {
        return false;
    }
    let Some(first) = time.first().copied().filter(|value| value.is_finite()) else {
        return false;
    };
    let Some(last) = time
        .get(sample_count.saturating_sub(1))
        .copied()
        .filter(|value| value.is_finite())
    else {
        return false;
    };

    first <= last && (first..=last).contains(&target_time)
}

fn sample_value_at_index(values: &[f32], index: usize) -> Option<f64> {
    values
        .get(index)
        .copied()
        .filter(|value| value.is_finite())
        .map(f64::from)
}

fn nearest_sample_value(time: &[f64], values: &[f32], target_time: f64) -> Option<f32> {
    if !target_time.is_finite() {
        return None;
    }

    let sample_count = time.len().min(values.len());
    if sample_count == 0 {
        return None;
    }

    let insertion_index =
        time[..sample_count].partition_point(|sample_time| *sample_time < target_time);
    let candidates = [
        insertion_index.checked_sub(1),
        (insertion_index < sample_count).then_some(insertion_index),
    ];
    let mut best = None;

    for index in candidates.into_iter().flatten() {
        let sample_time = time[index];
        let value = values[index];
        if !sample_time.is_finite() || !value.is_finite() {
            continue;
        }

        let distance = (sample_time - target_time).abs();
        match best {
            Some((best_distance, _)) if best_distance <= distance => {}
            _ => best = Some((distance, value)),
        }
    }

    best.map(|(_, value)| value)
}

fn nearest_sample_value_in_range(time: &[f64], values: &[f32], target_time: f64) -> Option<f32> {
    sample_time_contains(time, values.len(), target_time)
        .then(|| nearest_sample_value(time, values, target_time))
        .flatten()
}

fn step_sample_value_at_time(time: &[f64], values: &[f32], target_time: f64) -> Option<f32> {
    if !target_time.is_finite() {
        return None;
    }

    let sample_count = time.len().min(values.len());
    if sample_count == 0 {
        return None;
    }

    let index = time[..sample_count]
        .partition_point(|sample_time| *sample_time <= target_time)
        .saturating_sub(1)
        .min(sample_count - 1);
    let sample_time = time[index];
    let value = values[index];
    (sample_time.is_finite() && value.is_finite()).then_some(value)
}

fn time_at_plot_x(x: f32, rect: egui::Rect, time_range: (f64, f64)) -> Option<f64> {
    let (start, end) = normalized_range(time_range)?;
    if rect.width() <= 1.0 {
        return None;
    }

    let ratio = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
    Some(start + (end - start) * ratio)
}

fn box_zoom_view_range(
    box_zoom: &BoxZoomDrag,
    full_range: (f64, f64),
    min_span: f64,
) -> Option<(f64, f64)> {
    let selection_rect = box_zoom_screen_rect(box_zoom)?;
    if selection_rect.width() < BOX_ZOOM_MIN_PIXEL_WIDTH {
        return None;
    }

    let start = time_at_plot_x(
        selection_rect.left(),
        box_zoom.plot_rect,
        box_zoom.view_range,
    )?;
    let end = time_at_plot_x(
        selection_rect.right(),
        box_zoom.plot_rect,
        box_zoom.view_range,
    )?;
    Some(clamp_view_range((start, end), full_range, min_span))
}

fn plot_x_for_time(time: f64, rect: egui::Rect, time_range: (f64, f64)) -> Option<f32> {
    let (start, end) = normalized_range(time_range)?;
    if !time.is_finite() || rect.width() <= 1.0 {
        return None;
    }

    let ratio = ((time - start) / (end - start)) as f32;
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        return None;
    }

    Some(egui::lerp(rect.left()..=rect.right(), ratio))
}

fn trace_label(trace: &VisibleTrace) -> &str {
    trace.display_label.as_str()
}

fn compact_channel_label(label: &str, max_chars: usize) -> String {
    let char_count = label.chars().count();
    if char_count <= max_chars {
        return label.to_owned();
    }

    if max_chars <= 3 {
        return label.chars().take(max_chars).collect();
    }

    let mut compact = label.chars().take(max_chars - 3).collect::<String>();
    compact.push_str("...");
    compact
}

fn add_compact_label(ui: &mut egui::Ui, text: &str, max_chars: usize) -> egui::Response {
    let compact = compact_channel_label(text, max_chars);
    ui.add(egui::Label::new(compact).truncate())
}

fn add_compact_monospace_label(ui: &mut egui::Ui, text: &str, max_chars: usize) -> egui::Response {
    let compact = compact_channel_label(text, max_chars);
    ui.add(egui::Label::new(egui::RichText::new(compact).monospace()).truncate())
}

fn format_numeric_value(value: f64) -> String {
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

fn format_hover_value(value: f32) -> String {
    format_numeric_value(f64::from(value))
}

fn format_cursor_time_seconds(time: Option<f64>) -> String {
    time.map(|time| format!("{time:.9} s"))
        .unwrap_or_else(|| "-".to_owned())
}

fn format_optional_cursor_value(value: Option<f32>) -> String {
    value
        .map(format_hover_value)
        .unwrap_or_else(|| "-".to_owned())
}

fn format_optional_stat_value(value: Option<f64>) -> String {
    value
        .map(format_numeric_value)
        .unwrap_or_else(|| "-".to_owned())
}

fn format_range(range: Option<(f64, f64)>) -> String {
    match range {
        Some((start, end)) => format!("{start:.6} .. {end:.6}"),
        None => "-".to_owned(),
    }
}

fn bytes_to_mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn padded_range(min: f64, max: f64) -> (f64, f64) {
    if min == max {
        let pad = min.abs().max(1.0) * 0.05;
        return (min - pad, max + pad);
    }

    let pad = (max - min).abs() * 0.05;
    (min - pad, max + pad)
}

fn extend_range(range: Option<(f64, f64)>, value: f64) -> Option<(f64, f64)> {
    if !value.is_finite() {
        return range;
    }

    match range {
        Some((min, max)) => Some((min.min(value), max.max(value))),
        None => Some((value, value)),
    }
}

fn visible_sample_range(
    time: &[f64],
    value_count: usize,
    time_range: (f64, f64),
) -> Option<VisibleSampleRange> {
    let (range_start, range_end) = normalized_range(time_range)?;
    let sample_count = time.len().min(value_count);
    if sample_count == 0 {
        return None;
    }

    let time = &time[..sample_count];
    let start = time.partition_point(|time| *time < range_start);
    let end = time.partition_point(|time| *time <= range_end);
    let context_start = start.saturating_sub(usize::from(start > 0));
    Some(VisibleSampleRange {
        range_start,
        range_end,
        start,
        end,
        context_start,
        source_sample_count: end.saturating_sub(start),
        draw_sample_count: end.saturating_sub(context_start),
    })
}

fn build_raw_step_trace(
    time: &[f64],
    values: &[f32],
    time_range: (f64, f64),
    max_samples: usize,
) -> Option<RawStepTrace> {
    let sample_range = visible_sample_range(time, values.len(), time_range)?;
    if sample_range.draw_sample_count == 0 || sample_range.draw_sample_count > max_samples {
        return None;
    }

    let mut samples = Vec::with_capacity(sample_range.draw_sample_count);
    let mut value_range = None;

    if sample_range.context_start < sample_range.start {
        let value = values[sample_range.context_start];
        if value.is_finite() {
            samples.push(StepSample {
                time: sample_range.range_start,
                value,
            });
            value_range = extend_range(value_range, f64::from(value));
        }
    }

    for index in sample_range.start..sample_range.end {
        let sample_time = time[index];
        let value = values[index];
        if !sample_time.is_finite() || !value.is_finite() {
            continue;
        }

        samples.push(StepSample {
            time: sample_time.clamp(sample_range.range_start, sample_range.range_end),
            value,
        });
        value_range = extend_range(value_range, f64::from(value));
    }

    if samples.is_empty() || value_range.is_none() {
        return None;
    }

    Some(RawStepTrace {
        samples,
        source_sample_count: sample_range.source_sample_count,
        time_range: Some((sample_range.range_start, sample_range.range_end)),
        value_range,
        kind: StepTraceKind::RawSamples,
    })
}

#[cfg(test)]
fn build_change_point_step_trace(
    time: &[f64],
    values: &[f32],
    time_range: (f64, f64),
    max_change_points: usize,
) -> Option<RawStepTrace> {
    match build_change_point_step_trace_result(time, values, time_range, max_change_points) {
        StepTraceBuildResult::Trace(trace) => Some(trace),
        StepTraceBuildResult::TooManyChangePoints { .. } | StepTraceBuildResult::Empty => None,
    }
}

#[cfg(test)]
fn build_change_point_step_trace_result(
    time: &[f64],
    values: &[f32],
    time_range: (f64, f64),
    max_change_points: usize,
) -> StepTraceBuildResult {
    let Some(sample_range) = visible_sample_range(time, values.len(), time_range) else {
        return StepTraceBuildResult::Empty;
    };
    if sample_range.draw_sample_count == 0 {
        return StepTraceBuildResult::Empty;
    }

    let mut samples = Vec::new();
    let mut value_range = None;
    let mut previous_value = None;
    let mut change_points = 0usize;

    if sample_range.context_start < sample_range.start {
        let value = values[sample_range.context_start];
        if value.is_finite() {
            samples.push(StepSample {
                time: sample_range.range_start,
                value,
            });
            value_range = extend_range(value_range, f64::from(value));
            previous_value = Some(value);
        }
    }

    for index in sample_range.start..sample_range.end {
        let sample_time = time[index];
        let value = values[index];
        if !sample_time.is_finite() || !value.is_finite() {
            continue;
        }

        value_range = extend_range(value_range, f64::from(value));
        match previous_value {
            Some(previous) if previous == value => {}
            Some(_) => {
                if change_points >= max_change_points {
                    return StepTraceBuildResult::TooManyChangePoints {
                        source_sample_count: sample_range.source_sample_count,
                    };
                }
                samples.push(StepSample {
                    time: sample_time.clamp(sample_range.range_start, sample_range.range_end),
                    value,
                });
                previous_value = Some(value);
                change_points += 1;
            }
            None => {
                samples.push(StepSample {
                    time: sample_time.clamp(sample_range.range_start, sample_range.range_end),
                    value,
                });
                previous_value = Some(value);
            }
        }
    }

    if samples.is_empty() || value_range.is_none() {
        return StepTraceBuildResult::Empty;
    }

    StepTraceBuildResult::Trace(RawStepTrace {
        samples,
        source_sample_count: sample_range.source_sample_count,
        time_range: Some((sample_range.range_start, sample_range.range_end)),
        value_range,
        kind: StepTraceKind::ChangePoints,
    })
}

#[cfg(test)]
fn build_step_edge_cache(time: &[f64], values: &[f32]) -> StepEdgeCache {
    build_step_edge_cache_with_limit(time, values, None).expect("unbounded edge cache")
}

fn build_step_edge_cache_with_limit(
    time: &[f64],
    values: &[f32],
    view_limit: Option<(f64, f64, usize)>,
) -> Result<StepEdgeCache, ()> {
    let started = Instant::now();
    let sample_count = time.len().min(values.len());
    let mut edges = Vec::new();
    let mut previous_value = None;
    let mut visible_changes = 0usize;
    let cached_time_range = view_limit.map(|(start, end, _)| (start, end));
    let (start_index, end_index) = if let Some((start, end)) = cached_time_range {
        let start_index = time[..sample_count].partition_point(|t| *t < start);
        let end_index = time[..sample_count].partition_point(|t| *t <= end);
        // Preserve the full cache's last finite state even across missing values.
        // Only this context edge and the requested range need to be retained.
        if let Some(index) = (0..start_index)
            .rev()
            .find(|&i| time[i].is_finite() && values[i].is_finite())
        {
            edges.push(StepEdge {
                index,
                time: time[index],
                value: values[index],
            });
            previous_value = Some(values[index]);
        }
        (start_index, end_index)
    } else {
        (0, sample_count)
    };

    for index in start_index..end_index {
        let sample_time = time[index];
        let value = values[index];
        if !sample_time.is_finite() || !value.is_finite() {
            continue;
        }

        match previous_value {
            Some(previous) if previous == value => {}
            _ => {
                // Count precisely the changes the cached range lookup would
                // return, including the last finite state before the viewport.
                // Once dense Step must use an envelope, stop scanning and drop
                // the partial cache instead of collecting every remaining edge.
                if let Some((start, end, limit)) = view_limit {
                    if previous_value.is_some() && sample_time >= start && sample_time <= end {
                        if visible_changes >= limit {
                            return Err(());
                        }
                        visible_changes += 1;
                    }
                }
                edges.push(StepEdge {
                    index,
                    time: sample_time,
                    value,
                });
                previous_value = Some(value);
            }
        }
    }

    Ok(StepEdgeCache {
        sample_count,
        cached_time_range,
        edges,
        elapsed: started.elapsed(),
    })
}

fn build_change_point_step_trace_from_cache(
    time: &[f64],
    edge_cache: &StepEdgeCache,
    time_range: (f64, f64),
    max_change_points: usize,
) -> StepTraceBuildResult {
    let Some(sample_range) = visible_sample_range(time, edge_cache.sample_count, time_range) else {
        return StepTraceBuildResult::Empty;
    };
    if sample_range.draw_sample_count == 0 || edge_cache.edges.is_empty() {
        return StepTraceBuildResult::Empty;
    }

    let edge_start = edge_cache
        .edges
        .partition_point(|edge| edge.time < sample_range.range_start);
    let edge_end = edge_cache
        .edges
        .partition_point(|edge| edge.time <= sample_range.range_end);
    let visible_edge_count = edge_end.saturating_sub(edge_start);
    let has_context = edge_start > 0;
    let change_points = if has_context {
        visible_edge_count
    } else {
        visible_edge_count.saturating_sub(1)
    };

    if change_points > max_change_points {
        return StepTraceBuildResult::TooManyChangePoints {
            source_sample_count: sample_range.source_sample_count,
        };
    }

    let mut samples = Vec::with_capacity(visible_edge_count + usize::from(has_context));
    let mut value_range = None;

    if has_context {
        let edge = edge_cache.edges[edge_start - 1];
        samples.push(StepSample {
            time: sample_range.range_start,
            value: edge.value,
        });
        value_range = extend_range(value_range, f64::from(edge.value));
    }

    for edge in &edge_cache.edges[edge_start..edge_end] {
        samples.push(StepSample {
            time: edge
                .time
                .clamp(sample_range.range_start, sample_range.range_end),
            value: edge.value,
        });
        value_range = extend_range(value_range, f64::from(edge.value));
    }

    if samples.is_empty() || value_range.is_none() {
        return StepTraceBuildResult::Empty;
    }

    StepTraceBuildResult::Trace(RawStepTrace {
        samples,
        source_sample_count: sample_range.source_sample_count,
        time_range: Some((sample_range.range_start, sample_range.range_end)),
        value_range,
        kind: StepTraceKind::ChangePoints,
    })
}

fn min_view_span(full_range: (f64, f64), sample_count: usize) -> f64 {
    let full_span = full_range.1 - full_range.0;
    if !full_span.is_finite() || full_span <= 0.0 {
        return f64::EPSILON;
    }

    let sample_span = if sample_count > 1 {
        full_span / (sample_count - 1) as f64
    } else {
        full_span
    };

    sample_span.max(full_span * 1.0e-9).max(f64::EPSILON)
}

fn clamp_view_range(range: (f64, f64), full_range: (f64, f64), min_span: f64) -> (f64, f64) {
    let Some((full_start, full_end)) = normalized_range(full_range) else {
        return full_range;
    };
    let full_span = full_end - full_start;
    if full_span <= 0.0 {
        return (full_start, full_end);
    }

    let (start, end) = normalized_range(range).unwrap_or((full_start, full_end));
    let span = end - start;
    if start >= full_start && end <= full_end && span >= min_span.min(full_span) {
        // Reconstructing an already valid range from its center can round an
        // endpoint and unnecessarily invalidate every source's envelope cache.
        return (start, end);
    }
    let span = span.clamp(min_span.min(full_span), full_span);
    let center = ((start + end) * 0.5).clamp(full_start, full_end);
    range_with_span_around(center, span, (full_start, full_end))
}

fn pan_view_range(
    range: (f64, f64),
    full_range: (f64, f64),
    shift: f64,
    min_span: f64,
) -> (f64, f64) {
    let (start, end) = clamp_view_range(range, full_range, min_span);
    let span = end - start;
    range_with_start(start + shift, span, full_range)
}

fn wheel_zoom_sensitivity(sample_count: usize) -> f64 {
    if sample_count <= MIN_WHEEL_ZOOM_SAMPLE_COUNT {
        return MIN_WHEEL_ZOOM_SENSITIVITY;
    }
    if sample_count >= MAX_WHEEL_ZOOM_SAMPLE_COUNT {
        return MAX_WHEEL_ZOOM_SENSITIVITY;
    }

    let min_samples = MIN_WHEEL_ZOOM_SAMPLE_COUNT as f64;
    let max_samples = MAX_WHEEL_ZOOM_SAMPLE_COUNT as f64;
    let sample_count = sample_count as f64;
    let t = ((sample_count.ln() - min_samples.ln()) / (max_samples.ln() - min_samples.ln()))
        .clamp(0.0, 1.0);

    MIN_WHEEL_ZOOM_SENSITIVITY + (MAX_WHEEL_ZOOM_SENSITIVITY - MIN_WHEEL_ZOOM_SENSITIVITY) * t
}

fn x_zoom_factor(
    zoom_delta: f32,
    scroll_delta_y: f32,
    wheel_zoom_enabled: bool,
    wheel_zoom_sensitivity: f64,
) -> f64 {
    if zoom_delta != 1.0 {
        1.0 / f64::from(zoom_delta)
    } else if wheel_zoom_enabled && scroll_delta_y != 0.0 {
        (-f64::from(scroll_delta_y) * wheel_zoom_sensitivity).exp()
    } else {
        1.0
    }
}

fn zoom_view_range(
    range: (f64, f64),
    full_range: (f64, f64),
    anchor_ratio: f64,
    zoom_factor: f64,
    min_span: f64,
) -> (f64, f64) {
    let Some((full_start, full_end)) = normalized_range(full_range) else {
        return full_range;
    };
    let full_span = full_end - full_start;
    if full_span <= 0.0 {
        return (full_start, full_end);
    }

    let (start, end) = clamp_view_range(range, (full_start, full_end), min_span);
    let span = end - start;
    let anchor_ratio = anchor_ratio.clamp(0.0, 1.0);
    let anchor_time = start + span * anchor_ratio;
    let next_span = (span * zoom_factor).clamp(min_span.min(full_span), full_span);
    let next_start = anchor_time - next_span * anchor_ratio;

    range_with_start(next_start, next_span, (full_start, full_end))
}

fn range_with_span_around(center: f64, span: f64, full_range: (f64, f64)) -> (f64, f64) {
    range_with_start(center - span * 0.5, span, full_range)
}

fn range_with_start(start: f64, span: f64, full_range: (f64, f64)) -> (f64, f64) {
    let Some((full_start, full_end)) = normalized_range(full_range) else {
        return full_range;
    };
    let full_span = full_end - full_start;
    if span >= full_span {
        return (full_start, full_end);
    }

    let start = start.clamp(full_start, full_end - span);
    (start, start + span)
}

fn overscan_cache_range(view_range: (f64, f64), full_range: (f64, f64)) -> Option<(f64, f64)> {
    let (view_start, view_end) = normalized_range(view_range)?;
    let (full_start, full_end) = normalized_range(full_range)?;
    let view_span = view_end - view_start;
    if view_span <= 0.0 {
        return None;
    }

    let overscan = view_span * ENVELOPE_OVERSCAN_RATIO;
    let start = (view_start - overscan).max(full_start);
    let end = (view_end + overscan).min(full_end);
    normalized_range((start, end))
}

fn cache_bucket_count_for_view(
    view_range: (f64, f64),
    cache_range: (f64, f64),
    requested_bucket_count: usize,
) -> usize {
    let Some((view_start, view_end)) = normalized_range(view_range) else {
        return requested_bucket_count.max(1);
    };
    let Some((cache_start, cache_end)) = normalized_range(cache_range) else {
        return requested_bucket_count.max(1);
    };
    let view_span = view_end - view_start;
    let cache_span = cache_end - cache_start;
    if view_span <= 0.0 || cache_span <= 0.0 {
        return requested_bucket_count.max(1);
    }

    let scale = (cache_span / view_span).max(1.0);
    (((requested_bucket_count.max(1) as f64) * scale).ceil() as usize)
        .clamp(requested_bucket_count.max(1), MAX_CACHED_ENVELOPE_BUCKETS)
}

fn similar_span(left: f64, right: f64, tolerance: f64) -> bool {
    if !left.is_finite() || !right.is_finite() || left <= 0.0 || right <= 0.0 {
        return false;
    }

    let ratio = left.min(right) / left.max(right);
    ratio >= 1.0 - tolerance
}

fn normalized_range((start, end): (f64, f64)) -> Option<(f64, f64)> {
    if !start.is_finite() || !end.is_finite() {
        return None;
    }

    match start.partial_cmp(&end)? {
        std::cmp::Ordering::Less => Some((start, end)),
        std::cmp::Ordering::Greater => Some((end, start)),
        std::cmp::Ordering::Equal => None,
    }
}

fn range_intersection(left: (f64, f64), right: (f64, f64)) -> Option<(f64, f64)> {
    let (left_start, left_end) = normalized_range(left)?;
    let (right_start, right_end) = normalized_range(right)?;
    let start = left_start.max(right_start);
    let end = left_end.min(right_end);
    (start < end).then_some((start, end))
}

#[cfg(test)]
mod app_tests {
    use super::*;

    fn test_key(channel_path: &str) -> ChannelKey {
        ChannelKey::new(DEFAULT_SOURCE_ID, channel_path)
    }

    fn test_channel_info(index: usize, name: &str) -> parquet_schema::ColumnInfo {
        parquet_schema::ColumnInfo {
            index,
            name: name.to_owned(),
            path: name.to_owned(),
            physical_type: "FLOAT".to_owned(),
            logical_type: None,
            converted_type: "NONE".to_owned(),
            is_numeric: true,
            role: parquet_schema::ColumnRole::Channel,
        }
    }

    fn test_channel_info_with_path(
        index: usize,
        name: &str,
        path: &str,
    ) -> parquet_schema::ColumnInfo {
        let mut channel = test_channel_info(index, name);
        channel.path = path.to_owned();
        channel
    }

    fn test_schema(channels: &[&str]) -> parquet_schema::SchemaSummary {
        let channel_infos = channels
            .iter()
            .enumerate()
            .map(|(index, name)| test_channel_info(index + 1, name))
            .collect::<Vec<_>>();
        let time_column = parquet_schema::ColumnInfo {
            index: 0,
            name: "time".to_owned(),
            path: "time".to_owned(),
            physical_type: "DOUBLE".to_owned(),
            logical_type: None,
            converted_type: "NONE".to_owned(),
            is_numeric: true,
            role: parquet_schema::ColumnRole::Time,
        };
        let mut columns = vec![time_column.clone()];
        columns.extend(channel_infos.clone());

        parquet_schema::SchemaSummary {
            path: PathBuf::from("/tmp/test.parquet"),
            row_count: 4,
            row_group_count: 1,
            column_count: columns.len(),
            created_by: None,
            time_column: Some(time_column),
            channels: channel_infos,
            columns,
        }
    }

    fn test_channel_data(channel_path: &str) -> parquet_waveform::ChannelData {
        test_channel_data_with_values(channel_path, vec![0.0, 1.0, 0.0])
    }

    fn test_channel_data_with_values(
        channel_path: &str,
        values: Vec<f32>,
    ) -> parquet_waveform::ChannelData {
        parquet_waveform::ChannelData {
            path: PathBuf::new(),
            channel_name: channel_path.to_owned(),
            channel_path: channel_path.to_owned(),
            values: Arc::new(values),
            projected_column_index: 1,
            elapsed: Duration::ZERO,
        }
    }

    fn test_time_data() -> parquet_waveform::TimeData {
        test_time_data_with_samples(&[0.0, 0.5, 1.0])
    }

    fn test_time_data_with_samples(time: &[f64]) -> parquet_waveform::TimeData {
        parquet_waveform::TimeData {
            path: PathBuf::new(),
            column_name: "time".to_owned(),
            column_path: "time".to_owned(),
            time: Arc::new(time.to_vec()),
            projected_column_index: 0,
            elapsed: Duration::ZERO,
        }
    }

    fn install_test_source_waveform(
        workspace: &mut WorkspaceDataState,
        source_id: SourceId,
        time: &[f64],
        channel_path: &str,
        values: Vec<f32>,
    ) {
        let source = workspace.source_mut(source_id).expect("test source");
        source.shared_time = Some(test_time_data_with_samples(time));
        source
            .loaded_channels
            .insert_channel(test_channel_data_with_values(channel_path, values));
    }

    fn test_active_load_job(
        source_id: SourceId,
        source_generation: u64,
        channel_path: &str,
    ) -> ActiveLoadJob {
        ActiveLoadJob {
            source_id,
            source_generation,
            channel_key: ChannelKey::new(source_id, channel_path),
            target_row_id: 0,
            target_row_label: "Row 1".to_owned(),
            add_to_row: false,
            time_was_cached: false,
            channel_was_cached: false,
            started: Instant::now(),
        }
    }

    fn test_load_result(
        job_id: u64,
        source_id: SourceId,
        source_generation: u64,
        channel_path: &str,
        result: Result<LoadedChannelData, String>,
    ) -> LoadJobResult {
        LoadJobResult {
            job_id,
            source_id,
            source_generation,
            channel_path: channel_path.to_owned(),
            result,
        }
    }

    fn empty_line_tile_cache() -> parquet_waveform::LineTileCache {
        parquet_waveform::LineTileCache {
            tile_size: 2,
            sample_count: 3,
            tiles: Vec::new(),
            elapsed: Duration::ZERO,
        }
    }

    fn empty_min_max_envelope() -> parquet_waveform::MinMaxEnvelope {
        parquet_waveform::MinMaxEnvelope {
            buckets: Vec::new(),
            source_sample_count: 0,
            requested_bucket_count: 10,
            bucket_size: 0,
            time_range: None,
            value_range: None,
            elapsed: Duration::ZERO,
        }
    }

    #[test]
    fn default_workspace_contains_only_selected_source_one() {
        let workspace = WorkspaceDataState::with_parquet_path("/tmp/input/test.parquet".to_owned());

        assert_eq!(workspace.sources.len(), 1);
        assert_eq!(workspace.source_order, vec![DEFAULT_SOURCE_ID]);
        assert_eq!(workspace.selected_source_id(), Some(DEFAULT_SOURCE_ID));
        assert_eq!(workspace.source_number(DEFAULT_SOURCE_ID), Some(1));
        assert_eq!(workspace.next_source_id, 2);

        let source = workspace.selected_source().expect("Source #1");
        assert_eq!(source.id, DEFAULT_SOURCE_ID);
        assert_eq!(source.label, "test.parquet");
        assert_eq!(source.parquet_path, "/tmp/input/test.parquet");
        assert_eq!(source.time_transform, TimeTransform::default());
        assert_eq!(source.generation, 0);
    }

    #[test]
    fn multi_source_benchmark_rows_overlay_same_paths_and_keep_unique_paths() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .schema = Some(test_schema(&["iu", "torque", "gate_pwm"]));
        for channel_path in ["iu", "torque", "gate_pwm"] {
            install_test_source_waveform(
                &mut workspace,
                DEFAULT_SOURCE_ID,
                &[0.0, 0.5, 1.0],
                channel_path,
                vec![0.0, 1.0, 0.0],
            );
        }
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        workspace.source_mut(source_two).expect("Source #2").schema =
            Some(test_schema(&["iu", "speed", "gate_pwm"]));
        for channel_path in ["iu", "speed", "gate_pwm"] {
            install_test_source_waveform(
                &mut workspace,
                source_two,
                &[0.5, 0.75, 1.0],
                channel_path,
                vec![0.0, 2.0, 0.0],
            );
        }

        let rows = multi_source_benchmark_rows(&workspace);

        assert_eq!(rows.len(), 4);
        let iu_row = rows
            .iter()
            .find(|row| row.channels[0].key.channel_path == "iu")
            .expect("iu row");
        assert_eq!(iu_row.channels.len(), 2);
        assert_eq!(iu_row.channels[0].key.source_id, DEFAULT_SOURCE_ID);
        assert_eq!(iu_row.channels[1].key.source_id, source_two);
        let gate_row = rows
            .iter()
            .find(|row| row.channels[0].key.channel_path == "gate_pwm")
            .expect("gate row");
        assert_eq!(gate_row.channels.len(), 2);
        assert!(
            gate_row
                .channels
                .iter()
                .all(|channel| channel.draw_mode == DrawMode::Step)
        );
        assert_eq!(phase2_benchmark_mode_counts(&rows), (4, 2));
    }

    #[test]
    fn workspace_source_lifecycle_stress_preserves_source_one_and_monotonic_ids() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        install_test_source_waveform(
            &mut workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 0.5, 1.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        let mut previous_source_id = DEFAULT_SOURCE_ID;

        for cycle in 0..64 {
            let source_id = workspace.add_source(format!("/tmp/source_2_{cycle}.parquet"));
            assert!(source_id > previous_source_id);
            previous_source_id = source_id;
            install_test_source_waveform(
                &mut workspace,
                source_id,
                &[0.25, 0.5, 0.75],
                "iu",
                vec![4.0, 5.0, 6.0],
            );
            assert!(workspace.reload_source_parquet_path(
                source_id,
                format!("/tmp/source_2_{cycle}_reloaded.parquet")
            ));
            assert!(workspace.remove_source(source_id).is_some());

            assert_eq!(workspace.source_order, vec![DEFAULT_SOURCE_ID]);
            assert_eq!(workspace.selected_source_id(), Some(DEFAULT_SOURCE_ID));
            assert_eq!(
                workspace
                    .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                    .map(|channel| channel.values.as_slice()),
                Some([1.0, 2.0, 3.0].as_slice())
            );
        }

        assert_eq!(workspace.source_order.len(), 1);
        assert_eq!(
            workspace.recent_parquet_paths.len(),
            MAX_RECENT_PARQUET_PATHS
        );
    }

    #[test]
    fn persistence_policy_allows_multi_source_config_and_session_actions() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        for action in [
            PersistenceAction::PlotConfigLoad,
            PersistenceAction::PlotConfigApplyToCurrentData,
            PersistenceAction::PlotConfigSave,
            PersistenceAction::PlotConfigSaveAs,
            PersistenceAction::SessionPresetSave,
            PersistenceAction::SessionPresetApply,
        ] {
            assert_eq!(
                persistence_action_block_reason(&workspace, action),
                None,
                "single-source action should be available: {action:?}"
            );
        }

        workspace.add_source("/tmp/source_2.parquet".to_owned());
        assert_eq!(
            persistence_action_block_reason(&workspace, PersistenceAction::PlotConfigLoad),
            None
        );
        for action in [
            PersistenceAction::SessionPresetSave,
            PersistenceAction::SessionPresetApply,
        ] {
            assert_eq!(persistence_action_block_reason(&workspace, action), None);
        }
        assert_eq!(
            plot_config_load_workspace_notice(&workspace).as_deref(),
            Some(
                "Load replaces the current 2-source workspace; v1 restores one source, v2 restores its saved sources"
            )
        );
    }

    #[test]
    fn zero_source_persistence_policy_requires_one_source_except_for_v1_load() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        workspace.remove_source(DEFAULT_SOURCE_ID);

        assert_eq!(
            persistence_action_block_reason(&workspace, PersistenceAction::PlotConfigLoad),
            None
        );
        let reason = persistence_action_block_reason(&workspace, PersistenceAction::PlotConfigSave)
            .expect("zero-source Save must be blocked");
        assert_eq!(reason, "Plot config Save requires a data source");
    }

    #[test]
    fn plot_config_v1_replacement_creates_clean_source_one_and_keeps_recent_paths() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        install_test_source_waveform(
            &mut workspace,
            source_two,
            &[0.5, 1.0],
            "iu",
            vec![2.0, 3.0],
        );
        let schema = test_schema(&["iu", "torque"]);

        workspace.replace_with_plot_config_v1_source(
            "/tmp/config_source.parquet".to_owned(),
            schema.clone(),
        );

        assert_eq!(workspace.source_order, vec![DEFAULT_SOURCE_ID]);
        assert_eq!(workspace.selected_source_id(), Some(DEFAULT_SOURCE_ID));
        assert_eq!(workspace.next_source_id, 2);
        assert_eq!(workspace.sources.len(), 1);
        let source = workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        assert_eq!(source.parquet_path, "/tmp/config_source.parquet");
        let restored_schema = source.schema.as_ref().expect("restored schema");
        assert_eq!(restored_schema.row_count, schema.row_count);
        assert_eq!(restored_schema.channels.len(), schema.channels.len());
        assert_eq!(
            restored_schema
                .channels
                .iter()
                .map(|channel| channel.path.as_str())
                .collect::<Vec<_>>(),
            vec!["iu", "torque"]
        );
        assert!(source.shared_time.is_none());
        assert!(source.loaded_channels.raw_by_channel.is_empty());
        assert_eq!(
            workspace.recent_parquet_paths,
            vec![
                "/tmp/config_source.parquet",
                "/tmp/source_2.parquet",
                "/tmp/source_1.parquet"
            ]
        );
    }

    #[test]
    fn multi_source_backend_guards_do_not_capture_or_apply_partial_state() {
        let mut app = PanelYApp::default();
        app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        app.plot_config_path = std::env::temp_dir()
            .join(format!(
                "panel_y_multi_source_guard_{}.pyc.json",
                std::process::id()
            ))
            .display()
            .to_string();

        app.save_session_preset();
        assert!(app.session_preset.is_none());
        assert!(
            app.load
                .status
                .contains("Load schema before saving session preset")
        );

        app.apply_session_preset(&egui::Context::default());
        assert!(app.load.status.contains("No session preset saved"));

        app.save_plot_config();
        assert!(app.load.status.contains("Load schema before saving"));
        assert!(!Path::new(&app.plot_config_path).exists());
        app.save_plot_config_as();
        assert!(app.load.status.contains("Load schema before saving"));

        app.apply_plot_config_to_current_data(&egui::Context::default());
        assert!(app.load.status.contains("Invalid plot config path"));
    }

    #[test]
    fn ui_adds_more_than_two_sources_with_distinct_monotonic_ids() {
        let mut app = PanelYApp::default();
        for _ in 0..4 {
            app.add_source_from_ui();
        }
        let ids = app.workspace.source_order.clone();
        assert_eq!(ids.len(), 5);
        assert_eq!(ids.iter().copied().collect::<BTreeSet<_>>().len(), 5);
        assert_eq!(app.workspace.source_number(ids[4]), Some(5));
        app.remove_source_from_ui(ids[2]);
        assert_eq!(app.workspace.source_number(ids[3]), Some(3));
        app.add_source_from_ui();
        let newest = *app.workspace.source_order.last().unwrap();
        assert!(newest.value() > ids[4].value());
        assert_eq!(app.workspace.source_number(newest), Some(5));
        assert!(app.workspace.source(ids[2]).is_none());
    }

    #[test]
    fn three_source_matching_and_middle_removal_preserve_other_allocations_and_selection() {
        let mut app = PanelYApp::default();
        app.add_source_from_ui();
        app.add_source_from_ui();
        let ids = app.workspace.source_order.clone();
        let keys = ids
            .iter()
            .map(|id| ChannelKey::new(*id, "iu"))
            .collect::<Vec<_>>();
        for (index, id) in ids.iter().copied().enumerate() {
            install_test_source_waveform(
                &mut app.workspace,
                id,
                &[0.0, 0.5, 1.0],
                "iu",
                vec![index as f32; 3],
            );
            let source = app.workspace.source_mut(id).unwrap();
            source.schema = Some(test_schema(&["iu"]));
            source
                .loaded_channels
                .line_tile_cache
                .insert("iu".into(), empty_line_tile_cache());
        }
        let raw_ptrs = keys
            .iter()
            .map(|key| app.workspace.channel(key).unwrap().values.as_ptr())
            .collect::<Vec<_>>();
        app.add_matching_channels(&egui::Context::default(), &keys[1]);
        assert_eq!(
            app.view.rows[0]
                .channels
                .iter()
                .map(|channel| channel.key.clone())
                .collect::<Vec<_>>(),
            keys
        );
        app.add_matching_channels(&egui::Context::default(), &keys[1]);
        assert_eq!(app.view.rows[0].channels.len(), 3);
        assert!(!app.load.is_busy());
        app.view.selected_channel = Some(keys[2].clone());
        app.workspace.selected_source_id = Some(ids[2]);
        app.remove_source_from_ui(ids[1]);
        assert_eq!(app.workspace.source_order, vec![ids[0], ids[2]]);
        assert_eq!(app.workspace.selected_source_id(), Some(ids[2]));
        assert_eq!(app.workspace.source_number(ids[2]), Some(2));
        assert_eq!(app.view.selected_channel, Some(keys[2].clone()));
        assert_eq!(
            app.view.rows[0]
                .channels
                .iter()
                .map(|channel| channel.key.clone())
                .collect::<Vec<_>>(),
            vec![keys[0].clone(), keys[2].clone()]
        );
        for index in [0, 2] {
            assert_eq!(
                app.workspace.channel(&keys[index]).unwrap().values.as_ptr(),
                raw_ptrs[index]
            );
            assert!(
                app.workspace
                    .source(ids[index])
                    .unwrap()
                    .loaded_channels
                    .has_draw_caches()
            );
        }
    }

    #[test]
    fn zero_source_workspace_can_add_new_display_source_one() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/one.parquet".to_owned());
        assert!(workspace.remove_source(DEFAULT_SOURCE_ID).is_some());
        assert!(workspace.source_order.is_empty());
        assert_eq!(workspace.selected_source_id(), None);

        let replacement = workspace.add_source(String::new());
        assert_ne!(replacement, DEFAULT_SOURCE_ID);
        assert_eq!(workspace.source_number(replacement), Some(1));
        assert_eq!(workspace.selected_source_id(), Some(replacement));
        assert_eq!(
            source_presentation(&workspace, replacement)
                .expect("presentation")
                .short_label,
            "#1 (no file)"
        );
    }

    #[test]
    fn channel_presentation_is_always_source_numbered_and_preserves_identity() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/run_A.parquet".to_owned());
        let source_one_key = ChannelKey::new(DEFAULT_SOURCE_ID, "motor/iu");
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .schema = Some(parquet_schema::SchemaSummary {
            path: PathBuf::from("/tmp/run_A.parquet"),
            row_count: 10,
            row_group_count: 1,
            column_count: 2,
            created_by: None,
            time_column: None,
            channels: vec![test_channel_info_with_path(1, "iu", "motor/iu")],
            columns: Vec::new(),
        });

        let source_two = workspace.add_source("/tmp/run_B.parquet".to_owned());
        let source_two_key = ChannelKey::new(source_two, "motor/iu");
        let mut source_two_schema = workspace
            .source(DEFAULT_SOURCE_ID)
            .and_then(|source| source.schema.clone())
            .expect("Source #1 schema");
        source_two_schema
            .channels
            .push(test_channel_info_with_path(2, "speed", "motor/speed"));
        workspace.source_mut(source_two).expect("Source #2").schema = Some(source_two_schema);

        let one = channel_presentation(&workspace, &source_one_key).expect("Source #1 label");
        let two = channel_presentation(&workspace, &source_two_key).expect("Source #2 label");
        let speed = channel_presentation(&workspace, &ChannelKey::new(source_two, "motor/speed"))
            .expect("different-name channel label");
        assert_eq!(one.short_label, "iu #1");
        assert_eq!(two.short_label, "iu #2");
        assert_eq!(speed.short_label, "speed #2");
        assert!(one.tooltip.contains("run_A.parquet"));
        assert!(one.tooltip.contains("motor/iu"));
        assert_eq!(source_one_key.channel_path, "motor/iu");
        assert_eq!(source_two_key.channel_path, "motor/iu");
        assert!(!source_one_key.channel_path.contains("#1"));
    }

    #[test]
    fn presentation_falls_back_to_path_and_renumbers_after_remove() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/run_A.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/run_B.parquet".to_owned());
        workspace.source_mut(source_two).expect("Source #2").schema =
            Some(parquet_schema::SchemaSummary {
                path: PathBuf::from("/tmp/run_B.parquet"),
                row_count: 10,
                row_group_count: 1,
                column_count: 2,
                created_by: None,
                time_column: None,
                channels: vec![test_channel_info_with_path(1, "", "motor/iu")],
                columns: Vec::new(),
            });
        let key = ChannelKey::new(source_two, "motor/iu");
        assert_eq!(
            channel_presentation(&workspace, &key)
                .expect("Source #2")
                .short_label,
            "motor/iu #2"
        );

        workspace.remove_source(DEFAULT_SOURCE_ID);
        assert_eq!(
            channel_presentation(&workspace, &key)
                .expect("renumbered source")
                .short_label,
            "motor/iu #1"
        );
        assert_eq!(
            channel_presentation(&workspace, &ChannelKey::new(DEFAULT_SOURCE_ID, "iu")),
            None
        );
    }

    #[test]
    fn workspace_accessors_return_none_for_missing_source_and_key() {
        let mut workspace = WorkspaceDataState::default();
        let missing_source = SourceId::new(99);
        let missing_key = ChannelKey::new(missing_source, "iu");

        assert!(workspace.source(missing_source).is_none());
        assert!(workspace.source_mut(missing_source).is_none());
        assert!(workspace.channel(&missing_key).is_none());
        assert!(!workspace.has_channel(&missing_key));
        assert_eq!(workspace.source_number(missing_source), None);
    }

    #[test]
    fn workspace_channel_lookup_does_not_cross_sources() {
        let mut workspace = WorkspaceDataState::default();
        let source_two_id = SourceId::new(2);
        let mut source_two =
            DataSourceState::new(source_two_id, "/tmp/source_two.parquet".to_owned());

        let mut source_one_channel = test_channel_data("iu");
        source_one_channel.values = Arc::new(vec![1.0, 1.0, 1.0]);
        workspace
            .selected_source_mut()
            .expect("Source #1")
            .loaded_channels
            .insert_channel(source_one_channel);

        let mut source_two_channel = test_channel_data("iu");
        source_two_channel.values = Arc::new(vec![2.0, 2.0, 2.0]);
        source_two
            .loaded_channels
            .insert_channel(source_two_channel);
        workspace.sources.insert(source_two_id, source_two);
        workspace.source_order.push(source_two_id);
        workspace.next_source_id = 3;

        let source_one_key = ChannelKey::new(DEFAULT_SOURCE_ID, "iu");
        let source_two_key = ChannelKey::new(source_two_id, "iu");
        let missing_on_source_two = ChannelKey::new(source_two_id, "iv");

        assert_eq!(workspace.channel(&source_one_key).unwrap().values[0], 1.0);
        assert_eq!(workspace.channel(&source_two_key).unwrap().values[0], 2.0);
        assert!(workspace.channel(&missing_on_source_two).is_none());
        assert_eq!(workspace.source_number(source_two_id), Some(2));
    }

    #[test]
    fn changing_source_path_updates_label_and_advances_generation() {
        let mut source = DataSourceState::new(DEFAULT_SOURCE_ID, "/tmp/first.parquet".to_owned());

        assert!(source.set_parquet_path("/tmp/second.parquet".to_owned()));
        assert_eq!(source.label, "second.parquet");
        assert_eq!(source.generation, 1);
        assert!(!source.set_parquet_path("/tmp/second.parquet".to_owned()));
        assert_eq!(source.generation, 1);
    }

    #[test]
    fn explicit_same_path_reload_advances_generation_and_clears_source_cache() {
        let path = "/tmp/source_1.parquet";
        let mut workspace = WorkspaceDataState::with_parquet_path(path.to_owned());
        let source = workspace.source_mut(DEFAULT_SOURCE_ID).expect("Source #1");
        source.schema = Some(test_schema(&["iu"]));
        source.shared_time = Some(test_time_data());
        source
            .loaded_channels
            .insert_channel(test_channel_data("iu"));
        source
            .loaded_channels
            .line_tile_cache
            .insert("iu".to_owned(), empty_line_tile_cache());
        let generation = source.generation;

        assert!(workspace.reload_source_parquet_path(DEFAULT_SOURCE_ID, path.to_owned()));

        let source = workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        assert_eq!(source.generation, generation + 1);
        assert!(source.schema.is_none());
        assert!(source.shared_time.is_none());
        assert!(source.loaded_channels.raw_by_channel.is_empty());
        assert!(!source.loaded_channels.has_draw_caches());
    }

    #[test]
    fn workspace_source_id_counter_is_monotonic() {
        let mut workspace = WorkspaceDataState::default();

        let source_two = workspace.reserve_source_id();
        let source_three = workspace.reserve_source_id();

        assert_eq!(source_two.value(), 2);
        assert_eq!(source_three.value(), 3);
        assert_eq!(workspace.next_source_id, 4);
    }

    #[test]
    fn workspace_source_lifecycle_keeps_order_selection_and_ids_consistent() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());

        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        let source_three = workspace.add_source("/tmp/source_3.parquet".to_owned());
        assert_eq!(source_two.value(), 2);
        assert_eq!(source_three.value(), 3);
        assert_eq!(
            workspace.source_order,
            vec![DEFAULT_SOURCE_ID, source_two, source_three]
        );
        assert_eq!(workspace.source_number(source_two), Some(2));

        workspace.selected_source_id = Some(source_two);
        let removed = workspace
            .remove_source(source_two)
            .expect("remove Source #2");
        assert_eq!(removed.id, source_two);
        assert_eq!(workspace.selected_source_id(), Some(DEFAULT_SOURCE_ID));
        assert_eq!(
            workspace.source_order,
            vec![DEFAULT_SOURCE_ID, source_three]
        );
        assert_eq!(workspace.source_number(source_three), Some(2));

        let source_four = workspace.add_source("/tmp/source_4.parquet".to_owned());
        assert_eq!(source_four.value(), 4);
        assert!(workspace.source(source_two).is_none());
        assert!(!workspace.reset_source(SourceId::new(99)));
        assert!(workspace.remove_source(SourceId::new(99)).is_none());
    }

    #[test]
    fn removing_the_last_selected_source_falls_back_to_none() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());

        workspace.remove_source(DEFAULT_SOURCE_ID);

        assert!(workspace.sources.is_empty());
        assert!(workspace.source_order.is_empty());
        assert_eq!(workspace.selected_source_id(), None);
    }

    #[test]
    fn two_sources_hold_distinct_schema_time_and_same_path_channels() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());

        let mut source_one_channel = test_channel_data_with_values("iu", vec![1.0, 1.0, 1.0]);
        source_one_channel.path = PathBuf::from("/tmp/source_1.parquet");
        let source_one = workspace.source_mut(DEFAULT_SOURCE_ID).expect("Source #1");
        source_one.schema = Some(test_schema(&["iu", "torque"]));
        source_one.shared_time = Some(test_time_data_with_samples(&[0.0, 0.5, 1.0]));
        source_one
            .loaded_channels
            .insert_channel(source_one_channel);

        let mut source_two_channel = test_channel_data_with_values("iu", vec![2.0, 2.0, 2.0, 2.0]);
        source_two_channel.path = PathBuf::from("/tmp/source_2.parquet");
        let source_two_state = workspace.source_mut(source_two).expect("Source #2");
        source_two_state.schema = Some(test_schema(&["iu", "speed"]));
        source_two_state.shared_time = Some(test_time_data_with_samples(&[0.5, 0.75, 1.0, 1.25]));
        source_two_state
            .loaded_channels
            .insert_channel(source_two_channel);

        assert_eq!(
            workspace
                .source(DEFAULT_SOURCE_ID)
                .and_then(|source| source.schema.as_ref())
                .map(|schema| schema.channels[1].path.as_str()),
            Some("torque")
        );
        assert_eq!(
            workspace
                .source(source_two)
                .and_then(|source| source.schema.as_ref())
                .map(|schema| schema.channels[1].path.as_str()),
            Some("speed")
        );
        assert_eq!(
            workspace
                .source(DEFAULT_SOURCE_ID)
                .and_then(|source| source.shared_time.as_ref())
                .map(parquet_waveform::TimeData::sample_count),
            Some(3)
        );
        assert_eq!(
            workspace
                .source(source_two)
                .and_then(|source| source.shared_time.as_ref())
                .map(parquet_waveform::TimeData::sample_count),
            Some(4)
        );
        assert_eq!(
            workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .map(|channel| channel.values[0]),
            Some(1.0)
        );
        assert_eq!(
            workspace
                .channel(&ChannelKey::new(source_two, "iu"))
                .map(|channel| channel.values[0]),
            Some(2.0)
        );
    }

    #[test]
    fn workspace_display_range_and_min_zoom_span_cover_every_loaded_source() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .shared_time = Some(test_time_data_with_samples(&[0.0, 1.0, 2.0]));
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .time_transform = TimeTransform {
            offset_seconds: 1.0,
        };
        workspace
            .source_mut(source_two)
            .expect("Source #2")
            .shared_time = Some(test_time_data_with_samples(&[
            10.0, 10.25, 10.5, 10.75, 11.0,
        ]));
        workspace
            .source_mut(source_two)
            .expect("Source #2")
            .time_transform = TimeTransform {
            offset_seconds: -5.0,
        };

        assert_eq!(workspace.display_time_range(), Some((1.0, 6.0)));
        assert!(
            (workspace.display_min_view_span().expect("minimum span") - 0.25).abs() <= f64::EPSILON
        );

        let mut app = PanelYApp {
            workspace,
            ..PanelYApp::default()
        };
        app.view.x_range = Some((1.5, 2.0));
        app.reset_view_range();
        assert_eq!(app.view.x_range, Some((1.0, 6.0)));
    }

    #[test]
    fn workspace_overlap_intersects_all_loaded_sources_after_offsets() {
        let mut workspace = WorkspaceDataState::default();
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        let source_three = workspace.add_source("/tmp/source_3.parquet".to_owned());
        for (source_id, time, offset) in [
            (DEFAULT_SOURCE_ID, [0.0, 1.0, 2.0], 0.25),
            (source_two, [10.0, 11.0, 12.0], -9.0),
            (source_three, [1.5, 2.0, 2.5], -0.125),
        ] {
            let source = workspace.source_mut(source_id).unwrap();
            source.shared_time = Some(test_time_data_with_samples(&time));
            source.time_transform.offset_seconds = offset;
        }

        assert_eq!(workspace.display_overlap_time_range(), Some((1.375, 2.25)));
        assert_eq!(workspace.display_time_range(), Some((0.25, 3.0)));
        workspace.source_order.reverse();
        assert_eq!(workspace.display_overlap_time_range(), Some((1.375, 2.25)));
    }

    #[test]
    fn workspace_overlap_rejects_disjoint_and_touching_ranges() {
        for second_start in [2.0, 3.0] {
            let mut workspace = WorkspaceDataState::default();
            let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
            let source_three = workspace.add_source("/tmp/source_3.parquet".to_owned());
            for (source_id, time) in [
                (DEFAULT_SOURCE_ID, [0.0, 2.0]),
                (source_two, [second_start, second_start + 1.0]),
                (source_three, [0.0, 5.0]),
            ] {
                workspace.source_mut(source_id).unwrap().shared_time =
                    Some(test_time_data_with_samples(&time));
            }
            assert_eq!(workspace.display_overlap_time_range(), None);
            workspace.source_order.reverse();
            assert_eq!(workspace.display_overlap_time_range(), None);
        }
    }

    #[test]
    fn workspace_overlap_skips_unloaded_time_but_rejects_invalid_loaded_ranges() {
        let mut workspace = WorkspaceDataState::default();
        assert_eq!(workspace.display_overlap_time_range(), None);
        workspace.source_mut(DEFAULT_SOURCE_ID).unwrap().shared_time =
            Some(test_time_data_with_samples(&[0.0, 1.0, 2.0]));
        assert_eq!(workspace.display_overlap_time_range(), Some((0.0, 2.0)));

        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        workspace.source_mut(source_two).unwrap().schema = Some(test_schema(&["iu"]));
        assert_eq!(workspace.display_overlap_time_range(), Some((0.0, 2.0)));
        for time in [
            vec![],
            vec![1.0],
            vec![1.0, 1.0],
            vec![0.0, f64::NAN],
            vec![0.0, f64::INFINITY],
        ] {
            workspace.source_mut(source_two).unwrap().shared_time =
                Some(test_time_data_with_samples(&time));
            assert_eq!(workspace.display_overlap_time_range(), None);
        }
        workspace.remove_source(source_two);
        assert_eq!(workspace.display_overlap_time_range(), Some((0.0, 2.0)));
        workspace.remove_source(DEFAULT_SOURCE_ID);
        assert_eq!(workspace.display_overlap_time_range(), None);
    }

    #[test]
    fn fit_overlap_updates_view_after_offset_changes_and_source_removal() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        for (source_id, time) in [
            (DEFAULT_SOURCE_ID, [0.0, 1.0, 2.0]),
            (source_two, [0.5, 1.125, 1.75]),
        ] {
            install_test_source_waveform(&mut app.workspace, source_id, &time, "iu", vec![1.0; 3]);
        }
        app.view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"), 0),
            RowChannel::new(ChannelKey::new(source_two, "iu"), 1),
        ];
        app.analysis.cursor_a = Some(0.75);
        app.analysis.cursor_b = Some(1.5);
        app.reset_view_range();
        app.fit_overlap_view_range();
        assert_eq!(app.view.x_range, Some((0.5, 1.75)));
        assert_eq!(app.analysis.cursor_a, Some(0.75));
        assert_eq!(app.analysis.cursor_b, Some(1.5));
        assert_eq!(app.view.rows.len(), 1);
        assert_eq!(app.view.rows[0].channels.len(), 2);
        assert_eq!(app.workspace.cache_stats().loaded_channel_count, 2);

        app.apply_source_time_offset(source_two, 0.25);
        app.fit_overlap_view_range();
        assert_eq!(app.view.x_range, Some((0.75, 2.0)));
        app.apply_source_time_offset(source_two, -0.75);
        app.fit_overlap_view_range();
        assert_eq!(app.view.x_range, Some((0.0, 1.0)));

        app.apply_source_time_offset(source_two, 1.5);
        let before = app.view.x_range;
        assert_eq!(app.workspace.display_overlap_time_range(), None);
        app.fit_overlap_view_range();
        assert_eq!(app.view.x_range, before);
        app.reset_view_range();
        assert_eq!(app.view.x_range, Some((0.0, 3.25)));

        app.apply_source_time_offset(DEFAULT_SOURCE_ID, 0.125);
        app.remove_source_from_ui(source_two);
        app.fit_overlap_view_range();
        assert_eq!(app.view.x_range, Some((0.125, 2.125)));
        assert_eq!(app.view.rows[0].channels.len(), 1);
        assert_eq!(
            app.workspace
                .source(DEFAULT_SOURCE_ID)
                .unwrap()
                .shared_time
                .as_ref()
                .unwrap()
                .time
                .as_slice(),
            vec![0.0, 1.0, 2.0]
        );
    }

    #[test]
    fn fit_overlap_narrower_than_sample_spacing_survives_render_and_clamping() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        for (index, (source_id, time)) in [
            (DEFAULT_SOURCE_ID, [0.0, 1.0, 2.0]),
            (source_two, [1.75, 2.75, 3.75]),
        ]
        .into_iter()
        .enumerate()
        {
            install_test_source_waveform(&mut app.workspace, source_id, &time, "iu", vec![1.0; 3]);
            app.view.rows[0]
                .channels
                .push(RowChannel::new(ChannelKey::new(source_id, "iu"), index));
        }
        app.fit_overlap_view_range();
        let rows = app.visible_row_traces(64, true, false);
        assert_eq!(app.view.x_range, Some((1.75, 2.0)));
        assert_eq!(rows[0].traces.len(), 2);
        app.reconcile_view_range_to_workspace();
        assert_eq!(app.view.x_range, Some((1.75, 2.0)));
        assert_eq!(
            clamp_view_range(
                app.view.x_range.unwrap(),
                app.workspace.display_time_range().unwrap(),
                app.workspace.display_min_view_span().unwrap(),
            ),
            (1.75, 2.0)
        );
        app.reset_view_range();
        assert_eq!(app.view.x_range, Some((0.0, 3.75)));
    }

    #[test]
    fn source_time_offset_ui_changes_only_the_display_transform() {
        let mut app = PanelYApp::default();
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[10.0, 11.0, 12.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        app.view.x_range = app.workspace.display_time_range();
        app.analysis.cursor_a = Some(10.5);
        app.analysis.cursor_b = Some(11.5);
        app.analysis.interval_stats_cache.key = Some(IntervalStatsCacheKey {
            cursor_a_bits: 10.5f64.to_bits(),
            cursor_b_bits: 11.5f64.to_bits(),
            channels: Vec::new(),
        });
        app.analysis.fft.error = Some("stale result".to_owned());
        let raw_time_before = app
            .workspace
            .source(DEFAULT_SOURCE_ID)
            .and_then(|source| source.shared_time.as_ref())
            .expect("raw time")
            .time
            .clone();
        let generation_before = app
            .workspace
            .source(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .generation;

        assert_eq!(
            app.apply_source_time_offset(DEFAULT_SOURCE_ID, -9.0),
            Some(true)
        );

        let source = app.workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        assert_eq!(source.time_transform.offset_seconds, -9.0);
        assert_eq!(data_source_display_time_range(source), Some((1.0, 3.0)));
        assert_eq!(
            source.shared_time.as_ref().expect("raw time").time,
            raw_time_before
        );
        assert_eq!(source.generation, generation_before);
        assert_eq!(app.view.x_range, Some((1.0, 3.0)));
        assert_eq!(app.analysis.cursor_a, Some(10.5));
        assert_eq!(app.analysis.cursor_b, Some(11.5));
        assert!(app.analysis.interval_stats_cache.key.is_none());
        assert!(app.analysis.fft.error.is_none());

        assert_eq!(
            app.apply_source_time_offset(DEFAULT_SOURCE_ID, f64::NAN),
            None
        );
        assert_eq!(
            app.workspace
                .source(DEFAULT_SOURCE_ID)
                .expect("Source #1")
                .time_transform
                .offset_seconds,
            -9.0
        );
    }

    #[test]
    fn start_at_zero_aligns_one_or_all_loaded_sources_without_rewriting_raw_time() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[10.0, 11.0, 12.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        install_test_source_waveform(
            &mut app.workspace,
            source_two,
            &[20.0, 20.5, 21.5],
            "iu",
            vec![4.0, 5.0, 6.0],
        );
        let raw_one_before = app
            .workspace
            .source(DEFAULT_SOURCE_ID)
            .and_then(|source| source.shared_time.as_ref())
            .expect("Source #1 raw time")
            .time
            .clone();
        let raw_two_before = app
            .workspace
            .source(source_two)
            .and_then(|source| source.shared_time.as_ref())
            .expect("Source #2 raw time")
            .time
            .clone();
        app.view.x_range = app.workspace.display_time_range();
        app.analysis.cursor_a = Some(10.5);
        app.analysis.cursor_b = Some(11.5);
        app.analysis.fft.error = Some("stale result".to_owned());

        app.start_source_at_zero(source_two);
        assert_eq!(
            app.workspace
                .source(source_two)
                .expect("Source #2")
                .time_transform
                .offset_seconds,
            -20.0
        );
        assert_eq!(
            data_source_display_time_range(app.workspace.source(source_two).expect("Source #2")),
            Some((0.0, 1.5))
        );

        app.start_all_sources_at_zero();

        let source_one = app.workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        let source_two_state = app.workspace.source(source_two).expect("Source #2");
        assert_eq!(source_one.time_transform.offset_seconds, -10.0);
        assert_eq!(source_two_state.time_transform.offset_seconds, -20.0);
        assert_eq!(app.workspace.display_time_range(), Some((0.0, 2.0)));
        assert_eq!(app.view.x_range, Some((0.0, 2.0)));
        assert_eq!(
            source_one.shared_time.as_ref().expect("raw time").time,
            raw_one_before
        );
        assert_eq!(
            source_two_state
                .shared_time
                .as_ref()
                .expect("raw time")
                .time,
            raw_two_before
        );
        assert_eq!(app.analysis.cursor_a, Some(10.5));
        assert_eq!(app.analysis.cursor_b, Some(11.5));
        assert!(app.analysis.fft.error.is_none());
        assert!(app.load.status.contains("Aligned 2 loaded source(s)"));
    }

    #[test]
    fn source_time_offset_rejects_non_finite_values_and_canonicalizes_zero() {
        let mut workspace = WorkspaceDataState::default();

        assert_eq!(
            workspace.set_source_time_offset(DEFAULT_SOURCE_ID, f64::INFINITY),
            None
        );
        assert_eq!(
            workspace.set_source_time_offset(SourceId::new(999), 1.0),
            None
        );
        assert_eq!(
            workspace.set_source_time_offset(DEFAULT_SOURCE_ID, -0.0),
            Some(false)
        );
        assert_eq!(
            workspace
                .source(DEFAULT_SOURCE_ID)
                .expect("Source #1")
                .time_transform
                .offset_seconds
                .to_bits(),
            0.0f64.to_bits()
        );
    }

    #[test]
    fn visible_rows_overlay_different_rate_line_and_step_sources() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 1.0, 2.0],
            "iu",
            vec![1.0, 100.0, 1.0],
        );
        install_test_source_waveform(
            &mut app.workspace,
            source_two,
            &[10.0, 10.5, 11.0, 11.5, 12.0],
            "iu",
            vec![10.0, 20.0, 30.0, 40.0, 50.0],
        );
        app.workspace
            .source_mut(source_two)
            .expect("Source #2")
            .time_transform = TimeTransform {
            offset_seconds: -10.0,
        };
        let source_one_key = ChannelKey::new(DEFAULT_SOURCE_ID, "iu");
        let source_two_key = ChannelKey::new(source_two, "iu");
        app.view.rows[0].channels = vec![
            RowChannel::new(source_one_key.clone(), 0),
            RowChannel {
                draw_mode: DrawMode::Step,
                ..RowChannel::new(source_two_key.clone(), 1)
            },
        ];
        app.view.x_range = None;

        let visible_rows = app.visible_row_traces(64, true, false);
        let row = &visible_rows[0];

        assert_eq!(app.view.x_range, Some((0.0, 2.0)));
        assert_eq!(row.traces.len(), 2);
        assert_eq!(row.traces[0].key, source_one_key);
        assert_eq!(row.traces[1].key, source_two_key);
        assert!(matches!(row.traces[0].data, VisibleTraceData::Envelope(_)));
        let VisibleTraceData::RawStep(source_two_step) = &row.traces[1].data else {
            panic!("Source #2 should retain exact Step data");
        };
        assert_eq!(source_two_step.time_range, Some((10.0, 12.0)));
        assert_eq!(
            row.traces[1]
                .time_transform
                .display_range(source_two_step.time_range.expect("Step range")),
            (0.0, 2.0)
        );
        assert_eq!(combined_trace_value_range(&row.traces), Some((1.0, 100.0)));
        assert_eq!(app.view.rows[0].y_range.last_auto, Some((-3.95, 104.95)));
    }

    #[test]
    fn source_without_data_in_display_window_is_excluded_from_auto_y() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 0.5, 1.0],
            "torque",
            vec![1.0, 2.0, 1.0],
        );
        install_test_source_waveform(
            &mut app.workspace,
            source_two,
            &[10.0, 10.5, 11.0],
            "speed",
            vec![100.0, 200.0, 100.0],
        );
        app.view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "torque"), 0),
            RowChannel::new(ChannelKey::new(source_two, "speed"), 1),
        ];

        let full_range_rows = app.visible_row_traces(64, true, false);
        assert_eq!(full_range_rows[0].traces.len(), 2);
        assert_eq!(
            full_range_rows[0]
                .traces
                .iter()
                .map(|trace| trace.display_label.as_str())
                .collect::<Vec<_>>(),
            vec!["torque #1", "speed #2"]
        );

        app.view.x_range = Some((0.0, 1.0));

        let visible_rows = app.visible_row_traces(64, true, false);
        let row = &visible_rows[0];

        assert_eq!(row.traces.len(), 1);
        assert_eq!(row.traces[0].key.source_id, DEFAULT_SOURCE_ID);
        assert_eq!(row.out_of_range_channel_count, 1);
        assert_eq!(combined_trace_value_range(&row.traces), Some((1.0, 2.0)));
        assert_eq!(app.view.rows[0].y_range.last_auto, Some((0.95, 2.05)));
        assert_eq!(range_intersection((0.0, 1.0), (10.0, 11.0)), None);
    }

    #[test]
    fn source_reset_clears_only_target_raw_and_draw_caches() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());

        for source_id in [DEFAULT_SOURCE_ID, source_two] {
            let source = workspace.source_mut(source_id).expect("source");
            source.schema = Some(test_schema(&["iu"]));
            source.shared_time = Some(test_time_data());
            source
                .loaded_channels
                .insert_channel(test_channel_data("iu"));
            source
                .loaded_channels
                .line_tile_cache
                .insert("iu".to_owned(), empty_line_tile_cache());
            source.loaded_channels.step_fallback_hints.insert(
                "iu".to_owned(),
                StepFallbackHint {
                    min_span: 1.0,
                    min_source_sample_count: 3,
                },
            );
        }
        let source_one_generation = workspace
            .source(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .generation;
        let source_two_generation = workspace.source(source_two).expect("Source #2").generation;

        assert!(workspace.reset_source(source_two));

        let source_one = workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        assert_eq!(source_one.generation, source_one_generation);
        assert!(source_one.schema.is_some());
        assert!(source_one.shared_time.is_some());
        assert!(source_one.loaded_channels.has_channel("iu"));
        assert!(source_one.loaded_channels.has_draw_caches());

        let source_two_state = workspace.source(source_two).expect("Source #2");
        assert_eq!(
            source_two_state.generation,
            source_two_generation.saturating_add(1)
        );
        assert!(source_two_state.schema.is_none());
        assert!(source_two_state.shared_time.is_none());
        assert!(source_two_state.loaded_channels.raw_by_channel.is_empty());
        assert!(!source_two_state.loaded_channels.has_draw_caches());
    }

    #[test]
    fn removing_source_two_preserves_source_one_data_and_cache() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        for source_id in [DEFAULT_SOURCE_ID, source_two] {
            let source = workspace.source_mut(source_id).expect("source");
            source.schema = Some(test_schema(&["iu"]));
            source.shared_time = Some(test_time_data());
            source
                .loaded_channels
                .insert_channel(test_channel_data("iu"));
            source
                .loaded_channels
                .line_tile_cache
                .insert("iu".to_owned(), empty_line_tile_cache());
        }

        workspace.remove_source(source_two);

        let source_one = workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        assert!(source_one.schema.is_some());
        assert!(source_one.shared_time.is_some());
        assert!(source_one.loaded_channels.has_channel("iu"));
        assert!(source_one.loaded_channels.has_draw_caches());
        assert!(workspace.source(source_two).is_none());
    }

    #[test]
    fn source_remove_cleans_only_its_row_selection_and_analysis_references() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        let source_one_key = ChannelKey::new(DEFAULT_SOURCE_ID, "iu");
        let source_two_key = ChannelKey::new(source_two, "iu");
        app.workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .loaded_channels
            .insert_channel(test_channel_data("iu"));
        app.workspace
            .source_mut(source_two)
            .expect("Source #2")
            .loaded_channels
            .insert_channel(test_channel_data_with_values("iu", vec![8.0, 9.0]));
        app.view.rows[0].channels = vec![
            RowChannel::new(source_one_key.clone(), 0),
            RowChannel::new(source_two_key.clone(), 1),
        ];
        app.view.selected_channel = Some(source_two_key.clone());
        app.analysis.cursor_a = Some(0.25);
        app.analysis.cursor_b = Some(0.75);
        app.analysis.interval_stats_cache.key = Some(IntervalStatsCacheKey {
            cursor_a_bits: 0.25_f64.to_bits(),
            cursor_b_bits: 0.75_f64.to_bits(),
            channels: vec![IntervalStatsCacheChannel {
                key: source_two_key.clone(),
                source_generation: 0,
                time_offset_bits: 0.0_f64.to_bits(),
                time_sample_count: 2,
                time_start_bits: 0.0_f64.to_bits(),
                time_end_bits: 1.0_f64.to_bits(),
                sample_count: 2,
                time: Arc::new(vec![0.0, 1.0]),
                values: Arc::new(vec![8.0, 9.0]),
            }],
        });
        app.analysis.fft.target_channels = vec![source_one_key.clone(), source_two_key];
        app.analysis.fft.target_initialized = true;

        app.remove_source_from_ui(source_two);

        assert!(app.workspace.source(source_two).is_none());
        assert_eq!(app.view.rows[0].channels.len(), 1);
        assert_eq!(app.view.rows[0].channels[0].key, source_one_key);
        assert_eq!(app.view.selected_channel, None);
        assert_eq!(app.analysis.cursor_a, None);
        assert_eq!(app.analysis.cursor_b, None);
        assert!(app.analysis.interval_stats_cache.key.is_none());
        assert!(app.analysis.interval_stats_cache.stats.is_empty());
        assert_eq!(
            app.analysis.fft.target_channels,
            vec![ChannelKey::new(DEFAULT_SOURCE_ID, "iu")]
        );
        assert!(
            app.workspace
                .has_channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
        );
    }

    #[test]
    fn source_two_path_change_preserves_source_one_row_selection_and_cache() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        let source_one_key = ChannelKey::new(DEFAULT_SOURCE_ID, "iu");
        let source_two_key = ChannelKey::new(source_two, "iu");
        app.workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .loaded_channels
            .insert_channel(test_channel_data("iu"));
        app.view.rows[0].channels = vec![
            RowChannel::new(source_one_key.clone(), 0),
            RowChannel::new(source_two_key, 1),
        ];
        app.view.selected_channel = Some(source_one_key.clone());

        app.select_parquet_path_for_source(source_two, "/tmp/source_2_new.parquet".to_owned());

        assert_eq!(app.view.rows[0].channels.len(), 1);
        assert_eq!(app.view.rows[0].channels[0].key, source_one_key);
        assert_eq!(
            app.view.selected_channel,
            Some(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
        );
        assert!(
            app.workspace
                .has_channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
        );
    }

    #[test]
    fn path_edit_does_not_enter_recent_history_until_committed() {
        let mut app = PanelYApp::default();
        let initial_recent_paths = app.workspace.recent_parquet_paths.clone();

        app.edit_parquet_path_for_source(DEFAULT_SOURCE_ID, "/tmp/part".to_owned());

        assert_eq!(app.workspace.recent_parquet_paths, initial_recent_paths);

        app.select_parquet_path_for_source(DEFAULT_SOURCE_ID, "/tmp/committed.parquet".to_owned());

        assert_eq!(
            app.workspace
                .recent_parquet_paths
                .first()
                .map(String::as_str),
            Some("/tmp/committed.parquet")
        );
        assert_eq!(
            app.workspace
                .recent_parquet_paths
                .iter()
                .filter(|path| path.as_str() == "/tmp/committed.parquet")
                .count(),
            1
        );
    }

    #[test]
    fn selected_source_removal_rebases_the_default_plot_config_path() {
        let mut app = PanelYApp::default();
        app.select_parquet_path_for_source(DEFAULT_SOURCE_ID, "/tmp/source_1.parquet".to_owned());
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());

        app.remove_source_from_ui(DEFAULT_SOURCE_ID);

        assert_eq!(app.workspace.selected_source_id(), Some(source_two));
        assert_eq!(
            app.plot_config_path,
            default_plot_config_path("/tmp/source_2.parquet")
        );

        app.remove_source_from_ui(source_two);
        assert!(app.plot_config_path.is_empty());
        app.add_source_from_ui();
        let replacement = app
            .workspace
            .selected_source_id()
            .expect("replacement source");
        app.edit_parquet_path_for_source(replacement, "/tmp/replacement.parquet".to_owned());
        assert_eq!(
            app.plot_config_path,
            default_plot_config_path("/tmp/replacement.parquet")
        );
    }

    #[test]
    fn selected_source_removal_preserves_a_custom_plot_config_path() {
        let mut app = PanelYApp::default();
        app.select_parquet_path_for_source(DEFAULT_SOURCE_ID, "/tmp/source_1.parquet".to_owned());
        app.plot_config_path = "/tmp/custom-view.pyc.json".to_owned();
        app.workspace.add_source("/tmp/source_2.parquet".to_owned());

        app.remove_source_from_ui(DEFAULT_SOURCE_ID);

        assert_eq!(app.plot_config_path, "/tmp/custom-view.pyc.json");
    }

    #[test]
    fn active_load_labels_follow_source_renumbering() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        app.load
            .active_jobs
            .insert(1, test_active_load_job(source_two, 0, "iu"));
        app.load.refresh_progress(&app.workspace);
        assert_eq!(
            app.load.progress.as_deref(),
            Some("loading: iu #2 -> Row 1")
        );

        app.remove_source_from_ui(DEFAULT_SOURCE_ID);

        assert_eq!(
            app.load.progress.as_deref(),
            Some("loading: iu #1 -> Row 1")
        );

        app.apply_load_result(test_load_result(
            1,
            source_two,
            0,
            "iu",
            Err("boom".to_owned()),
        ));
        assert_eq!(app.load.status, "Waveform load failed: iu #1: boom");
    }

    #[test]
    fn source_specific_loading_guard_does_not_lock_other_source() {
        let mut workspace = WorkspaceDataState::default();
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        let mut load = LoadState::default();
        load.active_jobs
            .insert(1, test_active_load_job(DEFAULT_SOURCE_ID, 0, "iu"));

        assert!(!source_dangerous_actions_enabled(&load, DEFAULT_SOURCE_ID));
        assert!(source_dangerous_actions_enabled(&load, source_two));

        load.active_jobs
            .insert(2, test_active_load_job(source_two, 0, "speed"));
        assert!(!source_dangerous_actions_enabled(&load, source_two));
    }

    #[test]
    fn m5_analysis_channels_include_every_visible_source_by_full_key() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/one.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/two.parquet".to_owned());
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .shared_time = Some(test_time_data_with_samples(&[0.0, 1.0]));
        workspace
            .source_mut(source_two)
            .expect("Source #2")
            .shared_time = Some(test_time_data_with_samples(&[10.0, 20.0, 30.0]));
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .loaded_channels
            .insert_channel(test_channel_data_with_values("iu", vec![1.0, 2.0]));
        workspace
            .source_mut(source_two)
            .expect("Source #2")
            .loaded_channels
            .insert_channel(test_channel_data_with_values("iu", vec![7.0, 8.0, 9.0]));
        let mut view = ViewState::default();
        view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"), 0),
            RowChannel::new(ChannelKey::new(source_two, "iu"), 1),
        ];

        assert_eq!(unique_visible_loaded_channels(&workspace, &view).len(), 2);
        let analysis_channels = analysis_visible_loaded_channels(&workspace, &view);
        assert_eq!(analysis_channels.len(), 2);
        assert_eq!(analysis_channels[0].0.source_id, DEFAULT_SOURCE_ID);
        assert_eq!(analysis_channels[0].1.values.as_slice(), vec![1.0, 2.0]);
        assert_eq!(analysis_channels[1].0.source_id, source_two);
        assert_eq!(
            analysis_channels[1].1.values.as_slice(),
            vec![7.0, 8.0, 9.0]
        );
    }

    #[test]
    fn workspace_cache_stats_aggregate_every_source() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());

        let source_one = workspace.source_mut(DEFAULT_SOURCE_ID).expect("Source #1");
        source_one.shared_time = Some(test_time_data_with_samples(&[0.0, 0.5, 1.0]));
        source_one
            .loaded_channels
            .insert_channel(test_channel_data_with_values("iu", vec![1.0, 2.0, 3.0]));
        source_one
            .loaded_channels
            .line_tile_cache
            .insert("iu".to_owned(), empty_line_tile_cache());
        source_one.loaded_channels.envelope_cache.insert(
            EnvelopeKey::new("iu", (0.0, 1.0), 10),
            empty_min_max_envelope(),
        );

        let source_two_state = workspace.source_mut(source_two).expect("Source #2");
        source_two_state.shared_time = Some(test_time_data_with_samples(&[0.5, 1.0]));
        source_two_state
            .loaded_channels
            .insert_channel(test_channel_data_with_values("iu", vec![4.0, 5.0]));
        source_two_state.loaded_channels.step_edge_cache.insert(
            "iu".to_owned(),
            StepEdgeCache {
                cached_time_range: None,
                sample_count: 2,
                edges: vec![StepEdge {
                    index: 1,
                    time: 1.0,
                    value: 5.0,
                }],
                elapsed: Duration::ZERO,
            },
        );
        source_two_state.loaded_channels.step_fallback_hints.insert(
            "iu".to_owned(),
            StepFallbackHint {
                min_span: 0.5,
                min_source_sample_count: 2,
            },
        );

        let stats = workspace.cache_stats();
        assert_eq!(stats.source_count, 2);
        assert_eq!(stats.loaded_channel_count, 2);
        assert_eq!(stats.raw_time_memory_bytes, 5 * std::mem::size_of::<f64>());
        assert_eq!(
            stats.raw_channel_memory_bytes,
            5 * std::mem::size_of::<f32>()
        );
        assert_eq!(
            stats.total_raw_memory_bytes(),
            stats.raw_time_memory_bytes + stats.raw_channel_memory_bytes
        );
        assert_eq!(stats.envelope_count, 1);
        assert_eq!(stats.line_tile_cache_count, 1);
        assert_eq!(stats.step_edge_cache_count, 1);
        assert!(stats.step_edge_memory_bytes > 0);
        assert_eq!(stats.step_fallback_hint_count, 1);
    }

    #[test]
    fn unloading_and_clearing_caches_are_source_isolated() {
        let mut workspace =
            WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        for source_id in [DEFAULT_SOURCE_ID, source_two] {
            let source = workspace.source_mut(source_id).expect("source");
            source
                .loaded_channels
                .insert_channel(test_channel_data("iu"));
            source
                .loaded_channels
                .line_tile_cache
                .insert("iu".to_owned(), empty_line_tile_cache());
        }

        assert!(workspace.unload_channel(&ChannelKey::new(source_two, "iu")));
        assert!(
            workspace
                .source(DEFAULT_SOURCE_ID)
                .expect("Source #1")
                .loaded_channels
                .has_channel("iu")
        );
        assert!(
            workspace
                .source(DEFAULT_SOURCE_ID)
                .expect("Source #1")
                .loaded_channels
                .has_draw_caches()
        );
        assert!(
            !workspace
                .source(source_two)
                .expect("Source #2")
                .loaded_channels
                .has_channel("iu")
        );

        workspace
            .source_mut(source_two)
            .expect("Source #2")
            .loaded_channels
            .line_tile_cache
            .insert("iu".to_owned(), empty_line_tile_cache());
        assert!(workspace.clear_draw_caches_for_source(source_two));
        assert!(
            workspace
                .source(DEFAULT_SOURCE_ID)
                .expect("Source #1")
                .loaded_channels
                .has_draw_caches()
        );
        assert!(
            !workspace
                .source(source_two)
                .expect("Source #2")
                .loaded_channels
                .has_draw_caches()
        );
    }

    #[test]
    fn load_state_keys_same_path_jobs_by_source() {
        let mut workspace = WorkspaceDataState::default();
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        let source_one_key = ChannelKey::new(DEFAULT_SOURCE_ID, "iu");
        let source_two_key = ChannelKey::new(source_two, "iu");
        let mut load = LoadState::default();
        load.active_jobs
            .insert(1, test_active_load_job(DEFAULT_SOURCE_ID, 0, "iu"));
        load.active_jobs
            .insert(2, test_active_load_job(source_two, 4, "iu"));

        assert!(load.is_channel_loading(&source_one_key));
        assert!(load.is_channel_loading(&source_two_key));
        assert!(load.is_source_loading(DEFAULT_SOURCE_ID));
        assert!(load.is_source_loading(source_two));
        assert_eq!(
            load.loading_channel_keys(),
            BTreeSet::from([source_one_key, source_two_key])
        );
        let source_two_job = load.active_jobs.get(&2).expect("Source #2 load job");
        assert_eq!(source_two_job.source_id, source_two);
        assert_eq!(source_two_job.source_generation, 4);
        assert_eq!(
            source_two_job.channel_key,
            ChannelKey::new(source_two, "iu")
        );

        assert_eq!(load.cancel_source_jobs(source_two, &workspace), 1);
        assert!(load.is_source_loading(DEFAULT_SOURCE_ID));
        assert!(!load.is_source_loading(source_two));
        assert_eq!(load.active_jobs.len(), 1);
        assert_eq!(load.pending_jobs, 1);
        assert!(
            load.progress
                .as_deref()
                .is_some_and(|progress| progress.contains("iu"))
        );
    }

    #[test]
    fn channel_load_request_rejects_a_duplicate_full_key() {
        let mut app = PanelYApp::default();
        let key = ChannelKey::new(DEFAULT_SOURCE_ID, "iu");
        let source = app
            .workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1");
        source.schema = Some(test_schema(&["iu"]));
        let generation = source.generation;
        app.load.active_jobs.insert(
            71,
            test_active_load_job(DEFAULT_SOURCE_ID, generation, "iu"),
        );
        let next_job_id = app.next_load_job_id;
        let target_row_id = app.view.rows[0].id;

        app.request_channel_load(&egui::Context::default(), &key, target_row_id, false);

        assert_eq!(app.load.active_jobs.len(), 1);
        assert_eq!(app.next_load_job_id, next_job_id);
        assert!(
            app.load
                .status
                .starts_with("Already loading: iu #1, restore load already pending")
        );
    }

    #[test]
    fn current_generation_result_applies_only_to_job_source() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        let generation = app
            .workspace
            .source(source_two)
            .expect("Source #2")
            .generation;
        app.load
            .active_jobs
            .insert(7, test_active_load_job(source_two, generation, "iu"));

        app.apply_load_result(test_load_result(
            7,
            source_two,
            generation,
            "iu",
            Ok(LoadedChannelData {
                time: Some(test_time_data_with_samples(&[0.5, 0.75, 1.0, 1.25])),
                channel: Some(test_channel_data_with_values(
                    "iu",
                    vec![2.0, 2.0, 2.0, 2.0],
                )),
            }),
        ));

        assert!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .is_none()
        );
        assert_eq!(
            app.workspace
                .channel(&ChannelKey::new(source_two, "iu"))
                .map(|channel| channel.values.len()),
            Some(4)
        );
        assert_eq!(
            app.workspace
                .source(source_two)
                .and_then(|source| source.shared_time.as_ref())
                .map(parquet_waveform::TimeData::sample_count),
            Some(4)
        );
    }

    #[test]
    fn path_a_b_a_rejects_the_old_generation_result() {
        let mut app = PanelYApp::default();
        app.workspace
            .set_source_parquet_path(DEFAULT_SOURCE_ID, "/tmp/A.parquet".to_owned());
        let old_generation = app
            .workspace
            .source(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .generation;
        app.load.active_jobs.insert(
            11,
            test_active_load_job(DEFAULT_SOURCE_ID, old_generation, "iu"),
        );

        app.workspace
            .set_source_parquet_path(DEFAULT_SOURCE_ID, "/tmp/B.parquet".to_owned());
        app.workspace
            .set_source_parquet_path(DEFAULT_SOURCE_ID, "/tmp/A.parquet".to_owned());
        app.apply_load_result(test_load_result(
            11,
            DEFAULT_SOURCE_ID,
            old_generation,
            "iu",
            Ok(LoadedChannelData {
                time: Some(test_time_data()),
                channel: Some(test_channel_data("iu")),
            }),
        ));

        let source = app.workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        assert!(source.generation > old_generation);
        assert!(source.shared_time.is_none());
        assert!(!source.loaded_channels.has_channel("iu"));
    }

    #[test]
    fn removed_source_and_mismatched_result_identity_are_rejected() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        let generation = app
            .workspace
            .source(source_two)
            .expect("Source #2")
            .generation;
        app.load
            .active_jobs
            .insert(21, test_active_load_job(source_two, generation, "iu"));
        app.workspace.remove_source(source_two);

        app.apply_load_result(test_load_result(
            21,
            source_two,
            generation,
            "iu",
            Ok(LoadedChannelData {
                time: Some(test_time_data()),
                channel: Some(test_channel_data("iu")),
            }),
        ));
        assert!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .is_none()
        );

        app.load
            .active_jobs
            .insert(22, test_active_load_job(DEFAULT_SOURCE_ID, 0, "iu"));
        app.apply_load_result(test_load_result(
            22,
            SourceId::new(99),
            0,
            "iu",
            Ok(LoadedChannelData {
                time: Some(test_time_data()),
                channel: Some(test_channel_data("iu")),
            }),
        ));
        assert!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .is_none()
        );

        app.load
            .active_jobs
            .insert(23, test_active_load_job(DEFAULT_SOURCE_ID, 0, "iu"));
        app.apply_load_result(test_load_result(
            23,
            DEFAULT_SOURCE_ID,
            0,
            "iv",
            Ok(LoadedChannelData {
                time: Some(test_time_data()),
                channel: Some(test_channel_data("iv")),
            }),
        ));
        assert!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iv"))
                .is_none()
        );
    }

    #[test]
    fn sample_count_mismatch_does_not_mutate_other_sources() {
        let mut app = PanelYApp::default();
        let source_one = app
            .workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1");
        source_one.shared_time = Some(test_time_data());
        source_one
            .loaded_channels
            .insert_channel(test_channel_data_with_values("iu", vec![1.0, 1.0, 1.0]));

        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        app.workspace
            .source_mut(source_two)
            .expect("Source #2")
            .shared_time = Some(test_time_data_with_samples(&[0.0, 1.0]));
        let generation = app
            .workspace
            .source(source_two)
            .expect("Source #2")
            .generation;
        let mut job = test_active_load_job(source_two, generation, "iu");
        job.time_was_cached = true;
        app.load.active_jobs.insert(31, job);

        app.apply_load_result(test_load_result(
            31,
            source_two,
            generation,
            "iu",
            Ok(LoadedChannelData {
                time: None,
                channel: Some(test_channel_data_with_values("iu", vec![2.0, 2.0, 2.0])),
            }),
        ));

        assert_eq!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .map(|channel| channel.values.as_ref().clone()),
            Some(vec![1.0, 1.0, 1.0])
        );
        assert!(
            app.workspace
                .channel(&ChannelKey::new(source_two, "iu"))
                .is_none()
        );
        assert_eq!(
            app.workspace
                .source(source_two)
                .and_then(|source| source.shared_time.as_ref())
                .map(parquet_waveform::TimeData::sample_count),
            Some(2)
        );
    }

    #[test]
    fn source_two_channel_failure_preserves_source_one_data() {
        let mut app = PanelYApp::default();
        let source_one = app
            .workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1");
        source_one.shared_time = Some(test_time_data());
        source_one
            .loaded_channels
            .insert_channel(test_channel_data_with_values("iu", vec![1.0, 1.0, 1.0]));
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        let generation = app
            .workspace
            .source(source_two)
            .expect("Source #2")
            .generation;
        app.load
            .active_jobs
            .insert(41, test_active_load_job(source_two, generation, "iu"));

        app.apply_load_result(test_load_result(
            41,
            source_two,
            generation,
            "iu",
            Err("synthetic read failure".to_owned()),
        ));

        assert_eq!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .map(|channel| channel.values.as_ref().clone()),
            Some(vec![1.0, 1.0, 1.0])
        );
        assert!(
            app.workspace
                .channel(&ChannelKey::new(source_two, "iu"))
                .is_none()
        );
    }

    #[test]
    fn source_aware_schema_load_and_failure_are_isolated_if_fixtures_exist() {
        let source_one_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/multi_source_validation_1.parquet");
        let source_two_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/multi_source_validation_2.parquet");
        if !source_one_path.exists() || !source_two_path.exists() {
            return;
        }

        let mut app = PanelYApp::default();
        app.workspace =
            WorkspaceDataState::with_parquet_path(source_one_path.display().to_string());
        let source_two = app
            .workspace
            .add_source(source_two_path.display().to_string());
        app.load_schema_for_source(DEFAULT_SOURCE_ID);
        app.load_schema_for_source(source_two);

        assert_eq!(
            app.workspace
                .source(DEFAULT_SOURCE_ID)
                .and_then(|source| source.schema.as_ref())
                .map(|schema| schema.row_count),
            Some(100_001)
        );
        assert_eq!(
            app.workspace
                .source(source_two)
                .and_then(|source| source.schema.as_ref())
                .map(|schema| schema.row_count),
            Some(80_001)
        );

        app.workspace
            .set_source_parquet_path(source_two, "/tmp/missing-source.parquet".to_owned());
        app.load_schema_for_source(source_two);
        assert!(
            app.workspace
                .source(source_two)
                .expect("Source #2")
                .schema
                .is_none()
        );
        assert_eq!(
            app.workspace
                .source(DEFAULT_SOURCE_ID)
                .and_then(|source| source.schema.as_ref())
                .map(|schema| schema.row_count),
            Some(100_001)
        );
    }

    #[test]
    fn multi_source_benchmark_reports_each_source_if_fixtures_exist() {
        let source_one_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/multi_source_validation_1.parquet");
        let source_two_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/multi_source_validation_2.parquet");
        if !source_one_path.exists() || !source_two_path.exists() {
            return;
        }

        let report = benchmark_multi_source(
            source_one_path.display().to_string(),
            source_two_path.display().to_string(),
        )
        .expect("multi-source benchmark");

        assert!(report.contains("workspace: 2 sources"));
        assert!(report.contains("loaded channels 6"));
        assert!(report.contains("Source #1:"));
        assert!(report.contains("Source #2:"));
        assert!(report.contains("per-source pan cache:"));
        assert!(report.contains("frames=24"));
        assert!(report.contains("hover benchmark: 1000 positions, 6000 lookups"));
    }

    fn v2_fixture_config(name: &str, missing_second: bool, all_missing: bool) -> PathBuf {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let first = base.join("multi_source_validation_1.parquet");
        let second = base.join("multi_source_validation_2.parquet");
        assert!(first.exists() && second.exists());
        let path =
            std::env::temp_dir().join(format!("panely-v2-{name}-{}.json", std::process::id()));
        let config = serde_json::json!({"version":1,"data_path":first,"rows":[],"rust":{"version":2,
            "sources":[{"key":"one","data_path":if all_missing { PathBuf::from("/missing/a.parquet") } else { first },"label":"Run A","time_offset_seconds":0.125},
                       {"key":"two","data_path":if missing_second { PathBuf::from("/missing/b.parquet") } else { second },"time_offset_seconds":-0.25}],
            "rows":[{"ymin":-2,"ymax":2,"channels":[{"source_key":"one","path":"iu","visible":false},
                       {"source_key":"two","path":"iu","visible":false,"draw_mode":"step"}]}]}});
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
        path
    }

    #[test]
    fn v2_load_save_and_apply_preserve_source_identity_offsets_and_styles() {
        let path = v2_fixture_config("roundtrip", false, false);
        let mut app = PanelYApp::default();
        app.plot_config_path = path.display().to_string();
        app.load_plot_config(&egui::Context::default());
        assert_eq!(app.workspace.source_order.len(), 2);
        assert_eq!(app.view.rows[0].channels.len(), 2);
        let ids = app.workspace.source_order.clone();
        assert_eq!(app.workspace.source(ids[0]).unwrap().label, "Run A");
        assert_eq!(
            app.workspace
                .source(ids[0])
                .unwrap()
                .time_transform
                .offset_seconds,
            0.125
        );
        assert_eq!(
            app.workspace
                .source(ids[1])
                .unwrap()
                .time_transform
                .offset_seconds,
            -0.25
        );
        assert_eq!(app.view.rows[0].channels[1].draw_mode, DrawMode::Step);
        app.save_plot_config();
        assert!(app.load.error.is_none());
        let (saved, _) = plot_config::read_plot_config(&path).unwrap();
        assert_eq!(saved.rust.sources.unwrap().len(), 2);
        app.workspace.source_order.reverse();
        app.workspace.selected_source_id = Some(ids[1]);
        app.workspace.set_source_time_offset(ids[1], 0.75);
        install_test_source_waveform(
            &mut app.workspace,
            ids[1],
            &[0.5, 1.0],
            "iu",
            vec![2.0, 3.0],
        );
        let generation = app.workspace.source(ids[1]).unwrap().generation;
        app.view.rows.clear();
        app.apply_plot_config_to_current_data(&egui::Context::default());
        assert_eq!(app.view.rows[0].channels[1].key.source_id, ids[1]);
        assert_eq!(
            app.workspace
                .source(ids[1])
                .unwrap()
                .time_transform
                .offset_seconds,
            0.75
        );
        assert_eq!(app.workspace.source(ids[1]).unwrap().generation, generation);
        assert!(
            app.workspace
                .channel(&ChannelKey::new(ids[1], "iu"))
                .is_some()
        );
        assert!(
            app.plot_config_report
                .as_ref()
                .unwrap()
                .details()
                .unwrap()
                .contains("keeping current offset")
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn three_source_plot_config_roundtrip_keeps_duplicate_paths_distinct() {
        let path = v2_fixture_config("three-sources", false, false);
        let mut config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let mut third = config["rust"]["sources"][0].clone();
        third["key"] = serde_json::json!("three");
        third["label"] = serde_json::json!("Run C");
        third["time_offset_seconds"] = serde_json::json!(0.5);
        config["rust"]["sources"]
            .as_array_mut()
            .unwrap()
            .push(third);
        config["rust"]["rows"][0]["channels"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"source_key":"three","path":"iu","visible":false}));
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
        let mut app = PanelYApp::default();
        app.plot_config_path = path.display().to_string();
        app.load_plot_config(&egui::Context::default());
        assert_eq!(app.workspace.source_order.len(), 3);
        assert_eq!(app.view.rows[0].channels.len(), 3);
        let keys = app.view.rows[0]
            .channels
            .iter()
            .map(|channel| channel.key.source_id)
            .collect::<BTreeSet<_>>();
        assert_eq!(keys.len(), 3);
        let third = app.workspace.source_order[2];
        assert_eq!(app.workspace.source(third).unwrap().label, "Run C");
        assert_eq!(
            app.workspace
                .source(third)
                .unwrap()
                .time_transform
                .offset_seconds,
            0.5
        );
        app.save_plot_config();
        assert!(app.load.error.is_none());
        app.load_plot_config(&egui::Context::default());
        assert_eq!(app.workspace.source_order.len(), 3);
        assert_eq!(app.view.rows[0].channels.len(), 3);
        assert_eq!(
            app.view.rows[0]
                .channels
                .iter()
                .map(|channel| channel.key.source_id)
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn v2_load_missing_source_partially_restores_and_all_missing_preserves_workspace() {
        let path = v2_fixture_config("partial", true, false);
        let mut app = PanelYApp::default();
        app.plot_config_path = path.display().to_string();
        app.load_plot_config(&egui::Context::default());
        assert_eq!(app.workspace.source_order.len(), 1);
        assert_eq!(app.view.rows[0].channels.len(), 1);
        assert!(
            app.plot_config_report
                .as_ref()
                .unwrap()
                .details()
                .unwrap()
                .contains("Unavailable source two")
        );
        let before = app.workspace.parquet_path.clone();
        let bad_path = v2_fixture_config("all-missing", true, true);
        app.plot_config_path = bad_path.display().to_string();
        app.load_plot_config(&egui::Context::default());
        assert_eq!(app.workspace.parquet_path, before);
        assert_eq!(app.view.rows[0].channels.len(), 1);
        assert!(app.load.status.contains("preserved"));
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(bad_path).unwrap();
    }

    #[test]
    fn v2_apply_rejects_ambiguous_current_paths_without_mutation() {
        let path = v2_fixture_config("ambiguous", false, false);
        let mut app = PanelYApp::default();
        app.plot_config_path = path.display().to_string();
        app.load_plot_config(&egui::Context::default());
        let schema = app.workspace.schema.clone().unwrap();
        let id = app.workspace.add_source(app.workspace.parquet_path.clone());
        app.workspace.source_mut(id).unwrap().schema = Some(schema);
        app.view.rows[0].channels[0].line_width = 4.0;
        app.apply_plot_config_to_current_data(&egui::Context::default());
        assert!(app.load.error.as_ref().unwrap().contains("exactly one"));
        assert_eq!(app.workspace.source_order.len(), 3);
        assert_eq!(app.view.rows[0].channels[0].line_width, 4.0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn v2_async_restore_fits_union_after_all_channels_finish() {
        let path = v2_fixture_config("async", false, false);
        let mut config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for channel in config["rust"]["rows"][0]["channels"]
            .as_array_mut()
            .unwrap()
        {
            channel["visible"] = serde_json::json!(true);
        }
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
        let mut app = PanelYApp::default();
        app.plot_config_path = path.display().to_string();
        app.load_plot_config(&egui::Context::default());
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        while app.load.is_busy() && Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
            app.drain_load_results();
        }
        assert!(!app.load.is_busy());
        assert!(!app.fit_plot_config_after_load);
        assert_eq!(app.workspace.cache_stats().loaded_channel_count, 2);
        assert_eq!(app.view.x_range, Some((0.125, 2.125)));
        assert_eq!(app.view.rows[0].channels[1].draw_mode, DrawMode::Step);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn v1_apply_to_multi_source_reports_persistent_error_without_mutation() {
        let path = v2_fixture_config("v1-guard", false, false);
        let mut config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        config.as_object_mut().unwrap().remove("rust");
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
        let mut app = PanelYApp::default();
        app.workspace.add_source("/tmp/second.parquet".into());
        app.plot_config_path = path.display().to_string();
        app.apply_plot_config_to_current_data(&egui::Context::default());
        assert_eq!(app.workspace.source_order.len(), 2);
        assert!(
            app.load
                .error
                .unwrap()
                .contains("exactly one current source")
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn plot_config_v1_load_replaces_multi_source_workspace_if_fixture_exists() {
        let data_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/multi_source_validation_1.parquet");
        if !data_path.exists() {
            return;
        }
        let config_path = std::env::temp_dir().join(format!(
            "panel_y_m6_v1_load_{}.pyc.json",
            std::process::id()
        ));
        let config = serde_json::json!({
            "version": 1,
            "data_path": data_path.display().to_string(),
            "rows": [],
            "theme": "dark",
            "scroll_zoom": false
        });
        std::fs::write(&config_path, serde_json::to_string(&config).expect("json"))
            .expect("write Plot config v1");

        let mut app = PanelYApp::default();
        app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        app.analysis.set_cursor_mode(true);
        app.analysis.place_cursor(0.5);
        app.view.hover_x = Some(0.5);
        app.plot_config_path = config_path.display().to_string();

        app.load_plot_config(&egui::Context::default());
        let _ = std::fs::remove_file(config_path);

        assert_eq!(app.workspace.source_order, vec![DEFAULT_SOURCE_ID]);
        assert_eq!(app.workspace.selected_source_id(), Some(DEFAULT_SOURCE_ID));
        let source = app.workspace.source(DEFAULT_SOURCE_ID).expect("Source #1");
        assert_eq!(source.parquet_path, data_path.display().to_string());
        assert_eq!(
            source.schema.as_ref().map(|schema| schema.row_count),
            Some(100_001)
        );
        assert_eq!(app.view.hover_x, None);
        assert_eq!(app.analysis.cursor_a, None);
        assert!(!app.analysis.cursor_mode);
        assert!(app.view.rows.iter().all(|row| {
            row.channels
                .iter()
                .all(|channel| channel.key.source_id == DEFAULT_SOURCE_ID)
        }));
        assert!(
            app.load
                .status
                .starts_with("Loaded plot config; replaced 2-source workspace with Source #1")
        );
    }

    #[test]
    fn failed_plot_config_v1_load_preserves_multi_source_workspace() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let missing_data_path = std::env::temp_dir().join(format!(
            "panel_y_m6_missing_data_{}_{}.parquet",
            std::process::id(),
            unique
        ));
        assert!(!missing_data_path.exists());
        let config_path = std::env::temp_dir().join(format!(
            "panel_y_m6_failed_load_{}_{}.pyc.json",
            std::process::id(),
            unique
        ));
        let config = serde_json::json!({
            "version": 1,
            "data_path": missing_data_path.display().to_string(),
            "rows": []
        });
        std::fs::write(&config_path, serde_json::to_string(&config).expect("json"))
            .expect("write Plot config v1");

        let mut app = PanelYApp::default();
        app.workspace = WorkspaceDataState::with_parquet_path("/tmp/source_1.parquet".to_owned());
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        let original_source_order = app.workspace.source_order.clone();
        app.plot_config_path = config_path.display().to_string();

        app.load_plot_config(&egui::Context::default());
        let _ = std::fs::remove_file(config_path);

        assert_eq!(app.workspace.source_order, original_source_order);
        assert_eq!(
            app.workspace
                .source(DEFAULT_SOURCE_ID)
                .map(|source| source.parquet_path.as_str()),
            Some("/tmp/source_1.parquet")
        );
        assert_eq!(
            app.workspace
                .source(source_two)
                .map(|source| source.parquet_path.as_str()),
            Some("/tmp/source_2.parquet")
        );
        assert_eq!(app.load.status, "Plot config data_path is invalid");
        assert!(app.load.error.is_some());
    }

    fn fft_overlay_test_app() -> (PanelYApp, SourceId) {
        let mut app = PanelYApp::default();
        app.workspace = WorkspaceDataState::with_parquet_path("/tmp/fft_1.parquet".to_owned());
        let second = app.workspace.add_source("/tmp/fft_2.parquet".to_owned());
        for (id, rate, start) in [(DEFAULT_SOURCE_ID, 1000, 0.0), (second, 2000, 10.0)] {
            let time = (0..rate)
                .map(|i| start + i as f64 / rate as f64)
                .collect::<Vec<_>>();
            let values = (0..rate)
                .map(|i| (std::f64::consts::TAU * 50.0 * i as f64 / rate as f64).sin() as f32)
                .collect();
            install_test_source_waveform(&mut app.workspace, id, &time, "iu", values);
            app.workspace
                .source_mut(id)
                .unwrap()
                .time_transform
                .offset_seconds = -start;
        }
        app.view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"), 0),
            RowChannel::new(ChannelKey::new(second, "iu"), 1),
        ];
        app.analysis.cursor_mode = true;
        app.analysis.place_cursor(0.1);
        app.analysis.place_cursor(0.9);
        app.analysis.fft.target_channels = app.view.rows[0]
            .channels
            .iter()
            .map(|channel| channel.key.clone())
            .collect();
        app.analysis.fft.target_initialized = true;
        (app, second)
    }

    fn receive_fft_test_results(app: &PanelYApp, count: usize) -> Vec<fft_analysis::FftJobResult> {
        (0..count)
            .map(|_| {
                app.fft_result_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("FFT worker completed")
            })
            .collect()
    }

    #[test]
    fn fft_overlay_workers_complete_independently_with_native_sample_grids() {
        let (mut app, second) = fft_overlay_test_app();
        let raw_pointers = app
            .workspace
            .source_order
            .iter()
            .map(|id| {
                let source = app.workspace.source(*id).unwrap();
                (
                    source.shared_time.as_ref().unwrap().time.as_ptr(),
                    app.workspace
                        .channel(&ChannelKey::new(*id, "iu"))
                        .unwrap()
                        .values
                        .as_ptr(),
                )
            })
            .collect::<Vec<_>>();
        app.start_fft_jobs(&egui::Context::default());
        assert_eq!(app.analysis.fft.jobs.len(), 2);
        let mut results = receive_fft_test_results(&app, 2);
        results.sort_by_key(|result| result.job_id);
        let later_source = results.pop().unwrap();
        app.apply_fft_result(later_source);
        assert_eq!(app.analysis.fft.results.len(), 1);
        assert!(app.analysis.fft.results.contains_key(&second));
        assert!(app.analysis.fft.is_running());
        app.apply_fft_result(results.pop().unwrap());
        assert!(!app.analysis.fft.is_running());
        assert_eq!(app.analysis.fft.results.len(), 2);
        assert!(app.analysis.fft.source_errors.is_empty());
        for (id, n, dt) in [(DEFAULT_SOURCE_ID, 801, 0.001), (second, 1601, 0.0005)] {
            let result = &app.analysis.fft.results[&id];
            assert_eq!(result.sample_count, n);
            assert!((result.dt - dt).abs() < 1.0e-12);
            assert_eq!(result.frequency_hz.len(), n / 2 + 1);
            assert_eq!(result.spectra[0].key, ChannelKey::new(id, "iu"));
            assert!((result.spectra[0].peak_freq_hz.unwrap() - 50.0).abs() < 1.3);
        }
        for (index, id) in app.workspace.source_order.iter().enumerate() {
            assert_eq!(
                raw_pointers[index].0,
                app.workspace
                    .source(*id)
                    .unwrap()
                    .shared_time
                    .as_ref()
                    .unwrap()
                    .time
                    .as_ptr()
            );
            assert_eq!(
                raw_pointers[index].1,
                app.workspace
                    .channel(&ChannelKey::new(*id, "iu"))
                    .unwrap()
                    .values
                    .as_ptr()
            );
        }
        assert_eq!(
            (app.analysis.cursor_a, app.analysis.cursor_b),
            (Some(0.1), Some(0.9))
        );
    }

    #[test]
    fn fft_overlay_source_failure_survives_another_sources_later_success() {
        let (mut app, second) = fft_overlay_test_app();
        Arc::make_mut(
            &mut app
                .workspace
                .source_mut(second)
                .unwrap()
                .shared_time
                .as_mut()
                .unwrap()
                .time,
        )[500] += 0.0001;
        app.start_fft_jobs(&egui::Context::default());
        let mut results = receive_fft_test_results(&app, 2);
        let failed = results
            .iter()
            .position(|result| result.result.is_err())
            .unwrap();
        app.apply_fft_result(results.remove(failed));
        assert!(app.analysis.fft.source_errors[&second].contains("uniformly sampled"));
        assert!(app.analysis.fft.is_running());
        app.apply_fft_result(results.pop().unwrap());
        assert!(!app.analysis.fft.is_running());
        assert_eq!(app.analysis.fft.results.len(), 1);
        assert!(app.analysis.fft.results.contains_key(&DEFAULT_SOURCE_ID));
        assert!(app.analysis.fft.source_errors.contains_key(&second));

        app.analysis.place_cursor(0.2);
        assert!(app.analysis.fft.results.is_empty());
        assert!(app.analysis.fft.source_errors.is_empty());
    }

    #[test]
    fn fft_overlay_cancel_preserves_completed_results_and_ignores_old_run() {
        let (mut app, _) = fft_overlay_test_app();
        let ctx = egui::Context::default();
        app.start_fft_jobs(&ctx);
        let mut first_run = receive_fft_test_results(&app, 2);
        app.apply_fft_result(first_run.pop().unwrap());
        app.handle_fft_action(fft_analysis::FftUiAction::Cancel, &ctx);
        assert!(!app.analysis.fft.is_running());
        assert_eq!(app.analysis.fft.results.len(), 1);
        let canceled_result = first_run.pop().unwrap();
        app.start_fft_jobs(&ctx);
        assert_eq!(app.analysis.fft.jobs.len(), 2);
        assert!(app.analysis.fft.results.is_empty());
        app.apply_fft_result(canceled_result);
        assert_eq!(app.analysis.fft.jobs.len(), 2);
        assert!(app.analysis.fft.results.is_empty());
        for result in receive_fft_test_results(&app, 2) {
            app.apply_fft_result(result);
        }
        assert_eq!(app.analysis.fft.results.len(), 2);
        assert!(!app.analysis.fft.is_running());
        assert!(app.analysis.fft.error.is_none());
    }

    #[test]
    fn fft_overlay_offset_change_rejects_all_outstanding_results() {
        let (mut app, second) = fft_overlay_test_app();
        app.start_fft_jobs(&egui::Context::default());
        let mut results = receive_fft_test_results(&app, 2);
        app.apply_fft_result(results.pop().unwrap());
        app.apply_source_time_offset(second, -9.9);
        assert!(app.analysis.fft.results.is_empty());
        assert!(!app.analysis.fft.is_running());
        app.apply_fft_result(results.pop().unwrap());
        assert!(app.analysis.fft.results.is_empty());
        assert_eq!(
            (app.analysis.cursor_a, app.analysis.cursor_b),
            (Some(0.1), Some(0.9))
        );
    }

    #[test]
    fn fft_overlay_non_target_source_removal_clears_cursor_dependent_jobs() {
        let (mut app, second) = fft_overlay_test_app();
        app.analysis.fft.target_channels.truncate(1);
        app.start_fft_jobs(&egui::Context::default());
        let result = receive_fft_test_results(&app, 1).pop().unwrap();
        app.remove_source_from_ui(second);
        assert_eq!((app.analysis.cursor_a, app.analysis.cursor_b), (None, None));
        assert!(!app.analysis.fft.is_running());
        app.apply_fft_result(result);
        assert!(app.analysis.fft.results.is_empty());
        assert_eq!(
            app.analysis.fft.target_channels,
            vec![ChannelKey::new(DEFAULT_SOURCE_ID, "iu")]
        );
    }

    fn compact_analysis_frame(
        app: &mut PanelYApp,
        ctx: &egui::Context,
        size: egui::Vec2,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ui| app.draw_ui(ui),
        )
    }

    fn visible_text_center(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    let center = text.pos + text.galley.size() * 0.5;
                    shape.clip_rect.contains(center).then_some(center)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("visible control missing: {label}"))
    }

    fn click_compact_analysis_control(
        app: &mut PanelYApp,
        ctx: &egui::Context,
        size: egui::Vec2,
        label: &str,
    ) {
        let output = compact_analysis_frame(app, ctx, size, vec![]);
        let pos = visible_text_center(&output, label);
        assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains(pos));
        for pressed in [true, false] {
            compact_analysis_frame(
                app,
                ctx,
                size,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn compact_analysis_tabs_and_visibility_preserve_completed_fft() {
        let (mut app, _) = fft_overlay_test_app();
        let ctx = egui::Context::default();
        app.start_fft_jobs(&ctx);
        for result in receive_fft_test_results(&app, 2) {
            app.apply_fft_result(result);
        }
        let size = egui::vec2(1280.0, 800.0);
        // Settle the immediate-mode layout before clicking the actual controls.
        for _ in 0..3 {
            compact_analysis_frame(&mut app, &ctx, size, vec![]);
        }
        for (label, expected) in [
            ("FFT", AnalysisPane::Fft),
            ("Statistics", AnalysisPane::Statistics),
            ("Split", AnalysisPane::Split),
            ("FFT", AnalysisPane::Fft),
        ] {
            click_compact_analysis_control(&mut app, &ctx, size, label);
            assert_eq!(app.analysis.pane, expected);
            assert_eq!(app.analysis.fft.results.len(), 2);
            assert_eq!(
                (app.analysis.cursor_a, app.analysis.cursor_b),
                (Some(0.1), Some(0.9))
            );
        }
        click_compact_analysis_control(&mut app, &ctx, size, "Collapse");
        assert!(app.analysis.info_panel_collapsed);
        click_compact_analysis_control(&mut app, &ctx, size, "FFT");
        assert!(!app.analysis.info_panel_collapsed);
        let output = compact_analysis_frame(&mut app, &ctx, size, vec![]);
        visible_text_center(&output, "Run FFT");
        click_compact_analysis_control(&mut app, &ctx, size, "Hide");
        assert!(!app.analysis.info_panel_visible);
        assert_eq!(app.analysis.fft.results.len(), 2);
    }

    #[test]
    fn statistics_horizontal_scroll_reaches_complete_slope_in_clipped_panel() {
        let (mut app, _) = fft_overlay_test_app();
        let mut stats = wait_for_interval_statistics(&mut app.analysis, &app.workspace, &app.view);
        stats[0].slope_per_second = Some(12345.123456);
        let ctx = egui::Context::default();
        let frame = |events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 320.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    // A child can have a wider layout rect than its parent clip after grids expand.
                    ui.set_clip_rect(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 320.0),
                    ));
                    draw_interval_statistics_table(ui, &stats, 280.0);
                },
            )
        };
        for _ in 0..3 {
            frame(vec![]);
        }
        frame(vec![
            egui::Event::PointerMoved(egui::pos2(600.0, 70.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: egui::vec2(-1000.0, 0.0),
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        for _ in 0..20 {
            frame(vec![]);
        }
        let output = frame(vec![]);
        let (rect, clip) = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "12345.123456" => Some((
                    egui::Rect::from_min_size(text.pos, text.galley.size()),
                    shape.clip_rect,
                )),
                _ => None,
            })
            .expect("slope value is rendered");
        assert!(
            clip.contains_rect(rect),
            "complete slope value must scroll into view: {rect:?}, {clip:?}"
        );
    }

    #[test]
    fn compact_analysis_window_resize_preserves_requested_panel_height() {
        let (mut app, _) = fft_overlay_test_app();
        let ctx = egui::Context::default();
        app.analysis.pane = AnalysisPane::Fft;
        app.analysis.info_panel_height = 600.0;
        for size in [
            egui::vec2(1280.0, 1000.0),
            egui::vec2(1024.0, 600.0),
            egui::vec2(1280.0, 1000.0),
        ] {
            for _ in 0..3 {
                let output = compact_analysis_frame(&mut app, &ctx, size, vec![]);
                for label in ["Statistics", "FFT", "Collapse", "Hide", "Run FFT"] {
                    let center = visible_text_center(&output, label);
                    assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains(center));
                }
            }
            assert_eq!(app.analysis.info_panel_height, 600.0);
        }
    }

    #[test]
    fn analysis_split_layout_preserves_fft_minimum_when_height_allows() {
        let body_height = 360.0;
        let (interval_height, handle_height, fft_height) =
            analysis_split_layout(body_height, 260.0);

        assert_eq!(handle_height, ANALYSIS_SPLIT_HANDLE_HEIGHT);
        assert_eq!(interval_height, 232.0);
        assert_eq!(fft_height, FFT_SECTION_MIN_HEIGHT);
        assert!((interval_height + handle_height + fft_height - body_height).abs() < f32::EPSILON);
    }

    #[test]
    fn analysis_split_layout_compacts_for_short_panels() {
        let body_height = 90.0;
        let (interval_height, handle_height, fft_height) =
            analysis_split_layout(body_height, ANALYSIS_SPLIT_DEFAULT_HEIGHT);

        assert_eq!(handle_height, 0.0);
        assert_eq!(interval_height, ANALYSIS_SPLIT_MIN_HEIGHT);
        assert_eq!(fft_height, 18.0);
        assert!((interval_height + handle_height + fft_height - body_height).abs() < f32::EPSILON);
    }

    #[test]
    fn analysis_split_height_is_not_persistently_clamped_without_drag() {
        let desired_height = 260.0;
        let short_panel_height = 90.0;

        let stored_height =
            analysis_split_height_after_handle_response(desired_height, short_panel_height, None);
        let (restored_interval_height, _, restored_fft_height) =
            analysis_split_layout(360.0, stored_height);

        assert_eq!(stored_height, desired_height);
        assert_eq!(restored_interval_height, 232.0);
        assert_eq!(restored_fft_height, FFT_SECTION_MIN_HEIGHT);
    }

    #[test]
    fn analysis_split_height_drag_clamps_to_current_bounds() {
        let stored_height = analysis_split_height_after_handle_response(260.0, 90.0, Some(0.0));

        assert_eq!(stored_height, ANALYSIS_SPLIT_MIN_HEIGHT);
    }

    #[test]
    fn analysis_split_height_sanitizes_non_finite_values() {
        let stored_height = analysis_split_height_after_handle_response(f32::NAN, 360.0, None);

        assert_eq!(stored_height, ANALYSIS_SPLIT_DEFAULT_HEIGHT);
    }

    #[test]
    fn cursor_info_input_block_rect_covers_resize_handle_and_panel() {
        let available_rect =
            egui::Rect::from_min_size(egui::pos2(20.0, 10.0), egui::vec2(800.0, 600.0));

        let rect = cursor_info_input_block_rect(available_rect, 220.0, 8.0).expect("block rect");

        assert_eq!(rect.left(), available_rect.left());
        assert_eq!(rect.right(), available_rect.right());
        assert_eq!(rect.top(), available_rect.bottom() - 228.0);
        assert_eq!(rect.bottom(), available_rect.bottom());
        assert!(pointer_is_over_rect(
            Some(egui::pos2(40.0, 390.0)),
            Some(rect)
        ));
        assert!(pointer_is_over_rect(
            Some(egui::pos2(40.0, 500.0)),
            Some(rect)
        ));
        assert!(!pointer_is_over_rect(
            Some(egui::pos2(40.0, 300.0)),
            Some(rect)
        ));
    }

    #[test]
    fn cursor_info_input_block_rect_is_empty_without_panel_height() {
        let available_rect =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));

        assert!(cursor_info_input_block_rect(available_rect, 0.0, 0.0).is_none());
        assert!(!pointer_is_over_rect(Some(available_rect.center()), None));
    }

    #[test]
    fn cursor_info_input_block_rect_clamps_to_available_area() {
        let available_rect =
            egui::Rect::from_min_size(egui::pos2(0.0, 40.0), egui::vec2(800.0, 80.0));

        let rect = cursor_info_input_block_rect(available_rect, 96.0, 8.0).expect("block rect");

        assert_eq!(rect.top(), available_rect.top());
        assert_eq!(rect.bottom(), available_rect.bottom());
        assert!(pointer_is_over_rect(
            Some(available_rect.center()),
            Some(rect)
        ));
    }

    #[test]
    fn default_view_uses_plain_dark_display_mode() {
        let view = ViewState::default();

        assert_eq!(view.theme, ThemeMode::Dark);
        assert_eq!(view.visual_mode, VisualMode::Plain);
        assert_eq!(view.display_mode_choice(), DisplayModeChoice::PlainDark);
    }

    #[test]
    fn display_mode_choice_maps_to_theme_and_visual_mode() {
        let mut view = ViewState::default();

        view.set_display_mode_choice(DisplayModeChoice::PlainLight);
        assert_eq!(view.theme, ThemeMode::Light);
        assert_eq!(view.visual_mode, VisualMode::Plain);
        assert_eq!(view.display_mode_choice(), DisplayModeChoice::PlainLight);

        view.set_display_mode_choice(DisplayModeChoice::Rock);
        assert_eq!(view.theme, ThemeMode::Dark);
        assert_eq!(view.visual_mode, VisualMode::Rock);
        assert_eq!(view.display_mode_choice(), DisplayModeChoice::Rock);

        view.set_display_mode_choice(DisplayModeChoice::PlainDark);
        assert_eq!(view.theme, ThemeMode::Dark);
        assert_eq!(view.visual_mode, VisualMode::Plain);
        assert_eq!(view.display_mode_choice(), DisplayModeChoice::PlainDark);
    }

    #[test]
    fn plain_dark_copies_current_rock_skin_with_separate_control_token() {
        let plain_dark = visual_palette(ThemeMode::Dark, VisualMode::Plain);
        let rock = visual_palette(ThemeMode::Dark, VisualMode::Rock);

        assert_eq!(plain_dark.visual_mode, VisualMode::Plain);
        assert_eq!(rock.visual_mode, VisualMode::Rock);
        assert_eq!(plain_dark.skin, VisualSkin::Industrial);
        assert_eq!(rock.skin, VisualSkin::Industrial);
        assert_eq!(
            plain_dark.control_button_treatment,
            ControlButtonTreatment::PlainDarkMachinery
        );
        assert_eq!(
            plain_dark.control_toggle_treatment,
            ControlToggleTreatment::Standard
        );
        assert_eq!(
            plain_dark.control_section_header_treatment,
            ControlSectionHeaderTreatment::Standard
        );
        assert_eq!(
            plain_dark.control_status_indicator_treatment,
            ControlStatusIndicatorTreatment::Standard
        );
        assert_eq!(
            plain_dark.control_text_input_treatment,
            ControlTextInputTreatment::Standard
        );
        assert_eq!(
            plain_dark.control_surface_texture_treatment,
            ControlSurfaceTextureTreatment::Standard
        );
        assert_eq!(
            rock.control_button_treatment,
            ControlButtonTreatment::RockMachinery
        );
        assert_eq!(
            rock.control_toggle_treatment,
            ControlToggleTreatment::RockMachinery
        );
        assert_eq!(
            rock.control_section_header_treatment,
            ControlSectionHeaderTreatment::RockMachineryRack
        );
        assert_eq!(
            rock.control_status_indicator_treatment,
            ControlStatusIndicatorTreatment::RockMachineryLed
        );
        assert_eq!(
            rock.control_text_input_treatment,
            ControlTextInputTreatment::RockInstrumentDisplay
        );
        assert_eq!(
            rock.control_surface_texture_treatment,
            ControlSurfaceTextureTreatment::RockSubtleGrime
        );
        assert_ne!(
            plain_dark.control_button_treatment,
            rock.control_button_treatment
        );
        assert_ne!(
            plain_dark.control_toggle_treatment,
            rock.control_toggle_treatment
        );
        assert_ne!(
            plain_dark.control_section_header_treatment,
            rock.control_section_header_treatment
        );
        assert_ne!(
            plain_dark.control_status_indicator_treatment,
            rock.control_status_indicator_treatment
        );
        assert_ne!(
            plain_dark.control_text_input_treatment,
            rock.control_text_input_treatment
        );
        assert_ne!(
            plain_dark.control_surface_texture_treatment,
            rock.control_surface_texture_treatment
        );
        assert!(plain_dark.control_button_treatment.uses_machinery());
        assert!(rock.control_button_treatment.uses_machinery());
        assert_eq!(
            VisualPalette {
                visual_mode: VisualMode::Rock,
                control_button_treatment: ControlButtonTreatment::RockMachinery,
                control_toggle_treatment: ControlToggleTreatment::RockMachinery,
                control_section_header_treatment: ControlSectionHeaderTreatment::RockMachineryRack,
                control_status_indicator_treatment:
                    ControlStatusIndicatorTreatment::RockMachineryLed,
                control_text_input_treatment: ControlTextInputTreatment::RockInstrumentDisplay,
                control_surface_texture_treatment: ControlSurfaceTextureTreatment::RockSubtleGrime,
                ..plain_dark
            },
            rock
        );

        let plain_dark_button_accent =
            control_button_accent(plain_dark, ControlButtonRole::Default);
        assert_eq!(
            plain_dark_button_accent,
            control_button_accent(plain_dark, ControlButtonRole::Primary)
        );
        assert_eq!(
            plain_dark_button_accent,
            control_button_accent(plain_dark, ControlButtonRole::Destructive)
        );
        assert_eq!(
            plain_dark_button_accent,
            control_button_accent(plain_dark, ControlButtonRole::Utility)
        );

        let rock_primary_accent = control_button_accent(rock, ControlButtonRole::Primary);
        let rock_destructive_accent = control_button_accent(rock, ControlButtonRole::Destructive);
        let rock_utility_accent = control_button_accent(rock, ControlButtonRole::Utility);
        assert_eq!(
            rock_primary_accent,
            control_button_accent(rock, ControlButtonRole::Default)
        );
        assert_ne!(rock_primary_accent, rock_destructive_accent);
        assert_ne!(rock_primary_accent, rock_utility_accent);
        assert_ne!(rock_destructive_accent, rock_utility_accent);
        assert_eq!(rock_primary_accent.edge, rock.accent_blue);

        assert_eq!(control_toggle_accent(plain_dark), None);
        let rock_toggle_accent = control_toggle_accent(rock).expect("rock toggle accent");
        assert_eq!(rock_toggle_accent.edge, rock.accent_blue);
        assert_eq!(
            rock_toggle_accent.glow,
            egui::Color32::from_rgb(69, 216, 255)
        );
        assert_ne!(rock_toggle_accent.edge, rock_toggle_accent.off_edge);

        assert_eq!(control_section_header_accent(plain_dark), None);
        let rock_section_header_accent =
            control_section_header_accent(rock).expect("rock section header accent");
        assert_eq!(rock_section_header_accent.line, rock.accent_blue);
        assert_eq!(
            rock_section_header_accent.glow,
            egui::Color32::from_rgb(69, 216, 255)
        );
        assert_ne!(
            rock_section_header_accent.line,
            rock_section_header_accent.screw_edge
        );

        assert_eq!(
            control_status_indicator_accent(plain_dark, ControlStatusLevel::Good),
            None
        );
        let rock_good_status =
            control_status_indicator_accent(rock, ControlStatusLevel::Good).expect("good LED");
        let rock_info_status =
            control_status_indicator_accent(rock, ControlStatusLevel::Info).expect("info LED");
        let rock_busy_status =
            control_status_indicator_accent(rock, ControlStatusLevel::Busy).expect("busy LED");
        let rock_warning_status =
            control_status_indicator_accent(rock, ControlStatusLevel::Warning)
                .expect("warning LED");
        let rock_danger_status =
            control_status_indicator_accent(rock, ControlStatusLevel::Danger).expect("danger LED");
        assert_ne!(rock_good_status.core, rock_info_status.core);
        assert_ne!(rock_info_status.core, rock_busy_status.core);
        assert_ne!(rock_busy_status.core, rock_warning_status.core);
        assert_ne!(rock_warning_status.core, rock_danger_status.core);
        assert_eq!(rock_info_status.core, rock.accent_blue);
        assert_eq!(
            rock_danger_status.core,
            egui::Color32::from_rgb(255, 82, 82)
        );

        assert_eq!(control_text_input_accent(plain_dark), None);
        let rock_text_input_accent =
            control_text_input_accent(rock).expect("rock text input accent");
        assert_eq!(rock_text_input_accent.edge, rock.accent_blue);
        assert_eq!(rock_text_input_accent.selection_stroke, rock.accent_blue);
        assert_ne!(rock_text_input_accent.background, plain_dark.input_bg);
        assert_ne!(
            rock_text_input_accent.selection_bg,
            egui::Color32::TRANSPARENT
        );

        assert_eq!(control_surface_texture_accent(plain_dark), None);
        let rock_surface_texture_accent =
            control_surface_texture_accent(rock).expect("rock surface texture accent");
        assert!(rock_surface_texture_accent.scanline.a() <= 14);
        assert!(rock_surface_texture_accent.scratch.a() <= 28);
        assert!(rock_surface_texture_accent.grime.a() <= 22);
        assert!(rock_surface_texture_accent.separator_shadow.a() <= 80);
        assert!(rock_surface_texture_accent.separator_edge.a() <= 40);
        assert!(rock_surface_texture_accent.separator_glow.a() <= 24);
        assert_ne!(
            rock_surface_texture_accent.scanline,
            rock_surface_texture_accent.scratch
        );
        assert_ne!(
            rock_surface_texture_accent.separator_shadow,
            rock_surface_texture_accent.separator_edge
        );

        let plain_dark_visuals = view_visuals(ThemeMode::Dark, VisualMode::Plain);
        let rock_visuals = view_visuals(ThemeMode::Dark, VisualMode::Rock);

        assert!(plain_dark_visuals.dark_mode);
        assert!(rock_visuals.dark_mode);
        assert_eq!(plain_dark_visuals.panel_fill, rock_visuals.panel_fill);
        assert_eq!(
            plain_dark_visuals.extreme_bg_color,
            rock_visuals.extreme_bg_color
        );
        assert_eq!(
            plain_dark_visuals.widgets.hovered.bg_fill,
            rock_visuals.widgets.hovered.bg_fill
        );
        assert_eq!(
            plain_dark_visuals.selection.stroke.color,
            rock_visuals.selection.stroke.color
        );
    }

    #[test]
    fn status_indicator_levels_classify_load_channel_and_cache_state() {
        let ready_load = LoadState {
            status: "Ready".to_owned(),
            pending_jobs: 0,
            progress: None,
            error: None,
            active_jobs: BTreeMap::new(),
        };
        assert_eq!(
            load_status_indicator_level(&ready_load),
            ControlStatusLevel::Good
        );

        let busy_load = LoadState {
            status: "Loading: gate -> Row 1".to_owned(),
            pending_jobs: 1,
            progress: Some("loading: gate -> Row 1".to_owned()),
            error: None,
            active_jobs: BTreeMap::new(),
        };
        assert_eq!(
            load_status_indicator_level(&busy_load),
            ControlStatusLevel::Busy
        );

        let error_load = LoadState {
            status: "Schema load failed".to_owned(),
            pending_jobs: 0,
            progress: None,
            error: Some("invalid schema".to_owned()),
            active_jobs: BTreeMap::new(),
        };
        assert_eq!(
            load_status_indicator_level(&error_load),
            ControlStatusLevel::Danger
        );

        assert_eq!(
            channel_load_status_indicator_level(ChannelLoadState::Loaded),
            ControlStatusLevel::Good
        );
        assert_eq!(
            channel_load_status_indicator_level(ChannelLoadState::Loading),
            ControlStatusLevel::Busy
        );
        assert_eq!(
            channel_load_status_indicator_level(ChannelLoadState::Unloaded),
            ControlStatusLevel::Warning
        );
        assert_eq!(
            cache_count_status_indicator_level(0),
            ControlStatusLevel::Warning
        );
        assert_eq!(
            cache_count_status_indicator_level(1),
            ControlStatusLevel::Info
        );
    }

    #[test]
    fn adds_channels_to_the_selected_row() {
        let mut view = ViewState::default();

        let (added_first, first_row_id) = view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        assert!(added_first);
        assert_eq!(first_row_id, 0);
        assert_eq!(view.rows[0].channels.len(), 1);
        assert_eq!(view.rows[0].y_range.mode, YRangeMode::Auto);
        assert_eq!(view.rows[0].channels[0].draw_mode, DrawMode::Line);
        assert!(view.rows[0].channels[0].visible);
        assert_eq!(
            view.rows[0].channels[0].line_width,
            DEFAULT_TRACE_LINE_WIDTH
        );
        assert_eq!(view.rows[0].channels[0].color_override, None);

        let second_row_id = view.add_row();
        assert_eq!(view.rows[1].y_range, RowYRange::default());
        let (added_second, target_row_id) = view.add_channel_to_selected_row(&test_key("pwm_1kHz"));
        assert!(added_second);
        assert_eq!(target_row_id, second_row_id);
        assert!(
            view.rows[0]
                .channels
                .iter()
                .any(|ch| ch.key.channel_path == "sine_50Hz")
        );
        assert!(
            view.rows[1]
                .channels
                .iter()
                .any(|ch| ch.key.channel_path == "pwm_1kHz")
        );
    }

    #[test]
    fn rejects_duplicate_channels_within_a_row() {
        let mut view = ViewState::default();
        let same_path_other_source = ChannelKey::new(SourceId::new(2), "sine_50Hz");
        let different_path_other_source = ChannelKey::new(SourceId::new(2), "speed");

        let (added_first, _) = view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        let (added_duplicate, _) = view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        let (added_other_source, _) = view.add_channel_to_selected_row(&same_path_other_source);
        let (added_other_path, _) = view.add_channel_to_selected_row(&different_path_other_source);

        assert!(added_first);
        assert!(!added_duplicate);
        assert!(added_other_source);
        assert!(added_other_path);
        assert_eq!(view.rows[0].channels.len(), 3);
        assert_eq!(view.rows[0].channels[0].key.channel_path, "sine_50Hz");
        assert_eq!(view.rows[0].channels[1].key.channel_path, "sine_50Hz");
        assert_eq!(view.rows[0].channels[2].key.channel_path, "speed");
        assert_ne!(
            view.rows[0].channels[0].key.source_id,
            view.rows[0].channels[1].key.source_id
        );
    }

    #[test]
    fn row_remove_uses_the_full_channel_key() {
        let source_two = SourceId::new(2);
        let source_one_key = ChannelKey::new(DEFAULT_SOURCE_ID, "iu");
        let source_two_key = ChannelKey::new(source_two, "iu");
        let mut view = ViewState::default();
        view.rows[0].channels = vec![
            RowChannel::new(source_one_key.clone(), 0),
            RowChannel::new(source_two_key.clone(), 1),
        ];

        view.rows[0]
            .channels
            .retain(|channel| channel.key != source_two_key);

        assert_eq!(view.rows[0].channels.len(), 1);
        assert_eq!(view.rows[0].channels[0].key, source_one_key);
    }

    #[test]
    fn hidden_channels_are_not_counted_as_visible() {
        let mut view = ViewState::default();
        let (added, _) = view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        assert!(added);
        assert!(view.has_visible_channels());

        view.rows[0].channels[0].visible = false;

        assert!(!view.has_visible_channels());
    }

    fn session_preset_fixture_path(index: usize) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!(
                "../../data/multi_source_validation_{index}.parquet"
            ))
            .display()
            .to_string()
    }

    fn session_preset_fixture_app() -> (PanelYApp, SourceId) {
        let mut app = PanelYApp::default();
        app.workspace = WorkspaceDataState::with_parquet_path(session_preset_fixture_path(1));
        let second = app.workspace.add_source(session_preset_fixture_path(2));
        for id in app.workspace.source_order.clone() {
            let source = app.workspace.source_mut(id).unwrap();
            source.schema =
                Some(parquet_schema::read_schema_summary(Path::new(&source.parquet_path)).unwrap());
        }
        app.view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"), 0),
            RowChannel::new(ChannelKey::new(second, "iu"), 1),
        ];
        (app, second)
    }

    fn finish_session_preset_loads(app: &mut PanelYApp) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.load.is_busy() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            app.drain_load_results();
        }
        app.drain_load_results();
        assert!(!app.load.is_busy(), "session preset load timed out");
    }

    fn click_cursor_mode(app: &mut PanelYApp) {
        let ctx = egui::Context::default();
        let palette = app.view.visual_palette();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            draw_analysis_controls(ui, &mut app.analysis, &app.workspace, palette);
        });
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Cursor mode" => {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
            .expect("Cursor mode checkbox is rendered");
        for pressed in [true, false] {
            let _ = ctx.run_ui(
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| {
                    draw_analysis_controls(ui, &mut app.analysis, &app.workspace, palette);
                },
            );
        }
    }

    #[test]
    fn cursor_mode_uses_later_source_after_hidden_first_source_config_restore() {
        let (mut app, _) = session_preset_fixture_app();
        click_cursor_mode(&mut app);
        assert!(
            !app.analysis.cursor_mode,
            "schema alone cannot enable cursors"
        );
        app.view.rows[0].channels[0].visible = false;
        let path = std::env::temp_dir().join(format!(
            "panel_y_hidden_first_cursor_{}.pyc.json",
            std::process::id()
        ));
        app.plot_config_path = path.display().to_string();
        app.save_plot_config();
        app.load_plot_config(&egui::Context::default());
        finish_session_preset_loads(&mut app);
        assert!(app.load.error.is_none(), "{:?}", app.load.error);
        let ids = &app.workspace.source_order;
        assert_eq!(ids.len(), 2);
        assert_eq!(app.workspace.selected_source_id(), Some(ids[0]));
        assert!(app.workspace.source(ids[0]).unwrap().shared_time.is_none());
        assert!(app.workspace.source(ids[1]).unwrap().shared_time.is_some());
        assert!(!app.view.rows[0].channels[0].visible);
        assert!(app.view.rows[0].channels[1].visible);
        click_cursor_mode(&mut app);
        assert!(
            app.analysis.cursor_mode,
            "the loaded second source enables cursors"
        );
        assert!(app.analysis.place_cursor(0.75));
        assert!(app.analysis.place_cursor(1.25));
        assert_eq!(app.analysis.cursor_a, Some(0.75));
        assert_eq!(app.analysis.cursor_b, Some(1.25));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn plot_config_round_trip_with_unconfigured_first_source() {
        let mut app = PanelYApp::default();
        app.workspace = WorkspaceDataState::with_parquet_path(String::new());
        let second = app.workspace.add_source(session_preset_fixture_path(2));
        app.workspace.add_source(String::new());
        app.load_schema_for_source(second);
        app.workspace.set_source_time_offset(second, 0.25);
        app.view.rows[0].channels = vec![RowChannel::new(ChannelKey::new(second, "iu"), 1)];
        app.view.rows[0].channels[0].draw_mode = DrawMode::Step;
        app.request_channel_load(
            &egui::Context::default(),
            &ChannelKey::new(second, "iu"),
            0,
            false,
        );
        finish_session_preset_loads(&mut app);
        assert!(app.load.error.is_none(), "{:?}", app.load.error);
        assert!(app.workspace.selected_source().unwrap().schema.is_none());
        assert!(app.workspace.source(second).unwrap().shared_time.is_some());
        let path = std::env::temp_dir().join(format!(
            "panel_y_unconfigured_first_{}.pyc.json",
            std::process::id()
        ));
        app.plot_config_path = path.display().to_string();
        app.save_plot_config();
        assert!(
            app.load.status.starts_with("Saved plot config"),
            "{}",
            app.load.status
        );
        let (saved, report) = plot_config::read_plot_config(&path).unwrap();
        assert_eq!(report.warning_count(), 0);
        let sources = saved.rust.sources.as_ref().unwrap();
        assert_eq!(sources.len(), 1, "empty source cards are not persisted");
        assert_eq!(
            saved.data_path,
            PathBuf::from(session_preset_fixture_path(2))
        );
        assert_eq!(saved.rows[0].channels, vec!["iu"]);
        assert_eq!(
            saved.rust.rows[0].channels[0].source_key.as_deref(),
            Some(sources[0].key.as_str())
        );

        app.workspace.set_source_time_offset(second, -0.5);
        app.view.rows[0].channels.clear();
        app.apply_plot_config_to_current_data(&egui::Context::default());
        finish_session_preset_loads(&mut app);
        let report = app.plot_config_report.as_ref().unwrap();
        assert_eq!(report.warning_count(), 1);
        assert_eq!(
            report.ignored_fields,
            vec![format!(
                "{}.time_offset_seconds (keeping current offset)",
                sources[0].key
            )]
        );
        assert_eq!(app.load.error, report.details());
        assert_eq!(app.workspace.source_order.len(), 3);
        assert_eq!(app.view.rows[0].channels[0].key.source_id, second);
        assert_eq!(
            app.workspace
                .source(second)
                .unwrap()
                .time_transform
                .offset_seconds,
            -0.5
        );

        app.load_plot_config(&egui::Context::default());
        finish_session_preset_loads(&mut app);
        assert!(app.load.error.is_none(), "{:?}", app.load.error);
        assert_eq!(app.workspace.source_order.len(), 1);
        assert_eq!(app.plot_config_report.as_ref().unwrap().warning_count(), 0);
        let restored = app.workspace.selected_source().unwrap();
        assert_eq!(restored.time_transform.offset_seconds, 0.25);
        assert!(restored.shared_time.is_some());
        assert_eq!(app.view.rows[0].channels[0].key.source_id, restored.id);
        assert_eq!(app.view.rows[0].channels[0].draw_mode, DrawMode::Step);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn plot_config_default_path_uses_later_loaded_source() {
        let directory = std::env::temp_dir().join(format!(
            "panel_y_later_source_default_path_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let data_path = directory.join("second.parquet");
        std::fs::copy(session_preset_fixture_path(2), &data_path).unwrap();
        let mut app = PanelYApp::default();
        app.workspace = WorkspaceDataState::with_parquet_path(String::new());
        let second = app.workspace.add_source(data_path.display().to_string());
        app.load_schema_for_source(second);
        app.plot_config_path.clear();
        app.save_plot_config();
        let expected = directory.join("second.pyc.json");
        assert_eq!(Path::new(&app.plot_config_path), expected);
        assert!(expected.is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn three_source_session_preset_restores_removed_middle_source_and_selection() {
        let (mut app, second) = session_preset_fixture_app();
        app.add_source_from_ui();
        let third = app.workspace.source_order[2];
        app.select_parquet_path_for_source(third, session_preset_fixture_path(1));
        app.load_schema_for_source(third);
        app.workspace
            .source_mut(third)
            .unwrap()
            .time_transform
            .offset_seconds = 0.5;
        app.view.rows[0]
            .channels
            .push(RowChannel::new(ChannelKey::new(third, "iu"), 2));
        for channel in &mut app.view.rows[0].channels {
            channel.visible = false;
        }
        app.workspace.selected_source_id = Some(third);
        app.view.selected_channel = Some(ChannelKey::new(third, "iu"));
        app.save_session_preset();
        assert!(app.session_preset.is_some());
        app.remove_source_from_ui(second);
        app.apply_session_preset(&egui::Context::default());
        assert!(!app.load.is_busy());
        assert_eq!(app.workspace.source_order.len(), 3);
        assert_eq!(app.workspace.source_order[0], DEFAULT_SOURCE_ID);
        assert_eq!(app.workspace.source_order[2], third);
        let restored_middle = app.workspace.source_order[1];
        assert_ne!(restored_middle, second);
        assert_eq!(
            app.workspace.source(restored_middle).unwrap().parquet_path,
            session_preset_fixture_path(2)
        );
        assert_eq!(
            app.view.rows[0].channels[1].key,
            ChannelKey::new(restored_middle, "iu")
        );
        assert_eq!(app.view.rows[0].channels.len(), 3);
        assert_eq!(app.workspace.selected_source_id(), Some(third));
        assert_eq!(
            app.view.selected_channel,
            Some(ChannelKey::new(third, "iu"))
        );
        assert_eq!(
            app.workspace
                .source(third)
                .unwrap()
                .time_transform
                .offset_seconds,
            0.5
        );
        assert_eq!(app.workspace.source_number(third), Some(3));
    }

    #[test]
    fn multi_source_session_preset_restores_metadata_styles_and_reuses_raw_allocations() {
        let (mut app, second) = session_preset_fixture_app();
        for (id, values) in [
            (DEFAULT_SOURCE_ID, vec![1.0, 2.0, 3.0]),
            (second, vec![10.0, 20.0, 30.0]),
        ] {
            install_test_source_waveform(&mut app.workspace, id, &[0.0, 1.0, 2.0], "iu", values);
            let source = app.workspace.source_mut(id).unwrap();
            source
                .loaded_channels
                .line_tile_cache
                .insert("iu".to_owned(), empty_line_tile_cache());
            source.loaded_channels.envelope_cache.insert(
                EnvelopeKey::new("iu", (0.0, 2.0), 10),
                empty_min_max_envelope(),
            );
        }
        app.workspace.source_order = vec![second, DEFAULT_SOURCE_ID];
        app.workspace.selected_source_id = Some(second);
        app.workspace.source_mut(second).unwrap().label = "Scope B".to_owned();
        app.workspace
            .source_mut(second)
            .unwrap()
            .time_transform
            .offset_seconds = -0.25;
        app.workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .unwrap()
            .time_transform
            .offset_seconds = 0.125;
        app.view.selected_channel = Some(ChannelKey::new(second, "iu"));
        app.view.rows[0].channels[1].draw_mode = DrawMode::Step;
        app.view.rows[0].channels[1].visible = false;
        app.view.rows[0].channels[1].line_width = 3.0;
        app.view.rows[0].channels[1].color_override = Some(egui::Color32::from_rgb(17, 34, 51));
        app.view.rows[0].y_range = RowYRange {
            mode: YRangeMode::Manual,
            min: Some(-3.0),
            max: Some(5.0),
            last_auto: None,
        };
        let selected_row = app.view.add_row();
        app.view
            .add_channel_to_selected_row(&ChannelKey::new(second, "iu"));
        app.view.x_range = Some((0.5, 1.25));
        app.view.large_preview_enabled = true;
        let allocations = app
            .workspace
            .source_order
            .iter()
            .map(|id| {
                let source = app.workspace.source(*id).unwrap();
                (
                    *id,
                    source.shared_time.as_ref().unwrap().time.as_ptr(),
                    app.workspace
                        .channel(&ChannelKey::new(*id, "iu"))
                        .unwrap()
                        .values
                        .as_ptr(),
                )
            })
            .collect::<Vec<_>>();
        app.save_session_preset();
        assert!(app.session_preset.is_some());

        app.workspace.source_order.reverse();
        app.workspace.selected_source_id = Some(DEFAULT_SOURCE_ID);
        app.workspace.source_mut(second).unwrap().label = "Changed".to_owned();
        app.workspace
            .source_mut(second)
            .unwrap()
            .time_transform
            .offset_seconds = 8.0;
        app.workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .unwrap()
            .time_transform
            .offset_seconds = 3.0;
        app.view = ViewState::default();
        app.analysis.cursor_mode = true;
        app.analysis.cursor_a = Some(0.6);
        app.analysis.cursor_b = Some(0.8);
        app.apply_session_preset(&egui::Context::default());

        assert_eq!(app.workspace.source_order, vec![second, DEFAULT_SOURCE_ID]);
        assert_eq!(app.workspace.selected_source_id(), Some(second));
        assert_eq!(app.workspace.source(second).unwrap().label, "Scope B");
        assert_eq!(
            app.workspace
                .source(second)
                .unwrap()
                .time_transform
                .offset_seconds,
            -0.25
        );
        assert_eq!(
            app.workspace
                .source(DEFAULT_SOURCE_ID)
                .unwrap()
                .time_transform
                .offset_seconds,
            0.125
        );
        assert_eq!(
            app.view.selected_channel,
            Some(ChannelKey::new(second, "iu"))
        );
        assert_eq!(app.view.selected_row_id, Some(selected_row));
        assert_eq!(app.view.x_range, Some((0.5, 1.25)));
        assert!(app.view.large_preview_enabled);
        assert_eq!(app.view.rows.len(), 2);
        assert_eq!(
            app.view.rows[0].channels[0].key,
            ChannelKey::new(DEFAULT_SOURCE_ID, "iu")
        );
        assert_eq!(
            app.view.rows[0].channels[1].key,
            ChannelKey::new(second, "iu")
        );
        let restored = &app.view.rows[0].channels[1];
        assert_eq!(restored.draw_mode, DrawMode::Step);
        assert!(!restored.visible);
        assert_eq!(restored.line_width, 3.0);
        assert_eq!(
            restored.color_override,
            Some(egui::Color32::from_rgb(17, 34, 51))
        );
        assert_eq!(app.view.rows[0].y_range.mode, YRangeMode::Manual);
        assert_eq!(
            (app.view.rows[0].y_range.min, app.view.rows[0].y_range.max),
            (Some(-3.0), Some(5.0))
        );
        assert_eq!((app.analysis.cursor_a, app.analysis.cursor_b), (None, None));
        assert!(!app.analysis.cursor_mode);
        assert_eq!(app.load.pending_jobs, 0);
        for (id, time_ptr, value_ptr) in allocations {
            let source = app.workspace.source(id).unwrap();
            assert_eq!(source.shared_time.as_ref().unwrap().time.as_ptr(), time_ptr);
            assert_eq!(
                app.workspace
                    .channel(&ChannelKey::new(id, "iu"))
                    .unwrap()
                    .values
                    .as_ptr(),
                value_ptr
            );
            assert!(source.loaded_channels.line_tile_cache.is_empty());
            assert!(source.loaded_channels.envelope_cache.is_empty());
        }
        assert_eq!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .unwrap()
                .values
                .as_slice(),
            vec![1.0, 2.0, 3.0]
        );
        assert_eq!(
            app.workspace
                .channel(&ChannelKey::new(second, "iu"))
                .unwrap()
                .values
                .as_slice(),
            vec![10.0, 20.0, 30.0]
        );
    }

    #[test]
    fn multi_source_session_preset_remaps_removed_source_and_reloads_visible_channels() {
        let (mut app, second) = session_preset_fixture_app();
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 1.0, 2.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        app.workspace.selected_source_id = Some(second);
        app.workspace
            .source_mut(second)
            .unwrap()
            .time_transform
            .offset_seconds = 0.25;
        app.view.selected_channel = Some(ChannelKey::new(second, "iu"));
        app.view.x_range = Some((0.8, 1.2));
        app.save_session_preset();
        app.workspace.remove_source(second);
        app.view = ViewState::default();
        app.apply_session_preset(&egui::Context::default());
        assert_eq!(app.workspace.source_order.len(), 2);
        let restored_second = app.workspace.source_order[1];
        assert_ne!(restored_second, second);
        assert_eq!(app.workspace.selected_source_id(), Some(restored_second));
        assert_eq!(
            app.view.selected_channel,
            Some(ChannelKey::new(restored_second, "iu"))
        );
        assert_eq!(
            app.view.rows[0].channels[1].key,
            ChannelKey::new(restored_second, "iu")
        );
        finish_session_preset_loads(&mut app);
        assert_eq!(app.workspace.cache_stats().loaded_channel_count, 2);
        assert_eq!(
            app.workspace
                .source(restored_second)
                .unwrap()
                .time_transform
                .offset_seconds,
            0.25
        );
        assert_eq!(app.view.x_range, Some((0.8, 1.2)));
        let traces = app.visible_row_traces(64, false, false);
        assert_eq!(traces[0].traces.len(), 2);
        assert_eq!(app.view.x_range, Some((0.8, 1.2)));
        assert_eq!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
                .unwrap()
                .values
                .as_slice(),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn multi_source_session_preset_restores_from_empty_workspace_and_keeps_hidden_unloaded() {
        let (mut app, second) = session_preset_fixture_app();
        app.view.rows[0].channels[1].visible = false;
        app.workspace.selected_source_id = Some(second);
        app.view.x_range = Some((0.2, 0.8));
        app.save_session_preset();
        for id in app.workspace.source_order.clone() {
            app.workspace.remove_source(id);
        }
        app.view = ViewState::default();
        app.apply_session_preset(&egui::Context::default());
        finish_session_preset_loads(&mut app);
        assert_eq!(app.workspace.source_order.len(), 2);
        assert_eq!(app.view.rows[0].channels.len(), 2);
        assert_eq!(app.workspace.cache_stats().loaded_channel_count, 1);
        assert_eq!(app.view.x_range, Some((0.2, 0.8)));
        let hidden = &app.view.rows[0].channels[1];
        assert!(!hidden.visible);
        assert!(app.workspace.channel(&hidden.key).is_none());
        assert_eq!(
            app.workspace.selected_source_id(),
            Some(hidden.key.source_id)
        );
    }

    #[test]
    fn session_preset_does_not_reuse_same_source_id_after_path_replacement() {
        let (mut app, second) = session_preset_fixture_app();
        app.workspace.remove_source(second);
        app.view.rows[0].channels.truncate(1);
        app.view.rows[0].channels[0].visible = false;
        app.save_session_preset();
        assert!(
            app.workspace
                .set_source_parquet_path(DEFAULT_SOURCE_ID, session_preset_fixture_path(2))
        );
        app.workspace.schema = Some(test_schema(&["iu"]));
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 1.0, 2.0],
            "iu",
            vec![777.0, 777.0, 777.0],
        );
        app.apply_session_preset(&egui::Context::default());
        assert_eq!(app.workspace.source_order.len(), 1);
        let restored_id = app.workspace.source_order[0];
        assert_ne!(restored_id, DEFAULT_SOURCE_ID);
        assert_eq!(
            app.workspace.source(restored_id).unwrap().parquet_path,
            session_preset_fixture_path(1)
        );
        assert!(
            app.workspace
                .channel(&ChannelKey::new(restored_id, "iu"))
                .is_none()
        );
        assert_eq!(
            app.view.rows[0].channels[0].key,
            ChannelKey::new(restored_id, "iu")
        );
        assert!(!app.load.is_busy());
    }

    #[test]
    fn session_preset_partially_restores_missing_sources_and_missing_channels() {
        let mut app = PanelYApp::default();
        app.workspace =
            WorkspaceDataState::with_parquet_path("/tmp/panel-y-session-kept.parquet".to_owned());
        app.workspace.schema = Some(test_schema(&["iu", "gone"]));
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 1.0, 2.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        let missing_path = format!(
            "/tmp/panel-y-session-missing-{}.parquet",
            std::process::id()
        );
        assert!(!Path::new(&missing_path).exists());
        let missing = app.workspace.add_source(missing_path.clone());
        app.workspace.source_mut(missing).unwrap().schema = Some(test_schema(&["iu"]));
        app.workspace.selected_source_id = Some(missing);
        app.view.selected_channel = Some(ChannelKey::new(missing, "iu"));
        app.view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"), 0),
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "gone"), 1),
            RowChannel::new(ChannelKey::new(missing, "iu"), 2),
        ];
        app.save_session_preset();
        app.workspace.remove_source(missing);
        app.workspace.source_mut(DEFAULT_SOURCE_ID).unwrap().schema = Some(test_schema(&["iu"]));
        app.apply_session_preset(&egui::Context::default());
        assert_eq!(app.workspace.source_order, vec![DEFAULT_SOURCE_ID]);
        assert_eq!(app.workspace.selected_source_id(), Some(DEFAULT_SOURCE_ID));
        assert_eq!(app.view.rows[0].channels.len(), 1);
        assert_eq!(
            app.view.selected_channel,
            Some(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"))
        );
        let warning = app.load.error.as_deref().expect("partial restore warning");
        assert!(warning.contains(&missing_path));
        assert!(warning.contains("gone"));
        assert!(!app.load.is_busy());
    }

    #[test]
    fn session_preset_all_unavailable_sources_preserves_current_workspace_view_and_analysis() {
        let mut app = PanelYApp::default();
        let missing_path = format!(
            "/tmp/panel-y-session-all-missing-{}.parquet",
            std::process::id()
        );
        assert!(!Path::new(&missing_path).exists());
        app.workspace = WorkspaceDataState::with_parquet_path(missing_path.clone());
        app.workspace.schema = Some(test_schema(&["iu"]));
        app.view
            .add_channel_to_selected_row(&ChannelKey::new(DEFAULT_SOURCE_ID, "iu"));
        app.save_session_preset();
        app.workspace.set_source_parquet_path(
            DEFAULT_SOURCE_ID,
            "/tmp/panel-y-session-current.parquet".to_owned(),
        );
        app.workspace.schema = Some(test_schema(&["current"]));
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 1.0, 2.0],
            "current",
            vec![9.0, 8.0, 7.0],
        );
        app.view = ViewState::default();
        app.view
            .add_channel_to_selected_row(&ChannelKey::new(DEFAULT_SOURCE_ID, "current"));
        app.view.x_range = Some((0.5, 1.5));
        app.analysis.cursor_mode = true;
        app.analysis.cursor_a = Some(0.6);
        app.analysis.cursor_b = Some(0.9);
        let next_id = app.workspace.next_source_id;
        app.apply_session_preset(&egui::Context::default());
        assert_eq!(
            app.workspace.parquet_path,
            "/tmp/panel-y-session-current.parquet"
        );
        assert_eq!(app.workspace.next_source_id, next_id);
        assert_eq!(
            app.view.rows[0].channels[0].key,
            ChannelKey::new(DEFAULT_SOURCE_ID, "current")
        );
        assert_eq!(app.view.x_range, Some((0.5, 1.5)));
        assert_eq!(
            (app.analysis.cursor_a, app.analysis.cursor_b),
            (Some(0.6), Some(0.9))
        );
        assert!(app.analysis.cursor_mode);
        assert_eq!(
            app.workspace
                .channel(&ChannelKey::new(DEFAULT_SOURCE_ID, "current"))
                .unwrap()
                .values
                .as_slice(),
            vec![9.0, 8.0, 7.0]
        );
        assert!(app.load.error.as_deref().unwrap().contains(&missing_path));
        assert!(app.session_preset.is_some());
    }

    #[test]
    fn session_preset_backend_guards_busy_schema_less_and_empty_saves() {
        let mut app = PanelYApp::default();
        app.save_session_preset();
        assert!(app.session_preset.is_none());
        app.workspace.remove_source(DEFAULT_SOURCE_ID);
        app.save_session_preset();
        assert!(app.session_preset.is_none());

        let (mut app, _) = session_preset_fixture_app();
        app.view.rows[0]
            .channels
            .iter_mut()
            .for_each(|channel| channel.visible = false);
        app.view.x_range = Some((0.1, 0.9));
        app.save_session_preset();
        let saved_summary = app.session_preset.as_ref().unwrap().summary();
        let generation = app.workspace.source(DEFAULT_SOURCE_ID).unwrap().generation;
        app.load.active_jobs.insert(
            123,
            test_active_load_job(DEFAULT_SOURCE_ID, generation, "iu"),
        );
        app.load.refresh_progress(&app.workspace);
        app.view.add_row();
        app.view.x_range = Some((1.0, 1.5));
        app.save_session_preset();
        assert_eq!(
            app.session_preset.as_ref().unwrap().summary(),
            saved_summary
        );
        app.apply_session_preset(&egui::Context::default());
        assert_eq!(app.view.rows.len(), 2);
        assert_eq!(app.view.x_range, Some((1.0, 1.5)));
        assert_eq!(app.load.active_jobs.len(), 1);
        app.load.active_jobs.clear();
        app.load.refresh_progress(&app.workspace);
        app.apply_session_preset(&egui::Context::default());
        assert_eq!(app.view.rows.len(), 1);
        assert_eq!(app.view.x_range, Some((0.1, 0.9)));
        app.clear_session_preset();
        assert!(app.session_preset.is_none());
        assert_eq!(app.workspace.source_order.len(), 2);
    }

    #[test]
    fn session_preset_repeated_apply_reuses_remapped_sources_without_new_loads() {
        let (mut app, old_second) = session_preset_fixture_app();
        app.workspace.selected_source_id = Some(old_second);
        app.view.selected_channel = Some(ChannelKey::new(old_second, "iu"));
        app.view.x_range = Some((0.8, 1.2));
        app.save_session_preset();
        for id in app.workspace.source_order.clone() {
            app.workspace.remove_source(id);
        }
        app.apply_session_preset(&egui::Context::default());
        finish_session_preset_loads(&mut app);
        let restored_order = app.workspace.source_order.clone();
        let restored_second = restored_order[1];
        assert_ne!(restored_second, old_second);
        let allocations = restored_order
            .iter()
            .map(|id| {
                let source = app.workspace.source(*id).unwrap();
                (
                    *id,
                    source.shared_time.as_ref().unwrap().time.as_ptr(),
                    app.workspace
                        .channel(&ChannelKey::new(*id, "iu"))
                        .unwrap()
                        .values
                        .as_ptr(),
                )
            })
            .collect::<Vec<_>>();
        let next_source_id = app.workspace.next_source_id;
        let next_load_job_id = app.next_load_job_id;
        for _ in 0..2 {
            app.view.x_range = Some((1.0, 1.5));
            app.workspace
                .source_mut(restored_second)
                .unwrap()
                .time_transform
                .offset_seconds = 0.25;
            app.apply_session_preset(&egui::Context::default());
            assert!(!app.load.is_busy());
            assert_eq!(app.workspace.source_order, restored_order);
            assert_eq!(app.workspace.next_source_id, next_source_id);
            assert_eq!(app.next_load_job_id, next_load_job_id);
            assert_eq!(app.workspace.selected_source_id(), Some(restored_second));
            assert_eq!(
                app.view.selected_channel,
                Some(ChannelKey::new(restored_second, "iu"))
            );
            assert_eq!(
                app.view.rows[0].channels[1].key,
                ChannelKey::new(restored_second, "iu")
            );
            assert_eq!(app.view.x_range, Some((0.8, 1.2)));
            assert_eq!(
                app.workspace
                    .source(restored_second)
                    .unwrap()
                    .time_transform
                    .offset_seconds,
                0.0
            );
            for &(id, time_ptr, value_ptr) in &allocations {
                assert_eq!(
                    app.workspace
                        .source(id)
                        .unwrap()
                        .shared_time
                        .as_ref()
                        .unwrap()
                        .time
                        .as_ptr(),
                    time_ptr
                );
                assert_eq!(
                    app.workspace
                        .channel(&ChannelKey::new(id, "iu"))
                        .unwrap()
                        .values
                        .as_ptr(),
                    value_ptr
                );
            }
        }
    }

    #[test]
    fn session_preset_all_hidden_preserves_saved_range_when_rendered_without_time() {
        let (mut app, _) = session_preset_fixture_app();
        for channel in &mut app.view.rows[0].channels {
            channel.visible = false;
        }
        app.view.x_range = Some((0.8, 1.2));
        app.save_session_preset();
        for id in app.workspace.source_order.clone() {
            app.workspace.remove_source(id);
        }
        app.apply_session_preset(&egui::Context::default());
        assert!(!app.load.is_busy());
        assert_eq!(app.workspace.cache_stats().loaded_channel_count, 0);
        assert!(app.workspace.display_time_range().is_none());
        for _ in 0..2 {
            let rows = app.visible_row_traces(64, false, false);
            assert_eq!(rows.len(), 1);
            assert!(rows[0].traces.is_empty());
            assert_eq!(app.view.x_range, Some((0.8, 1.2)));
        }
        let key = app.view.rows[0].channels[0].key.clone();
        app.view.rows[0].channels[0].visible = true;
        app.request_channel_load(&egui::Context::default(), &key, app.view.rows[0].id, false);
        finish_session_preset_loads(&mut app);
        assert_eq!(app.visible_row_traces(64, false, false)[0].traces.len(), 1);
        assert_eq!(app.view.x_range, Some((0.8, 1.2)));
    }

    #[test]
    fn session_preset_restores_lightweight_view_state() {
        let schema = test_schema(&["sine_50Hz", "pwm_1kHz"]);
        let mut view = ViewState {
            selected_channel: Some(test_key("pwm_1kHz")),
            x_range: Some((1.25, 2.5)),
            large_preview_enabled: true,
            ..ViewState::default()
        };
        let (added_first, _) = view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        assert!(added_first);
        view.rows[0].y_range = RowYRange {
            mode: YRangeMode::Manual,
            min: Some(-2.0),
            max: Some(3.0),
            last_auto: Some((-1.0, 1.0)),
        };
        view.rows[0].channels[0].visible = false;
        view.rows[0].channels[0].draw_mode = DrawMode::Step;
        view.rows[0].channels[0].color_override = Some(egui::Color32::from_rgb(10, 20, 30));
        view.rows[0].channels[0].line_width = 2.5;
        let second_row_id = view.add_row();
        let (added_second, _) = view.add_channel_to_selected_row(&test_key("pwm_1kHz"));
        assert!(added_second);

        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/test.parquet".to_owned());
        let preset = SessionPreset::capture(&workspace, &view);
        let mut restored = ViewState::default();
        workspace.source_mut(DEFAULT_SOURCE_ID).unwrap().schema = Some(schema);
        let report = preset.apply_to_view(
            &mut restored,
            &workspace,
            &BTreeMap::from([(DEFAULT_SOURCE_ID, DEFAULT_SOURCE_ID)]),
        );

        assert_eq!(report.restored_rows, 2);
        assert_eq!(report.restored_channels, 2);
        assert!(report.missing_channels.is_empty());
        assert_eq!(restored.x_range, Some((1.25, 2.5)));
        assert!(restored.large_preview_enabled);
        assert_eq!(restored.selected_channel, Some(test_key("pwm_1kHz")));
        assert_eq!(restored.selected_row_id, Some(second_row_id));
        assert_eq!(restored.rows[0].y_range.mode, YRangeMode::Manual);
        assert_eq!(
            (restored.rows[0].y_range.min, restored.rows[0].y_range.max),
            (Some(-2.0), Some(3.0))
        );
        assert!(!restored.rows[0].channels[0].visible);
        assert_eq!(restored.rows[0].channels[0].draw_mode, DrawMode::Step);
        assert_eq!(
            restored.rows[0].channels[0].color_override,
            Some(egui::Color32::from_rgb(10, 20, 30))
        );
        assert_eq!(restored.rows[0].channels[0].line_width, 2.5);
    }

    #[test]
    fn session_preset_skips_channels_missing_from_schema() {
        let original_schema = test_schema(&["sine_50Hz", "missing_later"]);
        let target_schema = test_schema(&["sine_50Hz"]);
        let mut view = ViewState::default();
        let (added_first, _) = view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        assert!(added_first);
        let (added_second, _) = view.add_channel_to_selected_row(&test_key("missing_later"));
        assert!(added_second);

        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/test.parquet".to_owned());
        let preset = SessionPreset::capture(&workspace, &view);
        let mut restored = ViewState::default();
        workspace.source_mut(DEFAULT_SOURCE_ID).unwrap().schema = Some(target_schema);
        let report = preset.apply_to_view(
            &mut restored,
            &workspace,
            &BTreeMap::from([(DEFAULT_SOURCE_ID, DEFAULT_SOURCE_ID)]),
        );

        assert_eq!(original_schema.channels.len(), 2);
        assert_eq!(report.restored_rows, 1);
        assert_eq!(report.restored_channels, 1);
        assert_eq!(
            report.missing_channels,
            vec!["Source 1:missing_later".to_owned()]
        );
        assert_eq!(restored.rows[0].channels.len(), 1);
        assert_eq!(restored.rows[0].channels[0].key.channel_path, "sine_50Hz");
    }

    #[test]
    fn converts_plot_x_to_time_with_clamping() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 0.0), egui::vec2(100.0, 50.0));

        assert_eq!(time_at_plot_x(10.0, rect, (2.0, 12.0)), Some(2.0));
        assert_eq!(time_at_plot_x(60.0, rect, (2.0, 12.0)), Some(7.0));
        assert_eq!(time_at_plot_x(120.0, rect, (2.0, 12.0)), Some(12.0));
    }

    #[test]
    fn box_zoom_view_range_accepts_reversed_drag() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 0.0), egui::vec2(100.0, 50.0));
        let box_zoom = BoxZoomDrag {
            plot_rect: rect,
            start_pos: egui::pos2(90.0, 10.0),
            current_pos: egui::pos2(30.0, 40.0),
            view_range: (0.0, 10.0),
        };

        let range = box_zoom_view_range(&box_zoom, (0.0, 10.0), 0.01).expect("box zoom range");
        assert!((range.0 - 2.0).abs() < 1.0e-6);
        assert!((range.1 - 8.0).abs() < 1.0e-6);
    }

    #[test]
    fn box_zoom_view_range_clamps_outside_plot_rect() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 0.0), egui::vec2(100.0, 50.0));
        let box_zoom = BoxZoomDrag {
            plot_rect: rect,
            start_pos: egui::pos2(-50.0, 10.0),
            current_pos: egui::pos2(250.0, 40.0),
            view_range: (0.0, 10.0),
        };

        assert_eq!(
            box_zoom_view_range(&box_zoom, (0.0, 10.0), 0.01),
            Some((0.0, 10.0))
        );
    }

    #[test]
    fn box_zoom_view_range_ignores_tiny_drags_and_respects_min_span() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 0.0), egui::vec2(100.0, 50.0));
        let tiny_drag = BoxZoomDrag {
            plot_rect: rect,
            start_pos: egui::pos2(60.0, 10.0),
            current_pos: egui::pos2(61.0, 40.0),
            view_range: (0.0, 10.0),
        };
        assert_eq!(box_zoom_view_range(&tiny_drag, (0.0, 10.0), 0.01), None);

        let narrow_drag = BoxZoomDrag {
            plot_rect: rect,
            start_pos: egui::pos2(60.0, 10.0),
            current_pos: egui::pos2(65.0, 40.0),
            view_range: (0.0, 10.0),
        };
        let range = box_zoom_view_range(&narrow_drag, (0.0, 10.0), 1.0).expect("box zoom range");
        assert!((range.0 - 4.75).abs() < 1.0e-6);
        assert!((range.1 - 5.75).abs() < 1.0e-6);
    }

    #[test]
    fn line_hover_uses_nearest_sample_value() {
        let time = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 10.0, 20.0, 30.0];

        assert_eq!(
            hover_value_at_time(&time, &values, 1.6, DrawMode::Line),
            Some(20.0)
        );
        assert_eq!(
            hover_value_at_time(&time, &values, 1.5, DrawMode::Line),
            Some(10.0)
        );
        assert_eq!(
            hover_value_at_time(&time, &values, -0.1, DrawMode::Line),
            None
        );
        assert_eq!(
            hover_value_at_time(&time, &values, 3.1, DrawMode::Line),
            None
        );
    }

    #[test]
    fn step_hover_uses_held_sample_value() {
        let time = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 10.0, 20.0, 30.0];

        assert_eq!(
            hover_value_at_time(&time, &values, 1.9, DrawMode::Step),
            Some(10.0)
        );
        assert_eq!(
            hover_value_at_time(&time, &values, 2.0, DrawMode::Step),
            Some(20.0)
        );
        assert_eq!(
            hover_value_at_time(&time, &values, -0.1, DrawMode::Step),
            None
        );
        assert_eq!(
            hover_value_at_time(&time, &values, 3.1, DrawMode::Step),
            None
        );
    }

    #[test]
    fn cursor_placement_alternates_a_b_then_restarts_a() {
        let mut analysis = AnalysisState::default();
        analysis.set_cursor_mode(true);

        assert!(analysis.place_cursor(0.1));
        assert_eq!(analysis.cursor_a, Some(0.1));
        assert_eq!(analysis.cursor_b, None);
        assert_eq!(analysis.next_cursor, CursorSlot::B);

        assert!(analysis.place_cursor(0.3));
        assert_eq!(analysis.cursor_a, Some(0.1));
        assert_eq!(analysis.cursor_b, Some(0.3));
        assert_eq!(analysis.cursor_interval(), Some((0.1, 0.3)));
        assert_eq!(analysis.next_cursor, CursorSlot::A);

        assert!(analysis.place_cursor(0.2));
        assert_eq!(analysis.cursor_a, Some(0.2));
        assert_eq!(analysis.cursor_b, None);
        assert_eq!(analysis.next_cursor, CursorSlot::B);
    }

    #[test]
    fn disabling_cursor_mode_clears_cursors() {
        let mut analysis = AnalysisState::default();
        analysis.set_cursor_mode(true);
        analysis.place_cursor(0.1);
        analysis.place_cursor(0.3);

        analysis.set_cursor_mode(false);

        assert!(!analysis.cursor_mode);
        assert_eq!(analysis.cursor_a, None);
        assert_eq!(analysis.cursor_b, None);
        assert_eq!(analysis.next_cursor, CursorSlot::A);
    }

    #[test]
    fn cursor_readouts_use_unique_visible_loaded_channels() {
        let mut workspace = WorkspaceDataState::default();
        workspace.shared_time = Some(test_time_data());
        workspace
            .loaded_channels
            .insert_channel(test_channel_data("sine_50Hz"));
        workspace
            .loaded_channels
            .insert_channel(test_channel_data("hidden"));

        let mut view = ViewState::default();
        view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        view.add_row();
        view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        view.add_channel_to_selected_row(&test_key("hidden"));
        view.rows[1].channels[1].visible = false;

        let mut analysis = AnalysisState::default();
        analysis.cursor_a = Some(0.49);
        analysis.cursor_b = Some(0.9);

        let readouts = cursor_channel_readouts(&analysis, &workspace, &view);

        assert_eq!(readouts.len(), 1);
        assert_eq!(readouts[0].channel_path, "sine_50Hz");
        assert_eq!(readouts[0].value_a, Some(1.0));
        assert_eq!(readouts[0].value_b, Some(0.0));
        assert_eq!(readouts[0].delta, Some(-1.0));
    }

    #[test]
    fn multi_source_hover_and_cursor_use_each_source_time_transform_and_range() {
        let mut app = PanelYApp::default();
        let source_two = app.workspace.add_source("/tmp/source_2.parquet".to_owned());
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 1.0, 2.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        install_test_source_waveform(
            &mut app.workspace,
            source_two,
            &[10.0, 10.5, 11.0],
            "iu",
            vec![10.0, 20.0, 30.0],
        );
        app.workspace
            .source_mut(source_two)
            .expect("Source #2")
            .time_transform = TimeTransform {
            offset_seconds: -10.0,
        };
        app.view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"), 0),
            RowChannel::new(ChannelKey::new(source_two, "iu"), 1),
        ];
        app.view.x_range = Some((0.0, 2.0));
        app.view.hover_x = Some(0.49);

        let rows = app.visible_row_traces(64, false, false);
        assert_eq!(rows[0].traces.len(), 2);
        assert_eq!(rows[0].traces[0].hover_value, Some(1.0));
        assert_eq!(rows[0].traces[1].hover_value, Some(20.0));

        app.view.hover_x = Some(1.5);
        let rows = app.visible_row_traces(64, false, false);
        assert_eq!(rows[0].traces[0].hover_value, Some(2.0));
        assert_eq!(rows[0].traces[1].hover_value, None);

        app.analysis.cursor_a = Some(0.49);
        app.analysis.cursor_b = Some(1.5);
        let readouts = cursor_channel_readouts(&app.analysis, &app.workspace, &app.view);
        assert_eq!(readouts.len(), 2);
        assert_eq!(readouts[0].display_label, "iu #1");
        assert_eq!(readouts[0].value_a, Some(1.0));
        assert_eq!(readouts[0].value_b, Some(2.0));
        assert_eq!(readouts[1].display_label, "iu #2");
        assert_eq!(readouts[1].value_a, Some(20.0));
        assert_eq!(readouts[1].value_b, None);
        assert_eq!(readouts[1].delta, None);
    }

    #[test]
    fn visible_loaded_channels_are_unique_by_full_channel_key() {
        let mut workspace = WorkspaceDataState::default();
        let source_two_id = SourceId::new(2);
        let mut source_two =
            DataSourceState::new(source_two_id, "/tmp/source_two.parquet".to_owned());
        workspace
            .loaded_channels
            .insert_channel(test_channel_data("iu"));
        source_two
            .loaded_channels
            .insert_channel(test_channel_data("iu"));
        workspace.sources.insert(source_two_id, source_two);

        let mut view = ViewState::default();
        view.add_channel_to_selected_row(&test_key("iu"));
        view.add_channel_to_selected_row(&test_key("iu"));
        view.add_channel_to_selected_row(&ChannelKey::new(source_two_id, "iu"));

        let channels = unique_visible_loaded_channels(&workspace, &view);

        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].0, test_key("iu"));
        assert_eq!(channels[1].0, ChannelKey::new(source_two_id, "iu"));
    }

    #[test]
    fn interval_statistics_use_raw_unique_visible_loaded_channels() {
        let mut workspace = WorkspaceDataState::default();
        workspace.shared_time = Some(test_time_data());
        workspace
            .loaded_channels
            .insert_channel(test_channel_data("sine_50Hz"));
        workspace
            .loaded_channels
            .insert_channel(test_channel_data("hidden"));

        let mut view = ViewState::default();
        view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        view.add_row();
        view.add_channel_to_selected_row(&test_key("sine_50Hz"));
        view.add_channel_to_selected_row(&test_key("hidden"));
        view.rows[1].channels[1].visible = false;

        let mut analysis = AnalysisState::default();
        analysis.cursor_a = Some(0.49);
        analysis.cursor_b = Some(0.9);

        let stats = wait_for_interval_statistics(&mut analysis, &workspace, &view);

        assert_eq!(stats.len(), 1);
        let stats = &stats[0];
        assert_eq!(stats.channel_path, "sine_50Hz");
        assert_eq!(stats.value_a, Some(1.0));
        assert_eq!(stats.value_b, Some(0.0));
        assert_eq!(stats.delta, Some(-1.0));
        assert_eq!(stats.mean, Some(0.5));
        assert_eq!(stats.max, Some(1.0));
        assert_eq!(stats.min, Some(0.0));
        assert_eq!(stats.peak_to_peak, Some(1.0));
        assert!((stats.rms.expect("rms") - 0.5_f64.sqrt()).abs() < 1.0e-12);
        assert!((stats.slope_per_second.expect("slope") - (-1.0 / (0.9 - 0.49))).abs() < 1.0e-12);
    }

    #[test]
    fn interval_statistics_use_each_sources_raw_cursor_range() {
        let mut workspace = WorkspaceDataState::default();
        let source_two = workspace.add_source("/tmp/source_2.parquet".to_owned());
        install_test_source_waveform(
            &mut workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 1.0, 2.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        install_test_source_waveform(
            &mut workspace,
            source_two,
            &[10.0, 10.5, 11.0],
            "iu",
            vec![10.0, 20.0, 30.0],
        );
        workspace
            .source_mut(source_two)
            .expect("Source #2")
            .time_transform = TimeTransform {
            offset_seconds: -10.0,
        };

        let mut view = ViewState::default();
        view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(DEFAULT_SOURCE_ID, "iu"), 0),
            RowChannel::new(ChannelKey::new(source_two, "iu"), 1),
        ];
        let mut analysis = AnalysisState::default();
        analysis.cursor_a = Some(0.49);
        analysis.cursor_b = Some(0.9);

        let stats = wait_for_interval_statistics(&mut analysis, &workspace, &view);

        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].display_label, "iu #1");
        assert_eq!(stats[0].value_a, Some(1.0));
        assert_eq!(stats[0].value_b, Some(2.0));
        assert_eq!(stats[0].mean, Some(1.5));
        assert_eq!(stats[1].display_label, "iu #2");
        assert_eq!(stats[1].value_a, Some(20.0));
        assert_eq!(stats[1].value_b, Some(30.0));
        assert_eq!(stats[1].mean, Some(25.0));
    }

    fn wait_for_interval_statistics(
        analysis: &mut AnalysisState,
        workspace: &WorkspaceDataState,
        view: &ViewState,
    ) -> Vec<IntervalChannelStats> {
        let ctx = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let stats = analysis.interval_channel_statistics(workspace, view, &ctx);
            if analysis.interval_stats_cache.key.is_some() {
                return stats;
            }
            assert!(Instant::now() < deadline, "statistics worker timed out");
            assert!(analysis.interval_stats_worker.error.is_none());
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn interval_statistics_cache_reuses_same_cursor_channel_set() {
        let mut workspace = WorkspaceDataState::default();
        workspace.shared_time = Some(test_time_data());
        workspace
            .loaded_channels
            .insert_channel(test_channel_data("sine_50Hz"));

        let mut view = ViewState::default();
        view.add_channel_to_selected_row(&test_key("sine_50Hz"));

        let mut analysis = AnalysisState::default();
        analysis.cursor_a = Some(0.49);
        analysis.cursor_b = Some(0.9);

        let first = wait_for_interval_statistics(&mut analysis, &workspace, &view);
        let first_key = analysis
            .interval_stats_cache
            .key
            .clone()
            .expect("cache key");
        let second =
            analysis.interval_channel_statistics(&workspace, &view, &egui::Context::default());

        assert_eq!(first, second);
        assert_eq!(analysis.interval_stats_cache.key, Some(first_key));
        assert_eq!(analysis.interval_stats_cache.stats, first);
    }

    #[test]
    fn interval_statistics_cache_key_tracks_source_generation_and_time_transform() {
        let mut workspace = WorkspaceDataState::default();
        install_test_source_waveform(
            &mut workspace,
            DEFAULT_SOURCE_ID,
            &[0.0, 0.5, 1.0],
            "iu",
            vec![1.0, 2.0, 3.0],
        );
        let mut view = ViewState::default();
        view.add_channel_to_selected_row(&test_key("iu"));
        let mut analysis = AnalysisState::default();
        analysis.cursor_a = Some(0.25);
        analysis.cursor_b = Some(0.75);

        let original =
            interval_stats_cache_key(&analysis, &workspace, &view).expect("original key");
        let source = workspace.source_mut(DEFAULT_SOURCE_ID).expect("Source #1");
        source.generation += 1;
        let next_generation =
            interval_stats_cache_key(&analysis, &workspace, &view).expect("generation key");
        assert_ne!(original, next_generation);

        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .time_transform = TimeTransform {
            offset_seconds: 0.25,
        };
        let next_transform =
            interval_stats_cache_key(&analysis, &workspace, &view).expect("transform key");
        assert_ne!(next_generation, next_transform);
    }

    #[test]
    fn auto_y_range_uses_padded_trace_range() {
        let range = display_value_range((1.0, 3.0), RowYRange::default()).expect("auto y range");

        assert!((range.0 - 0.9).abs() < 1.0e-12);
        assert!((range.1 - 3.1).abs() < 1.0e-12);
    }

    #[test]
    fn manual_y_range_uses_row_values_without_padding() {
        let range = display_value_range(
            (-10.0, 10.0),
            RowYRange {
                mode: YRangeMode::Manual,
                min: Some(-2.5),
                max: Some(7.5),
                ..RowYRange::default()
            },
        )
        .expect("manual y range");

        assert_eq!(range, (-2.5, 7.5));
    }

    #[test]
    fn manual_y_range_accepts_reversed_inputs() {
        let range = display_value_range(
            (-10.0, 10.0),
            RowYRange {
                mode: YRangeMode::Manual,
                min: Some(5.0),
                max: Some(-1.0),
                ..RowYRange::default()
            },
        )
        .expect("manual y range");

        assert_eq!(range, (-1.0, 5.0));
    }

    #[test]
    fn manual_y_range_seeds_from_last_auto_range() {
        let mut y_range = RowYRange::default();
        y_range.set_last_auto((-2.0, 4.0));

        y_range.set_manual_from_last_auto();

        assert_eq!(y_range.mode, YRangeMode::Manual);
        assert_eq!((y_range.min, y_range.max), (Some(-2.0), Some(4.0)));
    }

    #[test]
    fn manual_y_range_seed_falls_back_to_default_without_auto_range() {
        let mut y_range = RowYRange::default();

        y_range.set_manual_from_last_auto();

        assert_eq!(y_range.mode, YRangeMode::Manual);
        assert_eq!((y_range.min, y_range.max), (Some(-1.0), Some(1.0)));
    }

    #[test]
    fn manual_y_range_reset_uses_latest_auto_range() {
        let mut y_range = RowYRange::default();
        y_range.set_last_auto((-2.0, 4.0));
        y_range.set_manual_from_last_auto();
        y_range.min = Some(-10.0);
        y_range.max = Some(10.0);
        y_range.set_last_auto((-0.5, 0.5));

        y_range.set_manual_from_last_auto();

        assert_eq!(y_range.mode, YRangeMode::Manual);
        assert_eq!((y_range.min, y_range.max), (Some(-0.5), Some(0.5)));
    }

    #[test]
    fn deleting_selected_row_moves_selection_to_an_existing_row() {
        let mut view = ViewState::default();
        view.add_row();
        view.add_row();
        let selected_before_delete = view.selected_row_id.expect("selected row");

        assert!(view.remove_row(selected_before_delete));
        assert_eq!(view.rows.len(), 2);
        assert!(
            view.selected_row_id
                .is_some_and(|row_id| view.rows.iter().any(|row| row.id == row_id))
        );
    }

    #[test]
    fn row_plot_rects_remain_valid_for_many_rows() {
        let content_height = waveform_content_height(160.0, 8);
        assert!(content_height >= MIN_WAVEFORM_ROW_HEIGHT * 8.0);

        let outer =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, content_height));
        let rows = row_outer_rects(outer, 8);

        assert_eq!(rows.len(), 8);
        let plot_rects = rows.iter().copied().map(plot_area_rect).collect::<Vec<_>>();
        for adjacent_plots in plot_rects.windows(2) {
            let plot_gap = adjacent_plots[1].top() - adjacent_plots[0].bottom();
            assert!((plot_gap - 40.0).abs() < f32::EPSILON);
        }
        for row in rows {
            let plot = plot_area_rect(row);
            assert!(plot.width() > 0.0);
            assert!(plot.height() > 0.0);
            assert!(outer.contains_rect(row));
        }
    }

    #[test]
    fn plot_position_resolves_matching_row_id() {
        let outer = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 600.0));
        let row_rects = row_outer_rects(outer, 3);
        let plot_rects = row_rects
            .iter()
            .copied()
            .map(plot_area_rect)
            .collect::<Vec<_>>();
        let mut view = ViewState::default();
        view.add_row();
        view.add_row();

        assert_eq!(
            plot_row_id_at_position(plot_rects[1].center(), &plot_rects, &view.rows),
            Some(view.rows[1].id)
        );

        let label_area_pos = egui::pos2(row_rects[1].left() + 4.0, row_rects[1].center().y);
        assert_eq!(
            plot_row_id_at_position(label_area_pos, &plot_rects, &view.rows),
            None
        );

        let gap_pos = egui::pos2(
            row_rects[0].center().x,
            (row_rects[0].bottom() + row_rects[1].top()) * 0.5,
        );
        assert_eq!(
            plot_row_id_at_position(gap_pos, &plot_rects, &view.rows),
            None
        );
    }

    #[test]
    fn builds_raw_step_trace_with_previous_state_at_range_start() {
        let time = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 1.0, 0.0, 1.0];

        let trace = build_raw_step_trace(&time, &values, (1.5, 2.5), 10).expect("raw step trace");

        assert_eq!(trace.source_sample_count, 1);
        assert_eq!(
            trace.samples,
            vec![
                StepSample {
                    time: 1.5,
                    value: 1.0,
                },
                StepSample {
                    time: 2.0,
                    value: 0.0,
                },
            ]
        );
        assert_eq!(trace.value_range, Some((0.0, 1.0)));
    }

    #[test]
    fn raw_step_trace_respects_sample_limit() {
        let time = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 1.0, 0.0, 1.0];

        let trace = build_raw_step_trace(&time, &values, (0.0, 3.0), 2);

        assert!(trace.is_none());
    }

    #[test]
    fn change_point_step_trace_preserves_edges_when_sample_count_is_high() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let values = [0.0, 0.0, 0.0, 5.0, 5.0, 0.0, 0.0];

        let trace = build_change_point_step_trace(&time, &values, (0.0, 6.0), 3)
            .expect("change-point step trace");

        assert_eq!(trace.kind, StepTraceKind::ChangePoints);
        assert_eq!(trace.source_sample_count, 7);
        assert_eq!(
            trace.samples,
            vec![
                StepSample {
                    time: 0.0,
                    value: 0.0,
                },
                StepSample {
                    time: 3.0,
                    value: 5.0,
                },
                StepSample {
                    time: 5.0,
                    value: 0.0,
                },
            ]
        );
        assert_eq!(trace.value_range, Some((0.0, 5.0)));
    }

    #[test]
    fn change_point_step_trace_carries_previous_state_at_range_start() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0];
        let values = [0.0, 5.0, 5.0, 0.0, 0.0];

        let trace = build_change_point_step_trace(&time, &values, (2.5, 4.0), 3)
            .expect("change-point step trace");

        assert_eq!(
            trace.samples,
            vec![
                StepSample {
                    time: 2.5,
                    value: 5.0,
                },
                StepSample {
                    time: 3.0,
                    value: 0.0,
                },
            ]
        );
        assert_eq!(trace.value_range, Some((0.0, 5.0)));
    }

    #[test]
    fn change_point_step_trace_respects_change_limit() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0];
        let values = [0.0, 1.0, 0.0, 1.0, 0.0];

        let trace = build_change_point_step_trace(&time, &values, (0.0, 4.0), 2);

        assert!(trace.is_none());
    }

    #[test]
    fn change_point_step_trace_reports_dense_fallback() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0];
        let values = [0.0, 1.0, 0.0, 1.0, 0.0];

        let result = build_change_point_step_trace_result(&time, &values, (0.0, 4.0), 2);

        assert_eq!(
            result,
            StepTraceBuildResult::TooManyChangePoints {
                source_sample_count: 5
            }
        );
    }

    #[test]
    fn step_edge_cache_trace_preserves_edges_by_range_lookup() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let values = [0.0, 0.0, 0.0, 5.0, 5.0, 0.0, 0.0];
        let cache = build_step_edge_cache(&time, &values);

        let result = build_change_point_step_trace_from_cache(&time, &cache, (0.0, 6.0), 3);
        let StepTraceBuildResult::Trace(trace) = result else {
            panic!("expected cached edge trace");
        };

        assert_eq!(cache.edge_count(), 3);
        assert_eq!(trace.kind, StepTraceKind::ChangePoints);
        assert_eq!(trace.source_sample_count, 7);
        assert_eq!(
            trace.samples,
            vec![
                StepSample {
                    time: 0.0,
                    value: 0.0,
                },
                StepSample {
                    time: 3.0,
                    value: 5.0,
                },
                StepSample {
                    time: 5.0,
                    value: 0.0,
                },
            ]
        );
        assert_eq!(trace.value_range, Some((0.0, 5.0)));
    }

    #[test]
    fn step_edge_cache_trace_carries_previous_state_at_range_start() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0];
        let values = [0.0, 5.0, 5.0, 0.0, 0.0];
        let cache = build_step_edge_cache(&time, &values);

        let result = build_change_point_step_trace_from_cache(&time, &cache, (2.5, 4.0), 3);
        let StepTraceBuildResult::Trace(trace) = result else {
            panic!("expected cached edge trace");
        };

        assert_eq!(
            trace.samples,
            vec![
                StepSample {
                    time: 2.5,
                    value: 5.0,
                },
                StepSample {
                    time: 3.0,
                    value: 0.0,
                },
            ]
        );
        assert_eq!(trace.value_range, Some((0.0, 5.0)));
    }

    #[test]
    fn step_edge_cache_trace_reports_dense_fallback() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0];
        let values = [0.0, 1.0, 0.0, 1.0, 0.0];
        let cache = build_step_edge_cache(&time, &values);

        let result = build_change_point_step_trace_from_cache(&time, &cache, (0.0, 4.0), 2);

        assert_eq!(
            result,
            StepTraceBuildResult::TooManyChangePoints {
                source_sample_count: 5
            }
        );
    }

    #[test]
    fn bounded_step_build_matches_complete_cache_for_sparse_dense_and_missing_values() {
        let time = (0..96).map(|i| (i / 2) as f64).collect::<Vec<_>>();
        let patterns = [
            vec![f32::NAN; time.len()],
            vec![2.0; time.len()],
            (0..time.len()).map(|i| (i % 2) as f32).collect(),
            (0..time.len())
                .map(|i| match i {
                    0..=5 => f32::NAN,
                    6..=19 => 2.0,
                    20..=29 => f32::INFINITY,
                    30..=49 => -3.0,
                    50..=69 => f32::NEG_INFINITY,
                    _ => (i % 3) as f32,
                })
                .collect(),
        ];
        for values in patterns {
            let complete = build_step_edge_cache(&time, &values);
            for range in [
                (-2.0, 49.0),
                (0.0, 47.0),
                (12.25, 20.5),
                (20.0, 12.0),
                (35.0, 45.0),
                (10.0, 10.0),
                (100.0, 101.0),
            ] {
                for limit in [0, 1, 4, 100] {
                    let expected =
                        build_change_point_step_trace_from_cache(&time, &complete, range, limit);
                    let mut channel = test_channel_data("gate_pwm");
                    channel.values = Arc::new(values.clone());
                    let mut store = ChannelStore::default();
                    store.insert_channel(channel);
                    let actual = store
                        .cached_change_point_step_trace_result("gate_pwm", &time, range, limit)
                        .unwrap();
                    assert_eq!(actual, expected, "range {range:?}, limit {limit}");
                    if matches!(actual, StepTraceBuildResult::TooManyChangePoints { .. }) {
                        assert!(store.step_edge_cache.is_empty());
                        assert_eq!(store.last_envelope_stats.step_edge_aborts, 1);
                    }
                }
            }
        }
    }

    #[test]
    fn bounded_step_build_retains_sparse_view_with_dense_edges_outside_it() {
        let time = (0..80).map(|i| i as f64).collect::<Vec<_>>();
        let mut channel = test_channel_data("gate_pwm");
        channel.values = Arc::new(
            (0..80)
                .map(|i| {
                    if (30..60).contains(&i) {
                        2.0
                    } else {
                        (i % 2) as f32
                    }
                })
                .collect(),
        );
        let mut store = ChannelStore::default();
        store.insert_channel(channel);
        let first = store
            .cached_change_point_step_trace_result("gate_pwm", &time, (40.0, 50.0), 1)
            .unwrap();
        assert!(matches!(first, StepTraceBuildResult::Trace(_)));
        assert_eq!(store.step_edge_cache.len(), 1);
        let second = store
            .cached_change_point_step_trace_result("gate_pwm", &time, (0.0, 79.0), 1)
            .unwrap();
        assert!(matches!(
            second,
            StepTraceBuildResult::TooManyChangePoints { .. }
        ));
        assert_eq!(store.last_envelope_stats.step_edge_hits, 0);
        assert_eq!(store.last_envelope_stats.step_edge_builds, 1);
        assert_eq!(store.last_envelope_stats.step_edge_aborts, 1);
        assert!(store.step_edge_cache.is_empty());
    }

    #[test]
    fn partial_step_cache_reuses_contained_ranges_and_rebuilds_on_pan() {
        let time = (0..200).map(|i| i as f64).collect::<Vec<_>>();
        let values = (0..200)
            .map(|i| {
                if (45..60).contains(&i) {
                    f32::NAN
                } else {
                    (i / 12) as f32
                }
            })
            .collect::<Vec<_>>();
        let complete = build_step_edge_cache(&time, &values);
        let mut channel = test_channel_data("gate_pwm");
        channel.values = Arc::new(values);
        let mut store = ChannelStore::default();
        store.insert_channel(channel);
        for (index, range) in [(50.5, 100.0), (65.0, 90.0), (10.0, 60.0), (90.0, 145.0)]
            .into_iter()
            .enumerate()
        {
            store.begin_envelope_frame();
            let actual = store
                .cached_change_point_step_trace_result("gate_pwm", &time, range, 100)
                .unwrap();
            assert_eq!(
                actual,
                build_change_point_step_trace_from_cache(&time, &complete, range, 100,)
            );
            assert_eq!(
                store.last_envelope_stats.step_edge_hits,
                usize::from(index == 1)
            );
            assert_eq!(
                store.last_envelope_stats.step_edge_builds,
                usize::from(index != 1)
            );
            assert!(store.step_edge_cache["gate_pwm"].edges.len() < complete.edges.len());
        }
    }

    #[test]
    fn channel_store_reuses_step_edge_cache() {
        let time = [0.0, 1.0, 2.0, 3.0, 4.0];
        let mut channel = test_channel_data("gate_pwm");
        channel.values = Arc::new(vec![0.0, 1.0, 0.0, 1.0, 0.0]);
        let mut store = ChannelStore::default();
        store.insert_channel(channel);

        let _ = store.cached_change_point_step_trace_result("gate_pwm", &time, (0.0, 4.0), 10);
        let first_stats = store.last_envelope_stats;
        let _ = store.cached_change_point_step_trace_result("gate_pwm", &time, (1.0, 3.0), 10);
        let second_stats = store.last_envelope_stats;

        assert_eq!(store.step_edge_cache.len(), 1);
        assert_eq!(first_stats.step_edge_builds, 1);
        assert_eq!(first_stats.step_edge_hits, 0);
        assert_eq!(second_stats.step_edge_builds, 1);
        assert_eq!(second_stats.step_edge_hits, 1);
    }

    #[test]
    fn dense_step_envelopes_reuse_tiles_and_preserve_visible_extrema() {
        let sample_count = LINE_TILE_MIN_SOURCE_SAMPLES + 100_000;
        let time = (0..sample_count)
            .map(|index| index as f64)
            .collect::<Vec<_>>();
        let mut values = (0..sample_count)
            .map(|index| if index % 14 < 7 { -2.0 } else { 3.0 })
            .collect::<Vec<_>>();
        values[50_000] = 100.0; // Outside both later viewports, inside their overscan cache.
        values[300_003] = -8.0; // A single-sample spike must survive the tile reduction.
        values[300_004] = f32::NAN;
        values[300_005] = f32::INFINITY;
        let mut app = PanelYApp::default();
        install_test_source_waveform(
            &mut app.workspace,
            DEFAULT_SOURCE_ID,
            &time,
            "gate_pwm",
            values,
        );
        app.view.rows[0].channels = vec![RowChannel {
            draw_mode: DrawMode::Step,
            ..RowChannel::new(test_key("gate_pwm"), 0)
        }];

        for (iteration, view_range) in [
            (0.0, (sample_count - 1) as f64),
            (100_000.0, 550_000.0),
            (110_000.0, 560_000.0),
        ]
        .into_iter()
        .enumerate()
        {
            app.view.x_range = Some(view_range);
            let rows = app.visible_row_traces(128, true, false);
            let VisibleTraceData::DenseStepEnvelope(envelope) = &rows[0].traces[0].data else {
                panic!("dense Step data must use an envelope");
            };
            let store = &app.workspace.loaded_channels;
            let raw = &store.channel("gate_pwm").unwrap().values;
            for bucket in &envelope.buckets {
                let finite = raw[bucket.start_index..bucket.end_index]
                    .iter()
                    .copied()
                    .filter(|value| value.is_finite());
                let expected = finite.fold((f32::INFINITY, f32::NEG_INFINITY), |range, value| {
                    (range.0.min(value), range.1.max(value))
                });
                assert_eq!((bucket.min, bucket.max), expected);
                assert!(time[bucket.start_index] >= view_range.0);
                assert!(time[bucket.end_index - 1] <= view_range.1);
            }
            assert_eq!(store.line_tile_cache.len(), 1);
            match iteration {
                0 => {
                    assert_eq!(store.last_envelope_stats.tile_builds, 1);
                    assert_eq!(store.last_envelope_stats.step_edge_aborts, 1);
                    assert!(store.step_edge_cache.is_empty());
                    assert_eq!(envelope.value_range, Some((-8.0, 100.0)));
                }
                1 => {
                    assert_eq!(store.last_envelope_stats.tile_builds, 0);
                    assert_eq!(store.last_envelope_stats.tile_hits, 1);
                    assert_eq!(envelope.value_range, Some((-8.0, 3.0)));
                }
                _ => {
                    assert_eq!(store.last_envelope_stats.context_hits, 1);
                    assert_eq!(store.last_envelope_stats.hits, 1);
                    assert_eq!(envelope.value_range, Some((-8.0, 3.0)));
                }
            }
        }

        app.view.x_range = Some((200_000.0, 200_020.0));
        let rows = app.visible_row_traces(128, true, false);
        let VisibleTraceData::RawStep(trace) = &rows[0].traces[0].data else {
            panic!("zooming into a small interval must restore exact Step data");
        };
        assert_eq!(trace.kind, StepTraceKind::RawSamples);
        assert!(
            trace
                .samples
                .windows(2)
                .any(|pair| pair[0].value != pair[1].value)
        );
    }

    #[test]
    fn channel_store_unloads_one_channel_and_its_draw_caches() {
        let mut store = ChannelStore::default();
        store.insert_channel(test_channel_data("gate_pwm"));
        store.insert_channel(test_channel_data("sine_50Hz"));
        store
            .line_tile_cache
            .insert("gate_pwm".to_owned(), empty_line_tile_cache());
        store.step_edge_cache.insert(
            "gate_pwm".to_owned(),
            StepEdgeCache {
                cached_time_range: None,
                sample_count: 3,
                edges: Vec::new(),
                elapsed: Duration::ZERO,
            },
        );
        store.step_fallback_hints.insert(
            "gate_pwm".to_owned(),
            StepFallbackHint {
                min_span: 1.0,
                min_source_sample_count: 3,
            },
        );
        store.envelope_cache.insert(
            EnvelopeKey::new("gate_pwm", (0.0, 1.0), 10),
            empty_min_max_envelope(),
        );

        assert!(store.unload_channel("gate_pwm"));

        assert!(!store.has_channel("gate_pwm"));
        assert!(store.has_channel("sine_50Hz"));
        assert!(store.line_tile_cache.is_empty());
        assert!(store.step_edge_cache.is_empty());
        assert!(store.step_fallback_hints.is_empty());
        assert!(store.envelope_cache.is_empty());
    }

    #[test]
    fn channel_store_clear_draw_caches_keeps_raw_channel_data() {
        let mut store = ChannelStore::default();
        store.insert_channel(test_channel_data("gate_pwm"));
        store
            .line_tile_cache
            .insert("gate_pwm".to_owned(), empty_line_tile_cache());
        store.step_edge_cache.insert(
            "gate_pwm".to_owned(),
            StepEdgeCache {
                cached_time_range: None,
                sample_count: 3,
                edges: Vec::new(),
                elapsed: Duration::ZERO,
            },
        );
        store.step_fallback_hints.insert(
            "gate_pwm".to_owned(),
            StepFallbackHint {
                min_span: 1.0,
                min_source_sample_count: 3,
            },
        );
        store.envelope_cache.insert(
            EnvelopeKey::new("gate_pwm", (0.0, 1.0), 10),
            empty_min_max_envelope(),
        );

        assert!(store.clear_draw_caches());

        assert!(store.has_channel("gate_pwm"));
        assert!(store.line_tile_cache.is_empty());
        assert!(store.step_edge_cache.is_empty());
        assert!(store.step_fallback_hints.is_empty());
        assert!(store.envelope_cache.is_empty());
        assert!(!store.clear_draw_caches());
    }

    #[test]
    fn unloading_cached_channel_keeps_row_reference() {
        let mut app = PanelYApp::default();
        app.workspace
            .loaded_channels
            .insert_channel(test_channel_data("gate_pwm"));
        let key = test_key("gate_pwm");
        let (added, _) = app.view.add_channel_to_selected_row(&key);
        assert!(added);

        app.unload_cached_channel(&key);

        assert!(!app.workspace.loaded_channels.has_channel("gate_pwm"));
        assert!(
            app.view.rows[0]
                .channels
                .iter()
                .any(|channel| channel.key.channel_path == "gate_pwm")
        );
        assert!(app.load.status.contains("row references remain"));
    }

    #[test]
    fn plot_config_apply_to_current_data_preserves_dataset_and_raw_cache() {
        let config_path = std::env::temp_dir().join(format!(
            "panel_y_apply_current_{}.pyc.json",
            std::process::id()
        ));
        let config = serde_json::json!({
            "version": 1,
            "data_path": "/tmp/config_data.parquet",
            "rows": [
                {
                    "channels": ["gate_pwm"],
                    "ymin": -2.0,
                    "ymax": 2.0,
                    "step_channels": ["gate_pwm"]
                }
            ],
            "theme": "light",
            "scroll_zoom": true,
            "rust": {
                "version": 1,
                "rows": [
                    {
                        "channels": [
                            {
                                "path": "gate_pwm",
                                "visible": false,
                                "draw_mode": "step",
                                "line_width": 2.5,
                                "color": "#123456",
                                "color_index": 4
                            }
                        ]
                    }
                ]
            }
        });
        std::fs::write(&config_path, serde_json::to_string(&config).expect("json"))
            .expect("write plot config");

        let mut app = PanelYApp::default();
        app.workspace.parquet_path = "/tmp/current.parquet".to_owned();
        app.workspace.schema = Some(test_schema(&["sine_50Hz", "gate_pwm"]));
        app.workspace.shared_time = Some(test_time_data());
        app.workspace
            .loaded_channels
            .insert_channel(test_channel_data("sine_50Hz"));
        app.workspace
            .loaded_channels
            .line_tile_cache
            .insert("sine_50Hz".to_owned(), empty_line_tile_cache());
        app.analysis.set_cursor_mode(true);
        app.analysis.place_cursor(0.2);
        app.analysis.place_cursor(0.4);
        app.view.x_range = Some((0.1, 0.9));
        app.view.hover_x = Some(0.2);
        app.view.set_display_mode_choice(DisplayModeChoice::Rock);
        app.plot_config_path = config_path.display().to_string();

        let ctx = egui::Context::default();
        app.apply_plot_config_to_current_data(&ctx);
        let _ = std::fs::remove_file(config_path);

        assert_eq!(app.workspace.parquet_path, "/tmp/current.parquet");
        assert!(app.workspace.schema.is_some());
        assert!(app.workspace.shared_time.is_some());
        assert!(app.workspace.loaded_channels.has_channel("sine_50Hz"));
        assert!(app.workspace.loaded_channels.line_tile_cache.is_empty());
        assert_eq!(app.view.x_range, Some((0.1, 0.9)));
        assert_eq!(app.view.hover_x, None);
        assert_eq!(app.analysis.cursor_a, None);
        assert_eq!(app.analysis.cursor_b, None);
        assert!(!app.analysis.cursor_mode);
        assert_eq!(app.view.theme, ThemeMode::Light);
        assert_eq!(app.view.visual_mode, VisualMode::Plain);
        assert_eq!(
            app.view.display_mode_choice(),
            DisplayModeChoice::PlainLight
        );
        assert!(app.view.scroll_zoom);
        assert_eq!(app.view.rows.len(), 1);
        assert_eq!(app.view.rows[0].y_range.mode, YRangeMode::Manual);
        assert_eq!(
            (app.view.rows[0].y_range.min, app.view.rows[0].y_range.max),
            (Some(-2.0), Some(2.0))
        );
        assert_eq!(app.view.rows[0].channels.len(), 1);
        let channel = &app.view.rows[0].channels[0];
        assert_eq!(channel.key.channel_path, "gate_pwm");
        assert!(!channel.visible);
        assert_eq!(channel.draw_mode, DrawMode::Step);
        assert_eq!(channel.line_width, 2.5);
        assert_eq!(app.load.pending_jobs, 0);
        assert!(app.plot_config_report.as_ref().is_some_and(|report| {
            report
                .ignored_fields
                .iter()
                .any(|field| field.contains("data_path=/tmp/config_data.parquet"))
        }));
        assert!(
            app.load
                .status
                .starts_with("Applied plot config to current data")
        );
    }

    #[test]
    fn plot_config_apply_restores_rust_visual_mode() {
        let config_path = std::env::temp_dir().join(format!(
            "panel_y_apply_visual_mode_{}.pyc.json",
            std::process::id()
        ));
        let config = serde_json::json!({
            "version": 1,
            "data_path": "/tmp/current.parquet",
            "rows": [
                {
                    "channels": ["gate_pwm"],
                    "ymin": null,
                    "ymax": null
                }
            ],
            "theme": "dark",
            "scroll_zoom": false,
            "rust": {
                "version": 1,
                "visual_mode": "rock",
                "rows": [
                    {
                        "channels": [
                            {
                                "path": "gate_pwm",
                                "visible": false
                            }
                        ]
                    }
                ]
            }
        });
        std::fs::write(&config_path, serde_json::to_string(&config).expect("json"))
            .expect("write plot config");

        let mut app = PanelYApp::default();
        app.workspace.parquet_path = "/tmp/current.parquet".to_owned();
        app.workspace.schema = Some(test_schema(&["gate_pwm"]));
        app.workspace.shared_time = Some(test_time_data());
        app.view
            .set_display_mode_choice(DisplayModeChoice::PlainLight);
        app.plot_config_path = config_path.display().to_string();

        let ctx = egui::Context::default();
        app.apply_plot_config_to_current_data(&ctx);
        let _ = std::fs::remove_file(config_path);

        assert_eq!(app.view.theme, ThemeMode::Dark);
        assert_eq!(app.view.visual_mode, VisualMode::Rock);
        assert_eq!(app.view.display_mode_choice(), DisplayModeChoice::Rock);
        assert!(!app.view.scroll_zoom);
        assert_eq!(app.view.rows.len(), 1);
        assert_eq!(app.view.rows[0].channels.len(), 1);
        assert!(!app.view.rows[0].channels[0].visible);
    }

    #[test]
    fn step_fallback_hint_applies_to_same_or_wider_ranges() {
        let mut store = ChannelStore::default();

        store.record_step_change_point_fallback("gate_pwm", (10.0, 20.0), 100_000);

        assert!(store.should_skip_step_change_points("gate_pwm", (10.0, 20.0), 95_000));
        assert!(store.should_skip_step_change_points("gate_pwm", (10.0, 30.0), 150_000));
        assert!(!store.should_skip_step_change_points("gate_pwm", (12.0, 18.0), 60_000));
        assert!(!store.should_skip_step_change_points("other", (10.0, 30.0), 150_000));
    }

    #[test]
    fn preview_bucket_count_is_capped_and_reduced() {
        assert_eq!(
            preview_envelope_bucket_count(100),
            MIN_VISIBLE_ENVELOPE_BUCKETS
        );
        assert_eq!(preview_envelope_bucket_count(1_200), 300);
        assert_eq!(
            preview_envelope_bucket_count(8_192),
            MAX_PREVIEW_ENVELOPE_BUCKETS
        );
    }

    #[test]
    fn preview_workload_gate_excludes_medium_class_data() {
        assert!(!preview_needed_for_workload(2_000_000, 1_000_000, 8, 3));
    }

    #[test]
    fn preview_workload_gate_excludes_small_large_data_windows() {
        assert!(!preview_needed_for_workload(10_000_000, 100_000, 8, 1));
    }

    #[test]
    fn preview_workload_gate_allows_large_wide_ranges() {
        assert!(preview_needed_for_workload(10_000_000, 5_000_000, 8, 1));
    }

    #[test]
    fn preview_is_disabled_by_default() {
        assert!(!ViewState::default().large_preview_enabled);
    }

    #[test]
    fn channel_filter_matches_name_or_path_case_insensitively() {
        let channel = parquet_schema::ColumnInfo {
            index: 3,
            name: "GatePWM".to_owned(),
            path: "motor/gate_pwm".to_owned(),
            physical_type: "FLOAT".to_owned(),
            logical_type: None,
            converted_type: "NONE".to_owned(),
            is_numeric: true,
            role: parquet_schema::ColumnRole::Channel,
        };

        assert!(channel_matches_filter(&channel, ""));
        assert!(channel_matches_filter(&channel, "gate"));
        assert!(channel_matches_filter(&channel, "MOTOR/"));
        assert!(!channel_matches_filter(&channel, "current"));
    }

    #[test]
    fn browser_filter_matches_presentation_source_number_and_file_name() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/Run_A.parquet".to_owned());
        let channel = test_channel_info_with_path(3, "GatePWM", "motor/gate_pwm");
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .schema = Some(parquet_schema::SchemaSummary {
            path: PathBuf::from("/tmp/Run_A.parquet"),
            row_count: 10,
            row_group_count: 1,
            column_count: 2,
            created_by: None,
            time_column: None,
            channels: vec![channel.clone()],
            columns: Vec::new(),
        });
        let presentation = channel_presentation(
            &workspace,
            &ChannelKey::new(DEFAULT_SOURCE_ID, "motor/gate_pwm"),
        )
        .expect("presentation");

        for filter in ["gatepwm", "MOTOR/", "gatepwm #1", "#1", "run_a.parquet"] {
            assert!(
                channel_matches_browser_filter(&channel, &presentation, filter),
                "filter={filter}"
            );
        }
        assert!(!channel_matches_browser_filter(
            &channel,
            &presentation,
            "source_2"
        ));
    }

    #[test]
    fn matching_prefers_exact_path_then_unique_channel_name() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/one.parquet".to_owned());
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .schema = Some(parquet_schema::SchemaSummary {
            path: PathBuf::from("/tmp/one.parquet"),
            row_count: 10,
            row_group_count: 1,
            column_count: 2,
            created_by: None,
            time_column: None,
            channels: vec![test_channel_info_with_path(1, "iu", "motor/iu")],
            columns: Vec::new(),
        });
        let source_two = workspace.add_source("/tmp/two.parquet".to_owned());
        workspace.source_mut(source_two).expect("Source #2").schema =
            Some(parquet_schema::SchemaSummary {
                path: PathBuf::from("/tmp/two.parquet"),
                row_count: 10,
                row_group_count: 1,
                column_count: 3,
                created_by: None,
                time_column: None,
                channels: vec![
                    test_channel_info_with_path(1, "different", "motor/iu"),
                    test_channel_info_with_path(2, "iu", "other/iu"),
                ],
                columns: Vec::new(),
            });

        let outcomes =
            matching_channel_outcomes(&workspace, &ChannelKey::new(DEFAULT_SOURCE_ID, "motor/iu"));
        assert_eq!(
            outcomes,
            vec![
                MatchingChannelOutcome::Match(ChannelKey::new(DEFAULT_SOURCE_ID, "motor/iu")),
                MatchingChannelOutcome::Match(ChannelKey::new(source_two, "motor/iu")),
            ]
        );

        workspace
            .source_mut(source_two)
            .expect("Source #2")
            .schema
            .as_mut()
            .expect("schema")
            .channels
            .remove(0);
        let outcomes =
            matching_channel_outcomes(&workspace, &ChannelKey::new(DEFAULT_SOURCE_ID, "motor/iu"));
        assert_eq!(
            outcomes[1],
            MatchingChannelOutcome::Match(ChannelKey::new(source_two, "other/iu"))
        );
    }

    #[test]
    fn matching_skips_missing_and_rejects_ambiguous_names() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/one.parquet".to_owned());
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .schema = Some(parquet_schema::SchemaSummary {
            path: PathBuf::from("/tmp/one.parquet"),
            row_count: 10,
            row_group_count: 1,
            column_count: 2,
            created_by: None,
            time_column: None,
            channels: vec![test_channel_info_with_path(1, "iu", "motor/iu")],
            columns: Vec::new(),
        });
        let source_two = workspace.add_source("/tmp/two.parquet".to_owned());
        let missing_outcomes =
            matching_channel_outcomes(&workspace, &ChannelKey::new(DEFAULT_SOURCE_ID, "motor/iu"));
        assert_eq!(
            missing_outcomes[1],
            MatchingChannelOutcome::MissingSchema(source_two)
        );

        workspace.source_mut(source_two).expect("Source #2").schema =
            Some(parquet_schema::SchemaSummary {
                path: PathBuf::from("/tmp/two.parquet"),
                row_count: 10,
                row_group_count: 1,
                column_count: 3,
                created_by: None,
                time_column: None,
                channels: vec![
                    test_channel_info_with_path(1, "iu", "left/iu"),
                    test_channel_info_with_path(2, "iu", "right/iu"),
                ],
                columns: Vec::new(),
            });
        let ambiguous =
            matching_channel_outcomes(&workspace, &ChannelKey::new(DEFAULT_SOURCE_ID, "motor/iu"));
        assert_eq!(ambiguous[1], MatchingChannelOutcome::Ambiguous(source_two));
    }

    #[test]
    fn channel_load_state_prioritizes_loaded_over_loading() {
        let mut store = ChannelStore::default();
        store.insert_channel(parquet_waveform::ChannelData {
            path: PathBuf::new(),
            channel_name: "iu".to_owned(),
            channel_path: "iu".to_owned(),
            values: Arc::new(vec![1.0, 2.0]),
            projected_column_index: 1,
            elapsed: Duration::ZERO,
        });
        let mut loading = BTreeSet::new();
        loading.insert(test_key("iu"));
        loading.insert(test_key("iv"));

        assert_eq!(
            channel_load_state(DEFAULT_SOURCE_ID, "iu", &store, &loading),
            ChannelLoadState::Loaded
        );
        assert_eq!(
            channel_load_state(DEFAULT_SOURCE_ID, "iv", &store, &loading),
            ChannelLoadState::Loading
        );
        assert_eq!(
            channel_load_state(DEFAULT_SOURCE_ID, "iw", &store, &loading),
            ChannelLoadState::Unloaded
        );
    }

    #[test]
    fn channel_state_filter_matches_expected_load_states() {
        assert!(ChannelStateFilter::All.matches(ChannelLoadState::Loaded));
        assert!(ChannelStateFilter::All.matches(ChannelLoadState::Loading));
        assert!(ChannelStateFilter::Loaded.matches(ChannelLoadState::Loaded));
        assert!(!ChannelStateFilter::Loaded.matches(ChannelLoadState::Unloaded));
        assert!(ChannelStateFilter::Unloaded.matches(ChannelLoadState::Unloaded));
        assert!(!ChannelStateFilter::Unloaded.matches(ChannelLoadState::Loading));
    }

    #[test]
    fn channel_list_entries_filter_and_sort_channels() {
        let channels = vec![
            test_channel_info(0, "zeta"),
            test_channel_info(1, "alpha"),
            test_channel_info(2, "beta"),
        ];
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/source.parquet".to_owned());
        workspace
            .source_mut(DEFAULT_SOURCE_ID)
            .expect("Source #1")
            .schema = Some(test_schema(&["zeta", "alpha", "beta"]));
        let mut store = ChannelStore::default();
        store.insert_channel(parquet_waveform::ChannelData {
            path: PathBuf::new(),
            channel_name: "beta".to_owned(),
            channel_path: "beta".to_owned(),
            values: Arc::new(vec![1.0, 2.0]),
            projected_column_index: 2,
            elapsed: Duration::ZERO,
        });
        let mut loading = BTreeSet::new();
        loading.insert(test_key("zeta"));

        let by_name = channel_list_entries(
            &workspace,
            DEFAULT_SOURCE_ID,
            &channels,
            "",
            ChannelStateFilter::All,
            ChannelSort::Name,
            &store,
            &loading,
        );
        assert_eq!(
            by_name
                .iter()
                .map(|(channel, _)| channel.path.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta", "zeta"]
        );

        let loaded_first = channel_list_entries(
            &workspace,
            DEFAULT_SOURCE_ID,
            &channels,
            "",
            ChannelStateFilter::All,
            ChannelSort::LoadedFirst,
            &store,
            &loading,
        );
        assert_eq!(
            loaded_first
                .iter()
                .map(|(channel, state)| (channel.path.as_str(), *state))
                .collect::<Vec<_>>(),
            vec![
                ("beta", ChannelLoadState::Loaded),
                ("zeta", ChannelLoadState::Loading),
                ("alpha", ChannelLoadState::Unloaded),
            ]
        );

        let unloaded = channel_list_entries(
            &workspace,
            DEFAULT_SOURCE_ID,
            &channels,
            "a",
            ChannelStateFilter::Unloaded,
            ChannelSort::SchemaOrder,
            &store,
            &loading,
        );
        assert_eq!(
            unloaded
                .iter()
                .map(|(channel, _)| channel.path.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha"]
        );
    }

    #[test]
    fn channel_row_membership_reports_rows_that_contain_channel() {
        let mut view = ViewState::default();
        view.add_channel_to_selected_row(&test_key("iu"));
        view.add_row();
        view.add_channel_to_selected_row(&test_key("iv"));
        view.add_row();
        view.add_channel_to_selected_row(&test_key("iu"));

        assert_eq!(
            channel_row_membership(&view, &test_key("iu")),
            vec!["Row 1", "Row 3"]
        );
        assert_eq!(
            channel_row_membership(&view, &test_key("iw")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn estimated_sample_rate_uses_sample_intervals() {
        assert_eq!(
            estimated_sample_rate_hz(1001, Some((0.0, 1.0))),
            Some(1000.0)
        );
        assert_eq!(estimated_sample_rate_hz(1, Some((0.0, 1.0))), None);
        assert_eq!(estimated_sample_rate_hz(100, Some((1.0, 1.0))), None);
    }

    #[test]
    fn frequency_formatter_scales_units() {
        assert_eq!(format_frequency_hz(500.0), "500.000 Hz");
        assert_eq!(format_frequency_hz(12_500.0), "12.500 kHz");
        assert_eq!(format_frequency_hz(2_500_000.0), "2.500 MHz");
    }

    #[test]
    fn x_zoom_factor_ignores_plain_vertical_wheel() {
        assert_eq!(
            x_zoom_factor(1.0, 120.0, false, MAX_WHEEL_ZOOM_SENSITIVITY),
            1.0
        );
    }

    #[test]
    fn x_zoom_factor_uses_modifier_wheel_or_pinch() {
        let modifier_zoom = x_zoom_factor(1.0, 120.0, true, MAX_WHEEL_ZOOM_SENSITIVITY);
        let pinch_zoom = x_zoom_factor(2.0, 0.0, false, MIN_WHEEL_ZOOM_SENSITIVITY);

        assert!(modifier_zoom < 1.0);
        assert_eq!(pinch_zoom, 0.5);
    }

    #[test]
    fn wheel_zoom_sensitivity_scales_with_sample_count() {
        assert_eq!(
            wheel_zoom_sensitivity(MIN_WHEEL_ZOOM_SAMPLE_COUNT),
            MIN_WHEEL_ZOOM_SENSITIVITY
        );
        assert_eq!(
            wheel_zoom_sensitivity(MAX_WHEEL_ZOOM_SAMPLE_COUNT),
            MAX_WHEEL_ZOOM_SENSITIVITY
        );

        let medium_sensitivity = wheel_zoom_sensitivity(100_000);
        assert!(medium_sensitivity > MIN_WHEEL_ZOOM_SENSITIVITY);
        assert!(medium_sensitivity < MAX_WHEEL_ZOOM_SENSITIVITY);
    }

    #[test]
    fn parquet_path_validation_accepts_existing_parquet_file() {
        let path =
            std::env::temp_dir().join(format!("panel_y_validation_{}.PARQUET", std::process::id()));
        std::fs::write(&path, b"placeholder").expect("write temp parquet path");

        let result = validate_parquet_path(&path.display().to_string());

        let _ = std::fs::remove_file(&path);
        assert_eq!(result, Ok(path));
    }

    #[test]
    fn parquet_path_validation_rejects_missing_file() {
        let path = std::env::temp_dir().join(format!(
            "panel_y_validation_missing_{}.parquet",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let error = validate_parquet_path(&path.display().to_string()).expect_err("missing path");

        assert!(matches!(error, ParquetPathValidationError::Missing { .. }));
    }

    #[test]
    fn parquet_path_validation_rejects_directory() {
        let error =
            validate_parquet_path(&std::env::temp_dir().display().to_string()).expect_err("dir");

        assert!(matches!(error, ParquetPathValidationError::NotFile { .. }));
    }

    #[test]
    fn parquet_path_validation_rejects_wrong_extension() {
        let path =
            std::env::temp_dir().join(format!("panel_y_validation_{}.csv", std::process::id()));
        std::fs::write(&path, b"placeholder").expect("write temp csv path");

        let error = validate_parquet_path(&path.display().to_string()).expect_err("wrong ext");

        let _ = std::fs::remove_file(&path);
        assert!(matches!(
            error,
            ParquetPathValidationError::WrongExtension { .. }
        ));
    }

    #[test]
    fn recent_parquet_paths_are_deduped_and_limited() {
        let mut recent_paths = Vec::new();
        for index in 0..(MAX_RECENT_PARQUET_PATHS + 2) {
            remember_recent_parquet_path(&mut recent_paths, format!("/tmp/{index}.parquet"));
        }

        assert_eq!(recent_paths.len(), MAX_RECENT_PARQUET_PATHS);
        assert_eq!(recent_paths[0], "/tmp/9.parquet");
        assert!(!recent_paths.iter().any(|path| path == "/tmp/0.parquet"));

        remember_recent_parquet_path(&mut recent_paths, "/tmp/5.parquet");

        assert_eq!(recent_paths.len(), MAX_RECENT_PARQUET_PATHS);
        assert_eq!(recent_paths[0], "/tmp/5.parquet");
        assert_eq!(
            recent_paths
                .iter()
                .filter(|path| path.as_str() == "/tmp/5.parquet")
                .count(),
            1
        );
    }

    #[test]
    fn overscan_envelope_cache_reuses_same_span_pan() {
        let time = (0..100).map(|value| value as f64).collect::<Vec<_>>();
        let values = (0..100).map(|value| value as f32).collect::<Vec<_>>();
        let mut store = ChannelStore::default();
        store.insert_channel(parquet_waveform::ChannelData {
            path: std::path::PathBuf::new(),
            channel_name: "ch".to_owned(),
            channel_path: "ch".to_owned(),
            values: Arc::new(values),
            projected_column_index: 1,
            elapsed: Duration::ZERO,
        });

        store.begin_envelope_frame();
        store.prepare_envelope_context(&time, time.len(), (10.0, 30.0), (0.0, 99.0), 10);
        let first = store
            .ensure_envelope("ch", &time, (10.0, 30.0), 10, true)
            .expect("first envelope");
        assert!(first.1);
        assert_eq!(first.0.time_range, Some((10.0, 30.0)));
        assert_eq!(store.last_envelope_stats.context_misses, 1);
        assert_eq!(store.last_envelope_stats.misses, 1);

        store.begin_envelope_frame();
        store.prepare_envelope_context(&time, time.len(), (15.0, 35.0), (0.0, 99.0), 10);
        let second = store
            .ensure_envelope("ch", &time, (15.0, 35.0), 10, true)
            .expect("second envelope");
        assert!(!second.1);
        assert_eq!(second.0.time_range, Some((15.0, 35.0)));
        assert_eq!(store.last_envelope_stats.context_hits, 1);
        assert_eq!(store.last_envelope_stats.hits, 1);
    }

    #[test]
    fn valid_view_range_clamping_preserves_endpoints() {
        for range in [(1.91, 7.91), (2.02, 8.02), (0.0, 10.0)] {
            let clamped = clamp_view_range(range, (0.0, 10.0), 0.001);
            assert_eq!(clamped, range);
            assert_eq!(clamp_view_range(clamped, (0.0, 10.0), 0.001), range);
        }
    }

    #[test]
    fn aligned_source_envelopes_survive_pan_and_small_zoom_roundoff() {
        let mut app = PanelYApp::default();
        app.view.rows[0].channels.clear();
        for (index, (sample_count, sample_rate, raw_start)) in [
            (1_000, 100.0, 0.0),
            (800, 80.0, 100.0),
            (1_200, 120.0, -20.0),
            (600, 60.0, 1_000.0),
        ]
        .into_iter()
        .enumerate()
        {
            let source_id = if index == 0 {
                DEFAULT_SOURCE_ID
            } else {
                app.workspace.add_source(format!("source-{index}.parquet"))
            };
            let time = (0..sample_count)
                .map(|sample| raw_start + sample as f64 / sample_rate)
                .collect::<Vec<_>>();
            install_test_source_waveform(
                &mut app.workspace,
                source_id,
                &time,
                "signal",
                (0..sample_count)
                    .map(|sample| (sample % 7) as f32)
                    .collect(),
            );
            app.workspace
                .source_mut(source_id)
                .unwrap()
                .time_transform
                .offset_seconds = -raw_start;
            app.view.rows[0]
                .channels
                .push(RowChannel::new(ChannelKey::new(source_id, "signal"), index));
        }
        app.view.x_range = Some((1.0, 7.0));
        let _ = app.visible_row_traces(128, true, false);
        assert_eq!(app.workspace.envelope_stats().misses, 4);

        let full_range = app.workspace.display_time_range().unwrap();
        let min_span = app.workspace.display_min_view_span().unwrap();
        let pan = pan_view_range((1.0, 7.0), full_range, 1.02, min_span);
        assert_eq!(pan, (2.02, 8.02));
        let zoom = zoom_view_range(pan, full_range, 0.5, 0.99, min_span);
        for range in [pan, pan, zoom, zoom] {
            app.view.x_range = Some(range);
            let rows = app.visible_row_traces(128, true, false);
            assert_eq!(rows[0].traces.len(), 4);
            let stats = app.workspace.envelope_stats();
            assert_eq!(stats.context_hits, 4);
            assert_eq!(stats.hits, 4);
            assert_eq!(stats.misses, 0);
            assert_eq!(app.view.x_range, Some(range));
        }
    }

    #[test]
    fn overscan_envelope_context_rejects_zoom_span_change() {
        let context = EnvelopeContext::for_view((10.0, 30.0), (0.0, 100.0), 100).expect("context");

        assert!(context.reuses_for_view((15.0, 35.0), 100));
        assert!(!context.reuses_for_view((15.0, 25.0), 100));
    }

    #[test]
    fn line_tile_batch_candidates_respect_source_visibility_mode_and_lod() {
        let mut app = PanelYApp::default();
        let time = (0..LINE_TILE_MIN_SOURCE_SAMPLES + 1)
            .map(|i| i as f64)
            .collect::<Vec<_>>();
        let full = (0.0, *time.last().unwrap());
        let second = app.workspace.add_source("second.parquet".into());
        let outside = app.workspace.add_source("outside.parquet".into());
        let mut contexts = BTreeMap::new();
        for id in [DEFAULT_SOURCE_ID, second, outside] {
            install_test_source_waveform(
                &mut app.workspace,
                id,
                &time,
                "signal",
                vec![1.0; time.len()],
            );
            let store = &mut app.workspace.source_mut(id).unwrap().loaded_channels;
            store.prepare_envelope_context(&time, time.len(), full, full, 128);
            if id != outside {
                contexts.insert(
                    id,
                    SourceRenderContext {
                        time_transform: TimeTransform::default(),
                        raw_view_range: full,
                        raw_full_range: full,
                    },
                );
            }
        }
        let mut hidden = test_channel_data("hidden");
        hidden.values = app
            .workspace
            .channel(&test_key("signal"))
            .unwrap()
            .values
            .clone();
        app.workspace.loaded_channels.insert_channel(hidden);
        app.workspace
            .loaded_channels
            .insert_channel(test_channel_data("step"));
        app.workspace
            .loaded_channels
            .prepare_envelope_context(&time, time.len(), full, full, 128);
        app.view.rows[0].channels = vec![
            RowChannel::new(test_key("signal"), 0),
            RowChannel::new(ChannelKey::new(second, "signal"), 1),
            RowChannel::new(ChannelKey::new(outside, "signal"), 2),
            RowChannel {
                visible: false,
                ..RowChannel::new(test_key("hidden"), 3)
            },
            RowChannel {
                draw_mode: DrawMode::Step,
                ..RowChannel::new(test_key("step"), 4)
            },
            RowChannel::new(test_key("missing"), 5),
        ];
        let mut duplicate = PlotRow::new(1);
        duplicate
            .channels
            .push(RowChannel::new(test_key("signal"), 0));
        app.view.rows.push(duplicate);
        assert_eq!(
            app.workspace
                .pending_line_tile_keys(&app.view.rows, &contexts),
            vec![test_key("signal"), ChannelKey::new(second, "signal")]
        );
        app.workspace
            .source_mut(second)
            .unwrap()
            .loaded_channels
            .envelope_plan
            .as_mut()
            .unwrap()
            .bucket_size = 1;
        assert_eq!(
            app.workspace
                .pending_line_tile_keys(&app.view.rows, &contexts),
            vec![test_key("signal")]
        );
        app.workspace
            .loaded_channels
            .line_tile_cache
            .insert("signal".into(), empty_line_tile_cache());
        assert!(
            app.workspace
                .pending_line_tile_keys(&app.view.rows, &contexts)
                .is_empty()
        );
    }

    #[test]
    fn large_line_batch_preserves_extrema_reuses_and_rebuilds_without_retaining_raw() {
        let n = LINE_TILE_BATCH_MIN_SAMPLES / 2;
        let time = (0..n).map(|i| i as f64 / n as f64).collect::<Vec<_>>();
        let mut app = PanelYApp::default();
        let second = app.workspace.add_source("second.parquet".into());
        let ids = [DEFAULT_SOURCE_ID, second];
        let mut raw_refs = Vec::new();
        for (index, id) in ids.into_iter().enumerate() {
            let mut values = vec![index as f32; n];
            values[100] = -10.0 - index as f32;
            values[n / 2] = 20.0 + index as f32;
            values[300] = f32::NAN;
            values[301] = f32::INFINITY;
            install_test_source_waveform(&mut app.workspace, id, &time, "signal", values);
            raw_refs.push(Arc::downgrade(
                &app.workspace
                    .channel(&ChannelKey::new(id, "signal"))
                    .unwrap()
                    .values,
            ));
            app.view.rows[0]
                .channels
                .push(RowChannel::new(ChannelKey::new(id, "signal"), index));
        }
        let mut duplicate = PlotRow::new(1);
        duplicate
            .channels
            .push(RowChannel::new(test_key("signal"), 0));
        app.view.rows.push(duplicate);
        app.view.x_range = Some((time[0], *time.last().unwrap()));
        let rows = app.visible_row_traces(128, true, false);
        assert_eq!(app.workspace.envelope_stats().tile_builds, 2);
        for (index, trace) in rows[0].traces.iter().enumerate() {
            let VisibleTraceData::Envelope(envelope) = &trace.data else {
                panic!("Line envelope")
            };
            assert_eq!(
                envelope.value_range,
                Some((-10.0 - index as f64, 20.0 + index as f64))
            );
        }
        assert!(raw_refs.iter().all(|raw| raw.strong_count() == 1));
        let _ = app.visible_row_traces(128, true, false);
        assert_eq!(app.workspace.envelope_stats().tile_builds, 0);
        assert_eq!(app.workspace.envelope_stats().tile_build_workers, 0);
        app.view.x_range = Some((0.1, 0.9));
        let _ = app.visible_row_traces(128, true, false);
        assert_eq!(app.workspace.envelope_stats().tile_builds, 0);
        for id in ids {
            app.workspace
                .source_mut(id)
                .unwrap()
                .loaded_channels
                .clear_draw_caches();
        }
        let _ = app.visible_row_traces(128, true, false);
        assert_eq!(app.workspace.envelope_stats().tile_builds, 2);
        app.workspace
            .loaded_channels
            .insert_channel(test_channel_data_with_values("signal", vec![7.0; n]));
        let rows = app.visible_row_traces(128, true, false);
        assert_eq!(app.workspace.envelope_stats().tile_builds, 1);
        let VisibleTraceData::Envelope(envelope) = &rows[0].traces[0].data else {
            panic!("Line envelope")
        };
        assert_eq!(envelope.value_range, Some((7.0, 7.0)));
        assert_eq!(raw_refs[0].strong_count(), 0);
        app.workspace.remove_source(second);
        assert_eq!(raw_refs[1].strong_count(), 0);
    }

    #[test]
    fn line_tile_lod_only_applies_to_wide_line_buckets() {
        let wide_plan = parquet_waveform::MinMaxEnvelopePlan {
            bucket_spans: Vec::new(),
            source_sample_count: LINE_TILE_MIN_SOURCE_SAMPLES,
            requested_bucket_count: 100,
            bucket_size: LINE_TILE_MIN_BUCKET_SIZE,
            time_range: Some((0.0, 1.0)),
        };
        let narrow_plan = parquet_waveform::MinMaxEnvelopePlan {
            bucket_size: LINE_TILE_MIN_BUCKET_SIZE - 1,
            ..wide_plan.clone()
        };
        let small_plan = parquet_waveform::MinMaxEnvelopePlan {
            source_sample_count: LINE_TILE_MIN_SOURCE_SAMPLES - 1,
            ..wide_plan.clone()
        };

        assert!(should_use_line_tile_lod(&wide_plan));
        assert!(!should_use_line_tile_lod(&narrow_plan));
        assert!(!should_use_line_tile_lod(&small_plan));
    }
}
