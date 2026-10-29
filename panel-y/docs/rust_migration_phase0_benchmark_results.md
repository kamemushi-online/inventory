---
project: Panel_y
doc_type: Performance Results
target: Rust Migration Phase 0
created: "2026-04-29"
updated: "2026-04-29"
status: Measuring
source_proto: "code/proto_3_1b"
plan: "docs/rust_migration_phase0_benchmark_plan.md"
---

# Rust Migration Phase 0 Performance Measurement Results

---

## 1. Purpose

Record the performance baseline of the Dash/Plotly version (`proto_3_1b`) and use it as the comparison reference for the Rust version from Phase 1 onward.

Measurements follow the plan in `docs/rust_migration_phase0_benchmark_plan.md`.

---

## 2. Measurement Policy

- Run the same data and the same operation three times as a rule
- Keep not just averages but also the worst case and qualitative notes
- Record memory for the Python/Dash process and the browser process separately
- Browser operations are noisy, so record freezing / lag / rendering glitches together with the elapsed time
- Cursor / statistics / FFT values are kept as reference for Rust Phase 3 and beyond

---

## 3. Measurement Status

| Item | Status | Notes |
|------|--------|-------|
| Result file created | Done | 2026-04-29 |
| Small-data environment check | Done | Confirmed by loading `test_100k.parquet` |
| Small-data manual measurement | Done | Recorded B-01 through B-11. B-07 succeeded on manual re-check |
| Medium-data preparation | Done | Synthesized `panely_medium_2m_8ch.parquet` |
| Large-data preparation | Done | Synthesized `panely_large_10s_1mhz_9ch.parquet` |
| Medium / large measurement | Done (backend) | Added Dash/Python backend measurements; browser rendering will be added as needed |

---

## 4. Measurement 2026-04-29: Small Data

### 4.1 Environment

Recorded at: 2026-04-29 10:17 JST

| Item | Value |
|------|-------|
| OS | macOS 26.4.1 build 25E253 |
| Machine | MacBook Pro MacBookPro18,3 |
| CPU | Apple M1 Pro, 10 cores |
| RAM | 16 GB |
| Python | 3.13.7 |
| Dash | 4.1.0 |
| Plotly | 6.7.0 |
| pandas | 3.0.2 |
| numpy | 2.4.4 |
| pyarrow | 23.0.1 |
| Browser | Codex in-app browser (version not recorded) |

### 4.2 Target Application

| Item | Value |
|------|-------|
| app | `code/proto_3_1b/app.py` |
| Start command | `cd code/proto_3_1b && source ../.venv/bin/activate && python app.py` |
| URL | `http://localhost:8050` |

### 4.3 Target Data

| Item | Value |
|------|-------|
| file | `code/proto_3_1b/data/test_100k.parquet` |
| rows | 100,000 |
| columns | `time`, `sine_50Hz`, `pwm_1kHz`, `noise`, `chirp_1_500Hz` |
| channels | 4 |
| time range | 0.0 s to 0.99999 s |
| size | 3,707,182 bytes |

A `pyarrow` warning about CPU information appeared during Parquet load, but the load itself succeeded.

The plot-config load measurement used the temporary file `/tmp/panely_benchmark_test_100k.pyc.json`.

### 4.4 Results

| ID | Item | Run 1 | Run 2 | Run 3 | Worst | Notes |
|----|------|-------|-------|-------|-------|-------|
| B-01 | App startup time | 1.368 s | 1.321 s | 1.324 s | 1.368 s | From `python app.py` to HTTP 200 |
| B-02 | Parquet load time | 0.622 s | 0.627 s | 0.623 s | 0.627 s | From load button to status display |
| B-03 | First waveform display time | 0.729 s | 0.625 s | 0.624 s | 0.729 s | From Update button until all 4 graphs are shown |
| B-04 | Memory usage | After load 241.0 MiB | After continuous zoom 252.9 MiB | After 3rd run 253.5 MiB | 253.5 MiB | Combined RSS of Dash parent + child. Browser-only memory not separated |
| B-05 | Single-zoom response | 0.463 s | 0.465 s | 0.459 s | 0.465 s | Horizontal drag on the first row until X-axis ticks update |
| B-06 | Continuous-zoom resilience | 10x / 4.104 s / max 0.465 s | 10x / 4.105 s / max 0.468 s | 10x / 4.093 s / max 0.462 s | max 0.468 s | All 10 zooms completed; no freezes |
| B-07 | Zoom reset time | 0.641 s (double-click) | 0.643 s (Reset axes) | 0.644 s (Autoscale) | 0.644 s | Manual re-check confirmed full-range restoration for all operations |
| B-08 | Hover response | 1.599 s | 1.592 s | 1.592 s | 1.599 s | From start of mouse motion to value-label update; includes automation travel time |
| B-09 | Cursor placement response | 1.237 s | 1.241 s | 1.240 s | 1.241 s | From start of A/B clicks to statistics panel display; about 0.62 s after the B click |
| B-10 | FFT execution time | 0.627 s | 0.633 s | 0.631 s | 0.633 s | From FFT button to FFT graph display; default first channel |
| B-11 | Plot config load time | 0.632 s | 0.632 s | 0.624 s | 0.632 s | From the sample-data state, loading `.pyc.json` restores the 4 graphs of test_100k |

### 4.5 Qualitative Notes

- For small data, loading, first draw, and single-zoom all feel essentially smooth.
- 10 consecutive zooms completed across all 3 trials with no UI freeze.
- B-05/B-06 used a horizontal drag on the first row's graph, waiting on the last row's X-axis tick update as the completion signal.
- B-07 was manually re-checked and confirmed to return to the full-range view via all of:
  - Double-click on the graph
  - Plotly modebar `Reset axes`
  - Plotly modebar `Autoscale`
  In the re-check, the test condition was that the last row's X-axis ticks transitioned from the zoomed-in `0.2, 0.3, 0.4` back to the full-range `0, 0.2, 0.4, 0.6, 0.8`.
  The previous "unmeasurable" result was most likely caused by instability right after Dash startup or by automation timing, and at this point it is not treated as a known risk of the current Dash version.
- B-08 includes the mouse-travel time of the browser automation, so it is not a pure Dash-callback time.
- B-09 includes both A and B clicks. From the second click to the statistics panel display was about 0.62 s.
- B-10 was measured with Cursors A/B already placed, and the FFT target channel defaulted to `sine_50Hz`.
- B-11 used the path of going from the `sample_waveform.parquet` view and loading `/tmp/panely_benchmark_test_100k.pyc.json`, since clicking right after reload was unstable for the automation.
- B-04 Dash-side RSS breakdown:
  - After load: parent 73.5 MiB + child 167.5 MiB = 241.0 MiB
  - After continuous zoom: parent 73.5 MiB + child 179.4 MiB = 252.9 MiB
  - After 3rd continuous-zoom run: parent 73.5 MiB + child 180.0 MiB = 253.5 MiB
- Because of `app.run(debug=True)`, Dash runs as a parent-child reloader pair.
- The Codex in-app browser memory could not be isolated as its own process, so this B-04 keeps the Dash-side RSS as the baseline.

---

## 5. Measurement 2026-04-29: Medium / Large Data (backend)

### 5.1 Measurement Method

Browser APIs were not available, so medium and large data were measured by directly invoking the Dash app's functions from `code/proto_3_1b/benchmark_phase1_datasets.py`.

Included:

- Parquet loading and float32 downcast
- Building `df_overview`
- Waveform Figure generation
- Plotly JSON serialization
- Figure regeneration on zoom-range change
- Hover-value lookup
- Cursor A/B range statistics
- FFT calculation and FFT Figure JSON serialization

Excluded:

- Browser draw time
- Dash HTTP traffic
- Browser-side memory

For this reason, the values in this section are not directly comparable to the small-data browser measurements, and should be treated as Dash/Python backend reference values.

Command:

```bash
cd code/proto_3_1b
../.venv/bin/python benchmark_phase1_datasets.py medium --run 1
../.venv/bin/python benchmark_phase1_datasets.py medium --run 2
../.venv/bin/python benchmark_phase1_datasets.py medium --run 3
../.venv/bin/python benchmark_phase1_datasets.py large --run 1
../.venv/bin/python benchmark_phase1_datasets.py large --run 2
../.venv/bin/python benchmark_phase1_datasets.py large --run 3
```

### 5.2 Target Data

| Class | file | rows | columns | size | Notes |
|-------|------|------|---------|------|-------|
| Medium | `panely_medium_2m_8ch.parquet` | 2,000,000 | `time` + 8 ch | 18,426,601 bytes | 100 kHz, ~20 s |
| Large | `panely_large_10s_1mhz_9ch.parquet` | 10,000,000 | `time` + 9 ch | 292,253,458 bytes | 1 MHz, ~10 s |

### 5.3 Results

| ID | Item | Med Run 1 | Med Run 2 | Med Run 3 | Med Worst | Large Run 1 | Large Run 2 | Large Run 3 | Large Worst | Notes |
|----|------|-----------|-----------|-----------|-----------|-------------|-------------|-------------|-------------|-------|
| B-02 | Parquet load | 0.188 s | 0.085 s | 0.153 s | 0.188 s | 0.406 s | 0.223 s | 0.232 s | 0.406 s | `load_file()` |
| B-03 | First waveform display equivalent | 0.319 s | 0.267 s | 0.332 s | 0.332 s | 0.349 s | 0.336 s | 0.288 s | 0.349 s | Row update + Figure build + JSON |
| B-04 | Backend peak RSS | 372.0 MiB | 374.3 MiB | 376.1 MiB | 376.1 MiB | 1222.8 MiB | 1405.6 MiB | 1404.3 MiB | 1405.6 MiB | Process peak through end of analysis |
| B-05 | Single-zoom response equivalent | 0.102 s | 0.098 s | 0.108 s | 0.108 s | 0.111 s | 0.110 s | 0.110 s | 0.111 s | Range 20%–40% |
| B-06 | Continuous-zoom resilience | 10x / 1.045 s / max 0.131 s | 10x / 1.034 s / max 0.127 s | 10x / 1.111 s / max 0.139 s | total 1.111 s / max 0.139 s | 10x / 1.127 s / max 0.150 s | 10x / 1.124 s / max 0.142 s | 10x / 1.122 s / max 0.141 s | total 1.127 s / max 0.150 s | No backend freezes |
| B-07 | Zoom-reset equivalent | 0.099 s | 0.099 s | 0.106 s | 0.106 s | 0.108 s | 0.107 s | 0.118 s | 0.118 s | Full-range Figure rebuild |
| B-08 | Hover-value update equivalent | 0.010 s | 0.006 s | 0.008 s | 0.010 s | 0.025 s | 0.029 s | 0.032 s | 0.032 s | `update_values()` |
| B-09 | Cursor statistics equivalent | 0.013 s | 0.012 s | 0.015 s | 0.015 s | 0.054 s | 0.054 s | 0.054 s | 0.054 s | 10%–20% range |
| B-10 | FFT execution equivalent | 0.046 s | 0.041 s | 0.062 s | 0.062 s | 0.189 s | 0.163 s | 0.163 s | 0.189 s | First channel, 10%–20% range |
| B-11 | Plot config load | Not measured | Not measured | Not measured | N/A | Not measured | Not measured | Not measured | N/A | Skipped this time; outside Phase 1 scope |

### 5.4 Qualitative / Design Notes

- Looking only at the backend, both 2M-row and 10M-row datasets keep zoom redraw within the display-width order thanks to the Min-Max envelope.
- Large-data backend peak RSS reached around 1.4 GiB at most. Memory will be a primary comparison item against the Rust Phase 1 implementation on the same data.
- Large-data FFT for a 1-second window of about 1,000,000 samples was at worst 0.189 s. FFT is out of scope for Rust Phase 1, but this is kept as a reference for Phase 3.
- The `pyarrow` CPU-information warning also appeared on medium / large measurements, but loading and computation succeeded.

---

## 6. Next Steps

1. If needed, supplement medium / large data with browser-side display checks.
2. Load the same medium / large data into the Rust Phase 1 prototype and compare load time, first display, pan/zoom, and memory.
