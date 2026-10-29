---
project: Panel_y
doc_type: Issue Report
target: Proto #3.1
created: "2026-04-02"
status: Investigated, fix in progress (→ proto_3_1b)
---

# Issue Report — Hang When Loading Large Datasets

---

## Occurrence

### Test environment
- Data: 9-channel current/voltage waveforms, 1 MHz sampling, 10 s recording
- Total data points: **9 ch × 10,000,000 points = 90,000,000 points**
- Browser: Chrome

### Symptom
- File conversion / load: succeeded
- Initial render: succeeded
- One zoom step: succeeded
- **Repeated zoom in/out → hang**
- Chrome tab memory usage: **over 3 GB**

### Reference (case that worked fine)
- 100 ch, 500 μs sampling (2 kHz), 5 s recording → works smoothly
  - Total points: 100 ch × 10,000 points = 1,000,000 points (1/90 of the 90 M case)

---

## Root-cause analysis

### Cause 1: `minmax_envelope` is a pure-Python loop (primary cause)

```python
# app.py: minmax_envelope()
for i in range(n_buckets):    # Python loop, n_buckets = 2500
    seg = data[i0:i1]
    d_min[i] = seg.min()
    d_max[i] = seg.max()
```

Each zoom step runs this Python loop 2500 times for each of the 9 rows.
A single zoom event runs the loop **22,500 times in total**, taking several to tens of seconds.
Continuous zooming queues up callbacks and ultimately hangs the app.

### Cause 2: A boolean mask over the entire dataset is regenerated every time

```python
# app.py: make_row_fig()
mask = (time_arr >= x_range[0]) & (time_arr <= x_range[1])
```

A 90 M-element boolean array is allocated on every zoom. Repeated memory allocation
puts pressure on the GC and inflates memory usage.

### Cause 3: DataFrame is fully expanded as float64

90 M rows × 10 cols × 8 bytes ≈ **720 MB** (raw data only).
Internal pandas copies and Plotly JSON serialization inflate that by **3–4×**, exceeding 3 GB.

---

## Fix plan (proto_3_1b branch)

| # | Action | Effect | Target |
|---|------|------|----------|
| 1 | Vectorize `minmax_envelope` with NumPy (reshape → max/min) | ~100× faster | `minmax_envelope()` |
| 2 | Replace the boolean mask with `np.searchsorted` index range | Less memory, faster | `make_row_fig()` |
| 3 | Downcast to float32 on load | Halves memory (720 MB → 360 MB) | `load_file()` |
| 4 | Pre-downsampled cache for the overview view | Faster initial render and zoom-out | `load_file()` / `make_row_fig()` |

---

## Lessons learned

- **1 MHz × 10 s × 9 ch = 90 M points** is beyond the scale Plotly/Dash typically targets
- The Min-Max envelope design is correct, but the implementation (Python loop) could not scale
- NumPy vectorized operations (reshape + nanmax/nanmin) can perform the same computation orders of magnitude faster
- Data-type choice (float64 vs. float32) directly affects the memory footprint

---

## Related

- Implementation branch: `proto_3_1b`
- Previous version: `code/proto_3_1/app.py`
