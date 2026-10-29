---
project: Panel_y
doc_type: Design Spec
target: Proto #1
created: "2026-03-15"
updated: "2026-03-16"
status: Complete
---

# Proto #1 Design Document — Practical Viewer

---

## 1. Goal

Bring the viewer to a **genuinely usable level** for everyday waveform inspection.
Inherit the established patterns from #0.2 (row layout, hover values, spike line, synchronized zoom), and add three new pillars: file selection, channel management, and delta calculation.

---

## 2. Feature List

| # | Feature | Delta vs. #0.2 | Status |
|---|------|--------------|------|
| 1 | **File selection UI** (path input + autocomplete + load button) | **NEW** | [x] |
| 2 | **Data preprocessing utility** (CSV/MAT → Parquet) | **NEW** (`utils/` + `multiconverter.py`) | [x] |
| 3 | **Channel show/hide** (dropdown-style multi-select) | **NEW** | [x] |
| 4 | **Cursor delta calculation** (measurement-mode toggle + alternating-click scheme) | **NEW** | [x] |
| ~~5~~ | ~~Reset hover values on mouse-out~~ | Rejected (keeping the last value is more practical) | — |
| 6 | Row layout `[ch name / value] [waveform plot]` | Carried over from #0.2 | [x] |
| 7 | Simultaneous hover-value display across all channels | Carried over from #0.2 | [x] |
| 8 | Spike line spanning all plots vertically | Carried over from #0.2 | [x] |
| 9 | Zoom/pan synchronization across plots | Carried over from #0.2 | [x] |
| 10 | Scroll zoom and toolbar | Carried over from #0.2 | [x] |

**Definition of done**: Load an actual motor-control log (Parquet) and confirm all 10 features above work end-to-end.

---

## 3. Usage

### Launch

```bash
cd proto_1
source .venv/bin/activate
python app.py
# → http://localhost:8050
```

### Preprocessing (offline conversion)

```bash
# CSV → Parquet
python -m utils.converter input.csv data/output.parquet

# MAT → Parquet (specify time column and unit)
python -m utils.converter data.mat data/output.parquet --time-col ECUTIME --time-unit ms

# Batch-convert all .mat files in a folder
python multiconverter.py --input ./input --output ./output
```

### Operation

| Action | Behavior |
|------|------|
| Enter file path | As you type, candidates (📁 / 📄) appear. Click a candidate to update the path. |
| "Load" button | Reads the entered `.parquet` file on the server side. |
| Header dropdown | Multi-select channels; waveform rows are generated dynamically. |
| Hover on a plot | Values for all channels are shown simultaneously in the left column, with a vertical spike line spanning all plots. |
| Mouse out | The last hover value remains (same as #0.2). |
| Drag | Pan the time axis; all plots stay in sync. |
| Mouse wheel | Zoom the time axis; all plots stay in sync. |
| Double-click | Reset zoom (return to full view). |
| "📏 Measure" button | Toggles measurement mode on/off (button is highlighted blue when ON). |
| **Click** (measurement mode ON) | Place cursors A and B alternately (A → B → update A → ...), then show the delta. |

---

## 4. File Layout

```
proto_1/
├── app.py                # Dash application
├── multiconverter.py     # CLI for batch-converting .mat files in a folder
├── requirements.txt      # Dependencies
├── assets/
│   └── style.css         # Dash custom CSS
├── data/                 # Place converted Parquet files here (excluded by .gitignore)
└── utils/                # Preprocessing utilities
    ├── __init__.py
    ├── converter.py       # Conversion entry point (public API + CLI)
    ├── reader_csv.py      # CSV reader
    ├── reader_mat.py      # MATLAB MAT reader (pre-v7.2 + v7.3 HDF5)
    ├── time_normalizer.py # Time-column detection and unit normalization
    ├── column_filter.py   # Remove non-numeric columns
    └── resampler.py       # Uniformity check and linear-interpolation resampling
```

---

## 5. Overall Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│ app.layout                                                                │
│                                                                          │
│  ┌─ Header bar ───────────────────────────────────────────────────────┐ │
│  │ [PanelY]  [____path input____][Load]  [▼ ch select ▾]  [📏 Measure] │ │
│  │           id="file-path-input"          id="ch-dropdown"            │ │
│  │           id="file-suggestions" (candidates)  id="measure-toggle-btn"│ │
│  │           id="load-btn"                 id="load-status" (right)    │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─ Delta panel (shown when cursors are placed) ──────────────────────┐ │
│  │ Cursor A: 0.02340 s   Cursor B: 0.04560 s                          │ │
│  │ Δt = 22.200 ms                                                     │ │
│  │ ch1: A=141.4  B=−141.4  Δ=−282.8                                  │ │
│  │ id="delta-panel"                                                    │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─ waveform-container ───────────────────────────────────────────────┐ │
│  │ [Channel / Value] [Waveform]                  ← header row         │ │
│  │ ┌──────────────┬──────────────────────────────────────────────┐    │ │
│  │ │ ch_name      │ dcc.Graph(id={type:"wf-graph",channel:ch})  │    │ │
│  │ │ hover value  │    [waveform + cursor A/B vertical lines]    │    │ │
│  │ │ id={wf-val}  │                                              │    │ │
│  │ ├──────────────┼──────────────────────────────────────────────┤    │ │
│  │ │ ch_name      │ dcc.Graph(id={type:"wf-graph",channel:ch})  │    │ │
│  │ │ hover value  │    [waveform + cursor A/B vertical lines]    │    │ │
│  │ │ id={wf-val}  │                                              │    │ │
│  │ └──────────────┴──────────────────────────────────────────────┘    │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  dcc.Store(id="hover-x-store")                                           │
│  dcc.Store(id="xaxis-range-store")                                       │
│  dcc.Store(id="data-store")         ← File path + channel-name list      │
│  dcc.Store(id="cursor-a-store")     ← X coordinate of cursor A           │
│  dcc.Store(id="cursor-b-store")     ← X coordinate of cursor B           │
│  dcc.Store(id="measure-mode")       ← Measurement mode on/off (bool)     │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Callback Flow

```
[User action]
      │
      ├─ Enter file path
      │       │
      │       ▼
      │  suggest_files
      │    Input:  file-path-input.value (path string being typed)
      │    Output: file-suggestions.children (candidate button list)
      │            file-suggestions.style (show/hide)
      │    Logic:  Walk the parent directory of the input path with iterdir()
      │            and return 📁 subdirectories + 📄 .parquet files as buttons (up to 30).
      │       │
      │       ├──▶ on_suggestion_click (when a candidate button is clicked)
      │       │      Input:  {type:"suggestion", path:ALL}.n_clicks
      │       │      Output: file-path-input.value (update path)
      │       │
      │       ▼
      │  load_file (when the "Load" button is clicked)
      │    Input:  load-btn.n_clicks
      │    State:  file-path-input.value
      │    Output: data-store.data (file path + channel info)
      │            ch-dropdown.options / value (channel list / empty selection [])
      │            load-status.children (✓ filename, channel count, sample count, Hz)
      │            cursor-a-store / cursor-b-store (reset)
      │    Logic:  pd.read_parquet(path) on the server → keep in global df.
      │
      ├─ Channel dropdown change
      │       │
      │       ▼
      │  update_waveform_rows
      │    Input:  ch-dropdown.value
      │    State:  data-store.data
      │    Output: waveform-container.children (dynamic waveform rows)
      │    Logic:  Generate and display waveform rows for selected channels; remove rows for deselected channels.
      │
      ├─ Measurement-mode toggle
      │       │
      │       ▼
      │  toggle_measure
      │    Input:  measure-toggle-btn.n_clicks
      │    Output: measure-mode.data (on/off)
      │            measure-toggle-btn.style (ON=blue, OFF=gray)
      │            cursor-a-store / cursor-b-store (reset when turned off)
      │
      ├─ Hover on a plot
      │       │
      │       ▼
      │  store_hover_x                                ← carried over from #0.2
      │    Input:  {type:"wf-graph", channel:ALL}.hoverData
      │    Output: hover-x-store.data
      │       │
      │       ├──▶ update_values                      ← carried over from #0.2
      │       │      Input:  hover-x-store.data
      │       │      State:  {type:"wf-val", channel:ALL}.id
      │       │      Output: {type:"wf-val", channel:ALL}.children
      │       │      Logic:  If x_val is None → "---"; otherwise look up the value in df.
      │       │
      │       └──▶ update_graphs                      ← carried over from #0.2, with cursor drawing added
      │              triggered="hover-x-store"
      │              → Draw spike line + cursor A/B vertical lines as shapes.
      │
      ├─ Zoom/pan
      │       │
      │       ▼
      │  store_xaxis_range                            ← carried over from #0.2
      │    Input:  {type:"wf-graph", channel:ALL}.relayoutData
      │    Output: xaxis-range-store.data
      │       │
      │       ▼
      │  update_graphs                                ← carried over from #0.2
      │    triggered="xaxis-range-store"
      │    → Sync xaxis.range across all plots using Patch.
      │
      ├─ Click (when measurement mode is ON)
      │       │
      │       ▼
      │  set_cursor
      │    Input:  {type:"wf-graph", channel:ALL}.clickData
      │    State:  measure-mode.data, cursor-a-store, cursor-b-store
      │    Output: cursor-a-store.data / cursor-b-store.data
      │    Logic:  Alternating-click scheme (A → B → update A (clear B) → B → ...).
      │       │
      │       ▼
      │  update_delta_panel
      │    Input:  cursor-a-store.data, cursor-b-store.data
      │    State:  ch-dropdown.value
    │    Output: delta-panel.children / style
      │    Logic:  Display cursor positions; once both are placed, compute Δt and ΔV for each channel.
```

---

## 7. Per-Feature Implementation Design

### 7-1. File-selection UI (text input + suggestion buttons + load button)

**Approach**: Free-form path entry via `dcc.Input`. While typing, the server walks the directory and presents candidates as a list of `html.Button`s. Clicking a candidate updates the path, and an explicit "Load" button reads the Parquet file.
Files are read directly on the server, so there is **no size limit** (supports files larger than 100 MB).

> `dcc.Upload` (browser-side base64 read) was rejected because large files make the browser sluggish.
> `dcc.Dropdown(searchable)` was rejected because it is poorly suited to drilling into directory hierarchies.

```python
dcc.Input(
    id="file-path-input",
    type="text",
    value=str(DATA_DIR) + "/",
    placeholder="Parquet ファイルパスを入力...",
    debounce=False,
)
html.Div(id="file-suggestions")   # Container for candidate buttons
html.Button("読み込み", id="load-btn")
```

**Autocomplete behavior**:

```
User input: /Users/sugusokothx/da
                                    │
                          suggest_files
                          Input: file-path-input.value
                                    │
                                    ▼
                          parent.iterdir() with filtering (up to 30 items)
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │ [📁 data/              ]  ← Button │
                    │ [📁 dashboard/         ]  ← Button │
                    └───────────────────────────────┘

Click button: data/  → on_suggestion_click → updates file-path-input.value
                    ┌───────────────────────────────┐
                    │ [📄 motor_log_001.parquet]     │
                    │ [📄 motor_log_002.parquet]     │
                    │ [📁 2026-03/             ]     │
                    └───────────────────────────────┘

Click button: motor_log_001.parquet → path confirmed
"Load" button → load_file → pd.read_parquet(path)
```

**`load_file` processing flow**:
1. Fires on `load-btn.n_clicks`; reads the path from `file-path-input.value`.
2. Validates: existence check / directory check / `.parquet` extension check.
3. `pd.read_parquet(path)` reads the file on the server; the result is held in the global `df`.
4. Sets the channel list on `ch-dropdown.options` and `value=[]` (nothing selected).
5. Displays filename, channel count, sample count, and sampling rate in `load-status`.
6. Resets cursors A/B.

**Initial directory**: The `data/` folder next to `app.py` (`DATA_DIR`) is shown as the initial value at startup.

---

### 7-2. Channel show/hide (dropdown multi-select)

**Approach**: Place a `dcc.Dropdown(multi=True)` in the header bar.
After loading data, **no waveforms are shown**; the user picks channels to display from the dropdown.

```python
dcc.Dropdown(
    id="ch-dropdown",
    options=[],          # Populated with channel list after file load
    value=[],            # Initially empty (no waveforms)
    multi=True,
    searchable=True,     # Enable search (filtering works for >30 channels)
    placeholder="チャンネルを選択...",
)
```

**Behavior**:
- File loaded → set the channel list in `options`, `value=[]` (no selection).
- User selects channels → waveform rows for those channels are generated dynamically.
- Deselecting a channel → its row is removed.
- The dropdown's search box supports incremental filtering of channel names.

---

### 7-3. Cursor delta calculation (Δt / ΔV / ΔA)

**Approach**: Measurement-mode toggle plus click-to-place cursors.

**UI components**:
```python
# Measurement-mode toggle (click to toggle on/off; highlighted blue when ON)
html.Button("📏 計測", id="measure-toggle-btn", n_clicks=0)

# Measurement-mode state
dcc.Store(id="measure-mode", data=False)

# Delta-display panel
html.Div(id="delta-panel", style={"display": "none"})
```

**Cursor rendering**:
- Cursor A: blue vertical line (`line.color = "#4fc3f7"`)
- Cursor B: red vertical line (`line.color = "#ef5350"`)
- Use `yref="paper"` so the line spans the full plot height (same approach as the spike line).
- Added to `shapes` inside Callback 4 (`update_graphs`).

**Delta-panel contents**:
```
Cursor A: 0.02340 s    Cursor B: 0.04560 s
Δt = 22.200 ms
voltage_u: A=141.4  B=−141.4  Δ=−282.8
current_u: A=8.66   B=−8.66   Δ=−17.32
```

**Alternating-click scheme**:
- Plotly's `clickData` does not include modifier-key information, so a Shift+click scheme is not viable.
- We use the **alternating-click scheme** (first click = A, second = B, third = update A (clear B), ...).
- The states of `cursor-a-store` and `cursor-b-store` determine which cursor to place next.

```python
# Alternating-click logic
if cursor_b is not None:
    # Both filled → update A (cycle)
    return x_val, None
elif cursor_a is not None:
    # Only A → place B
    return cursor_a, x_val
else:
    # Both None → place A
    return x_val, None
```

---

### ~~7-4. Reset hover values on mouse-out~~ → Rejected

On mouse-out, the last hover value is preserved (same behavior as #0.2).
Erasing the value would be inconvenient for workflows such as checking a value and then taking a note.
`dcc.Interval` and a `hover-timestamp-store` are not required.

---

## 8. Implementation Patterns Carried Over from #0.2

| Pattern | Description |
|---------|------|
| `hoverinfo="none"` | Hides the tooltip while still firing hoverData events. |
| `hovermode="x"` | Hover detection along the X axis. |
| Shapes with `yref="paper"` | Vertical lines spanning the full plot height. |
| `Patch()` partial updates | Update only the affected parts instead of returning the entire figure. |
| `ctx.triggered_id` branching | Handle multiple events in a single callback. |
| Dark theme | `template="plotly_dark"`, `paper/plot_bgcolor`. |
| Toolbar settings | `displayModeBar="hover"`, `scrollZoom=True`. |

---

## 9. Store Inventory

| Store ID | Type | Purpose |
|----------|-----|------|
| `hover-x-store` | float \| None | Current hover X coordinate |
| `xaxis-range-store` | [float, float] \| None | Currently synchronized X-axis range |
| `data-store` | dict | Loaded file path + channel-name list |
| `cursor-a-store` | float \| None | X coordinate of cursor A |
| `cursor-b-store` | float \| None | X coordinate of cursor B |
| `measure-mode` | bool | Measurement-mode on/off |

---

## 10. Reference Documents

- [企画書](./企画書.md) — Overall roadmap and technology stack
- [Parquet データ仕様](./parquet_data_spec.md) — Input-data constraints
- [前処理ユーティリティ設計書](./preprocessor_design.md) — Design and API of `utils/`
- [Proto #0.2 設計書](./proto_0_2_設計書.md) — Inherited implementation patterns
- [bug_report_0_1.md](./bug_report_0_1.md) — Spike-line issue (why `make_subplots` was rejected)
- [bug_report_0_2.md](./bug_report_0_2.md) — `hoverinfo`/`displayModeBar` misconfiguration
