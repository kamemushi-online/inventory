# Panel_y Preprocessing Script Operations Guide

How to convert data (CSV / MAT) into the Parquet format that Panel_y can load.

| Script | Purpose |
|-----------|------|
| `tools/preprocess/convert.py` | Convert a single file (CLI) |
| `tools/preprocess/multiconverter.py` | Batch-convert every .mat in a folder (CLI) |

---

## Prerequisites

Use Python 3.10 or newer. Run the examples from the repository root.
The current tools are in `tools/preprocess/`; they no longer require the legacy
Dash application's virtual environment. See the [tools guide](../tools/README.md).

```bash
python3 -m venv tools/.venv
source tools/.venv/bin/activate
python -m pip install -r tools/requirements.txt
```

On Windows, activate with `tools\.venv\Scripts\activate` in Command Prompt.

---

## Conversion Pipeline

```
Input file (CSV / MAT / Parquet)
  │
  ├─ [1] Load file
  ├─ [2] Detect the time column (automatic or specified)
  ├─ [3] Normalize the specified time unit to seconds (default: s) → rename to "time"
  ├─ [4] Drop non-numeric columns
  └─ [5] Check uniform spacing → linearly interpolate to resample if non-uniform
         │
         ▼
     Output Parquet (time + numeric channels)
```

---

## How to Run

### 1. Activate the venv

```bash
# From the repository root
source tools/.venv/bin/activate
```

### 2. Run the conversion command

#### Convert a CSV file (basic)

```bash
python tools/preprocess/convert.py input.csv data/output.parquet
```

#### CSV — specify the time column explicitly

When the time column is not one of `time`, `t`, `Time`, `TIME`, `timestamp`, `elapsed`, `ECUTIME`:

```bash
python tools/preprocess/convert.py input.csv data/output.parquet --time-col <column-name>
```

#### CSV — specify the time unit explicitly

Time units are never inferred from the values. Both single-file and batch
conversion treat the input as seconds when `--time-unit` is omitted. For
millisecond or microsecond input, specify the unit explicitly:

```bash
python tools/preprocess/convert.py input.csv data/output.parquet --time-unit ms
```

| `--time-unit` value | Meaning |
|--------------------|------|
| `s`                | Seconds |
| `ms`               | Milliseconds |
| `us`               | Microseconds |

#### Convert a MAT (MATLAB) file

Supports both MATLAB ≤ v7.2 and v7.3 (HDF5):

```bash
python tools/preprocess/convert.py data.mat data/output.parquet
```

#### All options together

```bash
python tools/preprocess/convert.py input.csv data/output.parquet \
  --time-col elapsed \
  --time-unit ms
```

---

## Batch conversion (multiconverter)

Use this when you want to convert every `.mat` directly under a folder in one go.

### How to run

```bash
# From the repository root
source tools/.venv/bin/activate

# Basic
python tools/preprocess/multiconverter.py --input ./input --output ./output

# To overwrite an existing .parquet
python tools/preprocess/multiconverter.py --input ./input --output ./output --force
```

### Options

| Option | Required | Description |
|-----------|------|------|
| `--input` | ○ | Folder containing the .mat files to convert |
| `--output` | ○ | Output folder for the Parquet files (created if missing) |
| `--time-unit` | — | Input time unit: `s` (default), `ms`, or `us`; batch conversion does not infer units |
| `--force` | — | Overwrite even if a .parquet of the same name already exists |

### Behavior

- Scans **only the top level** of `--input` for `.mat` files (subfolders are ignored)
- Output file name uses the input stem as-is (`foo.mat` → `foo.parquet`)
- Time column is auto-detected (`time`, `t`, `Time`, `TIME`, `timestamp`, `elapsed`, `ECUTIME`)
- Uses seconds by default; pass `--time-unit ms` or `--time-unit us` when needed
- Continues converting remaining files even if one fails
- Exits with code 1 if any file failed

### Example output

```
[Scan complete] Detected 3 .mat file(s)

--- foo.mat ---
[Load complete] foo.mat  shape=(50000, 8)
[Output complete] output/foo.parquet
  Channels: ['iu', 'iv', 'iw', 'vu', 'vv', 'vw', 'torque']
  Samples: 50,000 points
  Sampling: 10,000.0 Hz (Ts=0.1000 ms)

--- bar.mat ---
[Skipped] bar.mat → bar.parquet (already exists)

--- baz.mat ---
[Load complete] baz.mat  shape=(30000, 6)
[Output complete] output/baz.parquet
  ...

========================================
Conversion summary
========================================
  Succeeded : 2
  Skipped   : 1
  Failed    : 0
```

If any file failed, the summary lists them:

```
  Failed   : 1

Failed files:
  - broken.mat: Failed to read the MATLAB file. ...
```

---

## Example output (single-file conversion)

```
[Load complete] input.csv  shape=(50000, 8)
[Time conversion] Converting 'elapsed' from ms → s
[Output complete] data/output.parquet
  Channels: ['iu', 'iv', 'iw', 'vu', 'vv', 'vw', 'torque']
  Samples: 50,000 points
  Sampling: 10,000.0 Hz (Ts=0.1000 ms)
```

For non-uniformly-sampled data, the following line is added:

```
[Warning] Detected non-uniform time spacing. Resampling via linear interpolation.
```

---

## Troubleshooting

### The time column cannot be auto-detected

**Error:**
```
ValueError: 時間カラムを自動検出できませんでした。カラム一覧: ['...']
```

**Resolution:** Specify the column name with `--time-col`.

```bash
python tools/preprocess/convert.py input.csv data/out.parquet --time-col <column-name>
```

---

### The output time scale is wrong

**Symptom:** The values in the `time` column of the Parquet are abnormally large or small.

**Cause:** The converter defaults to seconds and does not infer units. Input in
milliseconds or microseconds needs an explicit unit; otherwise its numeric values
are interpreted as seconds.

**Resolution:** Specify the unit explicitly with `--time-unit`.

---

### MAT file fails to load

**Error:**
```
ValueError: MATLAB ファイルの読み込みに失敗しました。
```

**Checklist:**
- Is `h5py` installed? `pip install h5py`
- Was the MATLAB file saved in `-v7.3` format (HDF5) or in `-v7` or earlier?

---

### Zero waveform channels error

**Error:**
```
ValueError: 波形チャンネルが0本です（全て非数値 or 時間のみ）
```

**Cause:** The only numeric column is the time column. This happens, for example, when most CSV columns are strings.

**Resolution:** Verify that your input data actually contains numeric waveform channels.

---

## Output Parquet Specification

| Column | Type | Content |
|--------|-----|------|
| `time` | Numeric | Time axis in seconds; original numeric dtype may be retained when no scaling/resampling is needed |
| Each channel name | Numeric | Original numeric dtype, or interpolated values when resampling is needed |

- Uniform spacing guaranteed (non-uniform data has already been linearly interpolated)
- Save to a chosen output directory and select that Parquet in the Rust viewer. Shared fixtures live in [data/](../data/README.md).
- Single-file conversion overwrites its specified output. Batch conversion skips existing outputs unless `--force` is supplied.
