# Shared sample and validation data

This directory holds the datasets used by the Rust viewer, Python generators,
and legacy [archive/proto_3_1b](../archive/README.md) viewer. It moved from `code/proto_3_1b/data/` on
2026-10-28. The data files retain their contents and names.

Generated Parquet files are excluded from Git. Plot configs, this guide, and the
motor-log exclusion rule are tracked. A fresh checkout therefore needs fixture
generation before data-dependent Rust tests or the sample plot configs can run.

## Generate fixtures

First follow the [Python tools setup](../tools/README.md#setup). From the repository
root with that environment active:

```sh
# Required by the Rust sample and multi-source tests
python tools/data_generation/generate_test_data.py
python tools/data_generation/generate_multi_source_validation_data.py

# Optional visualization / benchmark datasets
python tools/data_generation/generate_step_validation_data.py
python tools/data_generation/generate_ripple_validation_data.py
python tools/data_generation/generate_phase1_datasets.py medium
python tools/data_generation/generate_phase1_datasets.py large

# Four sources (36M rows total), plus configs in a separate report directory
python tools/data_generation/generate_interval_statistics_sources.py build/interval-stats
```

All generators default to this directory independently of the caller's working
directory. Use `--output-dir` to select another destination. Existing outputs
require `--force` to replace, except interval-statistics sources: those are reused
only when their schema, row count, and provenance metadata match.

## Plot configs and other datasets

| Config | Required dataset | Generator |
|---|---|---|
| [test_100k.pyc.json](test_100k.pyc.json) | `test_100k.parquet` | `generate_test_data.py` |
| [multi_source_validation_1_v1.pyc.json](multi_source_validation_1_v1.pyc.json) | `multi_source_validation_1.parquet` | `generate_multi_source_validation_data.py` |
| [midium.pyc.json](midium.pyc.json) | `panely_medium_2m_8ch.parquet` | `generate_phase1_datasets.py medium` |

These configs refer to Parquets in the same folder; the Rust viewer resolves
their data paths relative to the config file. The existing `midium.pyc.json`
filename is retained for compatibility.

`motor_control_validation_20261026/` contains locally generated simulation CSVs,
Parquets, and conversion logs. Its existing `.gitignore` keeps those large files
out of Git. The [motor-log validation report](../docs/rust_motor_log_continuous_validation_results.md)
records their source and generation process; it is historical evidence and uses
the original directory name. These are simulated signals, not instrument logs.

## Paths after the move

From `apps/panel-y/`, the shared data directory is `../../data/`. For example:

```sh
cargo run -- --schema ../../data/test_100k.parquet
cargo run -- --load-channel ../../data/test_100k.parquet sine_50Hz
```

Historical benchmark reports and work logs retain the paths and revisions used
at the time. Regenerate interval-statistics configs after moving data, and update
external Plot configs or Session presets that contain the old absolute paths.
The full CLI reference is in the [viewer README](../apps/panel-y/README.md).
