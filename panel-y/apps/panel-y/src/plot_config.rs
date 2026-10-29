use crate::{
    ChannelKey, DEFAULT_TRACE_LINE_WIDTH, DrawMode, MAX_TRACE_LINE_WIDTH, MIN_TRACE_LINE_WIDTH,
    PlotRow, RowChannel, RowYRange, SourceId, ThemeMode, ViewState, VisualMode, WorkspaceDataState,
    YRangeMode, parquet_schema,
};
use eframe::egui;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

const DOCUMENT_VERSION: i64 = 1;
const RUST_EXT_VERSION: i64 = 2;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlotConfigDocument {
    pub(crate) version: i64,
    pub(crate) data_path: PathBuf,
    pub(crate) rows: Vec<PlotConfigRow>,
    pub(crate) ch_styles: BTreeMap<String, PlotConfigStyle>,
    pub(crate) theme: ThemeMode,
    pub(crate) scroll_zoom: bool,
    pub(crate) rust: RustPlotConfigExt,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlotConfigRow {
    pub(crate) channels: Vec<String>,
    pub(crate) y_min: Option<f64>,
    pub(crate) y_max: Option<f64>,
    pub(crate) step_channels: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlotConfigStyle {
    pub(crate) color: Option<egui::Color32>,
    pub(crate) width: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RustPlotConfigSource {
    pub(crate) key: String,
    pub(crate) data_path: PathBuf,
    pub(crate) label: String,
    pub(crate) time_offset_seconds: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RustPlotConfigExt {
    pub(crate) sources: Option<Vec<RustPlotConfigSource>>,
    pub(crate) visual_mode: VisualMode,
    pub(crate) rows: Vec<RustPlotConfigRow>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RustPlotConfigRow {
    pub(crate) y_min: Option<f64>,
    pub(crate) y_max: Option<f64>,
    pub(crate) channels: Vec<RustPlotConfigChannel>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RustPlotConfigChannel {
    pub(crate) source_key: Option<String>,
    pub(crate) path: String,
    pub(crate) visible: Option<bool>,
    pub(crate) draw_mode: Option<DrawMode>,
    pub(crate) color: Option<egui::Color32>,
    pub(crate) line_width: Option<f32>,
    pub(crate) color_index: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct PlotConfigLoadPlan {
    pub(crate) data_path: PathBuf,
    pub(crate) theme: ThemeMode,
    pub(crate) visual_mode: VisualMode,
    pub(crate) scroll_zoom: bool,
    pub(crate) rows: Vec<PlotRow>,
    pub(crate) selected_channel: Option<ChannelKey>,
    pub(crate) channels_to_load: Vec<ChannelKey>,
    pub(crate) report: PlotConfigApplyReport,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct PlotConfigApplyReport {
    pub(crate) restored_rows: usize,
    pub(crate) restored_channels: usize,
    pub(crate) channels_to_load: usize,
    pub(crate) missing_channels: Vec<String>,
    pub(crate) skipped_empty_rows: usize,
    pub(crate) invalid_y_ranges: Vec<String>,
    pub(crate) invalid_colors: Vec<String>,
    pub(crate) invalid_line_widths: Vec<String>,
    pub(crate) ignored_fields: Vec<String>,
}

#[derive(Debug)]
pub(crate) enum PlotConfigError {
    Io { path: PathBuf, message: String },
    Json { path: PathBuf, message: String },
    Invalid { message: String },
}

impl PlotConfigApplyReport {
    pub(crate) fn warning_count(&self) -> usize {
        self.missing_channels.len()
            + self.skipped_empty_rows
            + self.invalid_y_ranges.len()
            + self.invalid_colors.len()
            + self.invalid_line_widths.len()
            + self.ignored_fields.len()
    }

    pub(crate) fn summary(&self) -> String {
        let warning_count = self.warning_count();
        let warning_note = if warning_count == 0 {
            String::new()
        } else {
            format!(", {warning_count} warning(s)")
        };
        format!(
            "{} row(s), {} channel(s), {} visible load(s){warning_note}",
            self.restored_rows, self.restored_channels, self.channels_to_load
        )
    }

    pub(crate) fn details(&self) -> Option<String> {
        let mut lines = Vec::new();
        if !self.missing_channels.is_empty() {
            lines.push(format!(
                "Missing channel(s): {}",
                self.missing_channels.join(", ")
            ));
        }
        if self.skipped_empty_rows > 0 {
            lines.push(format!("Skipped empty row(s): {}", self.skipped_empty_rows));
        }
        if !self.invalid_y_ranges.is_empty() {
            lines.push(format!(
                "Invalid Y range(s): {}",
                self.invalid_y_ranges.join(", ")
            ));
        }
        if !self.invalid_colors.is_empty() {
            lines.push(format!(
                "Invalid color(s): {}",
                self.invalid_colors.join(", ")
            ));
        }
        if !self.invalid_line_widths.is_empty() {
            lines.push(format!(
                "Invalid line width(s): {}",
                self.invalid_line_widths.join(", ")
            ));
        }
        if !self.ignored_fields.is_empty() {
            lines.push(format!(
                "Ignored field(s): {}",
                self.ignored_fields.join(", ")
            ));
        }

        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    fn append(&mut self, mut other: PlotConfigApplyReport) {
        self.restored_rows = other.restored_rows;
        self.restored_channels = other.restored_channels;
        self.channels_to_load = other.channels_to_load;
        self.missing_channels.append(&mut other.missing_channels);
        self.skipped_empty_rows += other.skipped_empty_rows;
        self.invalid_y_ranges.append(&mut other.invalid_y_ranges);
        self.invalid_colors.append(&mut other.invalid_colors);
        self.invalid_line_widths
            .append(&mut other.invalid_line_widths);
        self.ignored_fields.append(&mut other.ignored_fields);
        self.sort_and_dedupe();
    }

    fn sort_and_dedupe(&mut self) {
        sort_dedupe(&mut self.missing_channels);
        sort_dedupe(&mut self.invalid_y_ranges);
        sort_dedupe(&mut self.invalid_colors);
        sort_dedupe(&mut self.invalid_line_widths);
        sort_dedupe(&mut self.ignored_fields);
    }
}

impl fmt::Display for PlotConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(formatter, "I/O failed for {}: {message}", path.display())
            }
            Self::Json { path, message } => {
                write!(formatter, "Invalid JSON in {}: {message}", path.display())
            }
            Self::Invalid { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PlotConfigError {}

pub(crate) fn read_plot_config(
    path: impl AsRef<Path>,
) -> Result<(PlotConfigDocument, PlotConfigApplyReport), PlotConfigError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|error| PlotConfigError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let value = serde_json::from_str::<Value>(&text).map_err(|error| PlotConfigError::Json {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    normalize_plot_config(value, path)
}

pub(crate) fn normalize_plot_config(
    value: Value,
    config_path: &Path,
) -> Result<(PlotConfigDocument, PlotConfigApplyReport), PlotConfigError> {
    let mut report = PlotConfigApplyReport::default();
    let Some(root) = value.as_object() else {
        return Err(PlotConfigError::Invalid {
            message: "Plot config root must be a JSON object".to_owned(),
        });
    };

    for key in root.keys() {
        if !matches!(
            key.as_str(),
            "version" | "data_path" | "rows" | "ch_styles" | "theme" | "scroll_zoom" | "rust"
        ) {
            report.ignored_fields.push(key.clone());
        }
    }

    let version = root
        .get("version")
        .and_then(Value::as_i64)
        .unwrap_or(DOCUMENT_VERSION);
    if version != DOCUMENT_VERSION {
        report
            .ignored_fields
            .push(format!("version={version} (read as v1-compatible)"));
    }

    let data_path = root
        .get("data_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| PlotConfigError::Invalid {
            message: "Plot config is missing data_path".to_owned(),
        })?;
    let data_path = resolve_data_path(data_path, config_path);

    let rows = parse_rows(root.get("rows"), &mut report);
    let ch_styles = parse_ch_styles(root.get("ch_styles"), &mut report);
    let theme = root
        .get("theme")
        .and_then(Value::as_str)
        .map(ThemeMode::from_config_str)
        .unwrap_or(ThemeMode::Dark);
    if root.get("theme").is_some_and(|theme| {
        theme.as_str().is_none_or(|theme| {
            ThemeMode::from_config_str(theme) == ThemeMode::Dark
                && !theme.eq_ignore_ascii_case("dark")
        })
    }) {
        report.ignored_fields.push("theme".to_owned());
    }
    let scroll_zoom = root
        .get("scroll_zoom")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if root
        .get("scroll_zoom")
        .is_some_and(|scroll_zoom| !scroll_zoom.is_boolean())
    {
        report.ignored_fields.push("scroll_zoom".to_owned());
    }
    let mut rust = parse_rust_ext(root.get("rust"), &mut report);
    rust.sources = parse_sources(root.get("rust"), config_path, &mut report)?;

    report.sort_and_dedupe();
    Ok((
        PlotConfigDocument {
            version,
            data_path,
            rows,
            ch_styles,
            theme,
            scroll_zoom,
            rust,
        },
        report,
    ))
}

pub(crate) fn build_apply_plan(
    document: PlotConfigDocument,
    schema: &parquet_schema::SchemaSummary,
    source_id: SourceId,
    mut report: PlotConfigApplyReport,
) -> PlotConfigLoadPlan {
    let visual_mode = document.rust.visual_mode;
    let theme = if visual_mode == VisualMode::Rock {
        if document.theme != ThemeMode::Dark {
            report
                .ignored_fields
                .push("theme=light (rock visual mode uses dark theme)".to_owned());
        }
        ThemeMode::Dark
    } else {
        document.theme
    };
    let path_set = schema
        .channels
        .iter()
        .map(|channel| channel.path.clone())
        .collect::<BTreeSet<_>>();
    let mut display_to_paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for channel in &schema.channels {
        display_to_paths
            .entry(channel.display_name().to_owned())
            .or_default()
            .push(channel.path.clone());
    }

    let mut rows = Vec::new();
    let mut restored_channels = 0usize;
    let mut channels_to_load = BTreeSet::new();
    let mut first_restored_channel = None;

    for (row_index, config_row) in document.rows.iter().enumerate() {
        let rust_row = document.rust.rows.get(row_index);
        let raw_channel_names = if config_row.channels.is_empty() {
            rust_row
                .map(|row| {
                    row.channels
                        .iter()
                        .map(|channel| channel.path.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            config_row.channels.clone()
        };

        let mut row_channels = Vec::new();
        for raw_channel in raw_channel_names {
            let Some(channel_path) =
                resolve_channel_path(&raw_channel, &path_set, &display_to_paths)
            else {
                report.missing_channels.push(raw_channel);
                continue;
            };
            let rust_channel = rust_row.and_then(|row| {
                row.channels
                    .iter()
                    .find(|channel| channel.path == channel_path || channel.path == raw_channel)
            });

            let color_index = rust_channel
                .and_then(|channel| channel.color_index)
                .unwrap_or(row_channels.len());
            let style = document
                .ch_styles
                .get(&channel_path)
                .or_else(|| document.ch_styles.get(&raw_channel));
            let color = rust_channel
                .and_then(|channel| channel.color)
                .or_else(|| style.and_then(|style| style.color));
            let line_width = rust_channel
                .and_then(|channel| channel.line_width)
                .or_else(|| style.and_then(|style| style.width))
                .unwrap_or(DEFAULT_TRACE_LINE_WIDTH)
                .clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH);
            let draw_mode = rust_channel
                .and_then(|channel| channel.draw_mode)
                .unwrap_or_else(|| {
                    if config_row.step_channels.contains(&channel_path)
                        || config_row.step_channels.contains(&raw_channel)
                    {
                        DrawMode::Step
                    } else {
                        DrawMode::Line
                    }
                });
            let visible = rust_channel
                .and_then(|channel| channel.visible)
                .unwrap_or(true);

            if visible {
                channels_to_load.insert(ChannelKey::new(source_id, channel_path.clone()));
            }
            first_restored_channel.get_or_insert_with(|| channel_path.clone());
            restored_channels += 1;
            row_channels.push(RowChannel {
                key: ChannelKey::new(source_id, channel_path),
                color_index,
                draw_mode,
                visible,
                color_override: color,
                line_width,
            });
        }

        if row_channels.is_empty() {
            report.skipped_empty_rows += 1;
            continue;
        }

        rows.push(PlotRow {
            id: rows.len() as u64,
            y_range: row_y_range_from_config(
                config_row.y_min,
                config_row.y_max,
                row_index,
                &mut report,
            ),
            channels: row_channels,
        });
    }

    if rows.is_empty() {
        rows.push(PlotRow::new(0));
    }

    let selected_channel = first_restored_channel
        .or_else(|| schema.channels.first().map(|channel| channel.path.clone()))
        .map(|channel_path| ChannelKey::new(source_id, channel_path));
    let channels_to_load = channels_to_load.into_iter().collect::<Vec<_>>();
    let mut build_report = PlotConfigApplyReport {
        restored_rows: rows.len(),
        restored_channels,
        channels_to_load: channels_to_load.len(),
        ..PlotConfigApplyReport::default()
    };
    build_report.sort_and_dedupe();
    report.append(build_report);

    PlotConfigLoadPlan {
        data_path: document.data_path,
        theme,
        visual_mode,
        scroll_zoom: document.scroll_zoom,
        rows,
        selected_channel,
        channels_to_load,
        report,
    }
}

pub(crate) fn capture_plot_config(
    workspace: &WorkspaceDataState,
    view: &ViewState,
) -> PlotConfigDocument {
    // Empty cards are placeholders, not file-backed sources. In particular, do
    // not turn an empty path into the current directory via absolute_data_path.
    let configured_sources = workspace
        .source_order
        .iter()
        .filter_map(|id| workspace.source(*id))
        .filter(|source| !source.parquet_path.trim().is_empty())
        .collect::<Vec<_>>();
    let dataset = configured_sources
        .first()
        .expect("plot config capture requires a source with a data path");
    let source_id = dataset.id;
    let source_keys = configured_sources
        .iter()
        .enumerate()
        .map(|(index, source)| (source.id, format!("source-{}", index + 1)))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();
    let mut ch_styles = BTreeMap::new();
    let theme = if view.visual_mode == VisualMode::Rock {
        ThemeMode::Dark
    } else {
        view.theme
    };
    let dark_mode = theme == ThemeMode::Dark;

    for row in &view.rows {
        let source_channels = row
            .channels
            .iter()
            .filter(|channel| channel.key.source_id == source_id)
            .collect::<Vec<_>>();
        if source_channels.is_empty() {
            continue;
        }

        let channels = source_channels
            .iter()
            .map(|channel| channel.key.channel_path.clone())
            .collect::<Vec<_>>();
        let step_channels = source_channels
            .iter()
            .filter(|channel| channel.draw_mode == DrawMode::Step)
            .map(|channel| channel.key.channel_path.clone())
            .collect::<BTreeSet<_>>();
        let (y_min, y_max) = match row.y_range.mode {
            YRangeMode::Auto => (None, None),
            YRangeMode::Manual => (row.y_range.min, row.y_range.max),
        };

        for channel in &source_channels {
            ch_styles
                .entry(channel.key.channel_path.clone())
                .or_insert_with(|| PlotConfigStyle {
                    color: Some(channel.color(dark_mode)),
                    width: Some(
                        channel
                            .line_width
                            .clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH),
                    ),
                });
        }

        rows.push(PlotConfigRow {
            channels,
            y_min,
            y_max,
            step_channels,
        });
    }

    let sources = configured_sources
        .iter()
        .map(|source| RustPlotConfigSource {
            key: source_keys[&source.id].clone(),
            data_path: absolute_data_path(source.parquet_path.trim()),
            label: source.label.clone(),
            time_offset_seconds: source.time_transform.offset_seconds,
        })
        .collect();
    let rust_rows = view
        .rows
        .iter()
        .map(|row| RustPlotConfigRow {
            y_min: (row.y_range.mode == YRangeMode::Manual)
                .then_some(row.y_range.min)
                .flatten(),
            y_max: (row.y_range.mode == YRangeMode::Manual)
                .then_some(row.y_range.max)
                .flatten(),
            channels: row
                .channels
                .iter()
                .filter_map(|channel| {
                    let source_key = source_keys.get(&channel.key.source_id)?;
                    Some(RustPlotConfigChannel {
                        source_key: Some(source_key.clone()),
                        path: channel.key.channel_path.clone(),
                        visible: Some(channel.visible),
                        draw_mode: Some(channel.draw_mode),
                        color: Some(channel.color(dark_mode)),
                        line_width: Some(
                            channel
                                .line_width
                                .clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH),
                        ),
                        color_index: Some(channel.color_index),
                    })
                })
                .collect(),
        })
        .collect();

    PlotConfigDocument {
        version: DOCUMENT_VERSION,
        data_path: absolute_data_path(dataset.parquet_path.trim()),
        rows,
        ch_styles,
        theme,
        scroll_zoom: view.scroll_zoom,
        rust: RustPlotConfigExt {
            sources: Some(sources),
            visual_mode: view.visual_mode,
            rows: rust_rows,
        },
    }
}

pub(crate) fn write_plot_config(
    path: impl AsRef<Path>,
    document: &PlotConfigDocument,
) -> Result<(), PlotConfigError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|error| PlotConfigError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }

    let value = document_to_json(document);
    let text = serde_json::to_string_pretty(&value).map_err(|error| PlotConfigError::Json {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    std::fs::write(path, format!("{text}\n")).map_err(|error| PlotConfigError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn parse_sources(
    rust: Option<&Value>,
    config_path: &Path,
    report: &mut PlotConfigApplyReport,
) -> Result<Option<Vec<RustPlotConfigSource>>, PlotConfigError> {
    let Some(value) = rust
        .and_then(|rust| rust.get("sources"))
        .filter(|v| !v.is_null())
    else {
        return Ok(None);
    };
    let invalid = |message: &str| PlotConfigError::Invalid {
        message: message.to_owned(),
    };
    if rust
        .and_then(|rust| rust.get("version"))
        .and_then(Value::as_i64)
        != Some(2)
    {
        return Err(invalid("rust.sources requires rust.version 2"));
    }
    if !rust.and_then(|rust| rust.get("rows")).is_some_and(|rows| {
        rows.as_array().is_some_and(|rows| {
            rows.iter().all(|row| {
                row.get("channels").is_some_and(|channels| {
                    channels.as_array().is_some_and(|channels| {
                        channels.iter().all(|channel| {
                            channel
                                .get("path")
                                .and_then(Value::as_str)
                                .is_some_and(|path| !path.is_empty())
                        })
                    })
                })
            })
        })
    }) {
        return Err(invalid(
            "rust.rows must contain rows with channel arrays and nonempty channel paths",
        ));
    }
    let values = value
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| invalid("rust.sources must be a nonempty array"))?;
    let mut keys = BTreeSet::new();
    let mut sources = Vec::new();
    for value in values {
        let key = value
            .get("key")
            .and_then(Value::as_str)
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| invalid("Every source requires a nonempty key"))?;
        if !keys.insert(key.to_owned()) {
            return Err(invalid(&format!("Duplicate source key: {key}")));
        }
        let path = value
            .get("data_path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| invalid(&format!("Source {key} requires data_path")))?;
        let offset = match value.get("time_offset_seconds") {
            None => 0.0,
            Some(value) => match value.as_f64().filter(|offset| offset.is_finite()) {
                Some(offset) => offset,
                None => {
                    report
                        .ignored_fields
                        .push(format!("{key}.time_offset_seconds (using 0)"));
                    0.0
                }
            },
        };
        sources.push(RustPlotConfigSource {
            key: key.to_owned(),
            data_path: resolve_data_path(path, config_path),
            label: value
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            time_offset_seconds: offset,
        });
    }
    Ok(Some(sources))
}

/// Source keys are file-local identities; the loader assigns runtime IDs only after
/// schemas have been checked. Never resolve a channel across another source.
pub(crate) fn build_multi_source_apply_plan(
    document: PlotConfigDocument,
    workspace: &WorkspaceDataState,
    source_ids: &BTreeMap<String, SourceId>,
    mut report: PlotConfigApplyReport,
) -> PlotConfigLoadPlan {
    let mut rows = Vec::new();
    let mut channels_to_load = BTreeSet::new();
    let mut selected_channel = None;
    for (row_index, row) in document.rust.rows.iter().enumerate() {
        let mut channels = Vec::new();
        let mut seen = BTreeSet::new();
        for channel in &row.channels {
            let source_key = channel
                .source_key
                .as_deref()
                .unwrap_or("(missing source_key)");
            let resolved = source_ids.get(source_key).copied().and_then(|id| {
                let schema = workspace.source(id)?.schema.as_ref()?;
                schema
                    .channels
                    .iter()
                    .find(|candidate| candidate.path == channel.path)
                    .map(|_| ChannelKey::new(id, channel.path.clone()))
            });
            let Some(key) = resolved else {
                report
                    .missing_channels
                    .push(format!("{source_key}:{}", channel.path));
                continue;
            };
            if !seen.insert(key.clone()) {
                report.ignored_fields.push(format!(
                    "rust.rows[{row_index}] duplicate {source_key}:{}",
                    channel.path
                ));
                continue;
            }
            let visible = channel.visible.unwrap_or(true);
            if visible {
                channels_to_load.insert(key.clone());
            }
            selected_channel.get_or_insert_with(|| key.clone());
            channels.push(RowChannel {
                key,
                color_index: channel.color_index.unwrap_or(channels.len()),
                draw_mode: channel.draw_mode.unwrap_or(DrawMode::Line),
                visible,
                color_override: channel.color,
                line_width: channel.line_width.unwrap_or(DEFAULT_TRACE_LINE_WIDTH),
            });
        }
        // Intentional empty rows are part of the layout; rows emptied by missing
        // data are omitted and reported.
        if channels.is_empty() && !row.channels.is_empty() {
            report.skipped_empty_rows += 1;
            continue;
        }
        rows.push(PlotRow {
            id: rows.len() as u64,
            channels,
            y_range: row_y_range_from_config(row.y_min, row.y_max, row_index, &mut report),
        });
    }
    if rows.is_empty() {
        rows.push(PlotRow::new(0));
    }
    report.restored_rows = rows.len();
    report.restored_channels = rows.iter().map(|row| row.channels.len()).sum();
    report.channels_to_load = channels_to_load.len();
    report.sort_and_dedupe();
    PlotConfigLoadPlan {
        data_path: document.data_path,
        theme: if document.rust.visual_mode == VisualMode::Rock {
            ThemeMode::Dark
        } else {
            document.theme
        },
        visual_mode: document.rust.visual_mode,
        scroll_zoom: document.scroll_zoom,
        rows,
        selected_channel,
        channels_to_load: channels_to_load.into_iter().collect(),
        report,
    }
}

fn parse_rows(value: Option<&Value>, report: &mut PlotConfigApplyReport) -> Vec<PlotConfigRow> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(rows) = value.as_array() else {
        report.ignored_fields.push("rows".to_owned());
        return Vec::new();
    };

    rows.iter()
        .enumerate()
        .filter_map(|(row_index, row_value)| {
            let row = row_value.as_object()?;
            let channels = row
                .get("channels")
                .and_then(Value::as_array)
                .map(|channels| {
                    channels
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| {
                    report
                        .ignored_fields
                        .push(format!("rows[{row_index}].channels"));
                    Vec::new()
                });
            let y_min = parse_optional_f64(row.get("ymin"), report, "ymin", row_index);
            let y_max = parse_optional_f64(row.get("ymax"), report, "ymax", row_index);
            let step_channels = row
                .get("step_channels")
                .and_then(Value::as_array)
                .map(|channels| {
                    channels
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();

            Some(PlotConfigRow {
                channels,
                y_min,
                y_max,
                step_channels,
            })
        })
        .collect()
}

fn parse_ch_styles(
    value: Option<&Value>,
    report: &mut PlotConfigApplyReport,
) -> BTreeMap<String, PlotConfigStyle> {
    let Some(value) = value else {
        return BTreeMap::new();
    };
    let Some(styles) = value.as_object() else {
        report.ignored_fields.push("ch_styles".to_owned());
        return BTreeMap::new();
    };

    styles
        .iter()
        .filter_map(|(channel, style_value)| {
            let style = style_value.as_object()?;
            Some((
                channel.clone(),
                PlotConfigStyle {
                    color: style.get("color").and_then(|value| {
                        parse_color_value(value, report, &format!("ch_styles.{channel}.color"))
                    }),
                    width: style.get("width").and_then(|value| {
                        parse_line_width_value(value, report, &format!("ch_styles.{channel}.width"))
                    }),
                },
            ))
        })
        .collect()
}

fn parse_rust_ext(value: Option<&Value>, report: &mut PlotConfigApplyReport) -> RustPlotConfigExt {
    let Some(value) = value else {
        return RustPlotConfigExt::default();
    };
    let Some(root) = value.as_object() else {
        report.ignored_fields.push("rust".to_owned());
        return RustPlotConfigExt::default();
    };
    let version = root.get("version").and_then(Value::as_i64).unwrap_or(1);
    if version != 1 && version != RUST_EXT_VERSION {
        report
            .ignored_fields
            .push(format!("rust.version={version} (read as v1-compatible)"));
    }

    let visual_mode = root
        .get("visual_mode")
        .map(|value| parse_visual_mode_value(value, report, "rust.visual_mode"))
        .unwrap_or(VisualMode::Plain);
    let rows = root
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .enumerate()
                .map(|(row_index, row_value)| RustPlotConfigRow {
                    y_min: parse_optional_f64(row_value.get("ymin"), report, "ymin", row_index),
                    y_max: parse_optional_f64(row_value.get("ymax"), report, "ymax", row_index),
                    channels: row_value
                        .get("channels")
                        .and_then(Value::as_array)
                        .map(|channels| {
                            channels
                                .iter()
                                .filter_map(|channel| {
                                    parse_rust_channel(channel, row_index, report)
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();

    RustPlotConfigExt {
        sources: None,
        visual_mode,
        rows,
    }
}

fn parse_visual_mode_value(
    value: &Value,
    report: &mut PlotConfigApplyReport,
    label: &str,
) -> VisualMode {
    let Some(text) = value.as_str() else {
        report.ignored_fields.push(label.to_owned());
        return VisualMode::Plain;
    };

    match text.trim().to_ascii_lowercase().as_str() {
        "plain" => VisualMode::Plain,
        "rock" => VisualMode::Rock,
        _ => {
            report.ignored_fields.push(label.to_owned());
            VisualMode::Plain
        }
    }
}

fn parse_rust_channel(
    value: &Value,
    row_index: usize,
    report: &mut PlotConfigApplyReport,
) -> Option<RustPlotConfigChannel> {
    let channel = value.as_object()?;
    let path = channel.get("path")?.as_str()?.to_owned();
    let visible = channel.get("visible").and_then(Value::as_bool);
    let draw_mode = channel
        .get("draw_mode")
        .and_then(Value::as_str)
        .and_then(DrawMode::from_config_str);
    let color = channel.get("color").and_then(|value| {
        parse_color_value(
            value,
            report,
            &format!("rust.rows[{row_index}].{path}.color"),
        )
    });
    let line_width = channel.get("line_width").and_then(|value| {
        parse_line_width_value(
            value,
            report,
            &format!("rust.rows[{row_index}].{path}.line_width"),
        )
    });
    let color_index = channel
        .get("color_index")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok());

    Some(RustPlotConfigChannel {
        source_key: channel
            .get("source_key")
            .and_then(Value::as_str)
            .map(str::to_owned),
        path,
        visible,
        draw_mode,
        color,
        line_width,
        color_index,
    })
}

fn parse_optional_f64(
    value: Option<&Value>,
    report: &mut PlotConfigApplyReport,
    key: &str,
    row_index: usize,
) -> Option<f64> {
    match value {
        None | Some(Value::Null) => None,
        Some(value) => match value.as_f64() {
            Some(number) if number.is_finite() => Some(number),
            _ => {
                report
                    .invalid_y_ranges
                    .push(format!("rows[{row_index}].{key}"));
                None
            }
        },
    }
}

fn parse_color_value(
    value: &Value,
    report: &mut PlotConfigApplyReport,
    label: &str,
) -> Option<egui::Color32> {
    let Some(text) = value.as_str() else {
        report.invalid_colors.push(label.to_owned());
        return None;
    };
    match parse_hex_color(text) {
        Some(color) => Some(color),
        None => {
            report.invalid_colors.push(label.to_owned());
            None
        }
    }
}

fn parse_line_width_value(
    value: &Value,
    report: &mut PlotConfigApplyReport,
    label: &str,
) -> Option<f32> {
    let Some(width) = value.as_f64() else {
        report.invalid_line_widths.push(label.to_owned());
        return None;
    };
    if !width.is_finite() {
        report.invalid_line_widths.push(label.to_owned());
        return None;
    }

    let width = width as f32;
    if !(MIN_TRACE_LINE_WIDTH..=MAX_TRACE_LINE_WIDTH).contains(&width) {
        report.invalid_line_widths.push(label.to_owned());
    }
    Some(width.clamp(MIN_TRACE_LINE_WIDTH, MAX_TRACE_LINE_WIDTH))
}

fn row_y_range_from_config(
    y_min: Option<f64>,
    y_max: Option<f64>,
    row_index: usize,
    report: &mut PlotConfigApplyReport,
) -> RowYRange {
    if let (Some(min), Some(max)) = (y_min, y_max)
        && min > max
    {
        report.invalid_y_ranges.push(format!("rows[{row_index}]"));
        return RowYRange::default();
    }

    if y_min.is_some() || y_max.is_some() {
        RowYRange {
            mode: YRangeMode::Manual,
            min: y_min,
            max: y_max,
            last_auto: None,
        }
    } else {
        RowYRange::default()
    }
}

fn resolve_channel_path(
    channel: &str,
    path_set: &BTreeSet<String>,
    display_to_paths: &BTreeMap<String, Vec<String>>,
) -> Option<String> {
    if path_set.contains(channel) {
        return Some(channel.to_owned());
    }

    match display_to_paths.get(channel).map(Vec::as_slice) {
        Some([path]) => Some(path.clone()),
        _ => None,
    }
}

fn absolute_data_path(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&path))
            .unwrap_or(path)
    }
}

fn resolve_data_path(data_path: &str, config_path: &Path) -> PathBuf {
    let path = PathBuf::from(data_path);
    if path.is_absolute() {
        return path;
    }

    let parent_candidate = config_path
        .parent()
        .map(|parent| parent.join(&path))
        .unwrap_or_else(|| path.clone());
    if parent_candidate.exists() {
        return parent_candidate;
    }

    match std::env::current_dir().map(|cwd| cwd.join(&path)) {
        Ok(candidate) if candidate.exists() => candidate,
        _ => parent_candidate,
    }
}

fn document_to_json(document: &PlotConfigDocument) -> Value {
    let rows = document
        .rows
        .iter()
        .map(|row| {
            let step_channels = row.step_channels.iter().cloned().collect::<Vec<_>>();
            json!({
                "channels": &row.channels,
                "ymin": row.y_min,
                "ymax": row.y_max,
                "step_channels": step_channels,
            })
        })
        .collect::<Vec<_>>();
    let ch_styles = document
        .ch_styles
        .iter()
        .map(|(channel, style)| {
            let mut style_json = Map::new();
            if let Some(color) = style.color {
                style_json.insert("color".to_owned(), Value::String(color_to_hex(color)));
            }
            if let Some(width) = style.width {
                style_json.insert("width".to_owned(), json!(width));
            }
            (channel.clone(), Value::Object(style_json))
        })
        .collect::<Map<_, _>>();
    let rust_rows = document
        .rust
        .rows
        .iter()
        .map(|row| {
            let channels = row
                .channels
                .iter()
                .map(|channel| {
                    json!({
                        "source_key": &channel.source_key,
                        "path": &channel.path,
                        "visible": channel.visible.unwrap_or(true),
                        "draw_mode": channel.draw_mode.unwrap_or(DrawMode::Line).config_label(),
                        "color": channel.color.map(color_to_hex),
                        "line_width": channel.line_width,
                        "color_index": channel.color_index,
                    })
                })
                .collect::<Vec<_>>();
            json!({ "channels": channels, "ymin": row.y_min, "ymax": row.y_max })
        })
        .collect::<Vec<_>>();

    json!({
        "version": document.version,
        "data_path": document.data_path.display().to_string(),
        "rows": rows,
        "ch_styles": ch_styles,
        "theme": document.theme.config_label(),
        "scroll_zoom": document.scroll_zoom,
        "rust": {
            "version": if document.rust.sources.is_some() { RUST_EXT_VERSION } else { 1 },
            "sources": document.rust.sources.as_ref().map(|sources| sources.iter().map(|source| json!({
                "key": source.key, "data_path": source.data_path, "label": source.label,
                "time_offset_seconds": source.time_offset_seconds,
            })).collect::<Vec<_>>()),
            "visual_mode": visual_mode_config_label(document.rust.visual_mode),
            "rows": rust_rows,
        },
    })
}

fn visual_mode_config_label(visual_mode: VisualMode) -> &'static str {
    match visual_mode {
        VisualMode::Plain => "plain",
        VisualMode::Rock => "rock",
    }
}

fn parse_hex_color(text: &str) -> Option<egui::Color32> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(egui::Color32::from_rgb(red, green, blue))
}

fn color_to_hex(color: egui::Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

fn sort_dedupe(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

impl DrawMode {
    fn from_config_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "line" => Some(Self::Line),
            "step" => Some(Self::Step),
            _ => None,
        }
    }

    fn config_label(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Step => "step",
        }
    }
}

impl ThemeMode {
    fn from_config_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    fn config_label(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_SOURCE_ID, parquet_schema::ColumnInfo, parquet_schema::ColumnRole};

    fn test_schema(paths: &[&str]) -> parquet_schema::SchemaSummary {
        parquet_schema::SchemaSummary {
            path: PathBuf::from("/tmp/test.parquet"),
            row_count: 10,
            row_group_count: 1,
            column_count: paths.len() + 1,
            created_by: None,
            time_column: None,
            channels: paths
                .iter()
                .enumerate()
                .map(|(index, path)| ColumnInfo {
                    index,
                    name: (*path).to_owned(),
                    path: (*path).to_owned(),
                    physical_type: "DOUBLE".to_owned(),
                    logical_type: None,
                    converted_type: "NONE".to_owned(),
                    is_numeric: true,
                    role: ColumnRole::Channel,
                })
                .collect(),
            columns: Vec::new(),
        }
    }

    #[test]
    fn parses_minimal_valid_v1() {
        let value = json!({
            "version": 1,
            "data_path": "/tmp/test.parquet",
            "rows": [{"channels": ["sine"], "ymin": null, "ymax": null, "step_channels": []}],
            "ch_styles": {"sine": {"color": "#50beff", "width": 1.25}},
            "theme": "dark",
            "scroll_zoom": false
        });

        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");

        assert_eq!(document.version, 1);
        assert_eq!(document.rows.len(), 1);
        assert_eq!(document.rust.visual_mode, VisualMode::Plain);
        assert_eq!(
            document.ch_styles["sine"].color,
            Some(egui::Color32::from_rgb(0x50, 0xbe, 0xff))
        );
        assert_eq!(report.warning_count(), 0);
    }

    #[test]
    fn missing_version_is_read_as_v1() {
        let value = json!({
            "data_path": "/tmp/test.parquet",
            "rows": []
        });

        let (document, _) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");

        assert_eq!(document.version, 1);
    }

    #[test]
    fn build_plan_skips_missing_channels_and_empty_rows() {
        let value = json!({
            "version": 1,
            "data_path": "/tmp/test.parquet",
            "rows": [
                {"channels": ["sine", "missing"], "ymin": null, "ymax": null},
                {"channels": ["missing"], "ymin": null, "ymax": null}
            ]
        });
        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");
        let plan = build_apply_plan(document, &test_schema(&["sine"]), DEFAULT_SOURCE_ID, report);

        assert_eq!(plan.rows.len(), 1);
        assert_eq!(plan.report.restored_channels, 1);
        assert_eq!(plan.report.skipped_empty_rows, 1);
        assert_eq!(plan.report.missing_channels, vec!["missing"]);
        assert_eq!(
            plan.rows[0].channels[0].key,
            ChannelKey::new(DEFAULT_SOURCE_ID, "sine")
        );
        assert_eq!(
            plan.selected_channel,
            Some(ChannelKey::new(DEFAULT_SOURCE_ID, "sine"))
        );
        assert_eq!(
            plan.channels_to_load,
            vec![ChannelKey::new(DEFAULT_SOURCE_ID, "sine")]
        );
    }

    #[test]
    fn invalid_y_range_falls_back_to_auto() {
        let value = json!({
            "version": 1,
            "data_path": "/tmp/test.parquet",
            "rows": [{"channels": ["sine"], "ymin": 5.0, "ymax": 1.0}]
        });
        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");
        let plan = build_apply_plan(document, &test_schema(&["sine"]), DEFAULT_SOURCE_ID, report);

        assert_eq!(plan.rows[0].y_range.mode, YRangeMode::Auto);
        assert_eq!(plan.report.invalid_y_ranges, vec!["rows[0]"]);
    }

    #[test]
    fn one_sided_y_range_is_preserved() {
        let value = json!({
            "version": 1,
            "data_path": "/tmp/test.parquet",
            "rows": [{"channels": ["sine"], "ymin": -2.0, "ymax": null}]
        });
        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");
        let plan = build_apply_plan(document, &test_schema(&["sine"]), DEFAULT_SOURCE_ID, report);

        assert_eq!(plan.rows[0].y_range.mode, YRangeMode::Manual);
        assert_eq!(plan.rows[0].y_range.min, Some(-2.0));
        assert_eq!(plan.rows[0].y_range.max, None);
    }

    #[test]
    fn rust_extension_restores_visibility_and_row_style() {
        let value = json!({
            "version": 1,
            "data_path": "/tmp/test.parquet",
            "rows": [{"channels": ["sine"], "ymin": null, "ymax": null}],
            "rust": {
                "version": 1,
                "rows": [{
                    "channels": [{
                        "path": "sine",
                        "visible": false,
                        "draw_mode": "step",
                        "color": "#010203",
                        "line_width": 2.5,
                        "color_index": 7
                    }]
                }]
            }
        });
        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");
        let plan = build_apply_plan(document, &test_schema(&["sine"]), DEFAULT_SOURCE_ID, report);
        let channel = &plan.rows[0].channels[0];

        assert!(!channel.visible);
        assert_eq!(channel.draw_mode, DrawMode::Step);
        assert_eq!(
            channel.color_override,
            Some(egui::Color32::from_rgb(1, 2, 3))
        );
        assert_eq!(channel.line_width, 2.5);
        assert_eq!(channel.color_index, 7);
        assert!(plan.channels_to_load.is_empty());
    }

    #[test]
    fn rust_visual_mode_restores_rock_and_forces_dark_theme() {
        let value = json!({
            "version": 1,
            "data_path": "/tmp/test.parquet",
            "rows": [{"channels": ["sine"], "ymin": null, "ymax": null}],
            "theme": "light",
            "rust": {
                "version": 1,
                "visual_mode": "rock",
                "rows": []
            }
        });
        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");

        assert_eq!(document.theme, ThemeMode::Light);
        assert_eq!(document.rust.visual_mode, VisualMode::Rock);

        let plan = build_apply_plan(document, &test_schema(&["sine"]), DEFAULT_SOURCE_ID, report);

        assert_eq!(plan.theme, ThemeMode::Dark);
        assert_eq!(plan.visual_mode, VisualMode::Rock);
        assert!(
            plan.report
                .ignored_fields
                .iter()
                .any(|field| { field == "theme=light (rock visual mode uses dark theme)" })
        );
    }

    #[test]
    fn invalid_rust_visual_mode_falls_back_to_plain() {
        let value = json!({
            "version": 1,
            "data_path": "/tmp/test.parquet",
            "rows": [],
            "rust": {
                "version": 1,
                "visual_mode": "chaos"
            }
        });
        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");

        assert_eq!(document.rust.visual_mode, VisualMode::Plain);
        assert_eq!(report.ignored_fields, vec!["rust.visual_mode"]);
    }

    #[test]
    fn capture_round_trips_rust_visual_mode() {
        let workspace = WorkspaceDataState::with_parquet_path("/tmp/test.parquet".to_owned());
        let mut view = ViewState::default();
        view.theme = ThemeMode::Dark;
        view.visual_mode = VisualMode::Rock;

        let captured = capture_plot_config(&workspace, &view);
        let value = document_to_json(&captured);

        assert_eq!(captured.theme, ThemeMode::Dark);
        assert_eq!(captured.rust.visual_mode, VisualMode::Rock);
        assert_eq!(value["theme"], "dark");
        assert_eq!(value["rust"]["visual_mode"], "rock");

        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");
        let plan = build_apply_plan(document, &test_schema(&[]), DEFAULT_SOURCE_ID, report);

        assert_eq!(plan.theme, ThemeMode::Dark);
        assert_eq!(plan.visual_mode, VisualMode::Rock);
    }

    #[test]
    fn capture_round_trips_rust_row_styles() {
        let workspace = WorkspaceDataState::with_parquet_path("/tmp/test.parquet".to_owned());
        let mut view = ViewState::default();
        view.theme = ThemeMode::Light;
        view.scroll_zoom = true;
        view.rows[0].channels.push(RowChannel {
            key: ChannelKey::new(DEFAULT_SOURCE_ID, "sine"),
            color_index: 3,
            draw_mode: DrawMode::Step,
            visible: false,
            color_override: Some(egui::Color32::from_rgb(9, 8, 7)),
            line_width: 3.0,
        });

        let captured = capture_plot_config(&workspace, &view);
        let value = document_to_json(&captured);
        assert_eq!(value["rows"][0]["channels"][0], "sine");
        assert_eq!(value["rust"]["rows"][0]["channels"][0]["path"], "sine");
        assert_eq!(value["rows"][0]["channels"][0].as_str(), Some("sine"));
        let (document, report) =
            normalize_plot_config(value, Path::new("/tmp/config.pyc.json")).expect("normalize");
        let plan = build_apply_plan(document, &test_schema(&["sine"]), DEFAULT_SOURCE_ID, report);
        let channel = &plan.rows[0].channels[0];

        assert_eq!(plan.theme, ThemeMode::Light);
        assert_eq!(plan.visual_mode, VisualMode::Plain);
        assert!(plan.scroll_zoom);
        assert_eq!(channel.key, ChannelKey::new(DEFAULT_SOURCE_ID, "sine"));
        assert_eq!(channel.draw_mode, DrawMode::Step);
        assert!(!channel.visible);
        assert_eq!(
            channel.color_override,
            Some(egui::Color32::from_rgb(9, 8, 7))
        );
        assert_eq!(channel.line_width, 3.0);
    }
    fn v2_value() -> Value {
        json!({"version":1,"data_path":"/tmp/legacy.parquet","rows":[{"channels":["wrong"]}],
            "rust":{"version":2,"sources":[
                {"key":"alpha","data_path":"a.parquet","label":"A","time_offset_seconds":0.25},
                {"key":"beta","data_path":"b.parquet","time_offset_seconds":-0.5}],
                "rows":[{"ymin":-2,"ymax":3,"channels":[
                    {"source_key":"beta","path":"iu","visible":false,"color":"#010203","draw_mode":"step","line_width":2.5},
                    {"source_key":"alpha","path":"iu","visible":true}]}]}})
    }

    #[test]
    fn v2_resolves_relative_paths_offsets_and_validates_identity() {
        let (doc, report) =
            normalize_plot_config(v2_value(), Path::new("/tmp/v2/config.json")).unwrap();
        let sources = doc.rust.sources.unwrap();
        assert_eq!(sources[0].data_path, Path::new("/tmp/v2/a.parquet"));
        assert_eq!(sources[1].time_offset_seconds, -0.5);
        assert_eq!(report.warning_count(), 0);
        for bad in [
            json!([]),
            json!({}),
            json!([{"key":"a","data_path":"a"},{"key":"a","data_path":"b"}]),
            json!([{"key":"a"}]),
        ] {
            let mut value = v2_value();
            value["rust"]["sources"] = bad;
            assert!(normalize_plot_config(value, Path::new("/tmp/c.json")).is_err());
        }
        let mut value = v2_value();
        value["rust"]["version"] = json!(99);
        assert!(normalize_plot_config(value, Path::new("/tmp/c.json")).is_err());
    }

    #[test]
    fn v2_invalid_offset_defaults_with_warning() {
        let mut value = v2_value();
        value["rust"]["sources"][0]["time_offset_seconds"] = json!("NaN");
        let (doc, report) = normalize_plot_config(value, Path::new("/tmp/c.json")).unwrap();
        assert_eq!(doc.rust.sources.unwrap()[0].time_offset_seconds, 0.0);
        assert!(report.details().unwrap().contains("time_offset_seconds"));
    }

    #[test]
    fn v2_rows_use_source_keys_and_preserve_order_styles_and_hidden_channels() {
        let (doc, report) = normalize_plot_config(v2_value(), Path::new("/tmp/c.json")).unwrap();
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/a.parquet".into());
        let second = workspace.add_source("/tmp/b.parquet".into());
        for id in workspace.source_order.clone() {
            workspace.source_mut(id).unwrap().schema = Some(test_schema(&["iu"]));
        }
        let map = BTreeMap::from([("alpha".into(), DEFAULT_SOURCE_ID), ("beta".into(), second)]);
        let plan = build_multi_source_apply_plan(doc, &workspace, &map, report);
        assert_eq!(plan.report.warning_count(), 0);
        assert_eq!(plan.rows[0].channels.len(), 2);
        let hidden = &plan.rows[0].channels[0];
        assert_eq!(hidden.key, ChannelKey::new(second, "iu"));
        assert!(!hidden.visible);
        assert_eq!(hidden.draw_mode, DrawMode::Step);
        assert_eq!(hidden.line_width, 2.5);
        assert_eq!(
            hidden.color_override,
            Some(egui::Color32::from_rgb(1, 2, 3))
        );
        assert_eq!(plan.rows[0].y_range.min, Some(-2.0));
        assert_eq!(plan.rows[0].y_range.max, Some(3.0));
        assert_eq!(
            plan.channels_to_load,
            vec![ChannelKey::new(DEFAULT_SOURCE_ID, "iu")]
        );
    }

    #[test]
    fn v2_missing_source_never_borrows_same_named_channel() {
        let (doc, report) = normalize_plot_config(v2_value(), Path::new("/tmp/c.json")).unwrap();
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/a.parquet".into());
        workspace.source_mut(DEFAULT_SOURCE_ID).unwrap().schema = Some(test_schema(&["iu"]));
        let map = BTreeMap::from([("alpha".into(), DEFAULT_SOURCE_ID)]);
        let plan = build_multi_source_apply_plan(doc, &workspace, &map, report);
        assert_eq!(plan.rows[0].channels.len(), 1);
        assert_eq!(plan.report.missing_channels, vec!["beta:iu"]);
    }

    #[test]
    fn v2_capture_projects_first_source_and_round_trips_complete_layout() {
        let mut workspace = WorkspaceDataState::with_parquet_path("/tmp/a.parquet".into());
        let second = workspace.add_source("/tmp/b.parquet".into());
        workspace.selected_source_id = Some(second);
        workspace.set_source_time_offset(second, -0.75);
        let mut view = ViewState::default();
        let make = |id| RowChannel {
            key: ChannelKey::new(id, "iu"),
            color_index: 1,
            visible: false,
            draw_mode: DrawMode::Step,
            color_override: None,
            line_width: 2.0,
        };
        view.rows[0].channels = vec![make(second)];
        let mut mixed = PlotRow::new(1);
        mixed.channels = vec![make(second), make(DEFAULT_SOURCE_ID)];
        mixed.y_range = RowYRange {
            mode: YRangeMode::Manual,
            min: Some(-3.0),
            max: None,
            last_auto: None,
        };
        view.rows.push(mixed);
        view.rows.push(PlotRow::new(2));
        let captured = capture_plot_config(&workspace, &view);
        let value = document_to_json(&captured);
        assert_eq!(value["data_path"], "/tmp/a.parquet");
        assert_eq!(value["rows"].as_array().unwrap().len(), 1);
        assert_eq!(value["rust"]["rows"].as_array().unwrap().len(), 3);
        assert_eq!(value["rust"]["sources"][1]["time_offset_seconds"], -0.75);
        let (restored, report) = normalize_plot_config(value, Path::new("/tmp/c.json")).unwrap();
        assert_eq!(report.warning_count(), 0);
        assert_eq!(captured, restored);
        for id in workspace.source_order.clone() {
            workspace.source_mut(id).unwrap().schema = Some(test_schema(&["iu"]));
        }
        let map = BTreeMap::from([
            ("source-1".into(), DEFAULT_SOURCE_ID),
            ("source-2".into(), second),
        ]);
        let plan = build_multi_source_apply_plan(restored, &workspace, &map, report);
        assert_eq!(plan.rows.len(), 3);
        assert!(plan.rows[2].channels.is_empty());
        assert_eq!(plan.rows[1].y_range.min, Some(-3.0));
        assert_eq!(plan.rows[1].y_range.max, None);
    }

    #[test]
    fn v2_capture_skips_empty_source_cards_and_preserves_channel_mapping() {
        let mut workspace = WorkspaceDataState::with_parquet_path(String::new());
        let second = workspace.add_source("/tmp/a.parquet".into());
        workspace.add_source("  ".into());
        let fourth = workspace.add_source("/tmp/b.parquet".into());
        workspace.add_source(String::new());
        workspace.set_source_time_offset(fourth, -0.75);
        let mut view = ViewState::default();
        view.rows[0].channels = vec![
            RowChannel::new(ChannelKey::new(fourth, "iu"), 0),
            RowChannel::new(ChannelKey::new(second, "iu"), 1),
        ];
        let captured = capture_plot_config(&workspace, &view);
        let sources = captured.rust.sources.as_ref().unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(captured.data_path, PathBuf::from("/tmp/a.parquet"));
        assert_eq!(captured.rows[0].channels, vec!["iu"]);
        assert_eq!(sources[1].time_offset_seconds, -0.75);
        assert_eq!(
            captured.rust.rows[0].channels[0].source_key.as_deref(),
            Some(sources[1].key.as_str())
        );
        assert_eq!(
            captured.rust.rows[0].channels[1].source_key.as_deref(),
            Some(sources[0].key.as_str())
        );
        let (restored, report) =
            normalize_plot_config(document_to_json(&captured), Path::new("/tmp/c.json")).unwrap();
        assert_eq!(report.warning_count(), 0);
        assert_eq!(restored, captured);
        for id in [second, fourth] {
            workspace.source_mut(id).unwrap().schema = Some(test_schema(&["iu"]));
        }
        let map = BTreeMap::from([
            (sources[0].key.clone(), second),
            (sources[1].key.clone(), fourth),
        ]);
        let plan = build_multi_source_apply_plan(restored, &workspace, &map, report);
        assert_eq!(plan.rows[0].channels[0].key.source_id, fourth);
        assert_eq!(plan.rows[0].channels[1].key.source_id, second);
        assert_eq!(plan.report.warning_count(), 0);
    }
}
