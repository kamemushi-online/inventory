//! Multi-source statistics scaling and a separate RSS/lifetime stress pass.

use crate::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::{
    Weak,
    atomic::{AtomicBool, Ordering},
};

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
    trials: usize,
    memory_repeats: usize,
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
            || !(1..=100).contains(&self.trials)
            || !(1..=1_000).contains(&self.memory_repeats)
        {
            return Err("need sources with finite offsets, increasing finite cursors, 1..100 trials and 1..1000 memory_repeats".into());
        }
        Ok(())
    }
}

fn interval_indices(
    time: &[f64],
    transform: workspace_data::TimeTransform,
    cursors: [f64; 2],
) -> Result<[usize; 2], String> {
    let mut indices = [0; 2];
    for (index, cursor) in cursors.into_iter().enumerate() {
        // A display-range check alone is insufficient: subtracting the offset
        // can round an endpoint just outside the raw time range.
        indices[index] = nearest_sample_index_in_range(time, transform.to_raw(cursor), time.len())
            .ok_or_else(|| {
                format!(
                    "cursor {} has no sample inside the raw time range",
                    index + 1
                )
            })?;
    }
    Ok(indices)
}

pub(super) fn run(config_path: String) -> Result<String, String> {
    let config: Config =
        serde_json::from_str(&std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    config.validate()?;
    let reloads = memory_reload_count()?;
    let diagnostics = memory_diagnostics_enabled();
    let memory_before_load = diagnostics.then(memory_snapshot);
    let rss_before_load = process_rss_mib();
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
        let indices = interval_indices(&time.time, transform, config.cursors)
            .map_err(|error| format!("source {}: {error}", index + 1))?;
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
            "source_id": id.value(), "file": spec.path, "offset_seconds": spec.offset_seconds,
            "samples_per_channel": time.sample_count(), "channels": channels,
            "time_column": time.column_path, "raw_range": raw_range, "display_range": display_range,
            "interval_indices": indices, "interval_samples_per_channel": indices[1] - indices[0] + 1,
            "raw_mib": bytes_to_mib(time.memory_bytes() + store.raw_memory_bytes()),
        }));
        let source = app.workspace.source_mut(id).expect("loaded source");
        source.schema = Some(schema);
        source.shared_time = Some(time);
        source.loaded_channels = store;
        source.time_transform = transform;
    }
    // Three logical channels per row; sources overlay within each group. Row count
    // and panel geometry stay constant as the number of sources increases.
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
    let cursors = (config.cursors[0], config.cursors[1]);
    let memory_after_load = diagnostics.then(memory_snapshot);
    eprintln!(
        "{} source(s): measuring {} UI trials",
        sources.len(),
        config.trials
    );
    let mut report = interval_stats_bench::measure(&mut app, range, cursors, config.trials)?;
    let stats = report
        .as_object_mut()
        .unwrap()
        .remove("full_range_statistics")
        .unwrap();
    report["interval_statistics"] = stats;
    report["sources"] = json!(sources);
    report["source_count"] = json!(config.sources.len());
    report["cursors"] = json!(config.cursors);
    report["rss_before_load_mib"] = json!(rss_before_load);
    report["config_file"] = json!(config_path);
    if diagnostics {
        report["memory_diagnostics_environment"] = json!({
            "MallocLargeCache": std::env::var("MallocLargeCache").ok(),
            "PANEL_Y_MEMORY_RELOADS": reloads,
        });
        report["memory_diagnostics_before_load"] = json!(memory_before_load);
        report["memory_diagnostics_after_load"] = json!(memory_after_load);
        report["memory_diagnostics_after_ui"] = memory_snapshot();
    }
    app.analysis.cursor_a = Some(cursors.0);
    app.analysis.cursor_b = Some(cursors.1);
    app.analysis.invalidate_interval_statistics();
    eprintln!(
        "{} source(s): separate {}-repeat memory pass and source removal",
        config.sources.len(),
        config.memory_repeats
    );
    report["memory"] = memory_pass(&mut app, config.memory_repeats)?;
    if reloads > 0 {
        eprintln!("{} raw-only reload/drop diagnostics", reloads);
        report["memory_reload_probe"] = memory_reload_probe(&config, reloads)?;
    }
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn memory_reload_count() -> Result<usize, String> {
    match std::env::var("PANEL_Y_MEMORY_RELOADS") {
        Err(std::env::VarError::NotPresent) => Ok(0),
        value => value
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|count| *count <= 3)
            .ok_or_else(|| "PANEL_Y_MEMORY_RELOADS must be 0..3".into()),
    }
}

fn memory_reload_probe(config: &Config, repeats: usize) -> Result<serde_json::Value, String> {
    let before = memory_snapshot();
    let mut cycles = Vec::new();
    for cycle in 0..repeats {
        let started = Instant::now();
        let mut raw = Vec::new();
        let mut buffers = Vec::new();
        let mut raw_bytes = 0;
        for (index, spec) in config.sources.iter().enumerate() {
            let schema = parquet_schema::read_schema_summary(&spec.path)?;
            let time = parquet_waveform::read_time_column(&spec.path, &schema)?;
            let mut channels = Vec::new();
            for channel in &schema.channels {
                channels.push(parquet_waveform::read_channel_values(
                    &spec.path,
                    &schema,
                    &channel.path,
                )?);
            }
            raw_bytes += time.memory_bytes();
            // Read every 4 KiB of the loaded buffers to include resident pages,
            // even when the OS's physical page size is larger than 4 KiB.
            for value in time.time.iter().step_by(4096 / std::mem::size_of::<f64>()) {
                std::hint::black_box(*value);
            }
            for channel in &channels {
                raw_bytes += channel.memory_bytes();
                for value in channel
                    .values
                    .iter()
                    .step_by(4096 / std::mem::size_of::<f32>())
                {
                    std::hint::black_box(*value);
                }
            }
            buffers.push(BufferReferences {
                id: SourceId::new(index as u64 + 1),
                time: Arc::downgrade(&time.time),
                values: channels
                    .iter()
                    .map(|channel| Arc::downgrade(&channel.values))
                    .collect(),
            });
            raw.push((time, channels));
        }
        let load_ms = started.elapsed().as_secs_f64() * 1000.0;
        let loaded = memory_snapshot();
        let started = Instant::now();
        drop(raw);
        let drop_ms = started.elapsed().as_secs_f64() * 1000.0;
        if !buffers.iter().all(BufferReferences::released) {
            return Err("raw reload probe retained buffers after drop".into());
        }
        cycles.push(json!({
            "cycle": cycle + 1, "raw_mib": bytes_to_mib(raw_bytes),
            "load_ms": load_ms, "drop_ms": drop_ms,
            "loaded": loaded, "after_drop": memory_snapshot(),
            "all_buffers_released": true,
        }));
    }
    Ok(json!({
        "before": before, "cycles": cycles,
        "scope": "Same-process raw-only reload/drop of the configured Parquet sources after all UI sources were removed; every 4 KiB touched. Excludes drawing, UI reload, statistics jobs and timed frame/RSS passes."
    }))
}

fn memory_diagnostics_enabled() -> bool {
    std::env::var_os("PANEL_Y_MEMORY_DIAGNOSTICS").is_some_and(|value| value == "1")
}

// Optional investigation only: none of these APIs run in the normal viewer or
// the timed UI pass. The OS reports resident, reusable and footprint separately.
#[cfg(target_os = "macos")]
mod macos_memory {
    use super::*;
    use std::ffi::{c_int, c_uint, c_void};

    // Layouts from the public macOS SDK malloc/malloc.h and mach/task_info.h.
    // TASK_VM_INFO_REV1_COUNT is 38 32-bit natural_t words (152 bytes); requesting
    // this stable prefix avoids depending on newer task_vm_info revisions.
    #[repr(C)]
    #[derive(Default)]
    struct MallocStatistics {
        blocks_in_use: c_uint,
        size_in_use: usize,
        max_size_in_use: usize,
        size_allocated: usize,
    }

    #[repr(C)]
    #[derive(Default)]
    struct TaskVmInfoV1 {
        virtual_size: u64,
        region_count: c_int,
        page_size: c_int,
        resident_size: u64,
        resident_size_peak: u64,
        device: u64,
        device_peak: u64,
        internal: u64,
        internal_peak: u64,
        external: u64,
        external_peak: u64,
        reusable: u64,
        reusable_peak: u64,
        purgeable_volatile_pmap: u64,
        purgeable_volatile_resident: u64,
        purgeable_volatile_virtual: u64,
        compressed: u64,
        compressed_peak: u64,
        compressed_lifetime: u64,
        phys_footprint: u64,
    }

    const _: () = assert!(std::mem::size_of::<MallocStatistics>() == 32);
    const _: () = assert!(std::mem::size_of::<TaskVmInfoV1>() == 152);
    const _: () = assert!(std::mem::offset_of!(TaskVmInfoV1, reusable) == 80);
    const _: () = assert!(std::mem::offset_of!(TaskVmInfoV1, phys_footprint) == 144);

    unsafe extern "C" {
        static mach_task_self_: c_uint;
        fn task_info(task: c_uint, flavor: c_int, info: *mut c_int, count: *mut c_uint) -> c_int;
        fn malloc_zone_statistics(zone: *mut c_void, stats: *mut MallocStatistics);
        fn malloc_zone_pressure_relief(zone: *mut c_void, goal: usize) -> usize;
    }

    pub(super) fn snapshot() -> serde_json::Value {
        let mut malloc = MallocStatistics::default();
        let mut vm = TaskVmInfoV1::default();
        let mut count = 38;
        // SAFETY: null selects all malloc zones; stats points to a correctly
        // sized writable C struct. task_info writes at most count 32-bit words
        // into the validated 152-byte prefix for this process's own task port.
        let status = unsafe {
            malloc_zone_statistics(std::ptr::null_mut(), &mut malloc);
            task_info(
                mach_task_self_,
                22, // TASK_VM_INFO
                (&mut vm as *mut TaskVmInfoV1).cast(),
                &mut count,
            )
        };
        let vm_available = status == 0 && count >= 38;
        let mib = |bytes: u64| vm_available.then_some(bytes as f64 / 1_048_576.0);
        json!({
            "platform": "macos", "rss_ps_mib": process_rss_mib(),
            "malloc_blocks_in_use": malloc.blocks_in_use,
            "malloc_in_use_mib": bytes_to_mib(malloc.size_in_use),
            "malloc_reserved_mib": bytes_to_mib(malloc.size_allocated),
            "mach_resident_mib": mib(vm.resident_size),
            "phys_footprint_mib": mib(vm.phys_footprint),
            "reusable_mib": mib(vm.reusable), "internal_mib": mib(vm.internal),
            "compressed_mib": mib(vm.compressed), "virtual_mib": mib(vm.virtual_size),
            "task_info_status": status, "task_info_words": count,
        })
    }

    pub(super) fn pressure_relief() -> usize {
        // SAFETY: the public malloc API accepts null for all zones and zero for
        // maximal best-effort relief. This is an explicit benchmark diagnostic.
        unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) }
    }
}

fn memory_snapshot() -> serde_json::Value {
    #[cfg(target_os = "macos")]
    return macos_memory::snapshot();
    #[cfg(not(target_os = "macos"))]
    json!({"platform": "unsupported", "rss_ps_mib": process_rss_mib()})
}

fn memory_pressure_relief() -> Option<usize> {
    #[cfg(target_os = "macos")]
    return Some(macos_memory::pressure_relief());
    #[cfg(not(target_os = "macos"))]
    None
}

fn finish(app: &mut PanelYApp, ctx: &egui::Context) -> Result<(), String> {
    let key = interval_stats_cache_key(&app.analysis, &app.workspace, &app.view)
        .ok_or("no statistics request")?;
    let started = Instant::now();
    while app.analysis.interval_stats_cache.key.as_ref() != Some(&key) {
        app.analysis
            .interval_channel_statistics(&app.workspace, &app.view, ctx);
        if let Some(error) = &app.analysis.interval_stats_worker.error {
            return Err(error.clone());
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err("memory pass statistics timed out".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn drain(app: &mut PanelYApp, ctx: &egui::Context) -> Result<(), String> {
    let started = Instant::now();
    loop {
        app.analysis.interval_stats_worker.update(
            None,
            &app.workspace,
            ctx,
            &mut app.analysis.interval_stats_cache,
        );
        if !app.analysis.interval_stats_worker.has_running_job() {
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(5) {
            return Err("cancelled worker failed to drain".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

struct BufferReferences {
    id: SourceId,
    time: Weak<Vec<f64>>,
    values: Vec<Weak<Vec<f32>>>,
}

impl BufferReferences {
    fn released(&self) -> bool {
        self.time.strong_count() == 0 && self.values.iter().all(|v| v.strong_count() == 0)
    }
}

// Polling ps is deliberately confined to the separate memory pass so subprocess
// creation and RSS sampling cannot contaminate the reported UI frame timings.
struct RssSampler {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<Vec<serde_json::Value>>>,
}

impl RssSampler {
    fn start() -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::Builder::new().name("statistics-rss-sampler".into()).spawn(move || {
            let start = Instant::now();
            let mut samples = Vec::new();
            while !flag.load(Ordering::Relaxed) {
                if let Some(rss) = process_rss_mib() {
                    samples.push(json!({"elapsed_ms": start.elapsed().as_secs_f64() * 1000.0, "rss_mib": rss}));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            samples
        }).map_err(|e| e.to_string())?;
        Ok(Self {
            stop,
            handle: Some(handle),
        })
    }

    fn finish(mut self) -> Result<Vec<serde_json::Value>, String> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .take()
            .unwrap()
            .join()
            .map_err(|_| "RSS sampler panicked".into())
    }
}

impl Drop for RssSampler {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn memory_pass(app: &mut PanelYApp, repeats: usize) -> Result<serde_json::Value, String> {
    let ctx = egui::Context::default();
    let diagnostics = memory_diagnostics_enabled();
    let mut diagnostic = json!({});
    if diagnostics {
        diagnostic["before_repeats"] = memory_snapshot();
    }
    let cursors = (app.analysis.cursor_a, app.analysis.cursor_b);
    let buffers: Vec<_> = app
        .workspace
        .source_order
        .iter()
        .map(|id| {
            let source = app.workspace.source(*id).unwrap();
            BufferReferences {
                id: *id,
                time: Arc::downgrade(&source.shared_time.as_ref().unwrap().time),
                values: source
                    .loaded_channels
                    .raw_by_channel
                    .values()
                    .map(|channel| Arc::downgrade(&channel.values))
                    .collect(),
            }
        })
        .collect();
    let expected = compute_interval_channel_statistics(&app.analysis, &app.workspace, &app.view);
    let before = process_rss_mib();
    let raw_before = app.workspace.cache_stats().total_raw_memory_bytes();
    let sampler = RssSampler::start()?;
    let mut after_each = Vec::new();
    for trial in 0..repeats {
        app.analysis.invalidate_interval_statistics();
        finish(app, &ctx)?;
        if app.analysis.interval_stats_cache.stats != expected {
            return Err(format!(
                "memory pass trial {} differs from reference",
                trial + 1
            ));
        }
        after_each.push(process_rss_mib());
    }
    let samples = sampler.finish()?;
    let peak = samples
        .iter()
        .filter_map(|s| s["rss_mib"].as_f64())
        .chain(after_each.iter().copied().flatten())
        .chain(before)
        .max_by(f64::total_cmp);
    let after_repeats = process_rss_mib();
    if diagnostics {
        diagnostic["after_repeats"] = memory_snapshot();
    }

    // Exercise the application's real cleanup path while a job still owns snapshots.
    app.analysis.invalidate_interval_statistics();
    app.analysis
        .interval_channel_statistics(&app.workspace, &app.view, &ctx);
    std::thread::sleep(Duration::from_millis(5));
    let removed = buffers.last().ok_or("no source to remove")?;
    let remove_started = Instant::now();
    app.remove_source_from_ui(removed.id);
    let remove_ui_ms = remove_started.elapsed().as_secs_f64() * 1000.0;
    drain(app, &ctx)?;
    let remove_to_drained_ms = remove_started.elapsed().as_secs_f64() * 1000.0;
    if !removed.released() {
        return Err("removed source raw buffers still have strong references after drain".into());
    }
    let after_remove = process_rss_mib();
    if diagnostics {
        diagnostic["after_one_removed"] = memory_snapshot();
    }
    let raw_after_remove = app.workspace.cache_stats().total_raw_memory_bytes();
    let mut remaining_count = 0;
    if !app.workspace.source_order.is_empty() {
        app.analysis.cursor_a = cursors.0;
        app.analysis.cursor_b = cursors.1;
        finish(app, &ctx)?;
        let reference =
            compute_interval_channel_statistics(&app.analysis, &app.workspace, &app.view);
        if app.analysis.interval_stats_cache.stats != reference {
            return Err("remaining source statistics differ after source removal".into());
        }
        remaining_count = reference.len();
    }
    for id in app.workspace.source_order.clone() {
        app.remove_source_from_ui(id);
    }
    drain(app, &ctx)?;
    if !buffers.iter().all(BufferReferences::released) {
        return Err("raw buffers retained after all sources removed".into());
    }
    let after_all_removed = process_rss_mib();
    if diagnostics {
        diagnostic["after_all_removed"] = memory_snapshot();
    }
    let settled_started = Instant::now();
    let mut settled_rss = Vec::new();
    for delay in [100, 400, 1_500] {
        std::thread::sleep(Duration::from_millis(delay));
        settled_rss.push(json!({
            "elapsed_ms": settled_started.elapsed().as_secs_f64() * 1000.0,
            "rss_mib": process_rss_mib(),
        }));
    }
    if diagnostics {
        diagnostic["after_settled"] = memory_snapshot();
        let started = Instant::now();
        let relieved = memory_pressure_relief();
        diagnostic["pressure_relief_bytes"] = json!(relieved);
        diagnostic["pressure_relief_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
        diagnostic["after_pressure_relief"] = memory_snapshot();
    }
    let mut report = json!({
        "repeats": repeats, "rss_before_mib": before, "rss_after_each_trial_mib": after_each,
        "rss_after_repeats_mib": after_repeats, "sampled_peak_rss_mib": peak,
        "rss_samples": samples, "sample_sleep_ms": 50,
        "raw_before_mib": bytes_to_mib(raw_before),
        "remove_source_id": removed.id.value(), "remove_ui_ms": remove_ui_ms,
        "remove_to_drained_ms": remove_to_drained_ms,
        "rss_after_remove_mib": after_remove, "raw_after_remove_mib": bytes_to_mib(raw_after_remove),
        "removed_source_buffers_released": true, "remaining_statistics_count": remaining_count,
        "rss_after_all_removed_mib": after_all_removed,
        "rss_after_all_removed_settled": settled_rss,
        "all_source_buffers_released": true,
        "raw_after_all_removed_mib": bytes_to_mib(app.workspace.cache_stats().total_raw_memory_bytes()),
        "numerical_reference_match": true,
        "scope": "Separate statistics-only pass without UI rendering; RSS sampled via ps with 50ms sleeps plus process-launch latency. Peak is sampled, not an OS high-water mark; allocator retention can keep RSS above logical raw bytes after release."
    });
    if diagnostics {
        report["diagnostics"] = diagnostic;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_indices_reports_offset_rounding_outside_raw_range() {
        let transform = workspace_data::TimeTransform {
            offset_seconds: 0.4,
        };
        assert_eq!(transform.display_range((0.1, 0.6)), (0.5, 1.0));
        assert!(transform.to_raw(0.5) < 0.1);
        assert!(interval_indices(&[0.1, 0.6], transform, [0.5, 1.0]).is_err());
        assert_eq!(
            interval_indices(&[0.1, 0.3, 0.6], transform, [0.7, 1.0]).unwrap(),
            [1, 2]
        );
    }

    #[test]
    fn memory_pass_releases_multiple_sources_and_recomputes_survivors() {
        let mut app = PanelYApp::default();
        for index in 0..2 {
            let id = if index == 0 {
                DEFAULT_SOURCE_ID
            } else {
                app.workspace.add_source("second.parquet".into())
            };
            let source = app.workspace.source_mut(id).unwrap();
            source.shared_time = Some(parquet_waveform::TimeData {
                path: "test.parquet".into(),
                column_name: "time".into(),
                column_path: "time".into(),
                time: Arc::new(vec![0.0, 0.5, 1.0]),
                projected_column_index: 0,
                elapsed: Duration::ZERO,
            });
            source
                .loaded_channels
                .insert_channel(parquet_waveform::ChannelData {
                    path: "test.parquet".into(),
                    channel_name: "signal".into(),
                    channel_path: "signal".into(),
                    values: Arc::new(vec![1.0 + index as f32; 3]),
                    projected_column_index: 1,
                    elapsed: Duration::ZERO,
                });
            app.view
                .add_channel_to_selected_row(&ChannelKey::new(id, "signal"));
        }
        app.analysis.cursor_a = Some(0.0);
        app.analysis.cursor_b = Some(1.0);
        let result = memory_pass(&mut app, 3).unwrap();
        assert_eq!(result["remaining_statistics_count"], 1);
        assert_eq!(result["all_source_buffers_released"], true);
        assert_eq!(result["raw_after_all_removed_mib"], 0.0);
        assert!(app.workspace.source_order.is_empty());
    }
}
