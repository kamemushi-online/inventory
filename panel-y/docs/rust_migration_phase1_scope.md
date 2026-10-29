---
project: Panel_y
doc_type: Phase 1 scope definition
target: Rust migration Phase 1
created: "2026-04-27"
updated: "2026-04-27"
status: Finalized
source_proto: "archive/proto_3_1b"
---

# Rust Migration Phase 1 — Scope Definition: Vertical-Slice Prototype

---

## 1. Purpose

In Phase 1, we verify the largest risk in the Rust migration: can we read large Parquet files, display waveforms with low overhead, and pan/zoom them?

At this stage we are not building a finished application. We do not chase every feature of `proto_3_1b`. Instead, we use a vertical slice to confirm only the core of the performance improvement.

---

## 2. Features to Implement

| ID | Feature | Required |
|----|------|------|
| P1-01 | Launches as a Rust desktop application | Required |
| P1-02 | Loads a specified Parquet file | Required |
| P1-03 | Recognizes the `time` column | Required |
| P1-04 | Treats columns other than `time` as channels | Required |
| P1-05 | Displays a channel list | Required |
| P1-06 | Lets the user select a single channel | Required |
| P1-07 | Displays one row of waveform | Required |
| P1-08 | Performs X-axis pan/zoom | Required |
| P1-09 | Uses range-aware min/max envelopes | Required |
| P1-10 | Does not freeze on 90M-point-class data | Required |
| P1-11 | Records basic load time and memory usage | Required |

---

## 3. Features Explicitly Out of Scope

Phase 1 does not implement the following:

- Multi-row display
- Multi-channel overlay in a single row
- Step display
- Color/line-width settings
- Y-axis range specification
- Hover sync
- Cursor A/B
- Range statistics
- FFT
- Plot config save/load
- Theme switching
- Sticky header
- Ctrl+Enter
- MAT/CSV conversion
- multiconverter reimplementation

---

## 4. Technical Verification Points

### Parquet loading

What to verify:

- Whether Rust can read the target Parquet files
- Whether we can obtain the time column and channel columns
- Whether projection (loading only the required columns) works
- Memory usage when loading all columns

### Data storage

What to verify:

- Whether time and value arrays can be held efficiently
- Whether float32/float64 strategies can be compared
- Whether copies stay bounded on large data

### LOD / min-max

What to verify:

- Whether the point count can be limited to the display width
- Whether min/max envelopes preserve peaks
- Whether pan/zoom processes only the necessary range
- Whether the build time and memory of LOD/tile cache stay within acceptable bounds

### Rendering

What to verify:

- That pan/zoom does not pass all points to the renderer
- That the draw point count stays on the order of screen width
- That continuous zoom does not lock up
- That GPU rendering or equivalent lightness can be achieved

---

## 5. Success Criteria

Phase 1 passes if all of the following hold:

- `test_100k.parquet` loads and displays a single channel
- Large data equivalent to `panely_large_10s_1mhz_9ch.parquet` loads without crashing
- Pan/zoom on large data does not become unresponsive
- Render processing scales roughly with display width rather than total sample count
- Memory does not grow continuously under repeated zoom
- Interaction on large data is noticeably lighter than the Dash/Plotly version

---

## 6. Failure Conditions

If any of the following hold, we revisit the Phase 1 technical direction:

- Pan/zoom is hard to implement due to GUI/render library constraints
- Even the Rust version ends up processing nearly all points on large data
- LOD/tile cache implementation cost is excessive
- Parquet-loading library constraints are significant
- C++/Qt or another alternative would clearly be faster to put into practical use in the short term

---

## 7. Recommended Implementation Order

1. Create a Rust project
2. Display an empty window
3. Load a Parquet file from a hard-coded path
4. Load only the time column and one channel into memory
5. Display the full range as a min/max envelope
6. Add X-axis pan/zoom
7. Re-extract per visible range
8. Add a channel selection UI
9. Measure on large data
10. Record results as the Phase 1 evaluation

---

## 8. Deliverables to Phase 2

- Rust prototype code
- Decision on Parquet loading strategy
- Decision on data storage strategy
- Decision on LOD/tile cache strategy
- Decision on rendering strategy
- Measurement results on small/large data
- Go/no-go decision
