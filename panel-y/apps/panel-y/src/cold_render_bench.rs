//! Full-range cold-render CPU measurements, separated from I/O and statistics.
//! In-process cache rebuilds are not independent process-start measurements.

use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

const REBUILD_TRIALS: usize = 5;
const WARM_FRAMES: usize = 10;
const ZOOM_FOLLOWUP_FRAMES: usize = 3;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceSpec {
    path: String,
    offset_seconds: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    sources: Vec<SourceSpec>,
    cursors: [f64; 2],
    #[serde(default = "default_viewport_height")]
    viewport_height: f32,
    // Accept the existing multi-source statistics configs. These counts control
    // that benchmark, not the fixed cold-render sequence below.
    trials: Option<usize>,
    memory_repeats: Option<usize>,
}

fn default_viewport_height() -> f32 {
    900.0
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        if self.sources.is_empty()
            || self
                .sources
                .iter()
                .any(|s| s.path.is_empty() || !s.offset_seconds.is_finite())
            || !self.cursors.iter().all(|x| x.is_finite())
            || self.cursors[0] >= self.cursors[1]
            || !self.viewport_height.is_finite()
            || !(900.0..=4096.0).contains(&self.viewport_height)
            || self.trials.is_some_and(|n| !(1..=100).contains(&n))
            || self
                .memory_repeats
                .is_some_and(|n| !(1..=1_000).contains(&n))
        {
            return Err("need sources with finite offsets, increasing finite cursors, finite viewport_height in 900..4096, and valid optional statistics benchmark counts".into());
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct FrameSample {
    total_ms: f64,
    ui_ms: f64,
    visible_traces_ms: f64,
    paint_traces_and_axes_ms: f64,
    tessellation_ms: f64,
    tile_build_ms: f64,
    tile_build_wall_ms: f64,
    tile_build_workers: usize,
    edge_build_ms: f64,
    edge_scan_ms: f64,
    ui_passes: usize,
    envelope_builds: usize,
    context_rebuilds: usize,
    tile_builds: usize,
    edge_builds: usize,
    edge_aborts: usize,
    shapes: usize,
    mesh_vertices: usize,
    prepared_rows: usize,
    prepared_channels: usize,
    draw_points: usize,
}

fn cache_build_seconds(app: &PanelYApp) -> (f64, f64) {
    app.workspace
        .sources
        .values()
        .fold((0.0, 0.0), |(tile, edge), source| {
            (
                tile + source.loaded_channels.line_tile_build_seconds(),
                edge + source.loaded_channels.step_edge_build_seconds(),
            )
        })
}

fn frame(app: &mut PanelYApp, ctx: &egui::Context, viewport_height: f32) -> FrameSample {
    let (tile_before, edge_before) = cache_build_seconds(app);
    let started = Instant::now();
    let mut visible_traces_ms = 0.0;
    let mut paint_traces_and_axes_ms = 0.0;
    let mut ui_passes = 0;
    let mut envelope_builds = 0;
    let mut context_rebuilds = 0;
    let mut tile_builds = 0;
    let mut tile_build_wall_ms = 0.0;
    let mut tile_build_workers = 0;
    let mut edge_builds = 0;
    let mut edge_scan_ms = 0.0;
    let mut edge_aborts = 0;
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, viewport_height),
            )),
            events: vec![egui::Event::PointerMoved(egui::pos2(650.0, 190.0))],
            ..Default::default()
        },
        |ui| {
            app.draw_ui(ui);
            // Include discarded sizing passes; otherwise the costly first pass
            // can disappear from the reported waveform preparation time.
            visible_traces_ms += app.perf.visible_ms;
            paint_traces_and_axes_ms += app.perf.draw_ms;
            ui_passes += 1;
            envelope_builds += app.perf.envelope_cache_misses;
            context_rebuilds += app.perf.envelope_context_misses;
            tile_builds += app.perf.line_tile_builds;
            edge_builds += app.perf.step_edge_builds;
            let envelope_stats = app.workspace.envelope_stats();
            tile_build_wall_ms += duration_ms(envelope_stats.tile_build_wall);
            tile_build_workers = tile_build_workers.max(envelope_stats.tile_build_workers);
            edge_scan_ms += duration_ms(envelope_stats.step_edge_scan_elapsed);
            edge_aborts += envelope_stats.step_edge_aborts;
        },
    );
    let ui_ms = duration_ms(started.elapsed());
    let shapes = output.shapes.len();
    let tessellation_started = Instant::now();
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let tessellation_ms = duration_ms(tessellation_started.elapsed());
    let mesh_vertices = primitives
        .iter()
        .map(|primitive| match &primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
            egui::epaint::Primitive::Callback(_) => 0,
        })
        .sum();
    drop(std::hint::black_box(primitives));
    let total_ms = duration_ms(started.elapsed());
    let (tile_after, edge_after) = cache_build_seconds(app);
    FrameSample {
        total_ms,
        ui_ms,
        visible_traces_ms,
        paint_traces_and_axes_ms,
        tessellation_ms,
        tile_build_ms: (tile_after - tile_before).max(0.0) * 1000.0,
        tile_build_wall_ms,
        tile_build_workers,
        edge_build_ms: (edge_after - edge_before).max(0.0) * 1000.0,
        edge_scan_ms,
        ui_passes,
        envelope_builds,
        context_rebuilds,
        tile_builds,
        edge_builds,
        edge_aborts,
        shapes,
        mesh_vertices,
        prepared_rows: app.perf.rows,
        prepared_channels: app.perf.channels,
        draw_points: app.perf.draw_points,
    }
}

fn summary(samples: &[f64]) -> serde_json::Value {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        sorted
            .get(((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1))
            .copied()
    };
    json!({
        "count": sorted.len(), "p50_ms": percentile(0.5),
        "p95_ms": percentile(0.95), "max_ms": sorted.last(),
    })
}

fn summarize_frames(frames: &[FrameSample]) -> serde_json::Value {
    let metric =
        |get: fn(&FrameSample) -> f64| summary(&frames.iter().map(get).collect::<Vec<_>>());
    json!({
        "total": metric(|f| f.total_ms),
        "ui": metric(|f| f.ui_ms),
        "visible_traces": metric(|f| f.visible_traces_ms),
        "paint_traces_and_axes": metric(|f| f.paint_traces_and_axes_ms),
        "tessellation": metric(|f| f.tessellation_ms),
        "tile_build": metric(|f| f.tile_build_ms),
        "tile_build_wall": metric(|f| f.tile_build_wall_ms),
        "max_tile_build_workers": frames.iter().map(|f| f.tile_build_workers).max(),
        "edge_build": metric(|f| f.edge_build_ms),
        "edge_scan": metric(|f| f.edge_scan_ms),
        "edge_aborts": frames.iter().map(|f| f.edge_aborts).sum::<usize>(),
        "frames": frames,
    })
}

fn clear_draw_caches(app: &mut PanelYApp) {
    for source in app.workspace.sources.values_mut() {
        source.loaded_channels.clear_draw_caches();
    }
}

fn cache_snapshot(app: &PanelYApp) -> serde_json::Value {
    let stats = app.workspace.cache_stats();
    json!({
        "raw_mib": bytes_to_mib(stats.total_raw_memory_bytes()),
        "envelopes": stats.envelope_count,
        "line_tiles": stats.line_tile_cache_count,
        "line_tile_mib": bytes_to_mib(stats.line_tile_memory_bytes),
        "step_edges": stats.step_edge_cache_count,
        "step_edge_mib": bytes_to_mib(stats.step_edge_memory_bytes),
        "step_fallback_hints": stats.step_fallback_hint_count,
    })
}

pub(super) fn run(config_path: String) -> Result<String, String> {
    let config: Config =
        serde_json::from_str(&std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    config.validate()?;
    let rss_before_load = process_rss_mib();
    let load_started = Instant::now();
    let mut app = PanelYApp::default();
    let mut sources = Vec::new();
    for (index, spec) in config.sources.iter().enumerate() {
        let id = if index == 0 {
            app.workspace.set_parquet_path(spec.path.clone());
            DEFAULT_SOURCE_ID
        } else {
            app.workspace.add_source(spec.path.clone())
        };
        let schema = parquet_schema::read_schema_summary(&spec.path)?;
        let time = parquet_waveform::read_time_column(&spec.path, &schema)?;
        let raw_range = time.time_range().ok_or("source has no time range")?;
        let transform = workspace_data::TimeTransform {
            offset_seconds: spec.offset_seconds,
        };
        let display_range = transform.display_range(raw_range);
        if config.cursors[0] < display_range.0 || config.cursors[1] > display_range.1 {
            return Err(format!(
                "cursors must lie inside source {} display range",
                index + 1
            ));
        }
        let mut indices = [0; 2];
        for (i, cursor) in config.cursors.iter().enumerate() {
            indices[i] = nearest_sample_index_in_range(
                &time.time,
                transform.to_raw(*cursor),
                time.sample_count(),
            )
            .ok_or_else(|| format!("cursor {} has no raw sample in source {}", i + 1, index + 1))?;
        }
        let mut channels = Vec::new();
        let mut store = ChannelStore::default();
        for channel in &schema.channels {
            let values = parquet_waveform::read_channel_values(&spec.path, &schema, &channel.path)?;
            if values.sample_count() != time.sample_count() {
                return Err(format!("time/value length mismatch in {}", spec.path));
            }
            channels.push(channel.path.clone());
            store.insert_channel(values);
        }
        if channels.is_empty() {
            return Err(format!("no channels in {}", spec.path));
        }
        sources.push(json!({
            "source_id": id.value(), "file": spec.path,
            "offset_seconds": spec.offset_seconds, "samples_per_channel": time.sample_count(),
            "channels": channels, "time_column": time.column_path,
            "raw_range": raw_range, "display_range": display_range,
            "interval_indices": indices,
            "raw_mib": bytes_to_mib(time.memory_bytes() + store.raw_memory_bytes()),
        }));
        let source = app.workspace.source_mut(id).expect("loaded source");
        source.schema = Some(schema);
        source.shared_time = Some(time);
        source.loaded_channels = store;
        source.time_transform = transform;
    }
    let load_seconds = load_started.elapsed().as_secs_f64();
    let rss_after_load = process_rss_mib();
    // Match the statistics benchmark: three logical channels per row, with
    // matching channel names from different sources overlaid in the same row.
    app.view.rows = multi_source_benchmark_rows(&app.workspace)
        .chunks(3)
        .enumerate()
        .map(|(i, group)| {
            let mut row = PlotRow::new(i as u64);
            row.channels = group
                .iter()
                .flat_map(|r| r.channels.iter().cloned())
                .collect();
            row
        })
        .collect();
    app.view.selected_row_id = app.view.rows.first().map(|row| row.id);
    app.view.next_row_id = app.view.rows.len() as u64;
    let range = app
        .workspace
        .display_time_range()
        .ok_or("no display range")?;
    app.view.x_range = Some(range);
    app.analysis.info_panel_height = 360.0;
    app.analysis.cursor_a = Some(config.cursors[0]);
    app.analysis.cursor_b = Some(config.cursors[1]);
    app.analysis.cursor_mode = true;
    let reference_started = Instant::now();
    let expected = compute_interval_channel_statistics(&app.analysis, &app.workspace, &app.view);
    let reference_ms = duration_ms(reference_started.elapsed());
    app.analysis.interval_stats_cache = IntervalStatsCache {
        key: interval_stats_cache_key(&app.analysis, &app.workspace, &app.view),
        stats: expected,
    };
    let cache_before_first = cache_snapshot(&app);
    let rss_before_first = process_rss_mib();
    eprintln!("{} source(s): first full-range cold frame", sources.len());
    let ctx = egui::Context::default();
    let first = frame(&mut app, &ctx, config.viewport_height);
    let rss_after_first = process_rss_mib();
    let cache_after_first = cache_snapshot(&app);
    let mut warm = Vec::new();
    for _ in 0..WARM_FRAMES {
        warm.push(frame(&mut app, &ctx, config.viewport_height));
    }
    let rss_after_warm = process_rss_mib();
    let mut cache_rebuild = Vec::new();
    let mut cache_rebuild_rss = Vec::new();
    for _ in 0..REBUILD_TRIALS {
        clear_draw_caches(&mut app);
        cache_rebuild.push(frame(&mut app, &ctx, config.viewport_height));
        cache_rebuild_rss.push(process_rss_mib());
    }
    // Context creation and cache clearing remain outside the frame timer. These
    // trials include egui's first layout/font use but retain resident raw data.
    let mut fresh_context = Vec::new();
    let mut fresh_context_rss = Vec::new();
    for _ in 0..REBUILD_TRIALS {
        clear_draw_caches(&mut app);
        let fresh_ctx = egui::Context::default();
        fresh_context.push(frame(&mut app, &fresh_ctx, config.viewport_height));
        fresh_context_rss.push(process_rss_mib());
    }
    let cache_after_rebuilds = cache_snapshot(&app);
    // Preserve the final full-range caches and fallback hints. Reuse the
    // original UI context to expose costs deferred until later zoom operations.
    let zoom_fractions = [0.6, 0.3, 0.1, 0.01, 1.0];
    let mut zoom_frames = Vec::new();
    let mut zoom_ranges = Vec::new();
    let mut zoom_rss = Vec::new();
    for fraction in zoom_fractions {
        let margin = (range.1 - range.0) * (1.0 - fraction) * 0.5;
        let zoom_range = (range.0 + margin, range.1 - margin);
        app.view.x_range = Some(zoom_range);
        zoom_ranges.push(zoom_range);
        zoom_frames.push(frame(&mut app, &ctx, config.viewport_height));
        for _ in 0..ZOOM_FOLLOWUP_FRAMES {
            zoom_frames.push(frame(&mut app, &ctx, config.viewport_height));
        }
        zoom_rss.push(process_rss_mib());
    }
    let zoom_sequence = zoom_frames
        .chunks_exact(1 + ZOOM_FOLLOWUP_FRAMES)
        .enumerate()
        .map(|(index, frames)| {
            json!({
                "range_fraction": zoom_fractions[index],
                "display_range": zoom_ranges[index],
                "operation_frame": frames[0],
                "followup_frames": summarize_frames(&frames[1..]),
            })
        })
        .collect::<Vec<_>>();
    let channel_count = app.workspace.cache_stats().loaded_channel_count;
    for sample in std::iter::once(&first)
        .chain(&warm)
        .chain(&cache_rebuild)
        .chain(&fresh_context)
        .chain(&zoom_frames)
    {
        if sample.prepared_channels != channel_count {
            return Err(format!(
                "trace preparation covered only {} of {channel_count} loaded channels",
                sample.prepared_channels
            ));
        }
    }
    if app.analysis.interval_stats_worker.has_running_job() {
        return Err("statistics unexpectedly ran during the cold-render benchmark".into());
    }
    let sampled_max_rss = [
        rss_before_load,
        rss_after_load,
        rss_before_first,
        rss_after_first,
        rss_after_warm,
    ]
    .into_iter()
    .chain(cache_rebuild_rss.iter().copied())
    .chain(fresh_context_rss.iter().copied())
    .chain(zoom_rss.iter().copied())
    .flatten()
    .reduce(f64::max);
    let report = json!({
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "config_file": config_path, "sources": sources,
        "source_count": config.sources.len(), "channel_count": channel_count,
        "cursors": config.cursors, "full_display_range": range,
        "viewport": [1280.0, config.viewport_height], "info_panel_height": 360.0,
        "emergency_preview": app.view.large_preview_enabled,
        "MallocLargeCache": std::env::var("MallocLargeCache").ok(),
        "load_seconds": load_seconds, "statistics_reference_ms": reference_ms,
        "first_cold_frame": first,
        "warm_full_range": summarize_frames(&warm),
        "cache_rebuild_same_context": summarize_frames(&cache_rebuild),
        "cache_rebuild_fresh_context": summarize_frames(&fresh_context),
        "zoom_sequence": zoom_sequence,
        "zoom_sequence_all_frames": summarize_frames(&zoom_frames),
        "cache_before_first": cache_before_first, "cache_after_first": cache_after_first,
        "cache_after_rebuilds": cache_after_rebuilds,
        "cache_after_trials": cache_snapshot(&app),
        "rss_mib": {
            "before_load": rss_before_load, "after_load": rss_after_load,
            "before_first_frame": rss_before_first, "after_first_frame": rss_after_first,
            "after_warm_frames": rss_after_warm,
            "after_same_context_rebuilds": cache_rebuild_rss,
            "after_fresh_context_rebuilds": fresh_context_rss,
            "after_zoom_stages": zoom_rss,
            "sampled_max": sampled_max_rss,
        },
        "scope": "CPU UI plus tessellation; all egui UI passes included. Cold and rebuild stages use the full display range. The later zoom sequence directly sets central 60%, 30%, 10%, 1%, then full ranges, measuring one operation frame and three followups per range without clearing caches or hints. It measures redraw costs, not OS zoom event delivery. Prepared row/channel counts verify trace preparation, not visibility inside the clipped viewport. The default 1280x900 geometry matches the prior statistics benchmark; three rows may extend below the clipped viewport. Viewport height is configurable. Visible/paint times are subsets of UI. tile_build_wall_ms sums batch wall time and lazy serial tile builds; it is a subset of visible preparation. tile_build_ms is the sum of individual retained cache construction durations, which overlap during parallel batches and must not be added to wall time. Tile worker counts include the calling thread. Edge construction is a subset of visible preparation. Edge build time counts only completed retained caches; edge scan time includes all attempts, including aborted dense scans, and overlaps edge build time. Raw loading and synchronous reference statistics occur before rendering and are excluded. First cold frame is one sample after loading; cache rebuild trials retain the same raw data in one process and are not independent process starts. No statistics worker runs. Excludes OS event delivery, GPU and presentation. RSS is sampled outside frame timers at phase boundaries, not continuous peak RSS."
    });
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}
