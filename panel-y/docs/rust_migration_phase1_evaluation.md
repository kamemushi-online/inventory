---
project: Panel_y
doc_type: Phase 1 evaluation memo
target: Rust migration Phase 1
created: "2026-04-30"
updated: "2026-04-30"
status: Cleared for Phase 2
source_proto: "archive/proto_3_1b"
implementation: "code/rust_phase1"
---

# Rust Migration Phase 1 — Evaluation Memo

---

## 1. Conclusion

The Rust Phase 1 vertical-slice prototype is cleared to proceed to Phase 2.

Technologies to continue with:

- GUI: `eframe` + `egui`
- Parquet loading: Apache Arrow Rust `parquet` crate
- Data storage: column-oriented arrays of `time: f64`, `value: f32`
- Render-side data: range-aware min/max envelopes
- Rendering: custom drawing via `egui::Painter` for the time being

Within the scope verified in Phase 1, pan/zoom after loading stays in the display-width order, and is clearly lighter than the Dash/Plotly version.

A caveat: the "90M-point class" mentioned in the original design document has not yet been measured on real files. The large-data measurement in this round used 10,000,000 rows of `panely_large_10s_1mhz_9ch.parquet`. Phase 1 is judged as "cleared for Phase 2," but supplementary measurement at the 90M-point class or with equivalent real-machine log data remains a required early task for Phase 2.

---

## 2. Results Against the Success Criteria

| Condition | Result | Evidence |
|------|------|------|
| `test_100k.parquet` loads and displays a single channel | Pass | Schema read, channel selection, and single-channel display implemented |
| Application does not crash on large data | Pass | Loaded and displayed 10,000,000-row data in the GUI |
| Pan/zoom on large data does not become unresponsive | Pass | Manual GUI testing shows pan/zoom/reset is practically stress-free |
| Render processing scales roughly with display width rather than total sample count | Pass | Visible envelope requested 1200 buckets, drawing ~2400 points |
| Memory does not grow continuously under repeated zoom | Pass | RSS remained at 130.0 MiB across 1000 visible-range envelope re-extractions |
| Interaction on large data is noticeably lighter than the Dash/Plotly version | Pass | Rust visible envelope max 0.0065 s vs. Dash backend zoom-equivalent max 0.111 s |
| Does not freeze on 90M-point-class data | Not yet | Verified at 10M rows. 90M-point-class measurement is deferred to early Phase 2 |

---

## 3. Highlights of the Measurement Results

### 10M-row data

Target:

- file: `panely_large_10s_1mhz_9ch.parquet`
- channel: `iu`
- rows: 10,000,000
- projected columns: `[0, 1]`

Result:

| Item | Result |
|------|------|
| array memory | 114.4 MiB |
| Parquet read | 0.278 s |
| full envelope | 0.013 s |
| visible envelope 24 runs | 0.059 s |
| visible envelope avg | 0.0025 s |
| visible envelope max | 0.0065 s |
| stress 1000 runs avg | 0.0032 s |
| stress 1000 runs max | 0.0090 s |
| RSS after load | 130.0 MiB |
| RSS after stress | 130.0 MiB |

Perceived behavior in the GUI:

- About 0.5 s from `Load Selected Channel` to waveform display
- Pan/zoom/reset feel good after display
- A first-display latency of ~0.5 s is acceptable

### Comparison with the Dash/Plotly version

Measurements of the Dash/Python backend on comparable large data:

| Item | Dash/Python backend | Rust Phase 1 |
|------|---------------------|--------------|
| Parquet read | 0.406 s | 0.278 s |
| Initial-display equivalent | 0.349 s | GUI subjective ~0.5 s |
| zoom max | 0.111 s | visible envelope max 0.0065 s |
| continuous zoom max | 0.150 s | stress max 0.0090 s |
| peak / RSS | 1405.6 MiB peak | 130.0 MiB RSS |

The initial-display measurement is not fully apples-to-apples. The Dash side measures only the backend and excludes browser rendering, while the Rust side is a GUI subjective value. The primary comparison points are post-load pan/zoom processing time and memory usage.

---

## 4. Why the Failure Conditions Do Not Apply

| Failure condition | Judgment |
|----------|------|
| Pan/zoom is hard to implement due to GUI/render library constraints | Does not apply. Drag pan, wheel/pinch zoom, horizontal-scroll pan, and double-click reset are implemented and verified |
| Even the Rust version ends up processing nearly all points on large data | Does not apply. Draw point count scales with bucket count rather than source sample count |
| LOD/tile cache implementation cost is excessive | Does not apply at this point. Phase 2 can proceed with a range-aware envelope, without a full tile pyramid |
| Parquet-loading library constraints are significant | Does not apply. Projection lets us read only `time` and the selected channel |
| C++/Qt or another alternative would clearly be faster to put into practical use in the short term | Does not apply at this point. Rust/egui let us validate the primary risks within a short period |

---

## 5. Technical Decisions

### Continue the Rust migration

For 10M-row data, post-load visible-range envelope re-extraction took at most 0.0065 s. RSS did not grow across 1000 re-extractions equivalent to continuous interaction.

Based on these results, the Rust version is worth continuing as a large-capacity waveform viewer.

### Continue with `egui::Painter`

In Phase 1, the draw point count is kept on the order of the display width, and there is no sign that `egui::Painter` will immediately become a bottleneck.

In Phase 2, draw point counts will grow with multi-row, multi-channel display. We will reconsider a `wgpu` custom renderer when any of the following occurs:

- Per-frame draw point count grows and pan/zoom feels laggy
- CPU-side shape generation becomes dominant in multi-row display
- Edge preservation for step display inflates the draw point count too much

### Do not yet introduce a full tile pyramid

In Phase 1, simply re-extracting min/max envelopes from the raw array per range was sufficiently light.

For the initial Phase 2 implementation, we will use the following baseline:

- Hold raw data per loaded channel
- Build range-aware envelopes per visible range and width
- Share envelopes when the same channel appears in multiple rows
- Add a tile pyramid only when multi-channel/multi-row needs or 90M-point-class measurements demand it

---

## 6. Hand-off to Phase 2

Phase 2 builds basic viewer features on top of `code/rust_phase1`.

Priority:

1. Extend the data layer so multiple channels can be loaded simultaneously
2. Implement overlay display of multiple channels in a single row
3. Implement multi-row display
4. Synchronize X-axis pan/zoom across all rows
5. Separate line / step display
6. Port Y-axis range specification, colors, and line widths
7. Port hover sync and channel value readout
8. Add asynchronous loading, progress display, and UI control while loading
9. Re-measure medium, large, and 90M-point-class datasets

Design principles:

- Rendering uses LOD or envelopes to stay on the order of display pixel count
- Analysis features (Phase 3 and later) must still be built on the assumption of raw data access
- UI state, data storage, render-side extraction, and analysis must remain separated
- Even in multi-row display, do not duplicate raw data and envelopes for the same channel

---

## 7. Open Items

- Measurement on 90M-point-class data or data equivalent to real-machine logs
- Memory upper bound when many channels are held simultaneously
- Edge-preservation scheme for step display
- UI design while asynchronous loading is in progress
- State model for v2 of the plot config in Phase 2 and beyond

---

## 8. References

- [Rust Migration Phase 1 Benchmark Results](rust_migration_phase1_benchmark_results.md)
- [Rust Migration Phase 1 Scope Definition](rust_migration_phase1_scope.md)
- [Rust Migration Phase 1 Technology Selection](rust_migration_phase1_tech_selection.md)
- [Rust Migration Phase 0 Performance Measurement Results](rust_migration_phase0_benchmark_results.md)
