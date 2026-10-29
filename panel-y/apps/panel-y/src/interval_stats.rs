//! One cancellable statistics job at a time. The UI never scans or copies raw samples.

use crate::{
    IntervalChannelStats, IntervalStatsCache, IntervalStatsCacheKey, WorkspaceDataState,
    channel_label_or_fallback, channel_tooltip_or_fallback,
    interval_channel_statistics_for_channel, parquet_waveform,
};
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

pub(crate) const CANCELLATION_CHUNK_SAMPLES: usize = 16_384;

#[derive(Debug, Default)]
pub(crate) struct IntervalStatsWorker {
    running: Option<RunningJob>,
    requested_key: Option<IntervalStatsCacheKey>,
    blocked_key: Option<IntervalStatsCacheKey>,
    next_id: u64,
    pub(crate) error: Option<String>,
    pub(crate) elapsed: Option<Duration>,
}

#[derive(Debug)]
struct RunningJob {
    id: u64,
    key: IntervalStatsCacheKey,
    cancelled: Arc<AtomicBool>,
    receiver: mpsc::Receiver<JobResult>,
}

struct JobInput {
    key: IntervalStatsCacheKey,
    channels: Vec<ChannelInput>,
}

struct ChannelInput {
    channel: parquet_waveform::ChannelData,
    label: String,
    tooltip: String,
}

struct JobResult {
    id: u64,
    key: IntervalStatsCacheKey,
    stats: Option<Vec<IntervalChannelStats>>,
    elapsed: Duration,
}

impl IntervalStatsWorker {
    pub(crate) fn invalidate(&mut self) {
        if let Some(job) = &self.running {
            job.cancelled.store(true, Ordering::Relaxed);
        }
        self.requested_key = None;
        self.blocked_key = None;
        self.error = None;
        self.elapsed = None;
    }

    pub(crate) fn cancel(&mut self) {
        if let Some(job) = &self.running {
            job.cancelled.store(true, Ordering::Relaxed);
        }
        self.blocked_key = self.requested_key.clone();
    }

    pub(crate) fn retry(&mut self) {
        self.blocked_key = None;
        self.error = None;
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.blocked_key.is_some() && self.error.is_none()
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.requested_key.is_some() && self.running.is_some() && self.blocked_key.is_none()
    }

    pub(crate) fn has_running_job(&self) -> bool {
        self.running.is_some()
    }

    pub(crate) fn update(
        &mut self,
        key: Option<IntervalStatsCacheKey>,
        workspace: &WorkspaceDataState,
        ctx: &egui::Context,
        cache: &mut IntervalStatsCache,
    ) {
        // Reconcile inputs BEFORE receiving results, including mutations made earlier this frame.
        if self.requested_key != key {
            self.invalidate();
            self.requested_key = key.clone();
        }
        if cache.key != key {
            *cache = IntervalStatsCache::default();
        }

        let received = self
            .running
            .as_ref()
            .and_then(|job| match job.receiver.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(received) = received {
            let job = self.running.take().expect("polled running job");
            let current = !job.cancelled.load(Ordering::Relaxed)
                && key.as_ref() == Some(&job.key)
                && self.blocked_key.as_ref() != Some(&job.key);
            if current {
                match received {
                    Ok(result) if result.id == job.id && result.key == job.key => {
                        if let Some(stats) = result.stats {
                            self.elapsed = Some(result.elapsed);
                            *cache = IntervalStatsCache {
                                key: Some(result.key),
                                stats,
                            };
                        }
                    }
                    _ => {
                        self.error = Some(
                            "Statistics worker stopped before completing. Retry the calculation."
                                .to_owned(),
                        );
                        self.blocked_key = key.clone();
                    }
                }
            }
        }

        if self.running.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        let Some(key) = key else {
            return;
        };
        if cache.key.as_ref() == Some(&key) || self.blocked_key.as_ref() == Some(&key) {
            return;
        }
        // Keep the cancelled worker until it exits. Rapid cursor changes cannot create
        // an unbounded set of threads, queued jobs, or retained data snapshots.
        if self.running.is_none() {
            let input = build_input(key.clone(), workspace);
            let cancelled = Arc::new(AtomicBool::new(false));
            let worker_cancelled = Arc::clone(&cancelled);
            let (sender, receiver) = mpsc::channel();
            self.next_id += 1;
            let id = self.next_id;
            let repaint = ctx.clone();
            match std::thread::Builder::new()
                .name("interval-statistics".to_owned())
                .spawn(move || {
                    let started = Instant::now();
                    let stats = compute(&input, &worker_cancelled);
                    let result = JobResult {
                        id,
                        key: input.key,
                        stats,
                        elapsed: started.elapsed(),
                    };
                    // Release channel snapshots before allowing the UI to start another job.
                    drop(input.channels);
                    let _ = sender.send(result);
                    repaint.request_repaint();
                }) {
                Ok(_) => {
                    self.running = Some(RunningJob {
                        id,
                        key,
                        cancelled,
                        receiver,
                    })
                }
                Err(error) => {
                    self.error = Some(format!("Could not start statistics: {error}"));
                    self.blocked_key = Some(key);
                }
            }
        }
        if self.running.is_some() {
            // Also detects a worker panic/disconnection, which cannot send a completion repaint.
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

impl Drop for IntervalStatsWorker {
    fn drop(&mut self) {
        self.invalidate();
        // Never join a worker on the UI thread, including at application shutdown.
    }
}

fn build_input(key: IntervalStatsCacheKey, workspace: &WorkspaceDataState) -> JobInput {
    let channels = key
        .channels
        .iter()
        .map(|entry| ChannelInput {
            // ChannelData clones only its metadata and Arc, not the raw values.
            channel: workspace
                .channel(&entry.key)
                .expect("key built from loaded channel")
                .clone(),
            label: channel_label_or_fallback(workspace, &entry.key),
            tooltip: channel_tooltip_or_fallback(workspace, &entry.key),
        })
        .collect();
    JobInput { key, channels }
}

fn compute(input: &JobInput, cancelled: &AtomicBool) -> Option<Vec<IntervalChannelStats>> {
    let mut stats = Vec::with_capacity(input.channels.len());
    for (entry, channel) in input.key.channels.iter().zip(&input.channels) {
        if cancelled.load(Ordering::Relaxed) {
            return None;
        }
        let offset = f64::from_bits(entry.time_offset_bits);
        if let Some(result) = interval_channel_statistics_for_channel(
            entry.key.clone(),
            channel.label.clone(),
            channel.tooltip.clone(),
            &entry.time,
            &channel.channel,
            f64::from_bits(input.key.cursor_a_bits) - offset,
            f64::from_bits(input.key.cursor_b_bits) - offset,
            cancelled,
        ) {
            stats.push(result);
        }
    }
    (!cancelled.load(Ordering::Relaxed)).then_some(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AnalysisState, ChannelKey, DEFAULT_SOURCE_ID, ViewState, interval_stats_cache_key,
    };

    fn fixture() -> (WorkspaceDataState, ViewState, AnalysisState) {
        let mut workspace = WorkspaceDataState::default();
        workspace.shared_time = Some(parquet_waveform::TimeData {
            path: "test.parquet".into(),
            column_name: "time".into(),
            column_path: "time".into(),
            time: Arc::new(vec![0.0, 0.5, 1.0]),
            projected_column_index: 0,
            elapsed: Duration::ZERO,
        });
        workspace
            .loaded_channels
            .insert_channel(parquet_waveform::ChannelData {
                path: "test.parquet".into(),
                channel_name: "signal".into(),
                channel_path: "signal".into(),
                values: Arc::new(vec![1.0, 2.0, 3.0]),
                projected_column_index: 1,
                elapsed: Duration::ZERO,
            });
        let mut view = ViewState::default();
        view.add_channel_to_selected_row(&ChannelKey::new(DEFAULT_SOURCE_ID, "signal"));
        let analysis = AnalysisState {
            cursor_a: Some(0.0),
            cursor_b: Some(1.0),
            ..Default::default()
        };
        (workspace, view, analysis)
    }

    // Hold a completion deterministically so cancellation races never depend on CPU speed.
    fn held_worker(
        key: &IntervalStatsCacheKey,
    ) -> (
        IntervalStatsWorker,
        mpsc::Sender<JobResult>,
        Arc<AtomicBool>,
    ) {
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        (
            IntervalStatsWorker {
                requested_key: Some(key.clone()),
                next_id: 1,
                running: Some(RunningJob {
                    id: 1,
                    key: key.clone(),
                    cancelled: Arc::clone(&cancelled),
                    receiver,
                }),
                blocked_key: None,
                error: None,
                elapsed: None,
            },
            sender,
            cancelled,
        )
    }

    fn completion(key: &IntervalStatsCacheKey, workspace: &WorkspaceDataState) -> JobResult {
        JobResult {
            id: 1,
            key: key.clone(),
            stats: compute(
                &build_input(key.clone(), workspace),
                &AtomicBool::new(false),
            ),
            elapsed: Duration::ZERO,
        }
    }

    fn finish(
        worker: &mut IntervalStatsWorker,
        key: &IntervalStatsCacheKey,
        workspace: &WorkspaceDataState,
        cache: &mut IntervalStatsCache,
    ) {
        let ctx = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        while cache.key.as_ref() != Some(key) {
            worker.update(Some(key.clone()), workspace, &ctx, cache);
            assert!(worker.error.is_none(), "{:?}", worker.error);
            assert!(Instant::now() < deadline, "worker did not complete");
            std::thread::yield_now();
        }
    }

    #[test]
    fn snapshots_share_raw_buffers_and_cache_identity_does_not_scan_values() {
        let (workspace, view, analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let input = build_input(key.clone(), &workspace);
        assert!(Arc::ptr_eq(
            &key.channels[0].time,
            &workspace.shared_time.as_ref().unwrap().time
        ));
        assert!(Arc::ptr_eq(
            &input.channels[0].channel.values,
            &workspace.channel(&key.channels[0].key).unwrap().values
        ));
        assert_eq!(key, input.key);
        let mut changed = key.clone();
        changed.channels[0].values = Arc::new(vec![1.0, 2.0, 3.0]);
        assert_ne!(key, changed); // Equal contents in a newly loaded allocation are a different input.
    }

    #[test]
    fn rapid_input_changes_cancel_one_worker_and_start_only_latest_request() {
        let (workspace, view, mut analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let (mut worker, sender, cancelled) = held_worker(&key);
        let mut cache = IntervalStatsCache::default();
        let ctx = egui::Context::default();
        let mut latest = key.clone();
        for step in 1..=100 {
            analysis.cursor_a = Some(step as f64 / 200.0);
            latest = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
            worker.update(Some(latest.clone()), &workspace, &ctx, &mut cache);
            assert_eq!(worker.next_id, 1);
            assert_eq!(worker.running.as_ref().unwrap().id, 1);
            assert!(cache.key.is_none());
        }
        assert!(cancelled.load(Ordering::Relaxed));
        sender.send(completion(&key, &workspace)).unwrap();
        worker.update(Some(latest.clone()), &workspace, &ctx, &mut cache);
        assert!(cache.key.is_none());
        assert_eq!(worker.next_id, 2);
        finish(&mut worker, &latest, &workspace, &mut cache);
        assert_eq!(cache.stats[0].mean, Some(2.5));
        for _ in 0..10 {
            worker.update(Some(latest.clone()), &workspace, &ctx, &mut cache);
        }
        assert_eq!(worker.next_id, 2);
    }

    #[test]
    fn cancel_then_retry_same_key_rejects_old_completion() {
        let (workspace, view, analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let (mut worker, sender, cancelled) = held_worker(&key);
        let mut cache = IntervalStatsCache::default();
        let ctx = egui::Context::default();
        worker.cancel();
        sender.send(completion(&key, &workspace)).unwrap();
        for _ in 0..10 {
            worker.update(Some(key.clone()), &workspace, &ctx, &mut cache);
        }
        assert!(cancelled.load(Ordering::Relaxed));
        assert!(worker.is_cancelled());
        assert!(cache.key.is_none());
        assert_eq!(worker.next_id, 1);
        worker.retry();
        finish(&mut worker, &key, &workspace, &mut cache);
        assert_eq!(worker.next_id, 2);
        assert_eq!(cache.stats[0].mean, Some(2.0));
    }

    #[test]
    fn changing_away_and_back_to_same_key_cannot_resurrect_cancelled_job() {
        let (workspace, view, analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let (mut worker, sender, _) = held_worker(&key);
        let mut cache = IntervalStatsCache::default();
        let ctx = egui::Context::default();
        worker.update(None, &workspace, &ctx, &mut cache);
        sender.send(completion(&key, &workspace)).unwrap();
        worker.update(Some(key.clone()), &workspace, &ctx, &mut cache);
        assert!(cache.key.is_none());
        assert_eq!(worker.next_id, 2);
        finish(&mut worker, &key, &workspace, &mut cache);
    }

    #[test]
    fn every_input_mutation_rejects_an_already_queued_result() {
        for mutation in 0..7 {
            let (mut workspace, mut view, mut analysis) = fixture();
            let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
            let (mut worker, sender, cancelled) = held_worker(&key);
            sender.send(completion(&key, &workspace)).unwrap();
            match mutation {
                0 => analysis.cursor_b = Some(0.5),
                1 => workspace.generation += 1,
                2 => workspace.time_transform.offset_seconds = 0.25,
                3 => view.rows[0].channels[0].visible = false,
                4 => {
                    workspace.remove_source(DEFAULT_SOURCE_ID);
                }
                5 => {
                    let mut channel = workspace.channel(&key.channels[0].key).unwrap().clone();
                    channel.values = Arc::new(vec![10.0, 20.0, 30.0]);
                    workspace.loaded_channels.unload_channel("signal");
                    workspace.loaded_channels.insert_channel(channel);
                }
                6 => workspace.shared_time.as_mut().unwrap().time = Arc::new(vec![0.0, 0.25, 1.0]),
                _ => unreachable!(),
            }
            let next = interval_stats_cache_key(&analysis, &workspace, &view);
            assert_ne!(Some(&key), next.as_ref());
            let mut cache = IntervalStatsCache::default();
            worker.update(next, &workspace, &egui::Context::default(), &mut cache);
            assert!(
                cache.key.is_none(),
                "mutation {mutation} accepted a stale result"
            );
            assert!(cancelled.load(Ordering::Relaxed));
        }
    }

    #[test]
    fn disconnected_worker_reports_failure_and_allows_retry() {
        let (workspace, view, analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let (mut worker, sender, _) = held_worker(&key);
        drop(sender);
        let mut cache = IntervalStatsCache::default();
        worker.update(
            Some(key.clone()),
            &workspace,
            &egui::Context::default(),
            &mut cache,
        );
        assert!(worker.error.is_some());
        assert_eq!(worker.next_id, 1);
        worker.retry();
        finish(&mut worker, &key, &workspace, &mut cache);
    }

    #[test]
    fn dropping_worker_signals_cancellation_without_waiting_for_completion() {
        let (workspace, view, analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let (worker, _sender, cancelled) = held_worker(&key);
        drop(worker);
        assert!(cancelled.load(Ordering::Relaxed));
    }

    #[test]
    fn cancellable_scan_preserves_nonfinite_and_empty_interval_semantics() {
        let flag = AtomicBool::new(false);
        assert_eq!(
            crate::interval_value_statistics(&[], &flag),
            Some((None, None, None, None, None))
        );
        assert_eq!(
            crate::interval_value_statistics(&[f32::NAN, f32::INFINITY], &flag),
            Some((None, None, None, None, None))
        );
        assert_eq!(
            crate::interval_value_statistics(&[3.0, f32::NAN, -3.0], &flag),
            Some((Some(0.0), Some(3.0), Some(-3.0), Some(6.0), Some(3.0)))
        );
        flag.store(true, Ordering::Relaxed);
        assert!(
            crate::interval_value_statistics(&vec![1.0; CANCELLATION_CHUNK_SAMPLES * 3], &flag)
                .is_none()
        );
    }

    #[test]
    fn reversed_equal_and_outside_cursors_keep_raw_sample_semantics() {
        let (workspace, view, mut analysis) = fixture();
        for (a, b, mean, slope) in [(1.0, 0.0, 2.0, Some(2.0)), (0.5, 0.5, 2.0, None)] {
            analysis.cursor_a = Some(a);
            analysis.cursor_b = Some(b);
            let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
            let mut worker = IntervalStatsWorker::default();
            let mut cache = IntervalStatsCache::default();
            finish(&mut worker, &key, &workspace, &mut cache);
            assert_eq!(cache.stats[0].mean, Some(mean));
            assert_eq!(cache.stats[0].slope_per_second, slope);
        }
        analysis.cursor_a = Some(-0.01);
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let mut worker = IntervalStatsWorker::default();
        let mut cache = IntervalStatsCache::default();
        finish(&mut worker, &key, &workspace, &mut cache);
        assert!(cache.stats.is_empty());
        worker.update(Some(key), &workspace, &egui::Context::default(), &mut cache);
        assert_eq!(worker.next_id, 1, "empty results must also be cached");
    }

    #[test]
    fn statistics_cancel_and_retry_buttons_operate_during_pending_job() {
        let (workspace, view, mut analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let (worker, sender, cancelled) = held_worker(&key);
        analysis.interval_stats_worker = worker;
        let ctx = egui::Context::default();
        for label in ["Cancel statistics", "Retry statistics"] {
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                crate::draw_analysis_split_body(ui, &mut analysis, &workspace, &view);
            });
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .expect("statistics action rendered");
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
                        crate::draw_analysis_split_body(ui, &mut analysis, &workspace, &view);
                    },
                );
            }
            assert_eq!(
                analysis.interval_stats_worker.is_cancelled(),
                label == "Cancel statistics"
            );
        }
        assert!(cancelled.load(Ordering::Relaxed));
        sender.send(completion(&key, &workspace)).unwrap();
        analysis.interval_channel_statistics(&workspace, &view, &ctx);
        assert!(analysis.interval_stats_cache.key.is_none());
        finish(
            &mut analysis.interval_stats_worker,
            &key,
            &workspace,
            &mut analysis.interval_stats_cache,
        );
    }

    #[test]
    fn hidden_panel_cancels_and_drains_worker_to_release_snapshots() {
        let (workspace, view, analysis) = fixture();
        let key = interval_stats_cache_key(&analysis, &workspace, &view).unwrap();
        let (worker, sender, cancelled) = held_worker(&key);
        let result = completion(&key, &workspace);
        let mut app = crate::PanelYApp {
            workspace,
            view,
            analysis,
            ..Default::default()
        };
        app.analysis.interval_stats_worker = worker;
        app.analysis.info_panel_visible = false;
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_ui(ui));
        assert!(cancelled.load(Ordering::Relaxed));
        sender.send(result).unwrap();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_ui(ui));
        assert!(!app.analysis.interval_stats_worker.has_running_job());
        assert!(app.analysis.interval_stats_cache.key.is_none());
    }
}
