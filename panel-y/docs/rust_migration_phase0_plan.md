---
project: Panel_y
doc_type: Detailed Plan
target: Rust Migration Phase 0
created: "2026-04-27"
updated: "2026-04-27"
status: Complete
source_proto: "archive/proto_3_1b"
parent_plan: "docs/rust_migration_plan.md"
---

# Rust Migration Phase 0 Detailed Plan — Freezing the `proto_3_1b` Specification

---

## 1. Purpose

In Phase 0, before any Rust implementation begins, we freeze `proto_3_1b` as the specification reference for the Rust port.

No Rust code is written at this stage. The goal is to make it unambiguous, for subsequent phases, "what we will reproduce," "what we will benchmark," and "which results are the ground truth."

---

## 2. Completion Criteria

Phase 0 is considered complete once all of the following are in place:

- A feature checklist for `proto_3_1b` exists
- Specifications for the main UI operations are recorded in text and/or screenshots
- The saved fields and compatibility policy for the `.pyc.json` plot configuration are organized
- Representative small, medium, and large datasets are defined
- Benchmark items for the Dash/Plotly baseline measurements are defined
- Numerical comparison references for Cursor, statistics, and FFT are defined
- The scope to implement in Rust Phase 1, and what is deferred, is decided

---

## 3. Deliverables Produced in Phase 0

| Deliverable | Proposed Location | Contents |
|--------|----------|------|
| Feature checklist | `docs/rust_migration_phase0_feature_checklist.md` | List of features to be reproduced in the Rust version |
| UI operation spec | `docs/rust_migration_phase0_ui_spec.md` | Screen layout, operations, and expected behavior of the current UI |
| Plot configuration spec | `docs/rust_migration_phase0_plotconfig_spec.md` | `.pyc.json` fields, compatibility policy, and samples |
| Dataset definition | `docs/rust_migration_phase0_dataset_plan.md` | Purpose, creation/storage policy for small/medium/large data |
| Benchmark plan | `docs/rust_migration_phase0_benchmark_plan.md` | Measurement items, procedures, and pass criteria |
| Numerical reference plan | `docs/rust_migration_phase0_analysis_reference.md` | Comparison targets and tolerances for statistics/FFT |
| Phase 1 scope definition | `docs/rust_migration_phase1_scope.md` | Implementation scope of the Rust vertical-slice prototype |

Because large data files and screenshots may not be tracked in Git, the documents themselves record their storage locations and procedures for regenerating them.

---

## 4. Work Breakdown

### 0-1. Confirm the Targets of the Spec Freeze

Targets:

- [archive/proto_3_1b/app.py](../archive/proto_3_1b/app.py)
- [archive/proto_3_1b/assets/style.css](../archive/proto_3_1b/assets/style.css)
- [archive/proto_3_1b/assets/resize_sync.js](../archive/proto_3_1b/assets/resize_sync.js)
- [tools/preprocess/utils/converter.py](../tools/preprocess/utils/converter.py)
- [docs/proto_3_1b_app_intro_slides.md](../docs/proto_3_1b_app_intro_slides.md)
- [docs/wave_viewer_roadmap.md](../docs/wave_viewer_roadmap.md)

Confirmation items:

- Treat `proto_3_1b` as the feature-complete version
- Confirm the diff against `proto_3_1` and clearly state that the ground truth for the Rust port is `proto_3_1b`
- Surface any known bugs or unmerged differences

Note:

- `proto_3_1` ships an exe-packaged `multiconverter.py`, while `proto_3_1b` still uses the older CLI style.
  For the Rust viewer spec freeze, prioritize `proto_3_1b`; the conversion CLI is handled separately.

Deliverable:

- `docs/rust_migration_phase0_feature_checklist.md`

---

### 0-2. Build the Feature Checklist

Categories:

- Data loading
- Row layout
- Waveform rendering
- Channel display settings
- X/Y axis operations
- Hover synchronization
- Measurement mode
- Interval statistics
- FFT
- Plot configuration save/load
- Theme / header / shortcuts
- Preprocessing / conversion tools

Each feature is tagged with the following:

| Item | Description |
|------|------|
| Current state | Working in `proto_3_1b` / Needs verification / Known risk |
| Rust priority | Phase 1 / Phase 2 / Phase 3 / Phase 4 / Deferred |
| Reproduction fidelity | Full reproduction / Spec change allowed / Candidate for removal |
| Verification method | Manual operation, numerical comparison, screenshot, etc. |

Deliverable:

- `docs/rust_migration_phase0_feature_checklist.md`

---

### 0-3. Record the UI Operation Specification

Screens and operations to record:

- Initial screen
- Loading a Parquet file
- Path suggestion display
- Adding / removing rows
- Overlaying multiple channels
- Step display configuration
- Color / line-width configuration
- Y-axis range configuration
- Update button / Ctrl+Enter
- Toggling the scroll-zoom lock
- Theme switching
- Pinning the header
- Hover value display
- X-axis zoom/pan synchronization
- Toggling measurement mode
- Placing Cursor A/B
- Interval statistics display
- Running FFT
- Saving the plot configuration
- Loading the plot configuration

How to record:

- Operation specs are written in Markdown
- Place screenshots in `docs/rust_migration_phase0_assets/` as needed
- If large images are not tracked in Git, record only the procedure to regenerate them

Deliverable:

- `docs/rust_migration_phase0_ui_spec.md`

---

### 0-4. Organize the `.pyc.json` Plot Configuration Spec

Items to organize:

- `version`
- `data_path`
- `rows`
- `rows[].channels`
- `rows[].ymin`
- `rows[].ymax`
- `rows[].step_channels`
- `ch_styles`
- `ch_styles.{channel}.color`
- `ch_styles.{channel}.width`
- `theme`
- `scroll_zoom`

Decisions to make:

- The Rust version must read the existing `.pyc.json` as a compatibility target
- If Rust-specific fields are needed, bump to `version: 2`
- Keep the `.pyc.json` extension compatible for the time being
- Consider `.panely.json` as the future official extension

Deliverables:

- `docs/rust_migration_phase0_plotconfig_spec.md`
- A list of sample configuration file paths

---

### 0-5. Define Representative Datasets

Dataset categories:

| Class | Rough size | Purpose |
|------|------|------|
| Small | 100k points | Fast iteration during development; CI candidate |
| Medium | A few million points | Feel close to real-world use |
| Large | ~90M points | Performance ceiling, memory, sustained zoom checks |

Existing candidates:

- [data/test_100k.parquet](../data/test_100k.parquet)
- [tools/data_generation/generate_test_data.py](../tools/data_generation/generate_test_data.py)

Decisions to make:

- Whether medium/large datasets are real instrument logs or synthetic data
- Large files are not tracked in Git
- Document the generation script or acquisition source
- Record the channel count, sampling frequency, and recording length for each dataset

Deliverable:

- `docs/rust_migration_phase0_dataset_plan.md`

---

### 0-6. Plan the Dash/Plotly Performance Baseline

Measurement items:

- Application startup time
- Parquet load time
- Time to first paint
- Memory footprint
- Response time after one zoom action
- Responsiveness under sustained zooming
- Zoom-reset time
- FFT execution time
- Plot configuration load time

Targets:

- Small data
- Medium data
- Large data

How to record:

- Measurement timestamp
- OS / CPU / RAM
- Python version
- Dash / Plotly / pandas / numpy versions
- Dataset information
- Procedure
- Results

Deliverables:

- `docs/rust_migration_phase0_benchmark_plan.md`
- Later, create `docs/rust_migration_phase0_benchmark_results.md`

---

### 0-7. Define Numerical References for Analysis Results

Targets:

- Cursor A/B values
- Δt
- Frequency conversion
- Mean
- Max / Min
- P-P
- RMS
- Slope
- FFT peak frequency
- FFT amplitude
- dB display

Decisions to make:

- Which dataset to use for comparison
- Fixed timestamps for Cursor A/B
- FFT target channel
- Window function
- Frequency range
- Tolerance

Deliverable:

- `docs/rust_migration_phase0_analysis_reference.md`

---

### 0-8. Define the Phase 1 Scope

To be implemented in Phase 1:

- Rust application startup
- Parquet loading
- Channel list retrieval
- Selecting one channel
- Single-row waveform display
- Pan / zoom
- Min/max envelope
- Baseline performance check on ~90M-point data

Not implemented in Phase 1:

- Multi-row UI
- Multi-channel overlay
- Cursor
- FFT
- Plot configuration save/load
- Theme switching
- Reimplementing the preprocessing CLI

Deliverable:

- `docs/rust_migration_phase1_scope.md`

---

## 5. Recommended Order

1. Build the feature checklist
2. Write the UI operation spec
3. Write the plot configuration spec
4. Define the datasets
5. Write the benchmark plan
6. Write the analysis reference plan
7. Finalize the Phase 1 scope

Rationale for this order:

- Without first freezing features and UI, downstream benchmarking and Rust scope inevitably drift
- The plot configuration affects compatibility, so it must be frozen early
- Benchmark design is more reproducible once the target data is fixed

---

## 6. Non-Goals of Phase 0

The following are out of scope in Phase 0:

- Rust implementation
- Final decision on the GUI library
- Large-scale rework of the Python version
- Adding features to `proto_3_1b`
- Adding large data files to Git
- Building out a full automated test infrastructure

That said, minor investigations and helper measurement scripts that are needed to freeze the spec are handled as separate tasks as the need arises.

---

## 7. Decision Gate

At Phase 0 completion, we decide on the following:

| Decision | Condition |
|------|------|
| Start Phase 1 | Reproduction targets, datasets, benchmark items, and Phase 1 scope are clear |
| Extend Phase 0 | Too many open questions in the current spec, or the large-data baseline is undecided |
| Reconsider port direction | The UI/rendering requirements to reproduce clearly mismatch the candidate Rust GUI stacks |

---

## 8. First File to Tackle

The first deliverable to produce is `docs/rust_migration_phase0_feature_checklist.md`.

Reasons:

- It is the foundation for deciding the Rust implementation scope
- The UI spec, plot configuration, benchmarks, and Phase 1 scope all depend on this list
- It lets us explicitly articulate what "complete" means for `proto_3_1b`

---

## 9. Execution Results

The following deliverables were produced for Phase 0.

| Deliverable | Path |
|--------|------|
| Feature checklist | `docs/rust_migration_phase0_feature_checklist.md` |
| UI operation spec | `docs/rust_migration_phase0_ui_spec.md` |
| Plot configuration spec | `docs/rust_migration_phase0_plotconfig_spec.md` |
| Dataset definition | `docs/rust_migration_phase0_dataset_plan.md` |
| Benchmark plan | `docs/rust_migration_phase0_benchmark_plan.md` |
| Numerical reference | `docs/rust_migration_phase0_analysis_reference.md` |
| Phase 1 scope definition | `docs/rust_migration_phase1_scope.md` |

Notes:

- Adopted `test_100k.parquet` as the small dataset
- Defined `panely_medium_2m_8ch` as the medium dataset spec
- Defined `panely_large_10s_1mhz_9ch` as the large dataset spec
- Computed and recorded fixed reference values for interval statistics and FFT
- Measured results for the Dash/Plotly version have not yet been collected; only the measurement plan was prepared
- The actual files for the medium and large datasets have not yet been generated; they will be prepared as real instrument logs or synthetic data before Phase 1
