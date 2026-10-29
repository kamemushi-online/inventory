---
project: Panel_y
doc_type: Technology selection
target: Rust migration Phase 1
created: "2026-04-29"
updated: "2026-04-29"
status: Provisional decision
source_proto: "archive/proto_3_1b"
scope: "docs/rust_migration_phase1_scope.md"
---

# Rust Migration Phase 1 — Technology Selection: Provisional Verification Stack

---

## 1. Purpose

Phase 1 does not finalize the technology stack for the completed product.
The goal is the fastest possible verification of: can we read a large Parquet file, display a single-channel waveform with low overhead, and pan/zoom it? To begin implementation, we pick a provisional stack.

The decisions in this document will be revisited based on the Phase 1 measurement results.

---

## 2. Provisional Stack

| Item | Provisional choice | Rationale |
|------|--------|------|
| Application / GUI | `eframe` + `egui` | Native Rust; lets us assemble a UI quickly. Well-suited to the single-screen Phase 1 prototype |
| GUI renderer | Prefer the `wgpu` backend of `eframe` | Aligns with future GPU custom rendering. Fall back to the `glow` backend temporarily if unstable |
| Parquet loading | Apache Arrow Rust `parquet` crate | The official Rust implementation of Parquet/Arrow. Well-suited to verifying column projection |
| Parquet alternative | `polars` lazy `scan_parquet` | Fallback if the direct Arrow API becomes too cumbersome. Heavier dependency and abstraction layer |
| Data storage | `time: Arc<[f64]>`, `value: Arc<[f32]>` | Preserve time precision and store waveform values as float32, matching the current direction |
| Channel storage | In Phase 1, hold full-resolution data only for the currently selected channel | Confirm the memory ceiling on 90M-point-class files first |
| Range search | Equivalent of `slice::partition_point` | Quickly obtain visible-range indices from the monotonic `time` array |
| LOD | Per-selected-channel min/max tile pyramid | Limit draw points to the order of screen width even in full-range or wide-range zoom |
| Rendering | Custom drawing area in `egui` + LOD-applied line/envelope | Avoid leaving control to `egui_plot`; explicitly control the number of points passed to the renderer |
| Pan/zoom | Manage via a custom `ViewRange` state | Avoid depending on Plot widget internal state; easier to extend to multi-row sync later |
| Async | UI thread + worker thread | Prevent Parquet loading and LOD construction from freezing the UI |
| Measurement | `std::time::Instant` + OS memory measurement | Keep instrumentation lightweight in Phase 1 while recording load/LOD/render times separately |

---

## 3. Not Adopted

| Candidate | Reason for not adopting in Phase 1 |
|------|--------------------------|
| Tauri + Plotly.js | Keeps the current main bottleneck — the browser/Plotly rendering pipeline |
| Continue with Dash | Phase 1's purpose is to escape the Dash/Plotly limits |
| `egui_plot` as primary renderer | Convenient, but turns point-count control and LOD switching into a black box at the 90M-point class |
| Direct `winit` + `wgpu` | Strong render control, but UI implementation cost is too high for Phase 1 |
| Iced / Slint | Good general UI candidates, but we prioritize waveform rendering and low-level draw-control verification |
| Loading all channels from the start | The main Phase 1 risk can be verified with a single-channel vertical slice. Don't worsen memory upfront |
| Plot config compatibility from the start | Out of Phase 1 scope. `.pyc.json` compatibility is handled in Phase 2 and beyond |

---

## 4. Provisional Architecture

```
eframe app
  │
  ├─ UI state
  │   ├─ parquet path
  │   ├─ selected channel
  │   └─ ViewRange { x_min, x_max }
  │
  ├─ Loader worker
  │   ├─ Parquet schema read
  │   ├─ time column validation
  │   ├─ channel column detection
  │   └─ read time + selected channel columns
  │
  ├─ WaveStore
  │   ├─ time: Arc<[f64]>
  │   ├─ value: Arc<[f32]>
  │   └─ metadata
  │
  ├─ LodStore
  │   ├─ raw threshold
  │   ├─ min/max base tiles
  │   └─ min/max pyramid levels
  │
  └─ WaveformView
      ├─ index range extraction
      ├─ raw line or envelope point generation
      └─ egui Painter drawing
```

---

## 5. Parquet Loading Strategy

For Phase 1, we implement first with the Apache Arrow Rust `parquet` crate.

Loading sequence:

1. Check the file extension of the file path
2. Read the schema and find the `time` column
3. Enumerate numeric columns other than `time` as channel candidates
4. Load only `time` and the single selected channel
5. Convert `time` to an `f64` contiguous array and waveform values to an `f32` contiguous array
6. If `time` is non-monotonic, stable-sort time/value together
7. Record load time, conversion time, and peak memory

Verification points:

- Whether projection of only the required columns works
- Number of copies when converting Arrow chunks to contiguous arrays
- Whether 10M rows x 9-channel-class files can read only the selected channel
- Differences in load time by compression scheme

Fallback:

- If projection and type conversion via the direct API are costly to implement, try `polars` lazy `scan_parquet` + `select([time, channel])`
- If memory is still high, consider row-group-level loading or compressed storage of the time column

---

## 6. Data Storage Strategy

Phase 1 baseline:

```rust
struct WaveStore {
    time: Arc<[f64]>,
    value: Arc<[f32]>,
    channel_name: String,
    sample_count: usize,
    x_min: f64,
    x_max: f64,
    y_min: f32,
    y_max: f32,
}
```

Policy:

- Phase 1 does not hold a DataFrame
- After loading, only the contiguous arrays needed for rendering are kept
- `time` remains `f64` to preserve analysis compatibility
- Waveform values default to `f32`, matching the current Dash version
- Holding all channels simultaneously is considered in Phase 2 and beyond

Conditions for revisiting:

- If the time array alone is too memory-heavy, store evenly-spaced data as `start + dt + len`
- If non-uniform data dominates, continue with `Arc<[f64]>`
- If `f32` quantization affects peak values or display, compare against `f64` storage

---

## 7. LOD / Tile Cache Strategy

In Phase 1, we build a min/max tile pyramid for the currently selected channel.

Baseline rules:

- Treat `time` as a monotonically increasing array
- Convert visible range `[x_min, x_max]` to an index range
- If samples-per-pixel is small, draw raw lines
- If samples-per-pixel is large, draw a min/max envelope
- Prioritize peak preservation in the envelope

Provisional parameters:

| Item | Initial value | Memo |
|------|--------|------|
| Raw draw threshold | `4 samples / pixel` or fewer | Start conservatively |
| Base tile size | `256 samples` | Balance build cost and peak preservation |
| Pyramid fanout | `4` | Keep level count and selection cost low |
| Draw-point target | `2-4 * viewport_width_px` | Stay on the order of screen width even including vertical envelope |

Tile storage candidate:

```rust
struct MinMaxTile {
    start_index: u32,
    end_index: u32,
    min_value: f32,
    max_value: f32,
    min_index: u32,
    max_index: u32,
}
```

Notes:

- `u32` indices suffice up to the 10M-row class, but leave room to switch to `usize` for future giant data
- In Phase 1 it is acceptable to rebuild LOD on channel switch
- Cache sharing on channel switch is handled in Phase 2 and beyond

---

## 8. Rendering Strategy

In Phase 1, we build a custom waveform view in the central region of `egui`.

Rendering responsibilities:

- Determine the plot rect
- Derive the displayed index range from `ViewRange`
- Generate raw-line or min/max envelope draw points
- Pass only screen-width-order points to `egui::Painter`
- Reflect wheel/drag operations into `ViewRange`

`egui_plot` is not used as the primary renderer.
However, brief use only for initial coordinate transforms or interaction sanity checks is permitted.

Conditions for revisiting:

- If `egui::Painter` is slow even with LOD-applied points alone, move to a `wgpu` custom renderer
- If rendering is unstable across `eframe` backends, compare the `glow` backend as well
- If pan/zoom interaction lacks freedom, reconsider direct `winit` + `wgpu`

---

## 9. Phase 1 Implementation Order

1. Create the Rust project in `code/rust_phase1`
2. Display an empty window with `eframe`
3. Display the schema of a hard-coded-path Parquet file
4. Load `time` and one channel
5. Convert into `WaveStore`
6. Display the full range as a raw or min/max envelope
7. Add wheel zoom and drag pan
8. Add LOD/tile cache
9. Add the channel selection UI
10. Measure load, first display, pan/zoom, and memory on small/large data

---

## 10. Pass Criteria

This provisional stack is a candidate for Phase 2 if all of the following hold:

- `test_100k.parquet` loads and displays a single channel
- `panely_large_10s_1mhz_9ch.parquet`-class data loads without crashing
- The UI does not become unresponsive during pan/zoom
- Render processing stays within the order of screen width rather than total sample count
- Memory does not grow continuously under repeated zoom
- Interaction on large data is noticeably lighter than the Dash/Plotly version

---

## 11. Conditions for Revisiting

We revisit this provisional stack in the following cases:

- Required pan/zoom interaction cannot be built naturally with `eframe` / `egui`
- Rendering is heavy even when only LOD-applied points are passed
- Parquet projection does not provide the expected benefit
- Single-channel storage with `time: f64` + `value: f32` is still memory-tight
- LOD build time is long and significantly delays first display
- Implementation volume is too heavy for a Phase 1 vertical-slice verification

Next candidates:

| Problem | Next candidate |
|------|----------|
| Heavy GUI/render | Direct `winit` + `wgpu` |
| Heavy `egui::Painter` | `wgpu` custom renderer |
| Heavy direct Parquet API | `polars` lazy |
| Heavy time-array memory | Evenly-spaced time compression |
| Heavy LOD construction | Lazy tile build / row-group LOD |

---

## 12. References

- [Rust Migration Phase 1 Scope Definition](rust_migration_phase1_scope.md)
- [Rust Migration Plan](rust_migration_plan.md)
- [Rust Migration Phase 0 Performance Measurement Results](rust_migration_phase0_benchmark_results.md)
- [eframe docs.rs](https://docs.rs/eframe/latest/eframe/)
- [egui docs.rs](https://docs.rs/egui/latest/egui/)
- [Apache Arrow Rust parquet docs.rs](https://docs.rs/parquet/latest/parquet/)
- [Polars docs.rs](https://docs.rs/polars/latest/polars/)
- [wgpu docs.rs](https://docs.rs/wgpu/latest/wgpu/)
