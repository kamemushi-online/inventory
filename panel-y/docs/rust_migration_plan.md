---
project: Panel_y
doc_type: Migration Plan
target: Rust Waveform Viewer
created: "2026-04-27"
updated: "2026-10-28"
status: Display, persistence, and background analysis implemented; broader workload and distribution validation remain
source_proto: "archive/proto_3_1b"
---

# Rust Migration Plan — From the Dash/Plotly Version to a High-Performance Native Waveform Viewer

---

## 1. Objective

Freeze the waveform viewer functionality completed in `proto_3_1b` as the specification reference,
and rebuild it as a Rust-based desktop application in order to overcome the performance limits of Dash/Plotly.

The primary objective is not "switching the language to Rust," but redesigning rendering, data retention, and zoom handling
to be suitable for large-volume waveform data.

### Current Position — 2026-10-28

The active implementation in [apps/panel-y](../apps/panel-y/) has progressed
beyond its original vertical prototype. It provides multi-row display, Cursor A/B,
interval statistics, FFT, and file persistence, with RFQ #1 extensions for multiple
sources, time alignment, FFT overlay, and save/restore.

On 2026-10-28, the app moved from `code/rust_phase1/` to `apps/panel-y/`, with
Cargo package and executable name `panel-y`. Python tools have subsequently moved
to `tools/`, and shared samples/validation data to [data/](../data/README.md).
The remaining Python prototypes and their dependency files now live in
[archive/](../archive/README.md).
Historical validation reports preserve their original
paths and revisions; the viewer README provides commands for the current layout.

| Migration stage | Current state |
|---|---|
| Phase 0: specification baseline | Complete; the Python prototype remains the reference. |
| Phase 1: vertical prototype | Evaluation gate passed; measured scale and remaining validation are documented in the evaluation. |
| Phase 2: display and performance | Display features implemented; initial cache-build cost and broader workload measurements remain performance follow-ups. |
| Phase 2.5: UI polish | Implemented; Session preset now supports multiple sources. |
| Phase 3: plot settings | Implemented and exercised by the Plot config v2 validation, including v1 compatibility. |
| Phase 4: analysis | Cursor/statistics/FFT and background statistics implemented; large synthetic checks and a 60-minute scripted three-source validation completed within their documented scopes. |
| Phase 5: distribution | Packaged-release and per-OS validation are not recorded as complete. |

RFQ #1 has its own phase numbering. Its Phase 1 M0–M7, Phase 2 Time Alignment,
Fit Overlap, Plot config v2, multi-source Session preset, FFT overlay, and
three-or-more-source UI have passed their respective checks. See the
[RFQ work plan](rust_rfq_1_multi_source_phase1_work_plan.md) and
[overall roadmap](wave_viewer_roadmap.md) for current scope and remaining candidates.

The milestone goals below retain the migration rationale. A goal or completion
criterion is not itself evidence of verification; current limitations and linked
results define the validated scope.

---

## 2. Background and Issues

### Python Reference Baseline

`proto_3_1b` has already implemented the following:

- Parquet loading
- Overlaying multiple channels per row
- Adding/removing rows, specifying Y-axis range
- Switching between line / step display
- Per-channel color and line-width settings
- Hover synchronization, zoom/pan synchronization
- Cursor A/B measurement
- Range statistics
- FFT
- Saving and loading plot settings as `.pyc.json`
- Min-Max envelope for large-volume data

### Limits of the Existing Version

Dash/Plotly is highly capable, but the following become bottlenecks with large-volume data:

- Data transfer between the Python server and the browser is heavy
- The cost of JSON serialization of Plotly figures and reflecting them in DOM/WebGL is large
- Dash callbacks chain on every zoom/pan
- Browser-side memory consumption tends to grow
- For data on the order of 90 M points, even "suppressing the number of rendered points" hits the UI responsiveness limit

---

## 3. Technical Approach

### Approach Adopted

| Item | Approach |
|------|------|
| Language | Rust |
| Application Form | Native desktop application |
| UI | `eframe` / `egui` |
| Rendering | Custom waveform drawing through `egui::Painter`, using range-aware envelopes and caches |
| Data Format | Continue using the same Parquet as the current version |
| Plot Settings | `.pyc.json` with top-level v1 compatibility and a `rust.version: 2` extension for multiple sources |
| Preprocessing | Concurrent use of the Python converter is allowed in the initial stage |

### Approach Not Adopted

A configuration like Tauri + Plotly.js, i.e., a Rust backend + Web/Plotly rendering, is not chosen as the first candidate.
This configuration improves distribution and backend performance, but it leaves the current primary bottleneck —
the browser/Plotly rendering pipeline — in place, so it is unlikely to deliver fundamental performance improvements.

---

## 4. Basic Architecture

```
Parquet sources
  │
  ▼
Data Loader
  - time column validation
  - channel column detection
  - type conversion
  - metadata extraction
  │
  ▼
Waveform Store
  - full resolution data
  - source-aware channel identity
  - independent raw time arrays per source
  │
  ▼
LOD / Tile Cache
  - min/max envelopes and lazy Line tiles
  - lightweight extraction per visible range
  - Step edge cache and dense fallback
  │
  ▼
Renderer
  - generate only the render points that match the visible pixel width
  - GPU rendering
  - do not push all points to the UI during pan/zoom
  │
  ▼
Analysis Layer
  - cursor values
  - range statistics
  - background FFT jobs per source
  - analysis always references raw data
```

Key principles:

- Make rendering proportional to the number of display pixels
- Analysis must always reference raw data
- Keep UI events lightweight; make heavy processing asynchronous and cancellable
- Separate the responsibilities of data loading, LOD construction, rendering, and analysis

Current implementation boundary: time alignment uses `display time = raw time + offset`.
Hover, cursors, and analysis resolve values through each source's raw time.
Interval statistics reuse cached results and run cache misses in one cancellable
background worker with shared raw buffers and obsolete-result rejection.

---

## 5. Migration Steps

### Phase 0: Freezing the specification of `proto_3_1b`

Status: Completed

Goal: Fix the comparison baseline for the Rust version.

Detailed plan: [rust_migration_phase0_plan.md](rust_migration_phase0_plan.md)

Work items:

- Treat `proto_3_1b` as the feature-complete version
- Finalize the list of existing features
- Prepare representative datasets
  - Small: 100k points
  - Medium: a few million points
  - Large: ~90 M points
- Organize the items saved in the plot settings file
- Record the operational specifications of the current UI via screenshots or documentation

Deliverables:

- Feature checklist
- Representative Parquet data
- Plot settings samples
- Initial performance baseline values

Completion criteria:

- It is clear which features must be reproduced in the Rust version and which can be deferred

---

### Phase 1: Rust Vertical-Slice Prototype

Status: Evaluation gate passed; subsequent migration phases are implemented

Goal: Validate the highest-risk technical area — "large-volume data loading + rendering + zoom" — first.

Evaluation results: [rust_migration_phase1_evaluation.md](rust_migration_phase1_evaluation.md)

Minimum features:

- Open a Parquet file
- Recognize the time column and channel columns
- Display one channel per row
- Support pan / zoom
- Render a min/max envelope according to the visible range
- Do not freeze on ~90 M-point data

Items deferred at the original Phase 1 checkpoint (now implemented in later stages):

- Multi-row UI
- Saving plot settings
- Cursor
- FFT
- Detailed settings such as color and line width

Focus areas for validation:

- Parquet loading time
- Memory usage
- Initial display time
- pan / zoom responsiveness
- Stability during continuous zooming
- Startup and rendering stability across OSes

Completion criteria:

- For large-volume data, "display, pan, zoom" is clearly lighter than the current Dash version
- Confirm that not all points are pushed through the rendering pipeline during interaction

---

### Phase 2: Migrating Basic Waveform-Viewer Features

Status: Phase 2 closeout completed (performance improvements have reached a checkpoint)

Goal: Migrate the main viewer-level operations of `proto_3_1b` to the Rust version.

Design approach: [rust_migration_phase2_design.md](rust_migration_phase2_design.md)

Performance measurements: [rust_migration_phase2_benchmark_results.md](rust_migration_phase2_benchmark_results.md)

Performance improvements: [rust_migration_phase2_performance_plan_2.md](rust_migration_phase2_performance_plan_2.md)

Migration targets:

- Overlaying multiple channels per row
- Adding / removing rows
- Switching between line / step display
- Per-channel color and line width
- Specifying Y-axis range
- X-axis zoom / pan synchronization
- Hover position synchronization
- Channel value display
- Light/dark theme

Design considerations:

- Even with multiple rows, each row must not independently re-extract all data
- Share a per-channel LOD cache
- For step display, prioritize edge retention rather than only peak retention
- Separate UI state and data state

Completion criteria:

- Routine waveform-checking work can be performed entirely in the Rust version
- The basic display features have operability equal to or better than `proto_3_1b`

Closeout judgment:

- As additional performance improvements after P2-12, the following were implemented: treating preview as a debug-only feature, overscan range cache, bucket boundary sharing, lazy Line tile / LOD, and Step edge cache / dense LOD
- Settled on a policy where Line uses caches/tiles that retain min/max, and Step uses edge retention with a dense fallback, so that the information needed for event search is not lost during pan/zoom
- Verified low-amplitude high-frequency components via ripple validation and 1-sample-difference step display via Step edge validation
- The initial Line tile build / Step edge cache build cost is treated as a known open item and is not a blocker for subsequent phases at this point
- Wheel-operation separation, box zoom, and data-load UI improvements were implemented after this checkpoint

---

### Phase 2.5: UI Polish for Field Validation

Status: Implementation completed

Goal: Prepare the load, navigation, and cache UI for field validation.

Detailed plan: [rust_migration_phase2_5_ui_polish.md](rust_migration_phase2_5_ui_polish.md)

Priority items:

- Enrich the data-load-related UI
- Session presets as a low-risk plot-setup aid
- Operations on loaded caches

Current behavior:

- File picker, schema inspection, channel filtering and load/add actions, and cache operations are available.
- Wheel operations and box zoom have distinct input behavior.
- Session preset remains an in-memory snapshot, now including multiple sources,
  offsets, row styles, selections, and X range. File persistence is provided by
  Plot config. See [Session preset results](rust_rfq_1_multi_source_session_preset_results.md).

Completion criteria:

- Opening actual logs, inspecting schemas, and loading multiple channels do not cause practical stress
- Channels can be loaded/added from a channel list, and loaded / loading / unloaded states are visible
- Low-risk session presets can restore the display state during field validation
- Loading, pan, zoom, row addition, and channel addition with representative data have all been verified through the GUI

---

### Phase 3: Early Migration of Plot Settings

Status: Implemented; current save/restore and v1 compatibility validated with Plot config v2

Goal: Save and restore display states used in actual work as files, separately from analysis state.

Original v1 design: [rust_migration_phase3_plotconfig_design.md](rust_migration_phase3_plotconfig_design.md).
Current behavior and evidence: [Plot config v2 results](rust_rfq_1_multi_source_plot_config_v2_results.md).
The original design's status and checklist are historical, not the current progress tracker.

Migration targets:

- Read compatibility for current `.pyc.json` version 1
- Saving plot settings in the Rust version
- Reloading plot settings saved by the Rust version
- Restoring data paths
- Restoring row configuration
- Restoring channel display modes
  - Line / Step
  - visible
- Restoring channel styles
  - color
  - line width
- Restoring Y-axis range
- Restoring theme
- Read compatibility for scroll-zoom-equivalent settings

Implemented:

- Added `plot_config.rs`, separating JSON read/write, tolerant parser, normalization, apply plan, and report
- Save / Save As write `rust.version: 2` with all sources, offsets, complete rows,
  styles, and Y ranges. Top-level v1 fields retain a first-source projection for legacy readers.
- Plot-settings load performs data-path validation, schema loading, and restoration of rows/channels/styles/visibility/theme/scroll zoom
- Only visible channels can be targeted by asynchronous on-demand loading
- `Apply to Current Data` matches v2 sources by unique normalized paths, applies
  display configuration, and preserves current offsets and raw caches. Missing or
  ambiguous matches are rejected; v1 Apply requires a single current source.
- Load supports partial restoration with warnings; if every saved source is
  unavailable, the current workspace is preserved. After loading, X fits the restored union.
- `cargo test` passes after the addition of the plot-settings application mode

Design considerations:

- What plot settings restore is limited to the display configuration and display style of raw channels
- Analysis states such as cursor position, Δt, statistics results, FFT conditions, and FFT results are not included in plot settings
- Regeneratable internal caches such as the rendering cache, LOD cache, Line tile, and Step edge cache are not saved
- Channel identity is handled, as much as possible, based on the channel path in the schema
- Unknown keys in `.pyc.json` version 1 are ignored; type errors or missing fields fall back safely on a per-restorable-item basis
- Rust-specific multi-source data lives in the versioned `rust` namespace while keeping the v1 compatibility fields

Completion criteria:

- Plot settings saved by the current Dash version can be loaded by the Rust version
- Plot settings saved by the Rust version can be reloaded by the Rust version, restoring the same display state
- The application does not crash on non-existent channels, empty rows, invalid colors, invalid line widths, invalid Y ranges, or invalid data_path; instead it surfaces an explicit error or performs a partial restore
- After plot-settings load, raw channel loading and LOD generation for the displayed data still follow the existing on-demand design

Validation checkpoint:

- On 2026-10-24, Plot config v2 passed 185 tests and macOS GUI checks covering
  Save / Save As / Load / Apply, offsets, rows, styles, missing data, and v1 compatibility.
- The later three-or-more-source milestone passed 208 tests and GUI validation,
  including three-source save/reload. See [multi-source UI results](rust_rfq_1_multi_source_ui_results.md).
- Arbitrary X range, selection, and Cursor/FFT state are not persisted by Plot config.
  Session preset restores X range and selections for the current application run;
  it also excludes analysis state.

---

### Phase 4: Migrating Analysis Features

Status: Features and background interval statistics implemented; representative synthetic large-data validation complete

Goal: On top of the display configuration restored by plot settings, layer the measurement and analysis features used in actual work as a separate state.

Migration targets:

- Cursor A/B
- Δt and frequency conversion
- Channel values at the cursor position
- Range statistics
  - Mean
  - Max / Min
  - P-P
  - RMS
  - Slope
- FFT
  - Target channel selection
  - Window function
  - Amplitude / dB
  - Frequency range specification

Current implementation:

- `AnalysisState` separates Cursor/statistics/FFT state from plot settings.
- A/B positions stay in display time. Values and statistics use each source's raw
  sample range, including time offsets, and do not clamp out-of-range values to endpoints.
- Interval statistics are cached by cursor/channel/source/time-transform and raw-buffer
  identity. Cache misses use one cancellable worker; raw arrays are shared, and rapid
  changes replace obsolete requests without creating a job queue. Cancel/Retry and
  cancellation on panel hide/collapse are implemented. See the
  [2026-10-25 results](rust_interval_statistics_async_results.md) for 223 tests,
  numerical verification, native GUI checks, and 10M-row / 9-channel response measurements.
- FFT supports three target channels in total across sources. Each source has an
  independent background job and retains its native time step and frequency bins.
  Results overlay on common axes; no cross-source resampling is performed.
- FFT input changes invalidate results, and cancellation/request identities prevent
  stale results from being applied. Input-size and copy-memory limits are enforced.
- Source-aware cursor/statistics behavior was verified in
  [M5](rust_rfq_1_multi_source_phase1_m5_results.md) and
  [time-alignment GUI checks](rust_rfq_1_multi_source_phase2_time_alignment_results.md).
  See [FFT overlay results](rust_rfq_1_multi_source_fft_overlay_results.md) for FFT verification.

Design considerations:

- Treat Cursor, statistics, FFT conditions, and analysis results as `AnalysisState`, separated from the display state derived from plot settings
- Analysis uses raw data, not the rendering LOD
- Do not run statistics/FFT over large ranges on the UI thread
- Allow cancellation or re-run overwrite during computation
- For very large FFT target ranges, provide a warning or impose an upper limit
- When plot settings are loaded, clear `AnalysisState` in the initial implementation
- If analysis state is saved in the future, define a separate schema change; the current `rust.version: 2` extension does not save it

Remaining work against the original analysis goals:

1. Extend the completed large 1/2/4-source synthetic measurements to real measurement
   logs, more than four large sources, and longer/larger sessions. The
   [60-minute / 30-cycle scripted native check](rust_long_session_validation_results.md)
   passed raw-buffer release, restore, and numerical checks for three simulation
   sources / 12ch, followed by 120 seconds with all data removed. The Mac was found
   locked at about 45 minutes; an always-visible unlocked GUI run remains open.
   Continue profiling initial cache construction and native OS/GPU latency after the
   [draw-cache and macOS memory improvements](rust_render_memory_results.md) and
   [cold Step preparation improvements](rust_cold_render_results.md) and
   [bounded parallel tile construction](rust_tile_build_results.md).
2. Extend the [compact analysis-panel checks](rust_compact_analysis_panel_results.md)
   to different DPI settings, still smaller windows, and other OSes. Focused
   Statistics/FFT views, a retained Split view, fixed FFT execution controls, and
   collapsible details now support the checked 1280×832 native workflow at 360px
   panel height. The statistics scrollbar also passed native checks for full
   rightmost-column access and return to column A. Horizontal-wheel movement
   remains unconfirmed; see the report for the verification boundaries.

Completion criteria:

- Cursor / statistics / FFT of `proto_3_1b` can be replaced by the Rust version
- The range selected by Cursor A/B can be used as the common input for statistics and FFT
- Background statistics preserve UI processing on the measured synthetic workload;
  wider real-data validation and cold drawing-cache latency remain separate.

---

### Phase 5: Preparing Distribution and Operational Features

Status: Packaged-release qualification and per-OS validation remain open

Goal: Set up a distribution form that can launch the Rust viewer without a Python environment, and put the finishing touches needed for actual operation.

Migration targets:

- exe / app distribution
- Per-OS startup and rendering verification
- README / instructions for distribution
- Regression verification of the already implemented Plot config v2 and v1 compatibility in packaged builds

The existing macOS development-app GUI checks demonstrate application behavior,
not completion of a distributable release. Define supported OSes, packaging, and
launch/operation checks before marking this phase complete. The Rust Parquet viewer
runs without Python; CSV/MAT preprocessing remains a separate Python workflow.

Design considerations:

- On top of the plot-settings compatibility from Phase 3 and the AnalysisState from Phase 4, confirm that saving, restoration, and analysis at distribution time do not break down
- If possible, migrate the extension to `.panely.json` in the future, while preserving read compatibility with existing `.pyc.json`

Completion criteria:

- The application can be launched without a Python environment
- On the target OSes, basic operations — loading representative data, loading plot settings, pan/zoom, and Cursor/analysis — can be verified

---

## 6. Performance Targets

Initial target values (goals, not guarantees for unmeasured workloads):

| Item | Target |
|------|------|
| 100k-point data | Near-instant operational feel |
| Multi-million-point data | After the initial display, pan/zoom is smooth |
| ~90 M-point data | No freezing; can withstand continuous zooming |
| Number of rendered points | Limited to the order of the display pixel width |
| Analysis reference | Always raw data |
| Memory | Significantly reduced compared to the Dash/Plotly version |

Measured items:

- Startup time
- File loading time
- LOD construction time
- Initial rendering time
- pan / zoom latency
- Maximum memory usage
- Memory growth during continuous operation

---

## 7. Risks and Mitigations

| Risk | Description | Mitigation |
|--------|------|------|
| UI operability | Source cards and analysis controls must remain accessible at different window sizes | Validate scrolling, panel resizing, and analysis visibility with representative layouts |
| Rendering and cache cost | Initial Line tile / Step edge builds and many-source memory may affect interaction | Measure cold-cache and repeated navigation workloads, preserving min/max and Step-edge behavior |
| Reimplementation cost of MAT/CSV conversion | MATLAB-format support has many details | Keep using the Python converter initially |
| Plot-settings compatibility | v1 readers only see the first-source projection of a v2 configuration | Preserve compatibility fields and exercise v1/v2 load, apply, and missing-data behavior |
| Mixing plot settings and analysis state | Putting Cursor/FFT into plot settings makes compatibility and restoration responsibilities unclear | Phase 3 saves only raw-channel display configuration; Phase 4 separates AnalysisState |
| Differences in analysis results | FFT/statistics drift from the Python version | Numerically compare against the Python version with representative data |
| Statistics responsiveness | Background execution is verified on one large synthetic workload | Retain cancellation/obsolete-result tests; extend measurements to real logs and large multi-source intervals |
| Distribution differences | Behavior differs between Windows/macOS | Decide target OSes early and prepare CI / manual verification procedures |

---

## 8. Next Work Candidates

Core display, persistence, and analysis features are already present. The next
milestone has not been scheduled. Candidate operational work is:

1. Physical real-data, many-source, initial-cache-build, memory, and longer/larger
   session validation beyond the completed 60-minute scripted three-source run.
2. Packaged builds and per-OS operation checks with distribution instructions.

Automatic alignment, source aliases/reordering, resampling, arbitrary multi-cursors,
filters, and an interactive command interface are separate extensions. Their scope
and order remain undecided; see the [overall roadmap](wave_viewer_roadmap.md).

---

## 9. Original Technology Decision Criteria

The [Phase 1 evaluation](rust_migration_phase1_evaluation.md) cleared continued
development with Rust / eframe / egui. The following criteria explain that decision
and remain useful when assessing future workloads; they are not a pending choice
between restarting the prototype and continuing migration.

Conditions for continuing the Rust migration:

- For large-volume data it is clearly lighter than the Dash/Plotly version
- The UI does not freeze during continuous zooming
- Memory usage stays within an acceptable range
- The complexity of the implementation is sustainable for future maintenance

Conditions for revising the approach:

- The constraints of the GUI/rendering library prevent the required operability
- The implementation cost of the Rust version turns out to be far higher than expected
- It becomes clear that C++/Qt or similar is clearly easier to bring into short-term practical use

---

## 10. Current Verification References

- On 2026-10-27, the optional native soak feature passed 243 tests and a 60-minute /
  30-cycle run plus 120-second final quiet observation. All raw buffers were released
  at empty checkpoints; independent numeric checks passed. Later during-run screen
  evidence was unavailable due to Mac lock, while native UI-frame/state logs continued.
  Post-unlock screenshots separately confirmed the final empty state in the same process.
  See [scope, RSS series, and results](rust_long_session_validation_results.md).
- On 2026-10-26, the current code passed 235 tests, debug/release builds, formatting, and diff checks.
- Background interval statistics passed 10M-row / 9-channel CPU responsiveness, native macOS display, and independent NumPy numerical checks; see [results](rust_interval_statistics_async_results.md).
- [Large multi-source statistics measurements](rust_interval_statistics_multi_source_results.md)
  cover 1/2/4 independent sources, up to 36M total rows / 36ch, 30 repeated calculations,
  cancellation, source removal and raw-buffer lifetime. Statistics-only RSS is stable
  over that run. The [follow-up investigation](rust_render_memory_results.md) fixed
  unnecessary envelope rebuilding and added macOS startup control of the large
  allocation cache. The [cold-render follow-up](rust_cold_render_results.md) reduced
  first-frame CPU time from 226.33 to 165.61 ms median and retained Step cache by
  36.62 MiB. [Bounded tile parallelism](rust_tile_build_results.md) subsequently
  reduced initial CPU frame time from 168.18 to 95.24 ms with unchanged tile-cache
  memory; remaining first-frame latency and broader OS validation remain.
- The latest [multi-source UI results](rust_rfq_1_multi_source_ui_results.md) record
  ten successful macOS GUI checks, including three-source FFT and config/preset restore.
  Source management was checked up to four sources in the GUI and five in automated tests.
- The [M7 benchmark](rust_rfq_1_multi_source_phase1_m7_results.md) covers two small
  deterministic sources. The [Phase 1 evaluation](rust_migration_phase1_evaluation.md)
  and [Phase 2 benchmark](rust_migration_phase2_benchmark_results.md) describe their
  own larger-data measurements. These are workload-specific evidence, not a guarantee
  for arbitrary source counts or dataset sizes.
- Implementation notes and screenshots are kept in [study/diary](../study/diary/).
