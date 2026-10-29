---
project: Panel_y
status: Core Rust viewer and multi-source comparison implemented; operational validation and extensions remain
created: "2026-03-24"
updated: "2026-10-28"
scope: Current Rust viewer, validation boundaries, and future work
---

# Panel_y Waveform-Viewer Roadmap

## 1. Current Position

The active product is the native Rust viewer in [apps/panel-y](../apps/panel-y/).
It moved from `code/rust_phase1/` on 2026-10-28; the Cargo package and executable
are now named `panel-y`. The implementation includes multiple rows,
Cursor A/B, interval statistics, FFT, plot settings, and multiple data sources.
The Python/Dash prototypes are retained in [archive/](../archive/README.md) as
references. Active preprocessing, data generation,
and numerical validation tools now live in [tools/](../tools/README.md).
Shared samples and validation datasets have moved to [data/](../data/README.md).

As of 2026-10-24, RFQ #1 Phase 1 M0–M7 and Phase 2 Time Alignment are complete.
Fit Overlap, Plot config v2, multi-source Session preset, FFT result overlay, and
three-or-more-source UI have also passed their implementation and validation checks.
On 2026-10-25, cancellable background interval statistics and 10M-row / 9-channel
responsiveness checks were completed. Further real-data validation, distribution,
and optional extensions remain; their next scope has not been scheduled.
On 2026-10-27, three motor-control simulation logs passed a native continuous-use
check: five source-removal/preset-restoration cycles, five plot-config loads, and
statistics/FFT before and after the repetitions. These are simulated signals;
physical measurement logs remain unvalidated.
The [compact analysis-panel update](rust_compact_analysis_panel_results.md) adds
Statistics/FFT/Split views and fixed FFT execution controls. Native 1280×832
operation passed with three sources and 12 channels at the final 360px panel height.
The statistics horizontal-scroll fix also passed native scrollbar operation;
all 240 automated tests and the release build passed.
The later [long-session validation](rust_long_session_validation_results.md)
completed 60 minutes / 30 scripted native cycles plus 120 seconds after final
removal. Preset/config restoration, raw-buffer lifetime, and statistics/FFT checks
passed; 243 tests passed with the optional validation feature. This is a
three-source simulation workload. The Mac was found locked at about 45 minutes;
during-run evidence after that point covers native UI frames and state logs, not
continuous visible screen presentation or OS pointer input. After user unlock,
post-run screenshots confirmed the same process had zero sources/channels/caches.

This is the project-wide status document. The [Rust migration plan](rust_migration_plan.md)
tracks migration stages; the [RFQ #1 work plan](rust_rfq_1_multi_source_phase1_work_plan.md)
tracks multi-source milestones. Their phase numbers belong to separate plans.
Older design documents describe the scope and decisions at the time they were written;
use current implementation results when a historical status differs.

## 2. Implemented Capabilities

| Area | Current behavior | Evidence / detail |
|---|---|---|
| Native viewer and loading | Rust / eframe / egui; Parquet schema inspection and selective channel loading; independent raw time and channel stores per source | [Viewer README](../apps/panel-y/README.md) |
| Display and large-data rendering | Multiple rows and channels; Line/Step; visibility, color, width, auto/manual Y ranges; synchronized Pan/Zoom and Hover; min/max Line tiles and Step edge caches with dense fallback | [Phase 2 benchmark](rust_migration_phase2_benchmark_results.md), [viewer README](../apps/panel-y/README.md) |
| Cursor and statistics | A/B, Δt, reciprocal frequency, raw sample values, Mean / Min / Max / P-P / RMS / slope; source-aware sample ranges, caching, and cancellable background jobs | [Async statistics results](rust_interval_statistics_async_results.md), [M5 results](rust_rfq_1_multi_source_phase1_m5_results.md), [time-alignment validation](rust_rfq_1_multi_source_phase2_time_alignment_results.md) |
| Multiple-source comparison | Arbitrary channels from different time arrays and sample rates on one display-time axis; source-numbered labels; matching-channel addition; isolated source removal | [M7 release gate](rust_rfq_1_multi_source_phase1_m7_results.md) |
| Time alignment | Per-source Offset, Reset, Start at zero, and Start all at zero; raw timestamps remain unchanged | [Phase 2 results](rust_rfq_1_multi_source_phase2_time_alignment_results.md) |
| View fitting | Fit All uses the display-time union; Fit Overlap uses the positive-width intersection of sources with loaded time data | [Fit Overlap results](rust_rfq_1_multi_source_fit_overlap_results.md) |
| File persistence | Plot config v2 stores sources with configured paths, offsets, rows, styles, and Y ranges; empty cards are omitted; v1 read compatibility and first-saved-source legacy projection; partial restore and all-missing protection | [Plot config v2 results](rust_rfq_1_multi_source_plot_config_v2_results.md) |
| Session snapshot | In-memory multi-source save/restore, including selections and X range; data reuse and missing-source handling | [Session preset results](rust_rfq_1_multi_source_session_preset_results.md) |
| FFT comparison | Independent background jobs per source; native frequency bins on shared Hz and amplitude/dB axes; window selection, frequency controls, Hover, cancellation, and obsolete-result rejection | [FFT overlay results](rust_rfq_1_multi_source_fft_overlay_results.md) |
| Analysis-panel access | Statistics/FFT focused views plus adjustable Split; fixed FFT Run/Cancel, collapsible details, and preferred panel-height retention across resizing | [Compact-panel results](rust_compact_analysis_panel_results.md) |
| Three or more sources | No fixed registration cap; bounded, scrollable source cards; three-source config/preset restore and FFT comparison verified | [Multi-source UI results](rust_rfq_1_multi_source_ui_results.md) |

## 3. Validation and Current Boundaries

On 2026-10-25, the current code passed **228 automated tests**, debug/release builds,
formatting and diff checks. Background statistics were measured on 10M rows × 9
channels, compared against NumPy, and exercised in the native macOS viewer.
Large multi-source statistics were then measured with 1/2/4 sources, up to 36M
total rows and 36 channels, with a separate 30-repeat memory/lifetime pass; see
[multi-source statistics results](rust_interval_statistics_multi_source_results.md).
The [follow-up investigation](rust_render_memory_results.md) improved warm rendering
and verified a macOS launcher setting that releases freed large allocations.
The earlier 2026-10-24 multi-source GUI suite passed all ten checks with eleven
screenshots. The statistics checks do not constitute a packaged-release qualification.
The [motor-log continuous-use check](rust_motor_log_continuous_validation_results.md)
used 100k/320k/320k-row No8/Prius simulation outputs, 12 displayed channels, and a
12-minute native session. All 70.3M CSV-to-Parquet cells matched the intended
conversion; observed statistics matched NumPy. Raw caches reached zero after
removing all sources. Short-session RSS observations do not establish leak-free
long-running operation.
The subsequent [60-minute run](rust_long_session_validation_results.md) completed
30 removal/restore cycles (15 preset, 15 config) and a final 120-second empty hold.
All 31 empty checkpoints had zero raw/draw caches and released original raw/time
buffers. Statistics/FFT passed 3,906 independent numeric comparisons and 1,581
exact count/index checks. Empty-state RSS medians were 196.59 MiB in cycles 6–15
and 186.45 MiB in cycles 21–30; final quiet RSS was 185.06 MiB. Continuous RSS
growth was not observed in this run. Lock-state/OS residency effects are not
isolated, and an always-visible, unlocked long GUI run remains unqualified.
Native compact-panel checks cover 1280×832, three sources / 12 channels, all 12
statistics rows at 360px panel height, FFT settings/results, Split, and collapse/hide
restoration. Initial-build checks also cover window enlargement and return.
All 240 tests and the release build passed, including a horizontal-scroll regression
test that fails without the viewport-width constraint. Native scrollbar operation
reached the full Slope values and returned to column A; see the
[results](rust_compact_analysis_panel_results.md). The 1024×600 and short-pane Cancel
checks are automated; native horizontal-wheel movement was not confirmed.

- **Source count:** GUI management and waveform display were checked with up to
  four sources; an automated add/remove/re-add test used five. There is no fixed
  source limit, but large source-count performance has not been established.
- **FFT:** at most three target channels across all sources. Each source retains
  its own sample interval and frequency bins; no cross-source resampling occurs.
  Input-size and copy-memory limits still apply.
- **Statistics:** cache misses run in a single cancellable worker without copying raw
  buffers. After the draw-cache fixes, the 4-source / 36-channel workload measured
  11.56 ms p95 full egui CPU frame while pending and 13.09 ms with cached statistics.
  The macOS launcher also disables the large-allocation cache at process startup:
  RSS after all sources were removed fell from 1,642 MiB to 75 MiB, with physical
  footprint about 13 MiB. Direct binary launches retain default allocator behavior.
  The [cold-render follow-up](rust_cold_render_results.md) measured 226.33→165.61 ms
  median for the initial CPU frame on macOS 27.0.1, saving 36.62 MiB of Step cache;
  large zoom redraws added about 3–6 ms. The [tile-build follow-up](rust_tile_build_results.md)
  then measured 168.18→95.24 ms in a fresh comparison, using up to four workers
  with unchanged tile-cache size. These CPU measurements exclude OS/GPU presentation
  and are workload-specific.
- **Persistence:** Session preset is lost when the app exits. Plot config persists
  workspace display settings, but not arbitrary X ranges, selections, or Cursor/FFT
  analysis state. Plot config Load fits the restored union; Session preset restores
  the saved X range. Neither saves analysis state.
- **Apply versus Load:** Plot config v2 Apply requires unique path matches and
  preserves current offsets and raw caches. v1 Apply requires one current source;
  v1 Load restores a single source.
- **Input and distribution:** the viewer opens Parquet. CSV/MAT preprocessing remains
  external to the Rust app. macOS development-app checks do not establish completion
  of packaged Windows/macOS distribution or per-OS validation.
- **Performance evidence:** the multi-source release gate used two deterministic
  fixtures (100,001 and 80,001 rows). Earlier large-data benchmarks have their own
  dataset and measurement scope. Do not extrapolate these results into a guarantee
  for arbitrary file sizes, source counts, or long-running sessions.

## 4. Remaining Work and Candidate Extensions

The following items are not a committed delivery sequence. Select scope and
acceptance criteria before starting the next implementation milestone.

### Operational and performance work

| Item | Current gap | Completion evidence to collect |
|---|---|---|
| Broader statistics workloads | Background implementation, large 1/2/4-source synthetic checks, and a 60-minute / 30-cycle three-source simulation run are complete | Physical measurement logs, more than four large sources, and longer/larger workloads |
| Multi-source rendering and memory release | Warm-frame reuse and macOS startup mitigation verified; cold CPU frame reduced to 95.24 ms median; 30 scripted native remove/restore cycles released raw buffers without sustained RSS growth in the tested run | Remaining first-frame latency, OS/GPU presentation, other OSes, larger workloads, and always-visible unlocked long GUI runs |
| Real-data and many-source validation | Three motor-control simulation logs passed native checks and a 60-minute scripted lifecycle run with statistics/FFT validation; GUI management checks cover up to four sources | Physical measurement logs, more than four large sources, multi-hour runs, and first-cache-build / presentation latency |
| Distribution and per-OS operation | No completed Rust packaged-release qualification is recorded | Define target OSes; build launchable packages; verify load, save/restore, navigation, and analysis; provide distribution instructions |
| Analysis-panel usability | Focused views and fixed FFT controls passed native 1280×832 / 3-source / 12-channel checks at 360px panel height; the statistics scrollbar reaches the complete rightmost values | Extend to different DPI settings, still smaller windows, and other OSes; horizontal-wheel movement remains unconfirmed |

### Multi-source extensions

| Candidate | State / scope to define |
|---|---|
| Automatic alignment | Unimplemented. Specify trigger/cursor reference or cross-correlation, target channels, and offset acceptance behavior. |
| Source aliases | User editing is unimplemented; labels already participate in workspace persistence. |
| Source reordering | Unimplemented; the UI uses addition order. Preserve source identity when changing order. |
| Cross-source resampling | Unimplemented and unnecessary for current waveform/FFT overlay. Define a separate analysis requirement before choosing interpolation rules. |

### Retained feature candidates from the original roadmap

| Candidate | Current distinction |
|---|---|
| Arbitrary multi-cursor placement | A/B and their statistics are implemented; arbitrary T1, T2, T3… cursors and selectable cursor pairs are not. |
| Zero-phase LPF and derived channels | Unimplemented; filter parameters, numerical validation, and derived-channel lifecycle need design. |
| THD, harmonic-order analysis, HPF/BPF/notch, moving average, envelope detection | Unplanned extensions. |
| Interactive command input and analysis commands | Unimplemented. Existing command-line schema/load/benchmark utilities are separate from an in-app command interface. |

## 5. Development Direction

The prototype-to-Rust migration has already established the display and analysis
foundation. Continue on the Rust implementation, retaining raw-data-based analysis,
source identity, display-time offsets, and separation between display configuration
and analysis state.

For the next milestone, choose between operational validation and a concrete user
workflow improvement. Representative real-data checks and
packaged distribution are existing gaps; automatic alignment and other extensions
are candidates rather than prerequisites for the completed RFQ #1 comparison features.

## 6. Milestone History

| Date | Milestone |
|---|---|
| 2026-10-28 | Moved the native app to `apps/panel-y/` and renamed its Cargo package/executable to `panel-y`. Passed 240 default / 243 feature-enabled tests, release build, sample/multi-source CLI checks, and native sample/config-restoration smoke checks. Python tools and data remain in place; see the [work log](../study/diary/2026-10-28.md#rust本体の移動と実行ファイル名変更). |
| 2026-10-27 | Completed 60 minutes / 30 scripted native cycles and 120-second final quiet observation with three simulation sources / 12ch. Raw-buffer release, both restore paths, and independent numeric checks passed; 243 feature-enabled tests. During-run screen evidence ends before lock detection at about 45 minutes; UI-frame/state logs continued. Post-unlock screenshots later confirmed the empty state in the same process. |
| 2026-10-27 | Added Statistics/FFT/Split views, fixed FFT execution controls, collapsible details, retained panel height, and a statistics horizontal-scroll fix; 240 tests and release build passed. Native 1280×832 operation passed at 360px panel height, including full rightmost-column access through the scrollbar. |
| 2026-10-27 | Generated three No8/Prius motor-control simulation Parquets (740k total rows, 94 channels each); validated conversion, native 3-source / 12-channel operations, five removal/preset cycles, five config loads, statistics and FFT. Physical measurement logs and long-session qualification remain open. |
| 2026-10-25 | 36ch warm-frame p95 reduced from 53.43 to 11.56 ms through draw-cache reuse; macOS large-allocation cache identified and startup launcher verified. 228 tests; 1/2/4-source remeasurement and three raw reload/drop cycles. |
| 2026-10-25 | Large interval-statistics validation extended to 1/2/4 sources, 36M total rows / 36ch, 30-repeat RSS measurements and buffer lifetime checks; 225 tests. Rendering latency and retained RSS identified as follow-ups. |
| 2026-10-25 | Cancellable background interval statistics, shared raw buffers, Cancel/Retry UI, 223 tests, 10M-row / 9-channel responsiveness and independent NumPy validation. |
| 2026-10-24 | Fit Overlap, Plot config v2, multi-source Session preset, FFT overlay, and three-or-more-source UI implemented and validated; 208 tests at the UI milestone, 212 after workspace-readiness regression fixes. Overall roadmap reconciled with implementation. |
| 2026-10-23 | RFQ #1 Phase 2 Time Alignment GUI validation completed. |
| 2026-08-21 | RFQ #1 Phase 1 M0–M7 release gate passed; time-alignment controls implemented. |
| 2026-06-05 | FFT plot Hover, X-range navigation, and axis-label polish recorded in the [work log](../study/diary/2026-06-05.md). |
| 2026-04-30 | Rust vertical prototype cleared for continued migration; see the [evaluation](rust_migration_phase1_evaluation.md) for measured datasets and remaining scale validation. |
| 2026-04-18 | Python prototype FFT recorded as complete. |
| 2026-04-02 | Python large-dataset fixes and Line/Step/style support recorded. |
| 2026-03-24 | Original prototype roadmap created; Proto #2.0 display improvements completed. |

Implementation notes and GUI evidence are maintained in [study/diary](../study/diary/).
