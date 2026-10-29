---
project: Panel_y
doc_type: Design Proposal
target: Proto #1b
created: "2026-03-16"
updated: "2026-03-16"
status: Planning
---

# Proto #1b Design Proposal — Practical Viewer (Panel + Bokeh Edition)

---

## 1. Purpose

Implement the **same specification** as Proto #1 (Dash + Plotly) using Panel (HoloViz) + Bokeh.
This is a **parallel prototype** for comparative evaluation of the technology stacks, and serves as input for deciding which stack to base Proto #2 onward on.

---

## 2. Features to Implement (Identical to #1)

| # | Feature | Status |
|---|------|------|
| 1 | File-selection UI (path input + autocomplete + load button) | [ ] |
| 2 | Data preprocessing utility (CSV/MAT → Parquet) | [x] ← Reuse `utils/` as-is |
| 3 | Channel show/hide (multi-select) | [ ] |
| 4 | Cursor delta calculation (measurement-mode toggle + alternating-click scheme) | [ ] |
| 5 | Row layout `[ch name / value] [waveform plot]` | [ ] |
| 6 | Simultaneous hover-value display across all channels | [ ] |
| 7 | Spike line spanning all plots vertically | [ ] |
| 8 | Zoom/pan synchronization across plots | [ ] |
| 9 | Scroll zoom and toolbar | [ ] |

---

## 3. Technology Stack

| Item | Proto #1 (Dash edition) | Proto #1b (Panel edition) |
|------|-------------------|---------------------|
| Web framework | Dash | **Panel** (HoloViz) |
| Plotting library | Plotly.py | **Bokeh** |
| Data handling | Pandas + NumPy | Pandas + NumPy (identical) |
| Preprocessing utilities | `utils/` | `utils/` (**shared**) |
| Standard data format | Parquet | Parquet (identical) |
| Server | Flask (built into Dash) | Tornado (built into Bokeh/Panel) |

### Why Bokeh

Panel supports multiple plot backends, but in this project we use **Bokeh natively**.

- **Linked ranges**: Sharing a single `x_range` object is enough to synchronize zoom and pan (simpler than Dash's Store + Callback approach).
- **CrosshairTool / HoverTool**: Built-in tools provide the basic spike-line and hover-value functionality.
- **CustomJS**: Client-side JS callbacks deliver fast interactions without server round-trips.
- **ColumnDataSource**: A clear, explicit model for synchronizing data between server and client.

---

## 4. Outlook for the Dash → Panel Migration

### 4-1. Zoom/pan synchronization (easy → native support)

**Dash #1 implementation**: `relayoutData` → `xaxis-range-store` → `Patch()` updates every plot (server round-trip).

**Panel/Bokeh implementation**: Just pass the same object as `x_range` for every figure.

```python
from bokeh.plotting import figure

shared_x_range = None

def make_figure(ch, df, is_first=False):
    nonlocal shared_x_range
    if is_first:
        fig = figure(...)
        shared_x_range = fig.x_range
    else:
        fig = figure(..., x_range=shared_x_range)
    fig.line(x='time', y=ch, source=source)
    return fig
```

**Assessment**: Bokeh's linked ranges run **entirely on the client side**.
The Store→Callback chain from the Dash version is no longer needed, with a high likelihood of better performance.

---

### 4-2. Spike line (requires care → CrosshairTool + CustomJS)

**Dash #1 implementation**: `hover-x-store` → `update_graphs` adds `shapes` to every plot via Patch (server round-trip).

**Panel/Bokeh implementation**:

Bokeh's `CrosshairTool` works **only within a single figure**.
To realize a spike line spanning all plots, the following approach is required:

**Approach: shared Span + CustomJS**

```python
from bokeh.models import Span, CrosshairTool, CustomJS

# Add a Span (vertical line) to each figure
spans = []
for fig in figures:
    span = Span(dimension='height', line_color='gray',
                line_alpha=0.7, line_width=1, visible=False)
    fig.add_layout(span)
    spans.append(span)

# A CustomJS that updates every Span from the HoverTool of one figure
code = """
const x = cb_data.geometry.x;
for (const span of spans) {
    span.location = x;
    span.visible = true;
}
"""
callback = CustomJS(args=dict(spans=spans), code=code)
hover = HoverTool(tooltips=None, callback=callback, mode='vline')
figures[0].add_tools(hover)
```

**Challenges**:
- HoverTool's callback fires **only on the figure that triggered it**.
- We need to attach a HoverTool to every figure, each updating every Span.
- Hiding the Span on mouse-out requires CustomJS hooked to a `PointerMove` or `MouseLeave` event.

**Assessment**: Slightly more verbose than the Dash version, but **no server round-trip**, so it is fast.
The initial CustomJS implementation is needed, but it is reusable once written.

---

### 4-3. Simultaneous hover-value display (requires care → CustomJS + Div updates)

**Dash #1 implementation**: `hover-x-store` → `update_values` callback (server round-trip to look up values in df).

**Panel/Bokeh implementation**:

**Option A: CustomJS (fully client-side)**

```python
from bokeh.models import Div, CustomJS

# A value-display Div per channel
value_divs = {}
for ch in channels:
    value_divs[ch] = Div(text="---", styles={"color": "#7ec8e3", "font-size": "20px"})

# HoverTool CustomJS that looks up values from ColumnDataSource
code = """
const x = cb_data.geometry.x;
const time = source.data['time'];
// Binary search for the nearest index
let lo = 0, hi = time.length - 1;
while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (time[mid] < x) lo = mid + 1;
    else hi = mid;
}
const idx = lo;
for (const [ch, div] of Object.entries(divs)) {
    const val = source.data[ch][idx];
    div.text = val.toFixed(4);
}
"""
```

**Option B: `param.watch` (server-side, equivalent to Dash #1)**

Use Panel's `param.watch` to run a Python callback that fetches values from df.
This involves a server round-trip but is simpler to implement.

**Assessment**: Option A (CustomJS) is recommended. Values update instantly with no server round-trip and responsiveness does not degrade for large datasets. However, the full dataset must be loaded into a `ColumnDataSource`, which increases memory usage (see §6 Constraints).

---

### 4-4. Cursor delta calculation (somewhat difficult → TapTool + CustomJS + Python interplay)

**Dash #1 implementation**: `clickData` → `set_cursor` (alternating logic) → `update_delta_panel` (server round-trip).

**Panel/Bokeh implementation**:

```python
from bokeh.models import TapTool, Span, CustomJS

# Add cursor Spans (A: blue, B: red) to every figure
cursor_a_spans = [Span(dimension='height', line_color='#4fc3f7',
                       line_width=2, line_dash='dashed', visible=False)
                  for _ in figures]
cursor_b_spans = [Span(dimension='height', line_color='#ef5350',
                       line_width=2, line_dash='dashed', visible=False)
                  for _ in figures]

# Implement alternating-click logic in CustomJS
code = """
const x = cb_data.geometries[0].x;
if (state.next === 'a') {
    for (const s of a_spans) { s.location = x; s.visible = true; }
    for (const s of b_spans) { s.visible = false; }
    state.cursor_a = x;
    state.cursor_b = null;
    state.next = 'b';
} else {
    for (const s of b_spans) { s.location = x; s.visible = true; }
    state.cursor_b = x;
    state.next = 'a';
}
// Update delta-panel (text)
if (state.cursor_a !== null && state.cursor_b !== null) {
    const dt = Math.abs(state.cursor_b - state.cursor_a) * 1000;
    delta_div.text = `Δt = ${dt.toFixed(3)} ms`;
}
"""
```

**Challenges**:
- The measurement-mode toggle is implemented with a Panel widget (`pn.widgets.Toggle`).
- When the toggle is off, the TapTool must be disabled (toggle `tool.active` via `CustomJS`).
- Computing ΔV per channel in the delta panel from a `ColumnDataSource` is possible in JS, but it grows the code base.
- **Alternative**: Compute deltas server-side only (`param.watch`) and update a `pn.pane.HTML`.

**Assessment**: Basic cursor placement and Δt display are achievable in CustomJS.
However, the CustomJS becomes lengthy once per-channel ΔV is included, so a **hybrid approach — read cursor positions in CustomJS, compute deltas in Python — is the realistic choice**.

---

### 4-5. File-selection UI (easy → Panel widgets)

**Dash #1 implementation**: `dcc.Input` + candidate button list + load button (three callbacks).

**Panel/Bokeh implementation**:

```python
import panel as pn

file_input = pn.widgets.TextInput(
    name='ファイルパス',
    value=str(DATA_DIR) + "/",
    placeholder='Parquet ファイルパスを入力...',
)
load_btn = pn.widgets.Button(name='読み込み', button_type='primary')
status = pn.pane.Markdown("", styles={"color": "#8bc34a"})

# Candidate list
suggestions = pn.Column()

def on_path_change(event):
    # Dynamically generate file candidates (same logic as Dash's list_path_suggestions)
    ...

def on_load_click(event):
    global df
    df = pd.read_parquet(file_input.value)
    ...

file_input.param.watch(on_path_change, 'value')
load_btn.on_click(on_load_click)
```

**Assessment**: Roughly the same implementation volume as the Dash version. Panel's widget callbacks are simpler to define (no decorators, no need to specify Output/Input/State).

---

### 4-6. Channel show/hide (easy → MultiChoice + dynamic layout)

**Panel/Bokeh implementation**:

```python
ch_selector = pn.widgets.MultiChoice(
    name='チャンネル選択',
    options=[],
    value=[],
)

waveform_area = pn.Column()

def update_channels(event):
    rows = []
    for i, ch in enumerate(ch_selector.value):
        fig = make_figure(ch, df, is_last=(i == len(ch_selector.value) - 1))
        label = pn.pane.Markdown(f"**{ch}**\\n---", ...)
        rows.append(pn.Row(label, fig))
    waveform_area.objects = rows

ch_selector.param.watch(update_channels, 'value')
```

**Assessment**: Panel's dynamic layout updates (`column.objects = [...]`) are equivalent to returning `children` in Dash. The MultiChoice widget corresponds to Dash's `dcc.Dropdown(multi=True)` and includes built-in search.

---

## 5. Overall Architecture (Panel Edition)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ pn.template.FastDarkTheme (or custom)                                    │
│                                                                          │
│  ┌─ header ──────────────────────────────────────────────────────────┐   │
│  │ [PanelY]  [TextInput: path] [Button: Load]                        │   │
│  │           [MultiChoice: ch select]  [Toggle: 📏 Measure]  [Markdown: status]│   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─ delta_panel (pn.pane.HTML) ──────────────────────────────────────┐   │
│  │ Cursor A: 0.02340 s   Cursor B: 0.04560 s   Δt = 22.200 ms      │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─ waveform_area (pn.Column) ───────────────────────────────────────┐   │
│  │ ┌─ Row ─────────────────────────────────────────────────────────┐ │   │
│  │ │ [ch name] │ Bokeh figure (x_range=shared)                     │ │   │
│  │ │ [value Div] │ line + Span(spike) + Span(cursorA) + Span(cursorB)│ │   │
│  │ ├───────────┼───────────────────────────────────────────────────┤ │   │
│  │ │ [ch name] │ Bokeh figure (x_range=shared)                     │ │   │
│  │ │ [value Div] │ line + Span(spike) + Span(cursorA) + Span(cursorB)│ │   │
│  │ └───────────┴───────────────────────────────────────────────────┘ │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Note: No dcc.Store equivalents are needed (Bokeh models hold the state).│
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Constraints and Risks

| Item | Description | Impact |
|------|------|--------|
| **ColumnDataSource memory** | Bokeh ships the entire dataset to the browser as JSON. With over a million points the browser memory may be strained. | High |
| **Size of CustomJS code** | The combined JS for spike line, hover values, and cursors is expected to exceed 100 lines. Debugging is harder than in Python. | Medium |
| **CrosshairTool limitation** | Works only within a single figure. Cross-figure behavior must be implemented from scratch. | Medium |
| **HoverTool exclusivity** | The HoverTool callback conflicts unless `tooltips=None`. Care is needed when coexisting with tooltips. | Low |
| **Panel dark theme** | `FastDarkTheme` template covers most needs. Bokeh figures need a separate `theme` setting. | Low |
| **Sharing `utils/` with the Dash edition** | `utils/` is pure Python and framework-agnostic. Shareable without issues. | — |

### Strategies for Large Datasets

In Dash #1 the DataFrame stays on the server, and only the Plotly figures (already-rendered points) are sent to the browser.
With Bokeh, the **entire raw dataset is sent** to the browser via `ColumnDataSource`.

Possible mitigations:
1. **Server-side downsampling**: Decimate to match the number of display pixels (e.g., the LTTB algorithm).
2. **WebGL rendering**: Use `output_backend="webgl"` to offload rendering of large point clouds to the GPU.
3. **Streaming ColumnDataSource**: Send only the data within the current zoom range (possible with Panel + Bokeh server, but introduces server round-trips).

---

## 7. Comparison Summary: Dash Edition vs. Panel Edition

| Aspect | Dash + Plotly (#1) | Panel + Bokeh (#1b) |
|------|--------------------|--------------------|
| **Zoom/pan synchronization** | Store + Callback (server round-trip) | **Linked range (client-only)** |
| **Spike line** | Callback + Patch shapes | CustomJS + Span (client-only) |
| **Hover values** | Callback + df lookup (server round-trip) | CustomJS + ColumnDataSource (client-only) |
| **Cursor delta** | Callback chain | CustomJS + Python hybrid |
| **Response performance** | Server round-trips on hover/zoom | **Almost everything stays on the client; fast** |
| **Large datasets** | Server-side retention is an advantage | Browser transfer required (downsampling needed) |
| **Code clarity** | Pure Python | Python + CustomJS (two languages) |
| **Estimated code size** | app.py: ~650 lines | app.py: ~500 lines + CustomJS: ~150 lines |
| **Dependencies** | dash, plotly | panel, bokeh |
| **Templates / theming** | Inline style + assets/CSS | FastDarkTheme + Bokeh Theme |

---

## 8. File Layout

```
proto_1b/
├── app.py                # Panel application
├── js/                   # CustomJS code (could be inlined in Python, but separation is recommended)
│   ├── spike_line.js     # Cross-figure spike line
│   ├── hover_values.js   # Simultaneous hover-value updates across channels
│   └── cursor.js         # Cursor placement and alternating logic
├── requirements.txt      # panel, bokeh, pandas, numpy, scipy, pyarrow, h5py
├── data/                 # → Symlink to ../proto_1/data/ or shared
└── utils -> ../proto_1/utils  # Shared via symlink
```

---

## 9. How to Run

```bash
cd proto_1b
source .venv/bin/activate  # Share the venv with proto_1, or create a new one
panel serve app.py --show
# → http://localhost:5006
```

---

## 10. Implementation Plan (Proposal)

Rather than implementing every Proto #1 feature at once, proceed in stages so that the Bokeh-specific technology checks are folded in.

| Phase | Content | Verification points |
|-------|------|-------------|
| **Phase 1** | Load a fixed Parquet, display 2 channels. Synchronize zoom via shared `x_range`. | Confirm linked-range behavior. |
| **Phase 2** | Full-height spike line + hover-value display (CustomJS). | Establish CustomJS implementation patterns. |
| **Phase 3** | File-selection UI + dynamic channel switching. | Panel widgets + dynamic layout. |
| **Phase 4** | Measurement mode + cursor delta panel. | TapTool + CustomJS/Python hybrid. |
| **Phase 5** | Unify dark theme + style polish + large-data verification. | Performance comparison with real data. |

After **Phase 2**, the feel of the Panel edition can be compared against the Dash edition, so we recommend treating that as a continue/stop decision point.

---

## 11. Selection Criteria (#1 vs. #1b)

The base for Proto #2 will be chosen by evaluating the following axes:

| Evaluation axis | Weight | Likely winner (prediction) |
|--------|------|-----------------|
| Hover/zoom responsiveness | High | #1b (client-only) |
| Handling large datasets (>500k points) | High | #1 (server-side retention) |
| Maintainability and readability of code | Medium | #1 (pure Python) |
| Extensibility (e.g., adding an FFT panel) | Medium | Equivalent |
| Polish of the dark theme | Low | #1 (Plotly dark template maturity) |

---

## 12. Reference Documents

- [Proto #1 設計書](./proto_1_設計書.md) — Dash-edition design for the same specification
- [Parquet データ仕様](./parquet_data_spec.md) — Input-data constraints
- [前処理ユーティリティ設計書](./preprocessor_design.md) — Design of the shared `utils/`
- [Panel official docs](https://panel.holoviz.org/)
- [Bokeh official docs](https://docs.bokeh.org/)
