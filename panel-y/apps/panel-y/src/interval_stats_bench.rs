//! Reproducible raw-data statistics and full egui CPU-frame responsiveness measurement.
//! Includes layout, input processing, waveform/analysis painting, and tessellation; no OS/GPU.

use crate::*;
use serde_json::json;

struct FrameSample {
    total_ms: f64,
    ui_ms: f64,
    visible_ms: f64,
    draw_ms: f64,
    tessellation_ms: f64,
    shapes: usize,
    vertices: usize,
    ui_passes: usize,
    envelope_misses: usize,
    context_misses: usize,
    tile_builds: usize,
    edge_builds: usize,
}

fn frame(app: &mut PanelYApp, ctx: &egui::Context, index: usize, interact: bool) -> FrameSample {
    let mut events = vec![egui::Event::PointerMoved(egui::pos2(
        650.0 + (index % 100) as f32,
        190.0,
    ))];
    if interact && index % 8 == 0 {
        events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(if index % 16 == 0 { 8.0 } else { -8.0 }, 0.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
    }
    if interact && index % 8 == 4 {
        events.push(egui::Event::Zoom(if index % 16 == 4 {
            1.01
        } else {
            1.0 / 1.01
        }));
    }
    let started = Instant::now();
    let mut visible_ms = 0.0;
    let mut draw_ms = 0.0;
    let mut ui_passes = 0;
    let mut envelope_misses = 0;
    let mut context_misses = 0;
    let mut tile_builds = 0;
    let mut edge_builds = 0;
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            app.draw_ui(ui);
            // egui can discard a sizing pass and run the UI again. Include all
            // passes, especially the first pass that builds cold drawing caches.
            visible_ms += app.perf.visible_ms;
            draw_ms += app.perf.draw_ms;
            ui_passes += 1;
            envelope_misses += app.perf.envelope_cache_misses;
            context_misses += app.perf.envelope_context_misses;
            tile_builds += app.perf.line_tile_builds;
            edge_builds += app.perf.step_edge_builds;
        },
    );
    let ui_ms = duration_ms(started.elapsed());
    let shapes = output.shapes.len();
    let tessellation_started = Instant::now();
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let tessellation_ms = duration_ms(tessellation_started.elapsed());
    let vertices = primitives
        .iter()
        .map(|primitive| match &primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
            egui::epaint::Primitive::Callback(_) => 0,
        })
        .sum();
    drop(std::hint::black_box(primitives));
    FrameSample {
        total_ms: duration_ms(started.elapsed()),
        ui_ms,
        visible_ms,
        draw_ms,
        tessellation_ms,
        shapes,
        vertices,
        ui_passes,
        envelope_misses,
        context_misses,
        tile_builds,
        edge_builds,
    }
}

fn frame_breakdown(frames: &[FrameSample]) -> serde_json::Value {
    let metric =
        |get: fn(&FrameSample) -> f64| summary(&frames.iter().map(get).collect::<Vec<_>>());
    json!({
        "ui": metric(|f| f.ui_ms),
        "visible_traces": metric(|f| f.visible_ms),
        "paint_traces_and_axes": metric(|f| f.draw_ms),
        "tessellation": metric(|f| f.tessellation_ms),
        "envelope_miss_frames": frames.iter().filter(|f| f.envelope_misses > 0).count(),
        "envelope_builds": frames.iter().map(|f| f.envelope_misses).sum::<usize>(),
        "context_rebuilds": frames.iter().map(|f| f.context_misses).sum::<usize>(),
        "tile_builds": frames.iter().map(|f| f.tile_builds).sum::<usize>(),
        "edge_builds": frames.iter().map(|f| f.edge_builds).sum::<usize>(),
        "max_shapes": frames.iter().map(|f| f.shapes).max(),
        "max_ui_passes": frames.iter().map(|f| f.ui_passes).max(),
        "max_mesh_vertices": frames.iter().map(|f| f.vertices).max(),
        "scope": "visible_traces and paint_traces_and_axes are subsets of ui; tessellation is separate. CPU timings exclude OS/GPU."
    })
}

fn frame_totals(frames: &[FrameSample]) -> serde_json::Value {
    summary(&frames.iter().map(|f| f.total_ms).collect::<Vec<_>>())
}

fn summary(samples: &[f64]) -> serde_json::Value {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        sorted
            .get(((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1))
            .copied()
    };
    json!({ "count": sorted.len(), "p50_ms": percentile(0.5), "p95_ms": percentile(0.95), "max_ms": sorted.last() })
}

pub(crate) fn run(path: String, channels: Vec<String>) -> Result<String, String> {
    let schema = parquet_schema::read_schema_summary(&path)?;
    let selected: Vec<String> = if channels.is_empty() {
        schema.channels.iter().map(|c| c.path.clone()).collect()
    } else {
        channels
    };
    if selected.is_empty() {
        return Err("no channels selected".into());
    }
    let time = parquet_waveform::read_time_column(&path, &schema)?;
    let range = time.time_range().ok_or("no time range")?;
    let samples = time.sample_count();
    let mut app = PanelYApp::default();
    app.workspace.set_parquet_path(path.clone());
    for channel in &selected {
        app.workspace
            .loaded_channels
            .insert_channel(parquet_waveform::read_channel_values(
                &path, &schema, channel,
            )?);
    }
    app.workspace.schema = Some(schema);
    app.workspace.shared_time = Some(time);
    app.view.rows = phase2_benchmark_rows(&selected);
    app.view.next_row_id = app.view.rows.len() as u64;
    let mut report = measure(&mut app, range, range, 5)?;
    report["file"] = json!(path);
    report["samples_per_channel"] = json!(samples);
    report["channels"] = json!(selected);
    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
}

pub(super) fn measure(
    app: &mut PanelYApp,
    range: (f64, f64),
    cursors: (f64, f64),
    trials: usize,
) -> Result<serde_json::Value, String> {
    app.view.x_range = Some(range);
    app.analysis.info_panel_height = 360.0;
    app.analysis.cursor_a = Some(cursors.0);
    app.analysis.cursor_b = Some(cursors.1);
    app.analysis.cursor_mode = true;
    let rss_after_load = process_rss_mib();
    let raw_bytes = app.workspace.cache_stats().total_raw_memory_bytes();

    let sync_started = Instant::now();
    let expected = compute_interval_channel_statistics(&app.analysis, &app.workspace, &app.view);
    let synchronous_ms = sync_started.elapsed().as_secs_f64() * 1000.0;
    // Keep the same cursors, analysis-panel height and waveform area for idle/active frames.
    // Seed the completed reference so warmup and idle frames do not launch a statistics job.
    app.analysis.interval_stats_cache = IntervalStatsCache {
        key: interval_stats_cache_key(&app.analysis, &app.workspace, &app.view),
        stats: expected.clone(),
    };

    let ctx = egui::Context::default();
    let cold_frame = frame(app, &ctx, 0, false);
    // Warm drawing caches before measuring statistics. Cold drawing remains a separate cost.
    app.view.x_range = Some((
        range.0 + (range.1 - range.0) * 0.2,
        range.0 + (range.1 - range.0) * 0.8,
    ));
    for i in 0..40 {
        frame(app, &ctx, i, true);
    }
    let mut idle_frames = Vec::new();
    for i in 0..60 {
        idle_frames.push(frame(app, &ctx, i, true));
    }
    let rss_after_warmup = process_rss_mib();
    let mut dispatch = Vec::new();
    let mut active_frames = Vec::new();
    let mut compute_times = Vec::new();
    let mut wall_times = Vec::new();
    let mut view_changes = 0;
    let mut zoom_changes = 0;
    let mut pan_changes = 0;
    for trial in 0..trials {
        app.analysis.invalidate_interval_statistics();
        let started = Instant::now();
        let request = Instant::now();
        app.analysis
            .interval_channel_statistics(&app.workspace, &app.view, &ctx);
        dispatch.push(request.elapsed().as_secs_f64() * 1000.0);
        let mut i = 0;
        while app.analysis.interval_stats_cache.key.is_none() {
            if started.elapsed() > Duration::from_secs(30) {
                return Err("statistics benchmark timed out".into());
            }
            if let Some(error) = &app.analysis.interval_stats_worker.error {
                return Err(error.clone());
            }
            let before = app.view.x_range;
            active_frames.push(frame(app, &ctx, i + trial * 100, true));
            view_changes += usize::from(before != app.view.x_range);
            if let Some((before, after)) = before.zip(app.view.x_range) {
                if ((before.1 - before.0) - (after.1 - after.0)).abs() > 1.0e-9 {
                    zoom_changes += 1;
                } else if before != after {
                    pan_changes += 1;
                }
            }
            i += 1;
            std::thread::sleep(Duration::from_millis(1));
        }
        if app.analysis.interval_stats_cache.stats != expected {
            return Err("asynchronous statistics differ from synchronous reference".into());
        }
        compute_times.push(
            app.analysis
                .interval_stats_worker
                .elapsed
                .unwrap()
                .as_secs_f64()
                * 1000.0,
        );
        wall_times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let rss_after_trials = process_rss_mib();
    if view_changes == 0 || pan_changes == 0 || zoom_changes == 0 || app.view.hover_x.is_none() {
        return Err(
            "benchmark did not exercise hover, pan and zoom while awaiting statistics".into(),
        );
    }

    // Cancel a real large scan, then drain it without launching a replacement.
    app.analysis.invalidate_interval_statistics();
    app.analysis
        .interval_channel_statistics(&app.workspace, &app.view, &ctx);
    std::thread::sleep(Duration::from_millis(5));
    let cancel_started = Instant::now();
    app.analysis.interval_stats_worker.cancel();
    while app.analysis.interval_stats_worker.has_running_job() {
        app.analysis
            .interval_channel_statistics(&app.workspace, &app.view, &ctx);
        if cancel_started.elapsed() > Duration::from_secs(5) {
            return Err("cancellation timed out".into());
        }
        std::thread::yield_now();
    }
    let cancel_ms = cancel_started.elapsed().as_secs_f64() * 1000.0;
    if app.analysis.interval_stats_cache.key.is_some() {
        return Err("cancelled result was applied".into());
    }

    // Burst changes must retain only the latest request and its correct numerical result.
    let mut changes = Vec::new();
    for i in 0..100 {
        app.analysis.cursor_a = Some(cursors.0 + (cursors.1 - cursors.0) * i as f64 / 200.0);
        let started = Instant::now();
        app.analysis
            .interval_channel_statistics(&app.workspace, &app.view, &ctx);
        changes.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let latest = interval_stats_cache_key(&app.analysis, &app.workspace, &app.view);
    let started = Instant::now();
    while app.analysis.interval_stats_cache.key != latest {
        app.analysis
            .interval_channel_statistics(&app.workspace, &app.view, &ctx);
        if let Some(error) = &app.analysis.interval_stats_worker.error {
            return Err(error.clone());
        }
        if started.elapsed() > Duration::from_secs(30) {
            return Err("latest request timed out".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if app.analysis.interval_stats_cache.stats
        != compute_interval_channel_statistics(&app.analysis, &app.workspace, &app.view)
    {
        return Err("latest request statistics differ from reference".into());
    }
    let report = json!({
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "raw_mib": bytes_to_mib(raw_bytes), "rss_after_load_mib": rss_after_load,
        "rss_after_warmup_mib": rss_after_warmup,
        "rss_after_trials_mib": rss_after_trials, "cold_full_ui_frame_ms": cold_frame.total_ms,
        "cold_frame_breakdown": frame_breakdown(std::slice::from_ref(&cold_frame)),
        "synchronous_reference_ms": synchronous_ms, "dispatch": summary(&dispatch),
        "worker_compute": summary(&compute_times), "completion_wall": summary(&wall_times),
        "idle_ui_frames": frame_totals(&idle_frames), "active_ui_frames": frame_totals(&active_frames),
        "idle_frame_breakdown": frame_breakdown(&idle_frames),
        "active_frame_breakdown": frame_breakdown(&active_frames),
        "view_changes_while_pending": view_changes, "pan_changes_while_pending": pan_changes,
        "zoom_changes_while_pending": zoom_changes, "cancel_to_exit_ms": cancel_ms,
        "rapid_100_requests": summary(&changes), "numerical_reference_match": true,
        "full_range_statistics": expected.iter().map(|s| json!({
            "source_id": s.key.source_id.value(),
            "channel": s.channel_path, "mean": s.mean, "min": s.min, "max": s.max,
            "peak_to_peak": s.peak_to_peak, "rms": s.rms, "a": s.value_a, "b": s.value_b,
            "delta": s.delta, "slope_per_second": s.slope_per_second,
        })).collect::<Vec<_>>(),
        "scope": "Full egui CPU UI plus tessellation with hover/pan/zoom events; warmed draw caches; identical cursors and panel dimensions for idle/active frames, idle uses completed cached statistics; excludes native OS event delivery, GPU and presentation."
    });
    Ok(report)
}
