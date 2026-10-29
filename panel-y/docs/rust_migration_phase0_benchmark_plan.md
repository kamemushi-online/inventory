---
project: Panel_y
doc_type: Performance Benchmark Plan
target: Rust Migration Phase 0
created: "2026-04-27"
updated: "2026-04-27"
status: Complete
source_proto: "archive/proto_3_1b"
---

# Rust Migration Phase 0 Performance Measurement Plan

---

## 1. Purpose

Establish a performance baseline for the Dash/Plotly version (`proto_3_1b`) that will serve as the comparison reference for the Rust version from Phase 1 onward.

This document defines the measurement items and procedures. The actual results are recorded separately in `docs/rust_migration_phase0_benchmark_results.md`.

---

## 2. Measurement Targets

| Target | Path |
|--------|------|
| Dash/Plotly version | `archive/proto_3_1b/app.py` |
| Python environment | `archive/.venv` (see the [archive setup](../archive/README.md#setup)) |
| Small dataset | `data/test_100k.parquet` |
| Medium dataset | TBD |
| Large dataset | TBD |

---

## 3. Environment Record

The following must always be recorded at measurement time.

- Date and time of measurement
- OS
- CPU
- RAM
- Python version
- Dash version
- Plotly version
- pandas version
- numpy version
- pyarrow version
- Browser name / version
- Data file name
- Number of data rows
- Number of channels
- File size

Example commands:

```bash
cd /Users/sugusokothx/life/factory/panel-y
archive/.venv/bin/python --version
archive/.venv/bin/python -c "import dash, plotly, pandas, numpy, pyarrow; print(dash.__version__, plotly.__version__, pandas.__version__, numpy.__version__, pyarrow.__version__)"
```

---

## 4. Measurement Items

| ID | Item | Description | Target Data |
|----|------|-------------|-------------|
| B-01 | App startup time | From `python app.py` until HTTP becomes responsive | Small / Medium / Large |
| B-02 | Parquet load time | From clicking the load button until the status is shown | Small / Medium / Large |
| B-03 | First waveform display time | From clicking Update until the waveform is fully drawn | Small / Medium / Large |
| B-04 | Memory usage | Python / browser memory after load and after operation | Small / Medium / Large |
| B-05 | Single-zoom response | From a zoom action until all rows are updated | Small / Medium / Large |
| B-06 | Continuous zoom resilience | Freezing / lag after 10 consecutive zooms | Medium / Large |
| B-07 | Zoom reset time | Until the full-range view is restored | Medium / Large |
| B-08 | Hover response | Perceived delay between spike line and value updates | Small / Medium / Large |
| B-09 | Cursor placement response | From clicking A/B until the statistics are displayed | Small / Medium / Large |
| B-10 | FFT execution time | From clicking the FFT button until the spectrum is shown | Small / Medium |
| B-11 | Plot config load time | From `.pyc.json` load until restoration completes | Small / Medium |

---

## 5. Manual Measurement Procedure

### Common Setup

```bash
# From the repository root, after the archive setup
cd archive/proto_3_1b
source ../.venv/bin/activate
python app.py
```

Open `http://localhost:8050` in the browser.

### B-02 / B-03 Load and First Display

1. Enter the Parquet path
2. Click `読み込み`
3. Record the time until the status is shown
4. Click `更新`
5. Record the time until the waveform is displayed

### B-05 / B-06 Zoom

1. Display the waveform
2. Zoom horizontally
3. Record the perceived time until all rows are updated
4. Perform 10 consecutive zoom / pan operations
5. Record any freezes, lag, or memory growth

### B-09 Cursor / Statistics

1. Enable measurement mode
2. Place Cursor A
3. Place Cursor B
4. Record the time until the statistics table is shown

### B-10 FFT

1. Place Cursors A and B
2. Select the target channel for FFT
3. Select the window function
4. Click `FFT`
5. Record the time until the spectrum is shown

---

## 6. Recording Format

```markdown
## Measurement YYYY-MM-DD

### Environment

- OS:
- CPU:
- RAM:
- Python:
- Dash:
- Plotly:
- pandas:
- numpy:
- pyarrow:
- Browser:

### Data

- file:
- rows:
- channels:
- size:

### Results

| ID | Result | Notes |
|----|------|------|
| B-01 | | |
| B-02 | | |
| B-03 | | |
| B-04 | | |
| B-05 | | |
| B-06 | | |
| B-07 | | |
| B-08 | | |
| B-09 | | |
| B-10 | | |
| B-11 | | |
```

---

## 7. Pass Criteria for Rust Phase 1

The following targets should be met in Phase 1.

- The UI does not become unresponsive after loading large data
- pan / zoom completes in roughly the order of displayed pixels
- Continuous zooming does not cause unbounded memory growth
- Zoom response is clearly lighter than the Dash/Plotly version
- First-display time for large data fits within an acceptable range

Specific second-level and memory-cap thresholds will be set after collecting the Dash/Plotly measurements.

---

## 8. Notes

Browser-rendering measurements taken by hand are noisy. With the same data and the same procedure, measure three times and keep the "worst case" and "perceptual notes" rather than the average.
