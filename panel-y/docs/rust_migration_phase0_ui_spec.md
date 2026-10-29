---
project: Panel_y
doc_type: UI Operation Spec
target: Rust Migration Phase 0
created: "2026-04-27"
updated: "2026-04-27"
status: Complete
source_proto: "archive/proto_3_1b"
---

# Rust Migration Phase 0 UI Operation Specification

---

## 1. Scope

This specification records the UI behavior of `archive/proto_3_1b/app.py` as a reference for the Rust port.
The Rust version may use a different implementation approach, but the user operations and expected behaviors recorded here are preserved in principle.

---

## 2. Screen Layout

### Header Bar

Laid out from left to right:

- Application name: `PanelY`
- Parquet file path input
- `読み込み` (Load) button
- Plot configuration path input
- Save plot-configuration button
- Load plot-configuration button
- Plot configuration status
- Scroll-zoom lock toggle
- Measurement mode toggle
- Data load status
- Theme toggle
- Header-pin toggle
- Credits

### Row Definition Area

- `+ 行追加` (Add row)
- `更新 (Ctrl+Enter)` (Update (Ctrl+Enter))
- `行設定` (Row settings) collapsible
- Per-row channel selection
- Per-row step-target selection
- Per-row Y-axis min/max specification
- Row-delete button

### Channel Settings Panel

- `チャンネル設定（色・太さ）` (Channel settings — color and width) collapsible
- Color selector per channel currently in use
- Line-width input per channel currently in use

### Analysis Panel

- Cursor A/B and Δt display
- Interval statistics table
- FFT controls
- FFT graph

### Waveform Display Area

- Header row: `Channel / Value`, `Waveform`
- Per-row label column
- Per-row waveform graph
- X-axis labels shown only on the bottom row

---

## 3. Data Loading Operations

### Parquet Path Input

Operation:

1. Type a path into the file path field
2. Suggestions appear while typing
3. Clicking a suggestion fills the input field

Expected behavior:

- Directory suggestions are shown with a trailing `/`
- Only `.parquet` files are eligible load candidates
- Up to 30 suggestions are shown

### Load

Operation:

1. Specify a `.parquet` file
2. Press `読み込み`

Expected behavior:

- Show an error if the file does not exist
- Show an error if a directory is specified
- Show an error for non-`.parquet` inputs
- Treat every column other than `time` as a channel
- Show an error if there are zero channels
- If the time column is non-monotonic, sort it and attach a warning to the status
- Auto-generate up to 8 initial rows, one channel per row
- Fill the plot-configuration path field with `{dataFileName}.pyc.json`

---

## 4. Row Configuration Operations

### Add Row

Operation:

- Press `+ 行追加`

Expected behavior:

- An empty row configuration is appended at the bottom
- The row number uses the `row-count` value at the time of addition

### Remove Row

Operation:

- Press the delete button on the target row

Expected behavior:

- Only the target row is removed
- The very last row is not allowed to disappear, even if delete is pressed
- Spurious firings of the pattern-matching callback must not cause a deletion

### Channel Selection

Operation:

- In the per-row dropdown, select one or more channels

Expected behavior:

- Multiple channels can be overlaid in the same row
- Empty rows are skipped when generating waveforms

### Step Display

Operation:

- In the per-row step dropdown, select the target channels

Expected behavior:

- The targeted channels are rendered as step plots
- In the current Dash version, this is equivalent to Plotly's `line_shape="hv"`

### Y-Axis Range Specification

Operation:

- Enter values into the per-row `Y: min ~ max`

Expected behavior:

- If both min and max are specified, the Y-axis range is fixed
- If only min or only max is specified, treat it as a one-sided range
- If both are blank, auto-range

---

## 5. Waveform Update Operations

Operation:

- Press `更新 (Ctrl+Enter)`
- Or press `Ctrl+Enter` / `Cmd+Enter`

Expected behavior:

- Generate the waveform rows from the row configuration
- Display only non-empty rows
- Regenerate the channel settings panel using only the channels currently in use
- Preserve existing color and line-width settings
- Preserve the current X-axis zoom range, if any

---

## 6. Waveform Operations

### Hover

Operation:

- Hover the mouse over any waveform

Expected behavior:

- Synchronize the hovered X coordinate across all rows
- Show a vertical spike line on every row
- On the left of each row, show the channel value at the nearest timestamp

### Zoom / Pan

Operation:

- Use the standard Plotly interactions to zoom or pan the X axis

Expected behavior:

- Synchronize the X-axis range across all rows
- Share the equivalent of `xaxis.range[0]` / `xaxis.range[1]`
- An autorange action restores the full view
- Re-extract the rendered data according to the new zoom range

### Scroll-Zoom Lock

Initial state:

- The displayed label is `スケール固定` (scale locked)
- `scrollZoom` is False

Operation:

- Press the button

Expected behavior:

- Odd-numbered clicks set `scrollZoom=True`
- Even-numbered clicks set `scrollZoom=False`
- The label and style toggle accordingly

---

## 7. Measurement Operations

### Measurement Mode

Initial state:

- OFF

Operation:

- Press the `計測` (Measure) button

Expected behavior:

- Odd-numbered clicks switch to ON
- Even-numbered clicks switch to OFF
- Switching to OFF clears Cursor A/B

### Placing Cursor A/B

Operation:

1. Turn on measurement mode
2. Click a waveform
3. Click a waveform again
4. Click once more

Expected behavior:

- The first click places Cursor A
- The second click places Cursor B
- When B is already placed, the next click places a new Cursor A and clears B
- A is shown as a blue dashed line, B as a red dashed line

### Interval Statistics

Expected behavior:

- Statistics are shown only when both A and B are placed
- Targets are all channels currently on display
- Duplicate channels are listed only once
- Columns shown: `A`, `B`, `Delta`, `Mean`, `Max`, `Min`, `P-P`, `RMS`, `Slope/s`

---

## 8. FFT Operations

Display condition:

- Both Cursor A and Cursor B are placed

Operation:

1. Select target channels for FFT
2. Select a window function
3. Select the Y-axis display
4. Optionally enter a frequency range
5. Press `FFT`

Expected behavior:

- Up to 3 target channels
- Window functions: Hanning / Hamming / Rectangular
- Y axis: Amplitude / dB
- Frequency range: min/max can be specified
- FFT target interval is between Cursor A and Cursor B
- The mean is subtracted from the signal to remove DC
- The FFT panel is shown
- The FFT panel is hidden whenever the cursors change

---

## 9. Plot Configuration Operations

### Save

Operation:

1. Specify a plot configuration path
2. Press the save button

Expected behavior:

- Create the parent directory if it does not exist
- Save as UTF-8 JSON
- On success, display the file name
- If no destination is specified, show an error

### Load

Operation:

1. Specify a `.pyc.json` path
2. Press the load button

Expected behavior:

- Load the JSON
- Load the Parquet file referenced by `data_path`
- Restore row composition, step settings, Y-axis range, colors, line widths, theme, and scrollZoom
- Clear Cursor A/B
- Render the waveforms directly from the restored state

Known risk:

- A code path has been previously reported in which channels end up scattered after a plot-config load.
  In the Rust version, decouple UI-state updates from waveform rendering at load time to avoid the same class of race condition.

---

## 10. Theme and Header

### Theme

Operation:

- Press the theme button

Expected behavior:

- Toggle between dark and light
- Toggle the className on the app root
- Switch the Plotly template to match the theme

### Header Pin

Operation:

- Press the header-pin button

Expected behavior:

- Toggle between sticky and non-sticky
- This is not a required feature in the Rust version; necessity is decided in Phase 4
