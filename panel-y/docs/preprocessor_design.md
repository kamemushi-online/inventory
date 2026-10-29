---
project: Panel_y
doc_type: Design Spec
target: Preprocessing Utility
created: "2026-03-15"
updated: "2026-10-28"
status: Implemented
---

# Preprocessing Utility Design

The current implementation moved to `tools/preprocess/` on 2026-10-28.
Time-unit inference was subsequently removed: single-file conversion, batch
conversion, and the API default to seconds. See the
[operations guide](preprocessing_guide.md) and [tools setup](../tools/README.md)
for current commands.

---

## 1. Purpose

Convert input data of varying formats and quality into Panel_y's standard Parquet format.
**Panel_y itself only accepts Parquet**, so this utility absorbs every other difference.

---

## 2. Responsibilities and Non-responsibilities

| Responsibilities (does do) | Non-responsibilities (does not do) |
|----------------|------------------|
| Normalize the time column to `time` (in seconds) | Plot or visualize data |
| Drop non-numeric columns | Launch the Panel_y app |
| Interpolate/resample to uniform spacing | Analyze data or run FFT |
| Convert CSV / MAT → Parquet | Manage where files are saved |
| Convert time units (ms, us → s) | |

---

## 3. Implemented Features

### Feature A: CSV → Parquet conversion

**Input**: a CSV file (any column layout)
**Output**: Panel_y standard Parquet

Steps:
1. Read with `pandas.read_csv()`
2. Identify the time column (see "Time column detection" below)
3. Normalize the time column to `time` in seconds
4. Drop non-numeric columns
5. Check uniform spacing → interpolate if non-uniform
6. Save with `to_parquet(index=False)`

---

### Feature B: MATLAB .mat → Parquet conversion

**Input**: a .mat file (≤ v7.2 or ≥ v7.3)
**Output**: Panel_y standard Parquet

Steps:
1. Detect version
   - ≤ v7.2: `scipy.io.loadmat()`
   - ≥ v7.3: load with `h5py`
2. Enumerate variable names and identify the time variable (below)
3. Normalize the time to `time` in seconds
4. Extract only numeric arrays and register them as channels
5. Check uniform spacing → interpolate if non-uniform
6. Save with `to_parquet(index=False)`

---

### Feature C: Time column detection and normalization

**Goal**: identify the time axis correctly even when the input is not named `time`.

Detection priority:
1. Column named `time` (exact match) → use as-is
2. Column named `t`, `Time`, `TIME`, `timestamp`, `elapsed`, or `ECUTIME` → rename to `time`
3. Otherwise → raise an error requesting an explicit `--time-col` argument

Time units are explicit; they are never inferred from the values:

| Input unit | Selection | Conversion |
|---------|--------|------|
| seconds (s) | Default, or `--time-unit s` | none |
| milliseconds (ms) | `--time-unit ms` | ÷ 1,000 |
| microseconds (μs) | `--time-unit us` | ÷ 1,000,000 |

The CLI defaults to `s`. Omitting `time_unit` or passing `None` to the API also
treats the input as seconds. The first timestamp is preserved after scaling;
normalization does not shift the start to zero.

---

### Feature D: Drop non-numeric columns

- Decision is based on `pandas.api.types.is_numeric_dtype()`
- Non-numeric columns are logged and then dropped
- The time column is exempt from removal

---

### Feature E: Uniform-spacing check / interpolation

**Check**:
```python
diffs = time.diff().dropna()
is_uniform = (diffs.max() - diffs.min()) / diffs.mean() < 1e-6  # within 1 ppm relative error
```

**If non-uniform**:
1. Generate a new uniform time axis spanning `time[0]` to `time[-1]`
   - Sampling period: `(time[-1] - time[0]) / (len(time) - 1)`
2. Linearly interpolate every channel with `numpy.interp()`
3. Log that the conversion happened

---

## 4. File Layout

```
tools/preprocess/
├── convert.py             # Single-file CLI
├── multiconverter.py      # Batch MAT CLI
├── build_macos.sh
├── build_windows.bat
└── utils/
    ├── __init__.py
    ├── converter.py       # Conversion entry point (public API)
    ├── reader_csv.py      # Feature A: CSV reader
    ├── reader_mat.py      # Feature B: MATLAB reader
    ├── time_normalizer.py # Feature C: time-column detection / normalization
    ├── column_filter.py   # Feature D: drop non-numeric
    └── resampler.py       # Feature E: uniform-spacing check / interpolation
```

---

## 5. Public API (`converter.py`)

```python
def convert_to_parquet(
    input_path: str | Path,
    output_path: str | Path,
    time_col: str | None = None,   # Explicit time-column name (auto-detect if omitted)
    time_unit: str | None = None,  # "s" | "ms" | "us"; None means seconds
) -> Path:
    """
    Convert any input file into a Panel_y standard Parquet and save it.
    Returns: the path of the written Parquet file.
    """
```

---

## 6. CLI Interface

Commands below run from the repository root with the tools environment active.

### Single-file conversion (`python tools/preprocess/convert.py`)

```bash
# Basic usage
python tools/preprocess/convert.py input.csv output.parquet

# Specify time column and unit explicitly
python tools/preprocess/convert.py input.csv output.parquet --time-col elapsed --time-unit ms

# MAT file
python tools/preprocess/convert.py data.mat output.parquet
```

### Batch conversion (`multiconverter.py`)

CLI that converts every `.mat` directly under a folder into Parquet.
Internally calls `converter.convert_to_parquet()`.

```bash
# Basic
python tools/preprocess/multiconverter.py --input ./input --output ./output

# Overwrite existing .parquet files
python tools/preprocess/multiconverter.py --input ./input --output ./output --force
```

| Option | Required | Description |
|-----------|------|------|
| `--input` | ○ | Folder that contains the .mat files to convert |
| `--output` | ○ | Output folder for Parquet (created if missing) |
| `--time-unit` | — | `s` (default), `ms`, or `us`; batch conversion does not infer the unit |
| `--force` | — | Overwrite even if a .parquet of the same name already exists |

---

## 7. Error-handling Policy

| Situation | Response |
|------|------|
| `time` column cannot be found | Raise an error / instruct the user to use --time-col |
| Zero waveform channels (all non-numeric or only time) | Raise an error and stop |
| Non-numeric column present | Log a warning, drop it, and continue |
| Non-uniform data | Log a warning, interpolate, and continue |
| MAT version unknown | Try v7.2, then v7.3; if both fail, raise an error |

---

## 8. Decisions (formerly open questions)

| Item | Decision |
|------|---------|
| Input time unit | Default to seconds in all entry points. Convert ms/us only when explicitly specified; no inference from values. |
| Interpolation method | Linear interpolation via `numpy.interp()`. |
| CLI shape | Adopt `python tools/preprocess/convert.py` (argparse). `multiconverter.py` is provided as an additional CLI. |
