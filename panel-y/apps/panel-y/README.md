# Panel-y Native Viewer

The active Rust / eframe waveform viewer for Panel-y, located in `apps/panel-y/`
with Cargo package and executable name `panel-y`. The implementation includes multi-row
Line/Step display, independent data sources and time alignment, Cursor A/B,
background interval statistics, FFT overlay, and multi-source settings restoration.
The Python implementation in [archive/proto_3_1b](../../archive/README.md) remains a historical reference.
Active preprocessing and sample-data generators are in [tools/](../../tools/README.md);
shared samples, validation data, and plot configs are in [data/](../../data/README.md).

## Status — 2026-10-28

Revision `aa5fd00` passed 240 default tests, 243 tests with `soak-validation`
enabled (including the default suite), an offline locked release build, and
formatting checks. Earlier native macOS validation covers the compact
Statistics/FFT/Split panel and a 60-minute / 30-cycle scripted three-source,
12-channel session with numeric checks and raw-buffer release.

The long-session workload used simulation data; a Mac lock detected at about
45 minutes limits the continuous visible-screen evidence. Physical measurement
logs, more than four large sources, multi-hour/unlocked GUI runs, OS/GPU
presentation, and packaged releases across target OSes remain validation work.
See the [project overview](../../README.md),
[overall roadmap](../../docs/wave_viewer_roadmap.md), and
[long-session results](../../docs/rust_long_session_validation_results.md).
Test counts in the milestone sections below describe those earlier checkpoints.

The app moved from `code/rust_phase1/` on 2026-10-28, and its package/executable
was renamed from `rust_phase1` to `panel-y`. Commands in this README use the new
layout; linked historical results retain their measured revisions and original
paths. Sample-data paths below are relative to this directory unless the command
explicitly starts from the repository root. Existing plot configurations that
refer to data through the removed `code/rust_phase1/../` directory should be
updated to the actual data path. The supplied `test_100k.pyc.json` now uses its
sibling `test_100k.parquet` and works independently of the checkout location.

After relocation, both test suites (240 default / 243 feature-enabled), the
offline locked release build, and formatting checks passed again. CLI checks
covered the sample and two independent sources; a native macOS smoke check
displayed the 100k-row sample and restored five rows / four unique channels from
the supplied config. See the [relocation work log](../../study/diary/2026-10-28.md#rust本体の移動と実行ファイル名変更).

## Run

Run from `apps/panel-y/`:

```bash
cargo run --release
```

For the validated macOS allocator configuration, use `cargo build --release --locked`
followed by `./run_macos.sh` in this directory. See
[macOS memory release](#rendering-cache-reuse-and-macos-memory-release) for details.

Schema detection can also be checked without opening the GUI:

```bash
cargo run -- --schema ../../data/test_100k.parquet
```

To verify the selective channel read path without opening the GUI:

```bash
cargo run -- --load-channel ../../data/test_100k.parquet sine_50Hz
```

To benchmark load time plus repeated visible-range envelope extraction:

```bash
cargo run --release -- --bench-channel ../../data/panely_large_10s_1mhz_9ch.parquet iu
```

To stress repeated visible-range extraction and sample RSS:

```bash
cargo run --release -- --stress-channel ../../data/panely_large_10s_1mhz_9ch.parquet iu 1000
```

To benchmark Phase 2 shared-time multi-channel cache memory:

```bash
cargo run --release -- --bench-multi-channel ../../data/panely_large_10s_1mhz_9ch.parquet
```

To benchmark Phase 2 multi-row visible trace generation and hover lookup:

```bash
cargo run --release -- --bench-phase2 ../../data/panely_large_10s_1mhz_9ch.parquet
```

To run the RFQ #1 two-source integration and performance benchmark:

```bash
cargo run --release -- --bench-multi-source \
  ../../data/multi_source_validation_1.parquet \
  ../../data/multi_source_validation_2.parquet
```

This loads every channel from both sources into one workspace, builds same-path
Line / Step overlay rows, measures visible-range trace generation and Hover
lookup, and reports aggregate plus per-source Pan cache hit/miss counters and
process RSS.

## Reference Data

Install the [tools dependencies](../../tools/README.md#setup) before running the
generators below. They retain the fixture location independently of the working
directory; `--output-dir` selects another destination. Existing files require
`--force` to replace (the interval-statistics generator only reuses matching files).

Use the Phase 0 datasets under:

```text
../../data/
```

The first target is:

```text
../../data/test_100k.parquet
```

The large-data target is:

```text
../../data/panely_large_10s_1mhz_9ch.parquet
```

For Step rendering edge-position checks, generate:

```bash
python3 ../../tools/data_generation/generate_step_validation_data.py
```

Then load:

```text
../../data/step_validation_100k.parquet
```

Channels:

```text
pwm_1kHz
pwm_1kHz_delay_1sample
pwm_1kHz_advance_1sample
```

For ripple envelope checks, generate:

```bash
python3 ../../tools/data_generation/generate_ripple_validation_data.py --force
```

Then load or benchmark:

```text
../../data/ripple_validation_2_4m.parquet
```

Channels:

```text
ripple_low_amp_80kHz
ripple_am_60kHz
ripple_noise_70kHz
ripple_bucket_near_2048samples
ripple_on_sine_80kHz
```

For RFQ #1 multi-source overlay validation, generate both deterministic
fixtures with a Python environment that provides `numpy` and `pyarrow`:

```bash
python3 ../../tools/data_generation/generate_multi_source_validation_data.py --force
```

The generator creates:

```text
../../data/multi_source_validation_1.parquet
  time: 0.0 .. 2.0 s
  sample rate: 50,000 Hz
  rows: 100,001
  channels: iu, torque, gate_pwm

../../data/multi_source_validation_2.parquet
  time: 0.5 .. 1.75 s
  sample rate: 64,000 Hz
  rows: 80,001
  channels: iu, speed, gate_pwm
```

These fixtures support the M3–M5 same-name `iu` overlay, different-name
`torque` / `speed` overlay, different sample-rate and sample-count handling,
the `0.0 .. 2.0` union and `0.5 .. 1.75` overlap ranges, and a two-source
`gate_pwm` Step overlay whose edges do not coincide.

Check both schemas and representative channel loads without opening the GUI:

```bash
cargo run -- --schema ../../data/multi_source_validation_1.parquet
cargo run -- --schema ../../data/multi_source_validation_2.parquet

cargo run -- --load-channel ../../data/multi_source_validation_1.parquet iu
cargo run -- --load-channel ../../data/multi_source_validation_2.parquet iu
cargo run -- --load-channel ../../data/multi_source_validation_1.parquet torque
cargo run -- --load-channel ../../data/multi_source_validation_2.parquet speed
cargo run -- --load-channel ../../data/multi_source_validation_1.parquet gate_pwm
cargo run -- --load-channel ../../data/multi_source_validation_2.parquet gate_pwm
```

Generated Parquet files are excluded from Git by `*.parquet`; regenerate them
from the script instead of adding them to version control. Without `--force`,
the generator refuses to overwrite either existing fixture.

## Current State

- `eframe` app shell is in place.
- The central waveform drawing surface is ready.
- Parquet schema loading is in place.
- The `time` column and numeric channel columns are detected.
- The selected channel read path loads only `time` plus that one channel (`time: f64`, value: `f32`).
- Phase 2 shared-time channel cache is in place for multi-channel display.
- Full-range min/max envelope drawing is in place for the selected channel.
- X-axis pan/zoom and `Shift + left-drag` X-only box zoom are in place.
- The visible X range is re-extracted into a min/max envelope sized to the current plot width.
- Step mode uses exact raw samples at low density, cached exact change points when sample density is high but edge count is still bounded, and a Step-specific dense min/max envelope only when there are too many changes to draw directly.
- Rows can overlay Line and Step channels from multiple Parquet sources with different
  raw time arrays, sample rates, and sample counts on one display-time X axis.
- `Fit All`, pan, wheel/pinch zoom, box zoom, and double-click reset use the
  display-time union of every loaded source; each source keeps its own raw-time
  envelope, line-tile, and Step-edge caches.
- Auto Y combines only traces that have data inside the current display-time
  window, so a source-only interval does not pull values from an absent source.
- Row channels can be hidden and styled with per-row color overrides and line widths.
- Hover X and Cursor A/B stay on the common display-time axis, while each
  visible channel resolves its value through its source-specific raw time.
  Readouts remain distinguished by `#1`, `#2`, `#3`, …, and return no value outside
  that source's time range instead of clamping to an endpoint.
- Interval statistics use each source's raw Cursor A/B sample range. Their
  cache identity includes the full channel key, source generation, time
  fingerprint, time transform, and shared raw-buffer identity. Cache misses run
  in a single cancellable worker; no raw samples are copied or scanned on the UI thread.
- FFT supports up to three targets in total across sources. Run groups targets by
  source into independent jobs and overlays their spectra on shared Hz and
  amplitude/dB axes. Each job retains its source's raw time, sample interval, and
  native frequency bins; mixed-source inputs within a single job remain invalid.
  Request identity rejects stale results after source or analysis inputs change.
- Rows support auto or manual Y ranges with per-row min/max controls seeded from the latest auto range.

## Analysis panel at smaller window sizes

After enabling Cursor mode, use `Statistics`, `FFT`, or `Split` in the analysis
header. Statistics is the default; the focused views use the full panel body,
while Split retains the adjustable divider. The initial panel height is 360px.
Vertical scrolling exposes additional statistics rows. The values area is bounded
to the visible width; use its horizontal scrollbar to expose the full rightmost
column and return to column A.

In FFT, `Run FFT` and the running job's `Cancel` stay above the scrolling body.
Expand `Targets` to change channels and `Result values and sample grids` to read
peak values. Window, amplitude/dB, and frequency controls remain in the body;
the plot and its legend precede the result details. Large-interval warnings stay
visible even when computation details are collapsed.

Switching views, collapsing, or hiding the panel preserves cursors and completed
FFT results. Window shrinkage limits the displayed panel height without replacing
the preferred height for subsequent expansion. These are session UI settings;
they do not extend Plot config persistence. See the
[compact-panel validation](../../docs/rust_compact_analysis_panel_results.md)
for the 240-test result and native 1280×832 checks. Native horizontal-wheel
movement remains unconfirmed; the scrollbar path passed.

## Plot config v2

Plot config Save / Save As persist every source with a configured file path, their
offsets and labels, complete rows, visibility, styles, and Y ranges in
`rust.version: 2`. Empty source cards are omitted. The top-level version 1 fields
retain a projection of the first saved source for legacy readers. Save, Save As,
and Apply are available when any source has a loaded schema and a file path;
the first source may remain unconfigured.
Load restores saved sources and reports unavailable sources/channels; if no
source can be opened, it preserves the current workspace. After waveform loading,
the initial X range fits all restored sources.

Apply to Current Data matches v2 sources by unique normalized file paths and
preserves current offsets and raw caches. Missing or ambiguous matches are rejected.
Version 1 Load still opens a single Source #1; v1 Apply requires one current source.

Implemented and verified on 2026-10-24: 185 tests passed and GUI checks covered
Save, Save As, Load, Apply, offsets, styles, missing data, and v1 compatibility.
See [Plot config v2 results](../../docs/rust_rfq_1_multi_source_plot_config_v2_results.md).

## Multi-source Session preset

Session preset Save now captures source order, paths, labels, offsets, selected
source/channel/row, complete rows with styles and Y ranges, the X range, and
large-data preview settings. It is an in-memory snapshot and is lost when the
application exits; use Plot config for file persistence.

Apply reuses raw data, time arrays, and generation for sources whose saved ID and
path still match a current source with a schema. Removed sources or changed paths
are checked before replacing the workspace and receive new source IDs; row and
selection references are remapped. That runtime ID mapping is retained for repeat
Apply operations, allowing restored sources to reuse their loaded data. Extra
current sources are removed. Unavailable
sources/channels produce warnings and allow partial restoration; if every saved
source is unavailable, the current workspace is preserved.

Visible channels not already loaded are loaded asynchronously. The saved X range
is restored after those loads finish; hidden channels do not trigger loads. The
saved X range is also retained when all channels are hidden. Apply
clears drawing caches and analysis state. Save and Apply are unavailable while
loading; Save also requires at least one source with a schema. Apply can restore a
preset into an empty workspace. Clear discards only the snapshot.

Implemented and verified on 2026-10-24. All 194 tests, the build, and all 10
macOS GUI checks passed.
See [Session preset results](../../docs/rust_rfq_1_multi_source_session_preset_results.md).

## RFQ #1 Multi-source Phase 1 Status

M0 through M7 are complete. The two-source functional, safety, future-readiness,
and quality release gates passed with the deterministic validation fixtures.

`Add matching channel from all sources` was implemented in M3. It prefers an
exact channel path, falls back to a unique original channel name in each source,
and skips missing schemas, missing matches, and ambiguous names.

See the [multi-source work plan](../../docs/rust_rfq_1_multi_source_phase1_work_plan.md)
for current milestone status and remaining work.

## RFQ #1 Multi-source Phase 2 Time Alignment

Status as of 2026-10-23: complete. All 170 tests passed again on 2026-09-08
for implementation commit `563da91`; all 10 GUI smoke checks passed on
2026-10-23 using the debug build of `23be017`. See the
[GUI smoke results and evidence](../../docs/rust_rfq_1_multi_source_phase2_time_alignment_results.md).

Each Data Source now provides a runtime time-alignment control:

- edit `Offset` in seconds, using `display time = raw time + offset`
- use `Reset` to restore the source's raw time positions
- use `Start at zero` to align that source's first raw timestamp to display time 0
- use `Start all at zero` to align every source with loaded time data

Time alignment does not rewrite raw timestamp arrays, channel data, or source
generation. Cursor positions remain in display-time coordinates; interval
statistics are recomputed and an existing or running FFT result is invalidated
when an offset changes. Plot config v1 does not persist offsets; Plot config v2
saves them to disk, and Session preset keeps them in memory for the current run.

## RFQ #1 Fit Overlap

Implemented and verified on 2026-10-24. `Fit Overlap`, next to `Fit All`, fits
the X range to the common display-time interval of all sources with loaded time
data, including their offsets. Sources without loaded time data are skipped;
with one loaded source, it fits that source's full range. It is disabled when
there is no positive-width common interval or a loaded time range is invalid.
The zoom minimum also permits common intervals shorter than a sample interval.

All 175 tests passed, and GUI checks covered offsets, disjoint/touching ranges,
re-enabling, narrow overlap, and source removal. See the
[Fit Overlap results and screenshots](../../docs/rust_rfq_1_multi_source_fit_overlap_results.md).

## Per-source FFT result overlay

Multi-source FFT overlay implementation, automated testing, and GUI verification
are complete: all 205 tests, fmt, build, diff checks, and 10 GUI checks passed.
The three-target limit applies across all sources. Run starts one independent job per
source, converting display-time Cursor A/B through that source's offset. Results
retain their own sample count, dt, frequency spacing, and frequency bins; plotting
uses common Hz and amplitude/dB axes without resampling.

Source labels and color legends distinguish traces, and per-source summaries
report N, dt, df, and peaks. Hover finds a nearby bin in each source's spectrum
and returns no value outside that source's frequency range, including beyond its
Nyquist limit. If one source fails, its error remains visible while successful
sources can still be displayed.

Changes to cursors, offsets, FFT window, or targets invalidate the entire result
set. Source removal and path changes clear cursors and therefore also invalidate
all FFT results. Cancellation and request identities prevent stale jobs from
restoring results. The single-job mixed-source guard remains in place.

GUI checks covered native frequency grids, amplitude/dB display, per-source
hover limits, positive and negative offsets, partial failures, input changes,
three targets, single-source operation, and source removal. Cancellation races
were verified by deterministic automated tests. See
[FFT overlay results](../../docs/rust_rfq_1_multi_source_fft_overlay_results.md).

## UI for three or more sources

The two-source UI cap has been removed. Implementation, automated testing, and
GUI verification are complete: all 208 tests, fmt, build, diff checks, and ten GUI
checks passed, with eleven screenshots recorded. `Add Source` remains available as the source
count grows, with no fixed source-count limit. Source cards use a bounded vertical
scroll area so the channel browser remains reachable. `Add Source` and
`Start all at zero` are above the scroll area.

Cards and their controls use stable `SourceId` identities. After removing a
middle source, display numbers are compacted without changing the surviving
sources' internal IDs or channel references. The FFT limit remains three target
channels in total, independently
of the number of registered sources.

See [3+ source UI results](../../docs/rust_rfq_1_multi_source_ui_results.md).

Remaining deferred features include automatic alignment, source alias editing
and reordering, and cross-source resampling.


## Background interval statistics

Implemented and validated on 2026-10-25 (223 tests). Cursor, offset, visible-channel,
source, and raw-data changes cancel obsolete work; only the newest request can
populate the table. `Cancel statistics` stops the current calculation and leaves
it cancelled until `Retry statistics` or an input change. Hiding or collapsing the
analysis panel cancels and drains its worker and clears the statistics cache.
While the panel stays open, completed results are cached until inputs change;
Pan/Zoom alone does not recalculate statistics.

The worker shares immutable `Arc<Vec<_>>` time/value buffers, checks cancellation
every 16,384 samples and between channels, and finishes before a replacement starts.
NaN/infinity handling, raw nearest-sample endpoints, reverse/equal cursor behavior,
source offsets, and missing-range behavior are preserved.

```bash
cargo run --release -- --bench-interval-stats ../../data/panely_large_10s_1mhz_9ch.parquet
# Optional channel selection:
cargo run --release -- --bench-interval-stats <path> iu iv iw
```

The JSON report includes synchronous-reference and worker times, dispatch latency,
full egui CPU UI/tessellation frames with Hover/Pan/Zoom, cancellation, rapid input
changes, RSS when available, and full-range numerical results. It excludes OS/GPU
presentation; initial drawing-cache cost is reported separately. See the
[results and reproduction instructions](../../docs/rust_interval_statistics_async_results.md).

### Large multi-source statistics benchmark

```bash
# From the repository root, with numpy and pyarrow installed:
python3 tools/data_generation/generate_interval_statistics_sources.py \
  build/interval-stats
apps/panel-y/target/release/panel-y --bench-interval-stats-multi \
  build/interval-stats/config-4-sources.json
```

Build the release binary first. The config supplies `sources` (`path`,
`offset_seconds`), increasing `cursors`, `trials` (1–100), and `memory_repeats`
(1–1000). Newly generated configs contain absolute source paths; older relative paths
resolve from the working directory. All source channels are loaded;
both cursors must be inside every source's display-time range and map to valid raw times. This is a large-data
benchmark and reports an error if the interval finishes before it can exercise
Hover/Pan/Zoom.

The 2026-10-25 1/2/4-source measurements used 6–12 million rows and 9 channels per
source with independent time arrays and offsets. At 4 sources, dispatch was
0.112 ms median and pending UI CPU frames were 56.89 ms p95; cached-statistics
frames were also slow (62.80 ms p95). A separate 30-repeat statistics-only pass
increased RSS by 0.109 MiB. All raw-buffer strong references were released on
source removal, but RSS remained high after two seconds. RSS polling is confined
to the separate memory pass; its sampled peak is not an OS high-water mark.

At this milestone, the lifecycle and offset-boundary tests brought the suite to 225 tests. See the
[full results, limitations, JSON evidence and reproduction commands](../../docs/rust_interval_statistics_multi_source_results.md).


### Rendering cache reuse and macOS memory release

The follow-up to the multi-source benchmark preserves already-valid X-range
endpoints during clamping, uses exact min/max tiles for dense Step envelopes, and
avoids copying the overscan envelope before clipping. That milestone passed 228 tests,
including regressions for aligned-source Pan/Zoom cache reuse and finite extrema.

On macOS, build the release binary and launch it with:

```bash
./apps/panel-y/run_macos.sh
```

Run this command from the repository root. The launcher sets `MallocLargeCache=0`
before starting the viewer, honoring an explicit caller override. The tested
macOS allocator otherwise retains freed large allocations even when all raw
buffer references are gone. Direct binary / `cargo run` launches retain their
existing allocator behavior; other operating systems are unaffected.

For the optional benchmark diagnostics, use `PANEL_Y_MEMORY_DIAGNOSTICS=1` and
`PANEL_Y_MEMORY_RELOADS=3` (0–3). Diagnostics distinguish RSS, live malloc bytes and
physical footprint, then optionally reload/drop the same raw data after all timed
passes. See [causes, measurements and remaining limits](../../docs/rust_render_memory_results.md).

### Native long-session validation

An opt-in validation build can repeat the existing application load, navigation,
statistics, FFT, removal and restoration paths while the normal native eframe
window renders. It records state checks and memory checkpoints; it is scripted
application operation, not OS mouse/keyboard automation. Normal builds exclude
this driver.

```bash
cargo build --release --offline --locked \
  --manifest-path apps/panel-y/Cargo.toml --features soak-validation
./apps/panel-y/run_macos.sh --native-soak /absolute/path/to/run.json
```

The run JSON specifies `plot_config_path`, a new `output_jsonl` path,
`duration_seconds` (default 3600), `cycles` (30), `final_quiet_seconds` (120), and
`close_when_done` (false). The current workload expects three sources, three rows,
twelve loaded channels, zero offsets, `Ia_phys` in each source, and data covering
0.04–0.09 seconds. It alternates Session preset and Plot config restoration after
all-source removal. Existing output files are never overwritten. Closing the
window early records an aborted run.
The feature-enabled binary also accepts the config path through
`PANEL_Y_NATIVE_SOAK_CONFIG` for direct native app-bundle launches; an explicit
`--native-soak` argument takes precedence.

The companion sampler records one explicit PID every two seconds. The analyzer
checks run completeness and compares equivalent memory checkpoints; the separate
NumPy reference comparator checks numerical results. See the
[validation plan and results](../../docs/rust_long_session_validation_results.md)
for the exact workload, commands, evidence and limitations.


### Cold rendering

The 2026-10-26 follow-up stops dense Step edge collection once its display limit
is exceeded and caches sparse Step edges only for the requested range plus the
preceding finite state. Contained ranges reuse that cache; Pan/Zoom beyond its
coverage rebuilds it. That milestone passed 231 tests, including missing values,
duplicate times, range boundaries and partial-cache reuse.

```bash
./apps/panel-y/run_macos.sh --bench-cold-render \
  study/diary/assets/2026-10-25-multi-source-stats/config-4-sources.json
```

This CLI accepts the existing multi-source configuration and optional
`viewport_height` (900–4096, default 900). It separates the first CPU frame,
repeated cache rebuilds, warm frames and staged zoom redraws; raw loading and
reference statistics are timed separately. It includes all egui passes and
tessellation, but excludes OS/GPU presentation. The default three-row layout can
extend below the clipped viewport; prepared-channel counts do not prove visibility.

Five independent process trials per version measured a 36ch initial-frame median
of 226.33→165.61 ms on macOS 27.0.1, and removed 36.62 MiB of retained Step edges.
Large zoom changes added about 3–6 ms, while warm full-range p95 stayed near 13.5 ms.
See [results, evidence and remaining tile-construction cost](../../docs/rust_cold_render_results.md).


### Initial Line tile construction

Large batches of unbuilt Line tiles now use up to four scoped workers, including
the calling thread. At least two eligible channels and 8M total samples are
required; smaller jobs, a single available CPU and Step tiles retain lazy serial
construction. Workers borrow raw slices and all finish before caches are published.
Hidden, out-of-range, unloaded and non-LOD channels are excluded; duplicate row
placements share one build. No worker is started for an already-cached frame.

The cold-render CLI additionally reports `tile_build_wall_ms` and
`tile_build_workers`. The old `tile_build_ms` remains the sum of individual cache
construction durations, which overlap under parallel execution and are not a
subset of UI wall time. Tile hits can include a lookup of a cache built by the
batch earlier in that same frame; build counts still count actual constructions.

The 2026-10-26 36ch comparison measured 103.27→35.47 ms median tile wall time and
168.18→95.24 ms initial CPU frame time, with the same 38.62 MiB of tile cache.
That milestone passed 235 tests. See [measurements, numerical checks and limitations](../../docs/rust_tile_build_results.md).
