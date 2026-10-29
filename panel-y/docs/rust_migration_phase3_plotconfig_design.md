---
project: Panel_y
doc_type: Phase 3 Plot Config Detailed Design
target: Rust migration Phase 3
created: "2026-05-06"
updated: "2026-05-06"
status: detailed design in progress
source_proto: "archive/proto_3_1b"
implementation: "code/rust_phase1"
---

# Rust Migration Phase 3 — Plot Config Detailed Design

---

## 1. Purpose

Before adding Cursor, statistics, and FFT in Phase 3, we want users to be able to save and restore the display configuration they actually use in practice, as a file.

The baseline is `.pyc.json` version 1 compatibility with the current Dash version (`proto_3_1b`). The Rust version already gained `SessionPreset` in Phase 2.5, but that is only an in-memory transient restore, not a proper plot configuration. Phase 3 builds on that foundation and adds file I/O, compatible parsing, partial restore, error reporting, and reload confirmation to the scope.

---

## 2. Scope

In scope:

- Read compatibility with `.pyc.json` version 1
- Saving the plot config from the Rust version
- Reloading plot configs saved by the Rust version
- Validation of `data_path` and schema loading
- Restoring row layout, channel layout, Line / Step mode, and Y-axis ranges
- Restoring channel color and line width
- Restoring the Rust-side `visible` flag
- Restoring theme and the scroll-zoom equivalent
- Partial restore of configs that contain missing channels, invalid values, or empty rows
- Asynchronous on-demand loading of required channels after a plot config is loaded

Out of scope:

- Saving Cursor A/B, statistics, FFT conditions, and FFT results
- Saving hover position, measurement results, and analysis state
- Saving raw data, LOD cache, Line tiles, Step edge cache, and envelope cache
- Full backward compatibility guarantees with the Dash version
- Opening multiple datasets from a single plot config

---

## 3. Relationship to existing implementation

The Phase 2.5 Rust implementation already has the following:

```text
PanelYApp
  DatasetState
    parquet_path
    schema
    shared_time
    loaded_channels
  ViewState
    selected_channel
    selected_row_id
    x_range
    rows: Vec<PlotRow>
  SessionPreset
    parquet_path
    selected_channel
    selected_row_index
    x_range
    large_preview_enabled
    rows
```

Items to reuse:

- The display-state extraction logic in `SessionPreset::capture()`
- The schema matching and missing-channel handling in `SessionPreset::apply_to_view()`
- `PlotRow` / `RowChannel` / `DrawMode` / `RowYRange`
- `validate_parquet_path()`
- Schema loading, asynchronous channel loading, and cache invalidation

Items that need extension:

- Add `serde` / `serde_json` to `Cargo.toml` as direct dependencies
- Add `plot_config.rs` to separate plot-config file read/write and normalization
- Give `ViewState` proper `theme` and `scroll_zoom`-equivalent state
- Factor out a loader function that can initiate channel loads without adding a row, for use when applying a plot config
- Clearly distinguish `SessionPreset` from the proper plot config in the UI

---

## 4. File format strategy

### 4.1 Compatible top level

When saving, we keep the top-level fields that the current Dash version can ignore safely.

```json
{
  "version": 1,
  "data_path": "/path/to/data.parquet",
  "rows": [
    {
      "channels": ["sine_50Hz", "pwm_1kHz"],
      "ymin": -1.0,
      "ymax": 6.0,
      "step_channels": ["pwm_1kHz"]
    }
  ],
  "ch_styles": {
    "sine_50Hz": {"color": "#50BEFF", "width": 1.25}
  },
  "theme": "dark",
  "scroll_zoom": false
}
```

Why we save with `version: 1`:

- The primary goal of Phase 3 is compatibility with the current `.pyc.json`
- The Dash version is designed to ignore unknown keys, so Rust-specific information can be tucked into a separate namespace
- `version: 2` will be revisited in Phase 5 or later, when handling AnalysisState or an extension change

### 4.2 Rust-specific namespace

Rust-specific state that cannot be expressed in version 1 is saved under a top-level `rust` key.

```json
{
  "version": 1,
  "data_path": "/path/to/data.parquet",
  "rows": [],
  "ch_styles": {},
  "theme": "dark",
  "scroll_zoom": false,
  "rust": {
    "version": 1,
    "rows": [
      {
        "channels": [
          {
            "path": "sine_50Hz",
            "visible": false,
            "draw_mode": "line",
            "color": "#50BEFF",
            "line_width": 1.25,
            "color_index": 0
          }
        ]
      }
    ]
  }
}
```

Belongs in `rust`:

- Per-row, per-channel `visible`
- Per-row, per-channel style overrides
- `color_index`

Does not belong in `rust`:

- `x_range`
- `selected_channel`
- `selected_row_id`
- `large_preview_enabled`
- Hover position
- Caches and LOD

`x_range` is included in the Phase 2.5 `SessionPreset`, but the current `.pyc.json` v1 does not include it. The proper Phase 3 plot config does not save it either. If we later want to persist the display range as well, we will handle it under `version: 2` or a separate namespace.

---

## 5. Internal model

Deserializing directly into `serde` types tends to fail the whole file when any known key has the wrong type. Phase 3 requires "fall back safely for items that can be restored," so at the boundary we read `serde_json::Value` and convert it to the internal model through a normalization function.

Proposed module:

```text
src/plot_config.rs
  PlotConfigDocument
  PlotConfigRow
  PlotConfigStyle
  RustPlotConfigExt
  PlotConfigLoadPlan
  PlotConfigApplyReport
  PlotConfigError

  read_plot_config(path) -> Result<PlotConfigDocument, PlotConfigError>
  normalize_plot_config(value, config_path) -> (PlotConfigDocument, PlotConfigApplyReport)
  capture_plot_config(dataset, view) -> PlotConfigDocument
  write_plot_config(path, document) -> Result<(), PlotConfigError>
  build_apply_plan(document, schema) -> PlotConfigLoadPlan
```

`PlotConfigDocument` mirrors the file spec, while `PlotConfigLoadPlan` is shaped to make it easy to apply to the Rust `ViewState`.

```text
PlotConfigLoadPlan
  data_path: PathBuf
  theme: ThemeMode
  scroll_zoom: bool
  rows: Vec<PlotRow>
  selected_channel: String
  channels_to_load: Vec<String>
  report: PlotConfigApplyReport
```

`PlotConfigApplyReport` is intended for UI display and carries restore counts and warnings.

```text
PlotConfigApplyReport
  restored_rows
  restored_channels
  channels_to_load
  missing_channels
  skipped_empty_rows
  invalid_y_ranges
  invalid_colors
  invalid_line_widths
  ignored_fields
```

---

## 6. Channel identity

The primary matching key during restore is `SchemaSummary.channels[].path`.

Compatibility fallbacks:

- Exact `path` match takes priority
- If there is no match, try an exact `display_name()` match
- If `display_name()` matches multiple channels, do not restore
- Case-insensitive matching is not done in the initial implementation

When saving, always save the Rust-side `channel_path`.

---

## 7. Y-axis range

The current `.pyc.json` stores `ymin` / `ymax` as `number` or `null`. The Phase 0 spec also allows specifying only one side.

In Phase 3 we extend `RowYRange` to support one-sided specifications.

```text
RowYRange
  mode: Auto | Manual
  min: Option<f64>
  max: Option<f64>
  last_auto: Option<(f64, f64)>
```

Resolution at display time:

- `Auto`: use the current auto range
- `Manual` with both sides specified: `min..max`
- `Manual` with one side specified: the unspecified side uses the current auto range
- If there is no auto range yet, the unspecified side uses the value from `RowYRange::default()`
- If `min > max`, if any value is non-finite, or if any value is not a number, invalidate the manual specification and fall back to `Auto`

When saving:

- `Auto`: `ymin: null`, `ymax: null`
- `Manual`: a number for the specified side, `null` for the unspecified side

---

## 8. Theme / scroll zoom

### theme

Add `theme: ThemeMode` to `ViewState`.

```text
ThemeMode
  Dark
  Light
```

Loading:

- `"dark"` -> `ThemeMode::Dark`
- `"light"` -> `ThemeMode::Light`
- Missing or invalid -> `ThemeMode::Dark`

Applying:

- `ThemeMode::Dark`: `ctx.set_visuals(egui::Visuals::dark())`
- `ThemeMode::Light`: `ctx.set_visuals(egui::Visuals::light())`

### scroll zoom

Add `scroll_zoom: bool` to `ViewState`.

Meaning:

- `false`: preserve current Rust behavior; a plain vertical wheel does not trigger X zoom
- `true`: allow plain vertical wheel zoom anchored at the pointer position

`x_zoom_factor()` should decide based on `scroll_zoom || zoom_modifier_pressed`. Pinch zoom remains allowed as before.

---

## 9. Load flow

Plot-config loading uses clear transaction boundaries so it does not break the current display.

```text
Load Plot Config
  1. Abort any active load job
  2. Validate the config path
  3. Read the JSON
  4. Normalize into PlotConfigDocument with a tolerant parser
  5. Resolve data_path and validate the Parquet path
  6. Read the schema into a local variable
  7. Build a PlotConfigLoadPlan from the schema and the plot config
  8. Once everything up to here has succeeded, commit to DatasetState/ViewState
  9. Discard raw cache / draw cache
  10. Start async load only for visible channels
```

If it fails before commit:

- Do not modify the current `DatasetState` / `ViewState`
- Show the reason in `load.status` and `load.error`

If a channel load fails after commit:

- Keep the row layout
- Leave the affected channel marked as unloaded
- Show the failing channels and the reason in `load.error`

---

## 10. Channel-load integration

The current `load_selected_channel()` couples "add the selected channel to a row" with "read raw data." When loading a plot config, the row layout has already been restored, so we need a function that starts loading without adding a row.

Factoring proposal:

```text
request_channel_load(ctx, channel_path, target_row_id, target_row_label, add_to_row)
```

Usage:

- Normal UI Load/Add: `add_to_row = true`
- Restore load after applying a plot config: `add_to_row = false`

Channels to load after applying a plot config:

- Channels with `visible == true`
- Channels that exist in the schema
- Duplicates are loaded only once
- Hidden channels remain in their rows but are excluded from the initial load

LOD / tile / edge cache:

- Not saved to the file
- Existing caches are discarded when a plot config is applied
- They are regenerated from raw data at display time through the existing on-demand design

---

## 11. Save flow

Saving generates a `PlotConfigDocument` from the current `DatasetState` and `ViewState` and writes it out.

Pre-save conditions:

- No active load job
- `dataset.schema` is present
- `dataset.parquet_path` is not empty
- Row state is normalized via `ensure_row_state()`

Save contents:

- `data_path`: the current `dataset.parquet_path`
- `rows`: row layout with empty-channel rows removed
- `step_channels`: channels in the row whose mode is `DrawMode::Step`
- `ch_styles`: a representative style per channel path
- `theme`: `ViewState.theme`
- `scroll_zoom`: `ViewState.scroll_zoom`
- `rust.rows`: per-row, per-channel `visible`, style, and `color_index`

Representative style in `ch_styles`:

- If the same channel appears in multiple rows, save the first occurrence's style into the version-1 compatible field
- Per-row deltas are saved under `rust.rows[].channels[]`
- When opened in the Dash version, only the representative style is reflected

Destination:

- The UI permits `.pyc.json` and `.json`
- If the parent directory does not exist, create it
- Write failures must not modify the current state; show an error instead

---

## 12. UI

Add a proper Plot config section to the left pane.

```text
Plot config
  Path [text edit]
  Browse...
  Load
  Save
  Save As...
  Status / warning summary
```

UI guidelines:

- Make it a separate section from `Session preset`
- Indicate that `Plot config` is `.pyc.json` compatible
- Disable Load/Save while a load job is active
- After Load, show the number of restored rows, restored channels, and missing channels in the status
- Detailed warnings should be reviewable via a collapsible panel

How to treat `Session preset`:

- It can stay during Phase 3 implementation
- Once the proper plot config is stable, mark it as debug-only or a deletion candidate
- Keep the UI wording as "Rust session only; not .pyc.json."

---

## 13. Validation and fallbacks

| Item | Policy |
|------|--------|
| Invalid JSON | Load fails; current state is not modified |
| Missing `version` | Treat as `1` |
| `version` other than 1 | Read what can be read, but warn |
| Unknown keys | Ignore |
| Missing/empty `data_path` | Load fails; current state is not modified |
| Relative `data_path` | Resolve relative to the config file's parent directory; on failure, also try relative to cwd |
| Parquet missing or wrong extension | Load fails; current state is not modified |
| Schema load failure | Load fails; current state is not modified |
| Invalid `rows` type | Treat as no rows and create a single fallback row |
| Invalid channel type within a row | Ignore only that channel |
| Non-existent channel | Ignore and record in the report |
| Row that becomes empty | Exclude |
| All rows empty | Create one empty row; do not auto-add the first schema channel |
| `step_channels` not in the row | Ignore |
| `ymin > ymax` | Invalidate that manual specification |
| Invalid color | Fall back to the default palette and record in the report |
| Invalid line width | Fall back to `1.0` and record in the report |
| Out-of-range line width | Clamp to `MIN_TRACE_LINE_WIDTH..=MAX_TRACE_LINE_WIDTH` |
| Invalid theme | Fall back to `dark` |
| Invalid scroll_zoom | Fall back to `false` |
| Invalid `rust` extension | Restore using only the version-1 compatible fields |

---

## 14. Test plan

Unit tests:

- Can load a minimal valid v1
- Treats missing `version` as v1
- Ignores unknown keys
- Does not panic on invalid `rows` / `ch_styles` types
- Skips missing channels and records them in the report
- Excludes empty rows
- Falls back to a single empty row when all rows are empty
- `step_channels` only applies to channels in the row
- `ymin > ymax` falls back to Auto
- Preserves a one-sided Y specification
- Converts `#RRGGBB` colors to `egui::Color32`
- Invalid colors fall back to the default
- Invalid line widths fall back to `1.0`; out-of-range values are clamped
- Restores `rust.rows[].channels[].visible`
- Round-trips per-row style differences for the same channel via the Rust extension
- Saved JSON does not contain cache, LOD, or analysis state

Integration-equivalent:

- A Dash-version sample `.pyc.json` loads in Rust with matching row / channel / style / theme
- A plot config saved by Rust reloads in Rust with matching row / channel / visible / style / theme / scroll_zoom
- A non-existent data path does not break the current display
- Configs that contain channels not in the schema result in a partial restore
- Only visible channels are targeted for async loading

Manual verification:

- Save/load against `data/test_100k.parquet`
- Mixed Line + Step rows
- Multiple rows
- Hidden channels
- Dark/light theme
- `scroll_zoom` true/false
- Configs broken by missing channels

---

## 15. Implementation order

### P3-01 Add `plot_config.rs`

- Add `serde` / `serde_json`
- Implement JSON read/write functions
- Implement the tolerant parser and report
- Implement normalization for hex color, line width, theme, and scroll zoom

Done when:

- Unit tests with no file I/O verify the normalization results

### P3-02 ViewState extension

- Add `ThemeMode`
- Add `ViewState.theme` and `ViewState.scroll_zoom`
- Add a `ctx.set_visuals()` application site
- Extend `RowYRange` to support one-sided specifications

Done when:

- The existing tests pass with the updated state model

### P3-03 Save implementation

- Implement `capture_plot_config(dataset, view)`
- Round-trip including the `rust` namespace
- Add the save dialog / save button to the UI

Done when:

- JSON saved by the Rust version can be parsed again

### P3-04 Load implementation

- Implement the transactional load flow
- Load the schema into a local variable before commit
- Commit the `PlotConfigLoadPlan` to `DatasetState` / `ViewState`
- Start async loads for visible channels

Done when:

- An invalid config or data path does not break the current display
- A valid config restores rows and channels

### P3-05 Factor out the loader

- Factor the channel-load initiation out of `load_selected_channel()`
- Handle both "add to row" and "do not add to row"
- Avoid duplicate loads

Done when:

- Both the normal Load/Add flow and the auto-load after a plot-config load work

### P3-06 UI polish and report display

- Add the `Plot config` section
- Display the warning summary
- Make the difference from `Session preset` clear

Done when:

- Success, partial restore, and failure states are distinguishable in the UI

### P3-07 Verification

- Add unit tests
- Make `cargo test` pass
- Verify save/load in the GUI with representative data

Done when:

- The Phase 3 completion criteria are satisfied

---

## 16. Open items

- Extension change to `.panely.json` will be considered in Phase 5
- `version: 2` will be considered when including AnalysisState or x_range persistence
- We do not guarantee full fidelity when opening a Rust-saved file in the Dash version
- Whether to use a relative data path inside the plot-config file at save time is not handled in the initial implementation; absolute-path saving is kept
- Whether to pre-load hidden channels after a plot-config load is not done in the initial implementation
