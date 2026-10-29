---
project: Panel_y
doc_type: Phase 2 Design Direction
target: Rust Migration Phase 2
created: "2026-04-30"
updated: "2026-05-02"
status: Phase 2 closeout complete
source_proto: "archive/proto_3_1b"
base_implementation: "code/rust_phase1"
---

# Rust Migration Phase 2 Design

---

## 1. Purpose

In Phase 2, we build on the foundation established by Rust Phase 1 ("large-Parquet loading + display-range envelope + pan/zoom") and port the day-to-day operations of the waveform viewer to the Rust version.

Target state for Phase 2:

- Multiple channels can be overlaid in a single row
- Multiple rows can be displayed
- X-axis pan/zoom is synchronized across all rows
- Line and step rendering can be selected per channel
- Per-channel color and line width can be configured
- Per-row Y-axis range can be specified
- The hover position and the value of each channel at that position can be inspected
- The UI does not freeze while loading

Cursor, statistics, FFT, and saving/loading of plot settings are deferred to Phase 3 and later. However, the Phase 2 state model is designed so those can be added on top later.

---

## 2. Basic Approach

We continue the technical direction from Phase 1.

| Item | Phase 2 approach |
|------|-------------|
| GUI | Continue with `eframe` + `egui` |
| Parquet | Continue with projection reads via the `parquet` crate |
| raw data | Share per loaded channel as the equivalent of `Arc<[f32]>` |
| time | Share per dataset; do not duplicate per channel |
| Render data | Use range-aware min/max envelopes as the baseline |
| tile pyramid | Not included in the initial implementation; add it when the need arises |
| Analysis | Continue to assume raw-data access for Phase 3 and beyond |
| UI processing | Separate heavy loading and extraction from the UI thread |

---

## 3. State Model

In Phase 2 we separate UI state from data state.

The main structures we have in mind:

```text
AppState
  DatasetState
    path
    schema
    time_column
    channel_metadata[]
    shared_time
    loaded_channels: ChannelStore

  ViewState
    x_range
    rows: PlotRow[]
    hover_x
    theme
    interaction_mode

  LoadState
    pending_jobs
    progress
    error
```

```text
ChannelStore
  raw_by_channel: channel_name -> ChannelData
  envelope_cache: EnvelopeKey -> MinMaxEnvelope
```

```text
PlotRow
  id
  channels: RowChannel[]
  y_range
  height
```

```text
RowChannel
  channel_name
  draw_mode: line | step
  color
  line_width
  visible
```

Key separations:

- `DatasetState` owns the file and the raw data
- `ViewState` holds only the current display state
- `PlotRow` carries only display composition and does not own raw data
- Even when the same channel appears in multiple rows, only one copy of its raw data is kept

---

## 4. Data-Loading Strategy

### Schema read

As in Phase 1, we first read only the Parquet metadata.

What the schema read does:

- Retrieves row count
- Retrieves row group count
- Detects the `time` column
- Detects numeric channels
- Records physical / logical types

### Channel loading

In Phase 2, only the channels needed for display are loaded on demand.

Basic rules:

- When a channel is added to a row and is not yet loaded, load it
- Already loaded channels are reused
- `time` is shared per dataset
- Loading all channels in bulk is avoided in the initial implementation

### Asynchronous loading

Synchronous loading was workable in Phase 1, but in Phase 2 we go asynchronous so the feel of loading multiple channels remains acceptable.

UI policy:

- Channels being loaded are shown in a loading state
- Risky operations such as schema changes, file changes, and deletion of the affected channel are temporarily disabled
- Once loading completes, the envelope cache is invalidated and the view is redrawn
- Errors are surfaced on screen so the user can review them

---

## 5. Rendering Strategy

### X-axis

`ViewState` holds a single `x_range`, shared across all rows.

Interactions:

- drag: pan
- wheel / pinch: zoom around the cursor position
- horizontal scroll: pan
- double click / reset: fit to full extent

### Y-axis

The Y-axis is per row.

Initial implementation:

- auto range
- manual min/max

To consider later:

- Auto range computed only from the channels visible in that row
- Robust auto range when outliers are present
- Y range reset

### Line rendering

Line rendering uses the range-aware min/max envelope from Phase 1.

Extraction rules:

- Compute sample indices within the display range using the equivalent of `partition_point`
- Choose the bucket count based on display width
- Draw the min/max of each bucket
- When the visible sample count is small enough, fall back to raw-line rendering

### Step rendering

For step rendering, min/max alone can leave edge positions ambiguous.

Initial-implementation policy:

- Gate/PWM-style channels can be drawn as step
- Value-change points within the display range are preserved with priority
- When change points are too dense, retain the first value, last value, min/max, and a representative change point inside each bucket

Step rendering is treated as a Phase 2 risk item; we stabilize line rendering first.

---

## 6. Envelope / LOD Strategy

In the early part of Phase 2 we do not build a full tile pyramid.

Baseline approach:

- Raw data is kept per loaded channel
- At display time, the required envelope is built from `x_range` and pixel width
- For the same channel, range, and bucket count, the envelope is shared across rows
- When the range changes during pan/zoom, re-extract as needed

Cache policy:

- Initially, keep no more than the "currently displayed envelope"
- Do not accumulate large numbers of historical ranges from pan
- Discard the cache on channel switch, width change, or data reload

Conditions for adding a tile pyramid:

- Pan/zoom becomes heavy with many channels / many rows
- Range-aware envelope re-extraction is perceptibly slow at the 90M-point scale
- Step edge extraction is heavy
- The same range is re-extracted repeatedly

---

## 7. UI Layout

Baseline Phase 2 screen:

```text
left panel
  file path
  schema / load controls
  channel list
  row controls
  style controls

central panel
  waveform rows
  shared x axis
  hover line
  value readout

status area
  load progress
  sample count
  memory / timing summary
  errors
```

UI implementation priority:

1. File and schema display
2. Row add / remove
3. Channel selection per row
4. Style editing
5. Hover value display
6. Theme switching

---

## 8. Implementation Order

Recommended order:

| ID | Task | Completion criteria |
|----|------|----------|
| P2-01 | Tidy up the state model | data/view/load state are separated |
| P2-02 | Make it possible to load multiple channels | loaded channels are cached and reused |
| P2-03 | Multiple channels in a single row | multiple waveforms can be overlaid in the same row |
| P2-04 | Multiple rows | rows can be added and removed |
| P2-05 | X-axis synchronization | pan/zoom/reset apply to all rows |
| P2-06 | Line / step rendering | after line stabilizes, verify edge preservation for step |
| P2-07 | Style settings | color, line width, and visibility can be changed |
| P2-08 | Y-axis range specification | auto/manual can be used per row |
| P2-09 | Hover synchronization | the same X position's value is shown across all rows |
| P2-10 | Asynchronous loading | the UI does not freeze while loading multiple channels |
| P2-11 | Theme switching | light/dark can be toggled |
| P2-12 | Performance re-measurement | retain measurement results at medium, large, and 90M-point scales |

Progress notes:

- 2026-04-30: P2-01 complete. Split `PanelYApp` into `DatasetState` / `ViewState` / `LoadState`
- 2026-04-30: P2-02 complete. Added dataset-level shared time, per-channel raw cache, and a per-display-range envelope cache
- 2026-04-30: P2-03 partially complete. Multiple channels can now be added to the first row and overlaid; row add/remove and style editing are not yet implemented
- 2026-04-30: P2-04 implemented. Row add/remove, adding channels to the selected row, and dividing the drawing rect per row are supported
- 2026-04-30: P2-10 implemented and verified in the GUI. Channel/time loading was moved to a worker thread, with the UI state updated only on completion
- 2026-04-30: Implementation foundation for P2-05 confirmed. `ViewState::x_range` is held as a single value, and pan/zoom/reset apply to a range shared by all rows. Manual multi-row GUI verification will continue next time
- 2026-04-30: P2-04 / P2-05 manual verification complete. Delete/Remove across multiple rows and pan/zoom/reset synchronized across all rows are smooth with no issues
- 2026-04-30: To address overflow as the number of rows grows, the left pane was made vertically scrollable, and the waveform pane was changed so that each row retains a minimum height and the pane scrolls vertically
- 2026-04-30: Recorded a UI improvement candidate: separate the waveform area's X zoom onto `Ctrl + wheel` and reserve plain wheel for vertical scroll on the waveform side. We will keep the current behavior through the performance-validation stage
- 2026-04-30: P2-06a implemented. Added `Line` / `Step` draw modes to `RowChannel`, with a per-channel toggle UI inside the row. In low-density display ranges, Step renders an exact staircase from the raw samples; in high-density ranges, it falls back to the existing min/max envelope
- 2026-04-30: P2-06a verified in the GUI. Line/Step switching works correctly while zoomed in on `pwm_1kHz` / `noise`. Added a script that generates `step_validation_100k.parquet` for edge-position verification
- 2026-04-30: Verified Line/Step rendering on `step_validation_100k.parquet` for `pwm_1kHz` / `pwm_1kHz_delay_1sample` / `pwm_1kHz_advance_1sample`, and confirmed that the 1-sample lead/lag relationships are accurate
- 2026-04-30: P2-06b implemented. Even at high density, if the change-point count is at or below `MAX_STEP_CHANGE_POINTS`, Step rendering preserves exact edge timing as change-point Step; only when change points are too dense does it fall back to the min/max envelope. Mid-range zoom on `gate_pwm` / `pwm_10kHz` confirms that change-point preservation is needed there
- 2026-04-30: P2-06b verified in the GUI. For `gate_pwm` Step rendering, both the change-point Step region and the min/max envelope fallback region look natural with no visual inconsistency
- 2026-04-30: P2-07 implemented. Visibility, color override, and line width can now be edited per channel within a row, and hidden channels are excluded from rendering, Y auto range, and the status summary
- 2026-04-30: P2-07 verified in the GUI. Visibility ON/OFF, immediate reflection of line-width changes, and custom color / reset all work as expected
- 2026-04-30: P2-09 implemented. The hover X position is shared via `ViewState`, with vertical-line display across all rows and a value readout for visible channels. Line shows the nearest-sample value; Step shows the sample-and-hold value
- 2026-04-30: P2-09 GUI verification complete. Verified on `test_100k.parquet` / `panely_large_10s_1mhz_9ch.parquet` that multi-row hover, Line/Step value display, exclusion of hidden channels, clearing hover outside the plot, switching the readout between left and right, and update responsiveness all behave as expected
- 2026-04-30: P2-08 implemented. Added Auto/Manual Y-axis range settings to `PlotRow`, supporting per-row min/max input and manual-range rendering. In Manual mode the trace is clipped to the plot rect
- 2026-04-30: P2-08 GUI verification complete. Confirmed that Manual min/max take effect, that the initial value and Reset return to `-1..1`, and that hiding a channel while Manual is active keeps the plot min/max at the Manual setting
- 2026-04-30: P2-08 improvement. Changed the Manual-toggle and Manual-Reset initial min/max to be derived from the most recent Auto-displayed Y range, rather than the fixed `-1..1`. Falls back to `-1..1` only when no Auto range has been captured yet
- 2026-04-30: P2-08 improvement GUI verification complete. Confirmed that the initial value on Manual toggle, and the return to the most recent Auto range on Reset, both behave correctly
- 2026-04-30: P2-12 measurement complete. Added `--bench-phase2` and measured multi-row display-data generation, hover, and RSS for the medium dataset (all 8 ch), the large dataset (all 9 ch), and the large dataset (representative 4 ch). For all 9 ch on the large dataset, visible-trace generation was avg 31.3 ms / max 92.5 ms with RSS-after-bench of 435.4 MiB. The results are recorded in [rust_migration_phase2_benchmark_results.md](rust_migration_phase2_benchmark_results.md)
- 2026-04-30: Organized post-P2-12 performance-improvement candidates. Rather than immediately introducing tile / LOD, we proceed in the order: GUI frame timing, in-interaction preview / debounce, earlier Step wide-range fallback, channel parallelization / overscan cache, and finally coarse tile / LOD. This is recorded in [rust_migration_phase2_performance_plan.md](rust_migration_phase2_performance_plan.md)
- 2026-05-02: Closed out Phase 2 performance improvements. Demoted preview to a debug feature, and implemented and verified the overscan range cache, shared bucket boundaries, Line lazy tile / LOD, ripple validation, and Step edge cache / dense LOD. The initial-build cost remains a known open item but will not block Phase 3

---

## 9. Phase 2 Completion Criteria

Phase 2 is considered complete when all of the following are met.

- Routine waveform inspection can be done with the Rust version alone
- Multiple channels can be overlaid in a single row
- Multiple rows can be displayed
- X-axis pan/zoom is synchronized across all rows
- Line / step rendering, color, line width, and Y-axis range can be operated at a minimum level
- Channel values at the hover position can be inspected
- Even with multiple channels displayed, draw cost stays on the order of display width
- Operation does not become unusable on medium or large data
- A measurement strategy is settled for 90M-point or real-device log-equivalent data

Closeout outcome:

- The above criteria are treated as completed for Phase 2
- P2-11 theme switching remains as UI polish. Wheel-operation separation, box zoom, and the data-load UI improvements are handled in Phase 2.5, before Phase 3
- Background execution of the initial Line tile build and Step edge cache build will be reconsidered if real-world use surfaces a perceived issue

---

## 10. Risks

| Risk | Mitigation |
|--------|------|
| Memory grows too large when loading multiple channels | On-demand load, unload, and a cap on the channel cache |
| Edges become invisible in step rendering | Preserve change points separately from min/max |
| `egui::Painter` is heavy with many rows | Measure draw-point count and shape-generation time; if needed, move to a `wgpu` custom renderer |
| Async loading makes state complex | Separate data/view/load state and update the UI state only when a job completes |
| Compatibility of plot settings becomes hard later | Design the Phase 2 state model so it can map directly into setting persistence |

---

## 11. References

- [Rust Migration Phase 1 Evaluation Notes](rust_migration_phase1_evaluation.md)
- [Rust Migration Phase 1 Benchmark Results](rust_migration_phase1_benchmark_results.md)
- [Rust Migration Phase 2 Performance Measurement Results](rust_migration_phase2_benchmark_results.md)
- [Rust Migration Phase 2 Performance-Improvement Notes](rust_migration_phase2_performance_plan.md)
- [Rust Migration Phase 0 UI Specification](rust_migration_phase0_ui_spec.md)
- [Rust Migration Phase 0 Plot-Config Specification](rust_migration_phase0_plotconfig_spec.md)
- [Rust Migration Plan](rust_migration_plan.md)
