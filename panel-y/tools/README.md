# Panel-y Python tools

The current CSV/MAT converters, fixture generators, and independent numerical
validator live here. They were separated from `code/proto_3_1b/` on 2026-10-28.
The Rust viewer is in `apps/panel-y/`; the legacy Dash viewers and their Python
benchmark are in [archive/](../archive/README.md). The older `proto_3_1` tool copies are historical
snapshots; use these tools for current work.

## Setup

Use Python 3.10 or newer. From the repository root:

```sh
python3 -m venv tools/.venv
source tools/.venv/bin/activate
python -m pip install -r tools/requirements.txt
```

On Windows, activate with `tools\.venv\Scripts\activate` in Command Prompt.
These tools require NumPy, pandas, SciPy, PyArrow, and h5py; they do not require
Dash or Plotly. The build helpers additionally install `requirements-build.txt`.
All examples below start from the repository root with this environment active.
For another working directory, use the absolute script path and input/output
paths appropriate to that directory.

## Preprocessing

```sh
# One CSV / MAT / Parquet file
python tools/preprocess/convert.py input.csv output.parquet --time-unit ms
python tools/preprocess/convert.py input.mat output.parquet --time-unit s

# Every .mat directly inside a folder
python tools/preprocess/multiconverter.py --input ./input --output ./output --time-unit s
```

Single-file conversion accepts `--time-col` and `--time-unit`. Both single-file
and batch conversion default to seconds; time units are never inferred from the
values. Specify `--time-unit ms` or `--time-unit us` for other input units.
Batch conversion skips existing output files unless
`--force` is supplied and returns a nonzero status if any input fails.
Single-file conversion retains its existing overwrite behavior.
Both normalize time, drop non-numeric channels, and interpolate nonuniform data.
See the [operations guide](../docs/preprocessing_guide.md) for details.

The conversion API remains `convert_to_parquet` in `preprocess/utils/converter.py`.
From the repository root it can be imported as
`from tools.preprocess.utils import convert_to_parquet`. The original
`python -m utils.converter` command also works from `tools/preprocess/`.
Omitting `time_unit` or passing `None` to the API treats the input time as seconds.

## Data generation

The default output directory is the repository's [data/](../data/README.md),
resolved from the tool location rather than the working directory. It is shared
by the Rust viewer and the legacy `proto_3_1b` viewer. Every generator accepts `--output-dir`.
Sample, Step, ripple, Phase 1, and multi-source generators require `--force` before
replacing existing outputs. The interval-statistics generator never overwrites
Parquet files: it reuses matching schema/row-count/provenance metadata and rejects
unrelated or incomplete files.

| Script in `data_generation/` | Output / scale |
|---|---|
| `generate_test_data.py` | 100k rows, four channels: sine, PWM, noise, chirp |
| `generate_step_validation_data.py` | 100k rows, three PWM channels with one-sample offsets |
| `generate_multi_source_validation_data.py` | Two sources, 100,001 / 80,001 rows, different sample rates and ranges |
| `generate_ripple_validation_data.py` | Default 2.4M rows, five ripple channels; supports `--rows`, `--fs`, `--chunk-size` |
| `generate_phase1_datasets.py` | `medium`: 2M rows / eight channels; `large`: 10M rows / nine channels; `all`: both |
| `generate_interval_statistics_sources.py` | Four sources, 36M total rows / 36 channels; writes benchmark configs into the required `report_dir` |

```sh
python tools/data_generation/generate_test_data.py
python tools/data_generation/generate_step_validation_data.py
python tools/data_generation/generate_multi_source_validation_data.py
python tools/data_generation/generate_ripple_validation_data.py
python tools/data_generation/generate_phase1_datasets.py medium

# Isolated output, leaving the default fixtures intact
python tools/data_generation/generate_multi_source_validation_data.py --output-dir /tmp/panel-y-fixtures

# Existing matching Parquets are reused; reports go into the specified directory
python tools/data_generation/generate_interval_statistics_sources.py build/interval-stats
```

New interval-statistics configs contain absolute source paths, so the Rust
benchmark can read them from any working directory. Regenerate configs after
moving their data. Historical reports retain their original paths and revisions.
Generated Parquet files are excluded from Git.

## Numerical validation

Build the Rust viewer first, then generate and check a statistics report:

```sh
apps/panel-y/target/release/panel-y --bench-interval-stats \
  data/test_100k.parquet sine_50Hz > /tmp/panel-y-stats.json
python tools/validation/validate_interval_statistics.py /tmp/panel-y-stats.json
```

The validator also accepts multi-source reports. Relative Parquet paths in a
report are resolved from the validator's working directory; absolute paths work
independently of that directory.

## Checks and packaging

```sh
python -m unittest discover -s tools/tests -v
bash tools/preprocess/build_macos.sh
```

The tests run CLI conversion and generation from temporary directories and check
the conversion API directly. They cover CSV time normalization/interpolation,
MAT and HDF5 inputs, Parquet input, seconds defaults regardless of time magnitude,
explicit s/ms/us conversion, batch skip/overwrite/error behavior, fixture output
directories, and overwrite protection. They do not replace the large-data
performance benchmarks.

The build helpers produce two standalone programs in `tools/preprocess/dist/`:

- `panel-y-convert` (`.exe` on Windows): single CSV / MAT / Parquet conversion.
- `multiconverter` (`.exe` on Windows): batch conversion of a folder's MAT files.

Run `bash tools/preprocess/build_macos.sh` on macOS or
`tools\preprocess\build_windows.bat` in Windows Command Prompt, with the tools
virtual environment active. The helpers install the build dependencies and call
`preprocess/package_converters.py`. Use an isolated environment to avoid bundling
optional packages from an unrelated project.

Each build runs six converter test suites against the actual executables with
external Python removed from PATH and with Japanese characters/spaces in the
working directory. Only a successful test run produces a distribution ZIP:

| Target | ZIP in `tools/preprocess/dist/` |
|---|---|
| Windows x64 | `panel-y-converters-windows-x64.zip` |
| macOS Apple Silicon | `panel-y-converters-macos-arm64.zip` |
| macOS Intel | `panel-y-converters-macos-x64.zip` |

The ZIP includes both programs, [user instructions](preprocess/DISTRIBUTION.md),
test results, executable checksums, build metadata, and license documents from
Python and the build environment's dependencies. A `.zip.sha256` file is
also generated next to the archive. Users do not need to install Python.
These packages have no publisher code signature or Apple notarization.
Build outputs are excluded from Git.

The [Build Python converters workflow](../.github/workflows/build-converters.yml)
builds and tests all three targets on their respective GitHub-hosted runners and
stores ZIPs as Actions artifacts for 30 days. It runs on pushes affecting the
packaging branch `codex/package-python-converters`; once the workflow is on the
default branch it can also be started manually using `workflow_dispatch`.

Validation on 2026-10-28 passed all nine Python integration tests and 240 Rust
tests. Seven generated Parquets matched the pre-move generator values; single-
and four-source Rust statistics passed the NumPy comparison. The macOS binary
built successfully and converted both classic MAT and flat HDF5 MAT samples.
Windows packaging had not been run at that point. Details and evidence are in the
[development log](../study/diary/2026-10-28.md#pythonツールの分離).

After removal of automatic time-unit inference, all 12 Python tests passed,
including the new default-unit and explicit-unit regression checks. See the
[unit-change log](../study/diary/2026-10-28.md#時間単位の自動推定を除去).

Packaging validation on 2026-10-28 passed all six executable test suites on
Windows x64, macOS arm64, and macOS Intel x64. The packages were built from
`2b53fd9` in [Actions run 40687090924](https://github.com/sugusokothx/panel-y/actions/runs/40687090924).
Downloaded ZIP checksums, executable architectures, included licenses, and source
hashes were verified (accounting for Windows checkout CRLF line endings).
See the [packaging log](../study/diary/2026-10-28.md#pythonコンバータの実行ファイル化).
