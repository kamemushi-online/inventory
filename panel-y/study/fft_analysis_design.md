---
project: Panel_y
doc_type: FFT Analysis Design Memo
target: code/rust_phase1
created: "2026-05-15"
status: draft
---

# FFT Analysis Feature — Design Notes

## Conclusion

The right place for this FFT feature is inside the existing Cursor information bottom panel, not in a standalone new window.

The reasoning:

- The FFT input is the Cursor A/B interval — the same analysis context as the interval statistics.
- The existing bottom panel has already been validated against representative data, with resize, Collapse, and Hide all working.
- We can reuse `AnalysisState`, `unique_visible_loaded_channels()`, raw-data references, and the cache-invalidation triggers tied to cursor changes.
- A new OS-level window would require window lifetime management, focus handling, state synchronization, restore-after-close behavior, and managing multiple analysis screens — overkill for FFT v1.
- For a future XY-coordinate analysis viewer or step-response analysis, an independent window may make sense — but it's better to build a shared analysis-view foundation at that point.

The implementation approach: separate the FFT state, computation, and rendering functions, and render into the bottom panel first. If we later move to `egui::Window` or a native separate window, the same rendering function should be callable from a different host.

## Current State

As of 2026-05-15, the Cursor / statistics side of things is in the following state:

- Cursor A/B, the next cursor slot to place, and the bottom-panel visibility live in `AnalysisState`.
- Interval statistics are computed from raw data via `compute_interval_channel_statistics()`.
- Target channels go through `unique_visible_loaded_channels()`, handling visible and loaded channels with duplicates removed.
- The Cursor information bottom panel sits inside `CentralPanel` and renders below the plot area.
- Statistics display, pinned channel column, horizontal scrolling, and panel resize have all been verified against representative data.

Dropping FFT into this flow gives us an analysis feature that naturally falls into place with Cursor A/B as its input.

## UI Placement

### v1 Placement

Extend the existing `Cursor information` bottom panel so that it effectively becomes a "Cursor analysis panel."

Default display order:

1. Cursor A/B, dt, 1/|dt| summary
2. Interval statistics
3. FFT controls
4. FFT result summary
5. FFT spectrum plot

To prepare for cases where the bottom panel height is insufficient, make the FFT section collapsible. Tabs would also be reasonable up front, but the quickest path with the least code is to add an `FFT` section beneath `Interval statistics`.

### Interactions

- Enable FFT controls only when both Cursor A and Cursor B are present.
- Select target channels from the visible loaded channels.
- For compatibility with the proto, cap the FFT targets at 3 channels.
- Default to the first up-to-3 visible loaded channels.
- Window functions: Hanning / Hamming / Rectangular.
- Y axis: Amplitude / dB.
- Frequency range: optional min/max Hz.
- `Run FFT` kicks off the computation.
- While running, show `Running...`, sample count, and target channel count.
- Cursor changes, target channel changes, or window changes all mark the existing FFT result as stale.
- Y-axis or frequency-range changes alone regenerate the display from the saved linear amplitude — no re-FFT.

## Proposed Additions to AnalysisState

Attach FFT-specific state to the existing `AnalysisState`.

```rust
struct AnalysisState {
    cursor_mode: bool,
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    next_cursor: CursorSlot,
    info_panel_visible: bool,
    info_panel_collapsed: bool,
    info_panel_height: f32,
    interval_stats_cache: IntervalStatsCache,
    fft: FftAnalysisState,
}
```

The FFT side bundles settings, results, errors, and job state.

```rust
struct FftAnalysisState {
    enabled: bool,
    section_collapsed: bool,
    target_channels: Vec<String>,
    window: FftWindowKind,
    y_scale: FftYScale,
    freq_range: FftFrequencyRange,
    result: Option<FftResult>,
    error: Option<String>,
    job: FftJobState,
}

enum FftWindowKind {
    Hanning,
    Hamming,
    Rectangular,
}

enum FftYScale {
    Amplitude,
    Db,
}

struct FftFrequencyRange {
    min_hz: Option<f64>,
    max_hz: Option<f64>,
}
```

Have job state in place from the start so we don't block the UI thread.

```rust
enum FftJobState {
    Idle,
    Running {
        job_id: u64,
        key: FftRequestKey,
        sample_count: usize,
        channel_count: usize,
        started: std::time::Instant,
    },
}
```

`Cancel` doesn't need to force-stop the thread — it can simply invalidate `job_id` and ignore the returned result. Re-runs work the same way: issue a new `job_id` and discard the old result as stale.

## FFT Result Model

Keep linear amplitude as the stored result. dB conversion and frequency-range filtering happen at display time.

```rust
struct FftResult {
    key: FftRequestKey,
    cursor_a: f64,
    cursor_b: f64,
    index_start: usize,
    index_end_exclusive: usize,
    sample_count: usize,
    dt: f64,
    frequency_hz: Vec<f64>,
    spectra: Vec<FftChannelSpectrum>,
    elapsed: std::time::Duration,
}

struct FftChannelSpectrum {
    channel_name: String,
    channel_path: String,
    amplitude: Vec<f64>,
    peak_freq_hz: Option<f64>,
    peak_amp: Option<f64>,
    peak_db: Option<f64>,
}
```

`FftRequestKey` should include:

- Bit representations of Cursor A/B
- Time sample count
- Bit representations of the first and last time entries
- Target channel paths and sample counts
- Window function
- Extracted index range

`y_scale` and `freq_range` are display conditions, so don't include them in the key.

## Computation Spec

Match the FFT reference in `docs/rust_migration_phase0_analysis_reference.md`.

- Take the nearest sample indices from Cursor A/B.
- The interval is inclusive on both ends.
- `N < 4` is not executable.
- Set `dt = seg_time[1] - seg_time[0]`, prioritizing proto_3_1b compatibility.
- Error out if `dt` is non-finite or non-positive.
- For v1, non-uniformly-spaced data raises a warning or error. No interpolation.
- FFT operates on raw data — not on the rendering LOD or envelope.
- Remove DC per channel by subtracting the interval mean.
- Apply the window function.
- Compute the one-sided spectrum (the `rfft` equivalent).
- Amplitude is `abs(rfft(windowed)) * 2.0 / N`.
- dB display is `20 * log10(max(amplitude, 1e-12))`.

Match window functions to NumPy's definitions.

```text
Hanning:     0.5 - 0.5 * cos(2*pi*n/(N-1))
Hamming:     0.54 - 0.46 * cos(2*pi*n/(N-1))
Rectangular: 1.0
```

In v1, channels containing NaN/Inf are treated as a per-channel FFT failure. Interpolating or excluding them inside the mean calculation widens the gap from the Python version, so an explicit error is safer at first.

## FFT Core Implementation

Rust currently has no FFT dependency. The realistic move is to add `rustfft` at implementation time.

Implementation sketch:

- `build_fft_job_input()` clones the time-interval and per-channel value slices needed for the computation from the UI-thread state.
- Hand the `FftJobInput` to a background thread.
- Inside the thread, use `rustfft::FftPlanner<f64>` to run the FFT.
- Return the result via an `mpsc::Sender<FftJobResult>`.
- Use `ctx.request_repaint()` to refresh the UI.

This reuses the same pattern as the existing load job.

```rust
struct FftJobInput {
    key: FftRequestKey,
    cursor_a: f64,
    cursor_b: f64,
    index_start: usize,
    index_end_exclusive: usize,
    dt: f64,
    window: FftWindowKind,
    channels: Vec<FftJobChannelInput>,
}

struct FftJobChannelInput {
    channel_name: String,
    channel_path: String,
    values: Vec<f64>,
}
```

Copy volume is acceptable for v1, since we're capped at 3 channels and limited to the Cursor interval. As a guard against very large intervals, show the sample count and estimated memory before running, and confirm or refuse when over the limit.

## Handling Large Intervals

As laid out in the Phase 4 plan, FFT does not run on the UI thread.

Starting limits to use as guidance:

- Soft warning: 1,000,000 samples/channel or more
- Hard limit (proposed): 5,000,000 samples/channel or more, or estimated copy memory in the hundreds-of-MiB range
- Target channels capped at 3

Tune these limits via measurement. Phase 0 benchmarks showed FFT-equivalent work on a 1-second interval (~1,000,000 samples) of the large dataset running in under 0.2 s, but in the Rust UI we'll re-check including copy, thread launch, and rendering point counts.

For display: drawing every frequency bin verbatim may bog down the bottom panel. Keep all bins in the stored result, but for rendering use either min/max or decimated lines sized to the plot width.

## Rendering Approach

For v1, don't pull in a new drawing library — sketch the FFT plot on top of the existing `egui::Painter`. That's the minimal implementation.

What's shown:

- X axis: Frequency [Hz]
- Y axis: Amplitude or Magnitude [dB]
- One line per target channel, up to 3
- A small per-channel table of peak freq / amp / dB
- Display range that respects the freq min/max settings

Don't lock the FFT plot rendering function to the bottom panel — keep it callable from a future standalone window as well.

```rust
fn draw_fft_section(ui: &mut egui::Ui, analysis: &mut AnalysisState, dataset: &DatasetState, view: &ViewState);
fn draw_fft_plot(ui: &mut egui::Ui, result: &FftResult, settings: &FftAnalysisState);
```

## Outlook for a Standalone Window

We won't introduce a new window for FFT v1. But design with the following in mind:

- `FftAnalysisState` does not depend on the UI host.
- The FFT core computation stays close to a pure function, free of `AnalysisState` and egui dependencies.
- The FFT section rendering function takes only a `Ui`.
- Job results are validated by `FftRequestKey`, independent of the display host type.

When XY-coordinate analysis or step-response analysis is added in the future, consider a shared host like this:

```rust
enum AnalysisSurface {
    BottomPanel,
    FloatingWindow,
}

enum AnalysisViewKind {
    CursorStats,
    Fft,
    XyViewer,
    StepResponse,
}
```

Start by checking whether an in-app floating window via `egui::Window` is enough. An OS-native separate window requires designing multi-viewport handling, shutdown, focus, and save/restore — better deferred until the XY viewer / step response requirements solidify.

## Implementation Steps

1. Add FFT state types.
2. Invalidate FFT results on cursor changes, clear, dataset switch, and channel unload.
3. Implement the FFT core as a function independent of the UI.
4. Add unit tests for the window functions and dB conversion.
5. Add tests verifying that peak freq / amp / dB on representative data match the reference.
6. Add an mpsc sender/receiver for FFT jobs and a `next_fft_job_id` to `PanelYApp`.
7. Add FFT controls and result display to the bottom panel.
8. Add warnings / limits for large intervals.
9. Run `cargo fmt`, `cargo check`, and `cargo test`.
10. Manually verify with representative data.

## Verification Items

Automated tests:

- Coefficients for Hanning/Hamming/Rectangular match the expected values.
- Cursor A/B nearest indices and inclusive interval match the statistics path.
- After DC removal, amplitude is computed as `abs(rfft) * 2.0 / N`.
- `dB = 20 * log10(max(amp, 1e-12))`.
- `sine_50Hz`, `pwm_1kHz`, and `chirp_1_500Hz` from `docs/rust_migration_phase0_analysis_reference.md` match within tolerance on peak freq / amp / dB.
- Stale job results are ignored.
- Target candidates are derived from visible loaded channels, with duplicate channels removed.

Manual verification:

- FFT controls become enabled after Cursor A/B are set.
- FFT renders below the statistics, without interfering with Collapse/Hide.
- Spectra are readable up to 3 channels.
- Switching Amplitude/dB changes the display only — no re-FFT.
- The freq min/max settings change the display range.
- After cursor changes, no stale FFT result lingers.
- UI doesn't lock up on large intervals.

## Open Questions

- Should non-uniformly-spaced data be an error, or be accepted within some tolerance?
- The hard-limit value for very large intervals.
- Whether FFT results should be included in any future analysis-state persistence.
- Whether the statistics computation should also move onto the same job infrastructure as FFT.
- Whether the FFT plot stays on the existing Painter, or eventually moves to something like `egui_plot`.

For v1, treating non-uniform data as an error, limiting large intervals via warnings + caps, skipping result persistence, deferring the statistics-job migration, and going with Painter-based rendering strikes the right balance between implementation cost and safety.
