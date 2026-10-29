# Archived Python prototypes

The remaining contents of `code/` moved here on 2026-10-28. These are historical
Dash/Plotly implementations retained for reference and comparison. Current
development uses the [Rust viewer](../apps/panel-y/README.md), with active Python
preprocessing and data generation in [tools/](../tools/README.md).

| Directory | Scope | Default data directory |
|---|---|---|
| [proto_3_1/](proto_3_1/) | Earlier viewer, FFT/theme support, original conversion tools | `archive/proto_3_1/data/` |
| [proto_3_1b/](proto_3_1b/) | Large-data viewer and its backend benchmark | Repository `data/` |

Both retain their adjacent `assets/` directories. The common `requirements.txt`
and per-prototype copies are preserved. The older converter and generator in
`proto_3_1/` are snapshots; use `tools/` for current preprocessing and fixtures.

## Setup

From the repository root, use Python 3.10 or newer:

```sh
python3 -m venv archive/.venv
source archive/.venv/bin/activate
python -m pip install -r archive/requirements.txt
```

On Windows, activate with `archive\.venv\Scripts\activate` in Command Prompt.
This environment includes Dash/Plotly in addition to the numerical libraries;
the active Python tools have their own smaller dependency definition.

## Run a viewer

With the archive environment active, start one viewer at a time:

```sh
cd archive/proto_3_1b
python app.py
```

Open `http://localhost:8050`. For the earlier viewer, use
`cd archive/proto_3_1` from the repository root instead. Both use port 8050 and
the existing Dash development mode. Stop the process with Ctrl+C.

Load a Parquet by entering its path. The newer prototype defaults to the shared
`data/` directory; the earlier one keeps its own historical data directory. An
absolute path to `data/test_100k.parquet` works in either viewer. Generate missing
shared fixtures using the [data guide](../data/README.md).

The archived viewers resolve relative Plot config `data_path` values from the
process working directory. The Rust viewer instead resolves them from the config
file's directory. The old `proto_3_1/data/sample_waveform.pyc.json` now uses
`data/sample_waveform.parquet`, for launches from `archive/proto_3_1/`; the original
1k-row Parquet is not bundled. It needs `time`, `voltage_u`, and `current_u` columns.
Use absolute data paths in configs when comparing viewers from different working
directories. Shared Rust configs should be copied before adapting them for a
legacy viewer.

## Backend benchmark

After generating the medium fixture, run from the repository root:

```sh
python tools/data_generation/generate_phase1_datasets.py medium
python archive/proto_3_1b/benchmark_phase1_datasets.py medium
```

The generator refuses to overwrite existing data unless `--force` is supplied;
skip it when the fixture already exists. The benchmark also accepts `large`.
It calls the archived Dash backend directly; browser rendering is not measured.

Historical result reports, execution prompts, and work logs retain their original
paths and revisions. When locating archived source, map `code/proto_3_1*` to
`archive/proto_3_1*`. Former active tools and shared data are now in `tools/` and
`data/`, respectively; they are not under `archive/proto_3_1b/`.

The 2026-10-28 relocation check covered both viewers' HTTP/assets, sample loading,
Line/Step figure generation, FFT, and browser waveform display, plus the 2M-row
backend benchmark. See the [development log](../study/diary/2026-10-28.md#旧pythonプロトタイプのアーカイブ)
for environment details and evidence. This does not renew historical performance
claims or qualify the archived packaging scripts for distribution.
