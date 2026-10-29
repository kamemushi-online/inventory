---
project: Panel_y
doc_type: Data Spec
created: "2026-03-15"
updated: "2026-03-15"
status: Finalized
---

# Panel_y Parquet Data Specification

---

## 1. Format Overview

The standard data format that Panel_y consumes is **Apache Parquet**.

```
time,   ch_name_1,  ch_name_2,  ...
0.0000, 100.0,      5.0,        ...
0.0001, 101.3,      5.1,        ...
```

---

## 2. Column Specification

### 2-1. Time column

| Item | Specification |
|------|------|
| Column name | **Fixed as `time`** (case-sensitive, no aliases allowed) |
| Unit | **Seconds (s)** |
| Type | float64 |
| Constraint | Must be evenly spaced (see below) |
| Index | Do not use the Parquet row index (save with `index=False`) |

### 2-2. Waveform columns (channels)

| Item | Specification |
|------|------|
| Detection | Any column other than `time` is automatically treated as a waveform channel |
| Type | **Numeric type only (int / float)**. Non-numeric columns are excluded from loading |
| Column name | Used verbatim as the channel name shown in the UI |
| Unit | **Not shown** (no need to embed the unit in the column name) |
| Number of channels | 1 or more, no upper limit (handled dynamically) |

---

## 3. Sampling Rate

- The sampling period Ts is derived from `time[1] - time[0]` (the difference between the first two points)
- Sampling frequency: `fs = 1 / Ts`
- **Equal time spacing is assumed.** The interval is expected to be constant for every point.

---

## 4. Equal-spacing Assumption

- Panel_y only processes evenly spaced data
- Non-uniform data (e.g., jittery log capture) is **absorbed by the preprocessing utility through interpolation/resampling**

---

## 5. Handling Non-numeric Data

- Columns of non-numeric types (strings, bool, etc.) are not treated as waveform channels
- **The preprocessing utility removes them** (Panel_y itself does not perform the removal)

---

## 6. Supported Input Formats (via Preprocessing)

Panel_y itself reads only Parquet. Other formats are converted by the preprocessing utility.

| Input format | Conversion |
|---------|---------|
| **Parquet** | Loaded as-is (standard format) |
| **CSV** | `pandas.read_csv()` → shape → `to_parquet()` |
| **MATLAB .mat (≤ v7.2)** | `scipy.io.loadmat()` → shape → `to_parquet()` |
| **MATLAB .mat (≥ v7.3)** | `h5py` → shape → `to_parquet()` |

---

## 7. Summary of Constraints

| Constraint | Detail |
|------|------|
| Time column name | Fixed as `time` |
| Time unit | Fixed as seconds (s) |
| Uniform spacing | Required (non-uniform data must be preprocessed) |
| Channel type | Numeric only (non-numeric removed in preprocessing) |
| Unit display | None |
| Metadata | Not embedded in the Parquet file (sampling rate is derived from `time`) |
