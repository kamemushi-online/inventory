---
project: Panel_y
doc_type: FFT Panel Display Fix Plan
target: code/rust_phase1
created: "2026-05-15"
status: draft
---

# FFT Panel Display Fix Plan

## Background

When Cursor A/B are set, Interval statistics show up inside the Cursor information bottom panel.
Currently the FFT section is rendered beneath the statistics table, but the Cursor information panel as a whole has no vertical scroll — so when the panel height is insufficient, the FFT section gets clipped at the bottom.

The structure at the time of this investigation:

- `CentralPanel` is split vertically into the waveform area and the Cursor information bottom panel.
- The Cursor information bottom panel height is managed by `AnalysisState::info_panel_height`.
- Interval statistics use their own `ScrollArea::vertical()` inside `draw_interval_statistics_table()`.
- FFT is rendered after the statistics table via `draw_fft_section()`.
- The Cursor information panel as a whole has no outer vertical scroll.

As a result, once the statistics table consumes the available display area, the FFT controls become invisible or are clipped off the bottom.

## Approach

Roll the fix out in two stages.

1. Make the entire Cursor information panel vertically scrollable to immediately resolve the FFT-section clipping.
2. Extend the boundary between Interval statistics and FFT into a split UI that can be adjusted vertically inside the bottom panel.

The reason for splitting it in two: a global scroll alone reliably resolves the current issue at low risk. Then, once statistics and FFT are split into separate panes, we can refine things while observing how scroll regions and drag interactions interact.

## Recommended UI

The end goal targets this structure:

1. Cursor summary header
   - `Cursor information`
   - Cursor A/B
   - `dt`
   - `1/|dt|`
   - `Collapse`
   - `Hide`
2. Analysis content area
   - Top: Interval statistics
   - Middle: split resize handle
   - Bottom: FFT

The Cursor summary header is always shown at the top.
Statistics and FFT each have their own independent vertical scroll.
Bottom-panel overall height adjustment, Collapse, and Hide keep their existing behavior.

## Phase 1: Global Scroll

Goal: make sure the FFT section is reachable even when it ends up hidden below the statistics table.

Implementation idea:

- Wrap the body region of `draw_cursor_info_panel()` in an `egui::ScrollArea::vertical()`.
- Keep the header outside the scroll.
- Place `Interval statistics`, Cursor values, and FFT inside the same body scroll region.
- Preserve the existing horizontal scroll inside `draw_interval_statistics_table()`.

At this stage, don't implement boundary resizing between the statistics and FFT.
The behavior is: scrolling vertically inside the panel reveals the FFT controls.

Caveats:

- The statistics table also has its own vertical `ScrollArea`, so wheel events end up nested.
- Phase 1 prioritizes resolving the clipping; finer interaction polish is left to Phase 2.

## Phase 2: Statistics / FFT Split Resize

Goal: within the limited bottom-panel height, let users adjust the size ratio between the statistics table and the FFT analysis area.

Proposed state additions:

```rust
struct AnalysisState {
    // existing fields...
    info_panel_height: f32,
    interval_stats_cache: IntervalStatsCache,
    fft: FftAnalysisState,
    analysis_split_height: f32,
}
```

`analysis_split_height` represents the height of the upper Interval statistics region.

Proposed constants:

```rust
const ANALYSIS_SPLIT_DEFAULT_HEIGHT: f32 = 140.0;
const ANALYSIS_SPLIT_MIN_HEIGHT: f32 = 72.0;
const ANALYSIS_SPLIT_HANDLE_HEIGHT: f32 = 8.0;
const FFT_SECTION_MIN_HEIGHT: f32 = 120.0;
```

Proposed rendering structure:

```text
Cursor information header

available_body_height
├─ interval_stats_height
│  └─ Interval statistics ScrollArea
├─ split handle
└─ fft_height
   └─ FFT ScrollArea
```

Resize handling:

- Place a `Sense::drag()` handle between statistics and FFT.
- Update `analysis_split_height` based on the drag delta.
- The clamp range is determined by:
  - min: `ANALYSIS_SPLIT_MIN_HEIGHT`
  - max: `available_body_height - ANALYSIS_SPLIT_HANDLE_HEIGHT - FFT_SECTION_MIN_HEIGHT`
- When the panel height is small, automatically compress the split height.

## Scroll Design

In the final form, don't lean too hard on the outer global scroll as the primary path.
Pinning the header and giving the statistics pane and FFT pane their own scroll bars makes it clearer which area is being operated on.

Recommendation:

- Phase 1: body-wide scroll
- Phase 2: fixed header + two-pane independent scroll

By Phase 2, the global scroll should be unnecessary or kept to a minimal fallback.
The deeper the double-scroll nesting, the harder it gets for the user to know which region they're scrolling.

## Relationship to Existing Features

Preserved:

- Overall bottom-panel height resize
- Collapse / Hide
- Pinned channel column and horizontal scroll in Interval statistics
- FFT collapse
- FFT job background execution
- FFT result invalidation on cursor changes

Changed:

- The body display structure of the Cursor information panel
- Vertical size management of the statistics table
- Reserving display area for the FFT section

## Verification Items

Automated:

- `cargo fmt`
- `cargo check`
- `cargo test`

Manual:

- After setting Cursor A/B, the FFT controls are either visible or reachable via scrolling.
- Even with many statistics rows, the FFT section remains reachable.
- The bottom-panel overall resize and the statistics / FFT boundary resize do not interfere with each other.
- Layout doesn't break after Collapse / Hide.
- After running FFT, the result summary and spectrum plot are inspectable inside the lower pane.
- Amplitude / dB and frequency-range changes refresh the display without re-running FFT.

## Implementation Order

1. Make the Cursor information panel body vertically scrollable.
2. Set `max_height` on the statistics table's vertical `ScrollArea` to reduce conflict with the global scroll.
3. Add `analysis_split_height` to `AnalysisState`.
4. Add the statistics / FFT split handle.
5. Give the statistics pane and FFT pane independent scroll regions.
6. Tune the clamp behavior at small bottom-panel heights.
7. Run `cargo fmt`, `cargo check`, and `cargo test`.
8. Manually verify with representative data.

## Decision

Land Phase 1 first to resolve the FFT-section clipping.
Then add Phase 2 for statistics / FFT boundary resizing — that ordering balances safety and usability well.
