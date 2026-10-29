# panel-y

A native desktop waveform viewer for power-electronics and motor-control measurement data. Loads Parquet logs and lets you pan, zoom, and inspect long, high-rate signals interactively.

## Project Status — 2026-10-28

The active implementation is [the Rust viewer](apps/panel-y/), with Cargo package
and executable name `panel-y`. Multi-row waveform display,
Cursor A/B, background interval statistics, FFT, and plot-settings persistence
are implemented. RFQ #1 also provides independent data sources, time alignment,
multi-source save/restore, FFT overlay, and source management without a fixed
registration limit. The current stage is broader operational validation and
distribution preparation; packaged-release qualification remains incomplete.

### Verification completed

On 2026-10-28, revision `aa5fd00` passed **240 default tests**, **243 tests with
`soak-validation` enabled**, an offline locked release build, and formatting checks.
The feature-enabled count includes the default tests. Native GUI and performance
evidence comes from the earlier runs below.

The 2026-10-28 relocation to `apps/panel-y/` also passed both test suites, the
release build, and formatting checks. CLI checks loaded the sample and both
multi-source fixtures through the renamed binary/launcher. A native macOS smoke
check displayed the 100k-row sample and restored its five-row / four-channel
plot configuration. See the [relocation work log](study/diary/2026-10-28.md#rust本体の移動と実行ファイル名変更).

| Area | Verified scope | Results |
|---|---|---|
| Large-data statistics | Cancellable background statistics; 10M rows × 9 channels, then 1/2/4 sources up to 36M total rows / 36 channels; independent NumPy comparisons and memory checks | [Single-source](docs/rust_interval_statistics_async_results.md), [multi-source](docs/rust_interval_statistics_multi_source_results.md) |
| Multi-source GUI | All ten checks passed, including three-source FFT; source management and waveform display exercised with up to four sources | [GUI validation](docs/rust_rfq_1_multi_source_ui_results.md) |
| Compact analysis panel | Native macOS 1280×832, three sources / 12 channels: Statistics/FFT/Split views, fixed FFT execution controls, and scrollbar access to the full statistics table | [Panel validation](docs/rust_compact_analysis_panel_results.md) |
| Motor-log workflows | Three simulation logs, 740k total rows / 12 displayed channels; repeated preset/config restoration and statistics/FFT checks | [Continuous-use validation](docs/rust_motor_log_continuous_validation_results.md) |
| Long session | 60 minutes / 30 scripted native removal/restore cycles plus 120 seconds after final removal; raw-buffer release at all 31 empty checkpoints; 3,906 numeric comparisons and 1,581 exact count/index checks passed | [Long-session validation](docs/rust_long_session_validation_results.md) |

The rendering follow-ups reduced the 36-channel warm CPU-frame p95 to 11.56 ms
while statistics were pending. The latest initial Line-tile build comparison
reduced the first CPU frame from 168.18 to 95.24 ms median. These are
workload-specific CPU timings and exclude OS/GPU presentation. The macOS launcher
also configures prompt release of freed large allocations. See the
[rendering and memory results](docs/rust_render_memory_results.md),
[cold-render results](docs/rust_cold_render_results.md), and
[tile-build results](docs/rust_tile_build_results.md).

### Remaining validation and delivery work

- Physical measurement logs: the motor-log checks used simulation outputs.
- Larger source counts and workloads: GUI checks cover up to four sources;
  performance with more than four large sources remains unvalidated.
- Always-visible, unlocked long GUI runs and multi-hour operation. The 60-minute
  run detected a locked Mac at about 45 minutes; later evidence covers native
  UI-frame/state logs and post-unlock screenshots, not continuous screen
  presentation or OS pointer input. Sustained RSS growth was not observed in
  that workload, which does not establish leak-free operation for arbitrary runs.
- Remaining first-frame latency, OS/GPU presentation, different DPI settings,
  and smaller native windows. Native horizontal-wheel scrolling in the analysis
  table remains unconfirmed; the scrollbar path passed.
- Launchable Rust packages and per-OS validation for the selected distribution
  targets. CSV/MAT conversion remains external to the viewer.

See the [overall roadmap](docs/wave_viewer_roadmap.md) for current boundaries and
optional extensions. Automatic alignment, source alias editing/reordering, and
additional analysis features remain candidates without a committed delivery order.

## Tech Stack

- Rust (edition 2024)
- [`eframe`](https://crates.io/crates/eframe) / `egui` for the desktop GUI
- [`arrow-array`](https://crates.io/crates/arrow-array) / [`parquet`](https://crates.io/crates/parquet) for column-oriented I/O
- [`rfd`](https://crates.io/crates/rfd) for the native file picker
- `rustfft` for FFT analysis
- Standard input format: Parquet (one `time` column plus numeric channel columns)

## Repository Layout

```
panel-y/
├── apps/
│   └── panel-y/            # Current native viewer; Cargo package: panel-y
│       ├── Cargo.toml
│       ├── Cargo.lock
│       ├── run_macos.sh    # Release launcher with allocator configuration
│       ├── assets/         # Bundled UI images
│       ├── src/
│       │   ├── main.rs
│       │   ├── workspace_data.rs
│       │   ├── plot_config.rs
│       │   ├── interval_stats.rs
│       │   ├── interval_stats_bench.rs
│       │   ├── interval_stats_multi_bench.rs
│       │   ├── cold_render_bench.rs
│       │   ├── soak_validation.rs # Optional native validation driver
│       │   ├── fft_analysis.rs
│       │   ├── parquet_schema.rs
│       │   └── parquet_waveform.rs
│       └── README.md       # CLI flags and benchmark commands
├── archive/                # Archived Python/Dash prototypes and dependencies
│   ├── proto_3_1/          # Legacy Python/Dash prototype
│   └── proto_3_1b/         # Legacy Python/Dash viewer
├── data/                   # Shared samples, validation data, and plot configs
├── tools/                  # Active Python tools (separate dependencies)
│   ├── preprocess/         # CSV/MAT/Parquet conversion and build helpers
│   ├── data_generation/    # Sample and benchmark fixture generators
│   ├── validation/         # Independent numerical checks
│   ├── tests/              # CLI integration checks
│   └── requirements.txt
├── docs/                   # Design notes, roadmap, bug reports
├── study/diary/             # Codex work logs and validation screenshots
├── AGENTS.md               # Codex working rules
└── CLAUDE.md               # Claude Code working rules
```

The Python viewers under [archive/](archive/README.md) are retained as historical references.
Active preprocessing and fixture generation live in [tools/](tools/README.md),
with dependencies separate from Dash/Plotly. Shared datasets live in
[data/](data/README.md). The Rust viewer opens Parquet directly; CSV/MAT conversion
is a separate Python preprocessing step.

The Rust app moved from `code/rust_phase1/` to `apps/panel-y/` on 2026-10-28.
The Cargo package and executable changed from `rust_phase1` to `panel-y`; the
release executable is `apps/panel-y/target/release/panel-y` (`panel-y.exe` on
Windows). Historical benchmark reports and work logs retain their original paths
and revisions. Use the commands here or in the viewer README for the current
checkout. The current Python tools were subsequently separated into `tools/`,
and the sample/validation data moved from `code/proto_3_1b/data/` to `data/`.
The remaining Python prototypes and their requirements moved from `code/` to
`archive/`; the `code/` directory is no longer used.
Regenerate benchmark configs or update external presets that contain the old
absolute paths; the bundled plot configs use paths relative to their own folder.

## Run

```bash
cd apps/panel-y
cargo run --release
```

Pick a `.parquet` file in Data Sources, inspect its schema, and load channels from
the channel browser. Use `Add Source` to compare another file in the same workspace.

For the validated macOS large-allocation release behavior, build and use the
launcher instead (from the repository root):

```bash
cargo build --release --locked --manifest-path apps/panel-y/Cargo.toml
./apps/panel-y/run_macos.sh
```

The launcher defaults `MallocLargeCache` to `0` before process startup and honors
an explicit caller override. Direct `cargo run` launches do not apply this setting.

Headless utilities (run from `apps/panel-y/`):

```bash
# Inspect a Parquet file's schema without opening the GUI
cargo run --release -- --schema <path/to/file.parquet>

# Load one channel and exit (sanity-check the narrow read path)
cargo run --release -- --load-channel <path/to/file.parquet> <channel_name>

# Benchmarks
cargo run --release -- --bench-channel       <path> <channel>
cargo run --release -- --stress-channel      <path> <channel> <iterations>
cargo run --release -- --bench-multi-channel <path>
cargo run --release -- --bench-phase2        <path>
cargo run --release -- --bench-interval-stats <path> [channels...]
cargo run --release -- --bench-interval-stats-multi <config.json>
cargo run --release -- --bench-cold-render <config.json>
cargo run --release -- --bench-multi-source  <path1> <path2>
```

See the [Rust viewer README](apps/panel-y/README.md) for CLI details and reference datasets.

## Current Capabilities

### Waveform display and navigation

- Native desktop shell with a multi-row waveform area sharing one X-axis.
- Parquet schema inspection: row count, row groups, column count, numeric channels, time column, time range, estimated sample rate.
- Selective channel loading — only `time` (`f64`) plus the chosen channel (`f32`) is read, not the whole file.
- Full-range min/max envelope plus visible-range re-extraction sized to plot width, so pan/zoom stays responsive on multi-million-sample logs.
- Step rendering with three regimes: exact raw samples at low density, cached exact change points at medium density, and a Step-specific dense min/max envelope for very high edge counts.
- Per-row channel visibility, color override, and line-width controls.
- Per-row Y range with auto/manual modes, seeded from the latest auto range.
- Hover X synchronized across rows with a vertical hover line and source-specific value readouts. Values outside a source's time range are left absent.
- Pan/zoom: left-drag X pan, `Shift + left-drag` X-only box zoom, horizontal-scroll X pan, `Ctrl/Cmd + wheel` (or trackpad pinch) X zoom, double-click to reset. Plain vertical wheel scrolls the row stack.
- Channel browser with text filter, state filter (`All / Loaded / Loading / Unloaded`), sort modes, and inline `Load` / `Add to row` actions.
- Cache panel: per-channel `Add to row` / `Unload`, plus `Unload all channels` and `Clear draw caches`.

### Multiple sources and time alignment

- Add independent Parquet sources with different time arrays, sample rates, and sample counts; overlay arbitrary channels in the same row using Line or Step.
- Source-numbered channel labels and matching-channel addition across all sources.
- Per-source `Offset`, `Reset`, and `Start at zero`, plus `Start all at zero`. These change display time while preserving raw timestamps and data.
- `Fit All` fits the union of loaded time ranges; `Fit Overlap` fits their common interval after offsets. No overlap or endpoint-only contact disables `Fit Overlap`.
- Scrollable source cards with no fixed registration limit. Removing a source keeps the remaining sources' identities and data intact.

### Cursor and FFT analysis

- Cursor A/B, time difference and reciprocal frequency, raw sample values, and interval Mean / Min / Max / P-P / RMS / slope for visible loaded channels.
- FFT of the Cursor A/B interval with Hanning, Hamming, or Rectangular windows, amplitude/dB display, frequency-range controls, plot pan/zoom, and Hover.
- Up to **three FFT target channels in total**, across any loaded sources. Each source runs an independent background job and retains its own sample interval and frequency bins; spectra share display axes without resampling.
- Input changes invalidate obsolete FFT results. Cancellation and request identities prevent old jobs from restoring stale results; a source failure can coexist with other sources' successful spectra.
- Interval statistics are cached; cache misses run in one cancellable background job using shared raw buffers. Cursor/source/visibility changes cancel obsolete work and automatically calculate the latest request. The panel provides Cancel / Retry; hiding or collapsing it stops pending work.
- The analysis panel offers Statistics, FFT, and adjustable Split views, with a 360px initial height. FFT Run/Cancel stays above the scrolling body; target and result details are collapsible. Statistics supports vertical scrolling and a horizontal scrollbar for all value columns.

### Save and restore

| Mechanism | Saved state | Lifetime and behavior |
|---|---|---|
| Plot config (`.pyc.json`, `rust.version: 2`) | Source order, paths, labels, offsets, complete row layout, visibility, Line/Step, styles, and Y ranges | File persistence. Load restores sources and fits their full display-time union after loading. The top-level v1 fields retain a first-source projection for legacy readers. |
| Session preset | Sources and offsets, row settings, selected source/channel/row, X range, and preview settings | In-memory snapshot for the current application run. Apply restores the saved view; exiting the app discards it. |

Both support partial restoration with warnings when data is missing, and preserve
the current workspace if every saved source is unavailable. Plot config
`Apply to Current Data` requires unique path matches and preserves current offsets
and raw caches. Version 1 Load remains supported; v1 Apply requires one current source.
Cursor/FFT analysis state is not saved by either mechanism, and arbitrary X ranges
are restored by Session preset rather than persisted in Plot config.

## Reference Data

Sample datasets live under [data/](data/README.md). Generators are in
`tools/data_generation/`; install [the tools dependencies](tools/README.md#setup) first:

```bash
# From the repository root, with the tools virtual environment active
python tools/data_generation/generate_test_data.py
python tools/data_generation/generate_step_validation_data.py
python tools/data_generation/generate_ripple_validation_data.py
python tools/data_generation/generate_multi_source_validation_data.py
```

Useful targets:

- `test_100k.parquet` — small smoke-test file
- `panely_large_10s_1mhz_9ch.parquet` — large multi-channel benchmark target
- `step_validation_100k.parquet` — Step-rendering edge-position checks
- `ripple_validation_2_4m.parquet` — ripple envelope checks
- `multi_source_validation_1.parquet` / `multi_source_validation_2.parquet` — different time ranges and 50 kHz / 64 kHz sampling for overlay and alignment checks

Generated Parquet files are excluded from Git. Generators accept `--output-dir`
to use another directory and refuse to overwrite existing files without `--force`.
The interval-statistics generator instead reuses matching files and rejects
unrelated files. See the [tools guide](tools/README.md) for commands and scope.

## Development Notes

- [Archived prototypes](archive/README.md): legacy setup, launch commands, and data paths.
- [Python tools](tools/README.md): setup, preprocessing, generation, validation, and tests.
- [Preprocessing guide](docs/preprocessing_guide.md): CSV/MAT conversion options.
- [Overall roadmap](docs/wave_viewer_roadmap.md): current capabilities, validation boundaries, and remaining candidates.
- [Rust migration plan](docs/rust_migration_plan.md): migration milestones and remaining operational work.
- [RFQ #1 work plan](docs/rust_rfq_1_multi_source_phase1_work_plan.md): detailed multi-source status and linked results.
- [Rust viewer README](apps/panel-y/README.md): feature behavior and benchmark commands.
- [Codex work logs](study/diary/): dated implementation notes and GUI evidence.

From the repository root:

```bash
cargo test --offline --locked --manifest-path apps/panel-y/Cargo.toml
cargo test --offline --locked --manifest-path apps/panel-y/Cargo.toml --features soak-validation
cargo fmt --check --manifest-path apps/panel-y/Cargo.toml
cargo build --release --offline --locked --manifest-path apps/panel-y/Cargo.toml
```

Offline commands require the dependencies to have been downloaded already.
The `soak-validation` feature adds the native long-session driver and its tests;
normal builds exclude the driver. Unit tests do not run the 60-minute GUI workload.
See the [long-session reproduction instructions](docs/rust_long_session_validation_results.md#再現方法)
for the separate feature-enabled build, input configuration, and measurement steps.
