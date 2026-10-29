---
project: Panel_y
doc_type: Design Spec
target: Proto #2.0
created: "2026-03-24"
updated: "2026-03-24"
status: Complete
---

# Proto #2.0 Design Document — Practical Viewer + Per-Row Dropdown UI

---

## 1. Goal

Merge Proto #1 (the practical viewer) with Proto #0.4 (the per-row dropdown UI).
Allow multiple channels to be selected per row via dropdowns and rendered as overlays,
while keeping every feature of #1 (file selection, measurement mode, hover sync, zoom sync).

In addition, add improved trackpad usability (scale lock) and numeric Y-axis range entry.

---

## 2. Feature List

| # | Feature | Difference from #1 | Status |
|---|---------|--------------------|--------|
| 1 | **Per-row dropdown (overlay of multiple channels)** | **NEW** (ported from #0.4) | [x] |
| 2 | **Add / delete rows** | **NEW** (ported from #0.4) | [x] |
| 3 | **Scale lock (scrollZoom ON/OFF)** | **NEW** | [x] |
| 4 | **Numeric Y-axis range entry (per row)** | **NEW** | [x] |
| 5 | **SI-prefix Y-axis tick labels** | **NEW** | [x] |
| 6 | **Update keyboard shortcut (Ctrl+Enter)** | **NEW** | [x] |
| 7 | File selection UI (path input + completion + load) | Carried over from #1 | [x] |
| 8 | Cursor delta calculation (measurement mode) | Carried over from #1 | [x] |
| 9 | Simultaneous hover-value display for all channels | Carried over from #1 | [x] |
| 10 | Spikelines spanning every graph vertically | Carried over from #1 | [x] |
| 11 | Zoom / pan sync across graphs | Carried over from #1 | [x] |
| ~~12~~ | ~~Single-channel dropdown in the header~~ | **Removed** (replaced by per-row dropdown) | — |

**Completion criteria**: Load an actual motor-control log (Parquet) and confirm that all 11 features above work.

---

## 3. Architecture Decision

### make_subplots approach vs. independent Graph approach

During Proto #2.0 development we experimented with the `make_subplots` approach but rejected it for the reasons below.

| Item | make_subplots (tried, rejected) | Independent Graphs (adopted) |
|------|---------------------------------|------------------------------|
| Hover sync | `hoversubplots="axis"` did not work (each trace references a different X axis) | All rows synced via `hover-x-store` |
| Spikelines | Cannot cross subplot boundaries | `yref="paper"` spans the full height of every Graph |
| Zoom sync | Automatic with `shared_xaxes=True` (but constrained as above) | All rows synced via `xaxis-range-store` |
| Implementation complexity | Low (single Figure) | Medium (Store + pattern-matching callbacks) |

**Conclusion**: Because hover sync and full-height spikelines are required, we adopt the same independent-Graph approach as #1.
This decision is consistent with the findings already documented in bug_report_0_1.md.

---

## 4. Usage

### Launch

```bash
cd panel-y
source .venv/bin/activate   # shared venv (directly under panel-y)
python proto_2_0/app.py
# → http://localhost:8050
```

### Controls

| Action | Behavior |
|--------|----------|
| Enter a file path | Typing a path shows candidates (📁 / 📄). Click a candidate to update the path |
| "Load" button | Server-side load of the entered .parquet file. Initial rows are auto-generated |
| Row dropdown | Multi-select channels per row (supports overlay rendering) |
| "+ Add row" button | Append an empty row to the end |
| "×" button | Delete the corresponding row |
| Y: min ~ max | Numerically specify the Y-axis range per row (blank = autorange) |
| "Update (Ctrl+Enter)" button | Re-render waveforms applying dropdown and Y-axis settings |
| "🔒 Lock scale" button | Toggle scroll zoom ON/OFF. Default OFF (page scroll has priority) |
| With scroll zoom released | Scroll zoom is allowed only along the X axis (Y axis is fixed) |
| Mouse hover over a graph | The left column shows channel values for every row simultaneously, with spikelines spanning all graphs |
| Drag | Pan the time axis. All graphs are synchronized |
| Double click | Reset zoom (return to full view) |
| "📏 Measure" button | Toggle measurement mode ON/OFF |
| Click (with measurement mode ON) | Place cursors A / B alternately → show the delta |
| Ctrl+Enter | Same as the "Update" button (keyboard shortcut) |

---

## 5. File Layout

```
proto_2_0/
├── app.py                # Dash application
├── multiconverter.py     # CLI for batch-converting .mat files in a folder (carried over from #1)
├── requirements.txt      # Dependencies
├── assets/
│   └── style.css         # Dash custom CSS
├── data/                 # Converted Parquet files go here (excluded by .gitignore)
└── utils/                # Preprocessing utilities (carried over from #1)
    ├── __init__.py
    ├── converter.py
    ├── reader_csv.py
    ├── reader_mat.py
    ├── time_normalizer.py
    ├── column_filter.py
    └── resampler.py
```

---

## 6. Overall Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│ app.layout                                                                │
│                                                                          │
│  ┌─ Header bar ───────────────────────────────────────────────────────┐ │
│  │ [PanelY] [____path input____][Load] [🔒 Lock scale] [📏 Measure]    │ │
│  │          id="file-path-input"         id="scroll-zoom-btn"         │ │
│  │          id="file-suggestions"        id="measure-toggle-btn"      │ │
│  │          id="load-btn"                id="load-status" (far right) │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─ Row definition area ──────────────────────────────────────────────┐ │
│  │ [+ Add row] [Update (Ctrl+Enter)]                                  │ │
│  │ Row 1: [▼ ch select (multi)] [Y: min ~ max] [×]                    │ │
│  │ Row 2: [▼ ch select (multi)] [Y: min ~ max] [×]                    │ │
│  │ ...                                                                │ │
│  │ id="rows-container"                                                │ │
│  │   {type:"row-dropdown", index:N}                                   │ │
│  │   {type:"ymin-input", index:N} / {type:"ymax-input", index:N}     │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─ Delta display panel ──────────────────────────────────────────────┐ │
│  │ Cursor A: 0.02340 s   Cursor B: 0.04560 s                          │ │
│  │ Δt = 22.200 ms                                                     │ │
│  │ ch1: A=141.4  B=−141.4  Δ=−282.8                                  │ │
│  │ id="delta-panel"                                                   │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─ waveform-container ───────────────────────────────────────────────┐ │
│  │ [Channel / Value] [Waveform]                  ← header row         │ │
│  │ ┌──────────────┬──────────────────────────────────────────────┐    │ │
│  │ │ ch1/ch2      │ dcc.Graph(id={type:"wf-graph",row:0})       │    │ │
│  │ │ hover values │   [overlaid waveform + cursor A/B + spikes] │    │ │
│  │ │ id={wf-val}  │                                              │    │ │
│  │ ├──────────────┼──────────────────────────────────────────────┤    │ │
│  │ │ ch3          │ dcc.Graph(id={type:"wf-graph",row:1})       │    │ │
│  │ │ hover values │   [waveform + cursor A/B + spikelines]      │    │ │
│  │ │ id={wf-val}  │                                              │    │ │
│  │ └──────────────┴──────────────────────────────────────────────┘    │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  dcc.Store(id="row-count")          ← row counter (sequential IDs for additions) │
│  dcc.Store(id="row-groups-store")   ← current row grouping [[ch,...],..]        │
│  dcc.Store(id="hover-x-store")      ← hover X coordinate                        │
│  dcc.Store(id="xaxis-range-store")  ← synchronized X-axis range                 │
│  dcc.Store(id="data-store")         ← loaded file path + channel-name list      │
│  dcc.Store(id="cursor-a-store")     ← cursor A X coordinate                     │
│  dcc.Store(id="cursor-b-store")     ← cursor B X coordinate                     │
│  dcc.Store(id="measure-mode")       ← measurement mode ON/OFF                   │
│  dcc.Store(id="scroll-zoom-store")  ← scroll zoom ON/OFF                        │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Callback Flow

```
[User action]
      │
      ├─ File path entry
      │       ▼
      │  suggest_files → on_suggestion_click → load_file
      │  (Identical to #1. See Proto #1 design document §7-1 for details.)
      │  Note: load_file also generates the initial row UI
      │     → places one dropdown row per channel in rows-container
      │
      ├─ Row dropdown change + "Update" button / Ctrl+Enter
      │       ▼
      │  update_waveform_rows
      │    Input:  update-btn.n_clicks, scroll-zoom-store.data
      │    State:  {type:"row-dropdown"}.value(ALL),
      │            {type:"ymin-input"}.value(ALL),
      │            {type:"ymax-input"}.value(ALL)
      │    Output: waveform-container.children (dynamically build waveform rows)
      │            row-groups-store.data (persist row grouping info)
      │    Logic:  Keep non-empty rows only → build waveform_row() for each
      │            Apply scrollZoom / Y-axis range / SI formatting to the Figure
      │
      ├─ Add / delete row
      │       ▼
      │  manage_rows
      │    Input:  add-row-btn.n_clicks,
      │            {type:"row-delete"}.n_clicks(ALL)
      │    Output: rows-container.children, row-count.data
      │
      ├─ Toggle scale lock
      │       ▼
      │  toggle_scroll_zoom
      │    Input:  scroll-zoom-btn.n_clicks
      │    Output: scroll-zoom-store.data (ON/OFF)
      │            scroll-zoom-btn label and styling
      │       │
      │       └──▶ update_waveform_rows (auto-fires because scroll-zoom-store is an Input)
      │            → Re-render with scrollZoom=True/False + yaxis.fixedrange
      │
      ├─ Hover over a graph
      │       ▼
      │  store_hover_x → update_values + update_graphs
      │  (Same pattern as #1. Values are shown per row by referring to row-groups-store.)
      │
      ├─ Zoom / pan
      │       ▼
      │  store_xaxis_range → update_graphs
      │  (Same pattern as #1. The range is always applied to every row.)
      │
      └─ Click (with measurement mode ON)
              ▼
         set_cursor → update_graphs + update_delta_panel
         (Same pattern as #1. Deltas are computed for all channels from row-groups-store.)
```

---

## 8. Per-Feature Implementation Design

### 8-1. Per-row dropdown (ported from #0.4)

**Approach**: Place a `dcc.Dropdown(multi=True)` per row in the row definition area.
Re-render waveforms when the "Update" button is clicked (or Ctrl+Enter).

```python
# UI for a single row
make_dropdown_row(row_index, selected)
  → html.Span("行N")
  → dcc.Dropdown(id={type:"row-dropdown", index:N}, multi=True)
  → dcc.Input(id={type:"ymin-input", index:N}, type="number")  # Y axis min
  → dcc.Input(id={type:"ymax-input", index:N}, type="number")  # Y axis max
  → html.Button("×", id={type:"row-delete", index:N})
```

**Overlay rendering**: When multiple channels are selected in a single row's dropdown, they are drawn on the same Figure as overlays.
Colors are assigned sequentially from the `TRACE_COLORS` palette. The legend is shown when there are two or more channels.

### 8-2. Scale lock

**Problem**: With a trackpad, swipes are consumed by scroll zoom, making it impossible to scroll the page.

**Approach**:
- Default: `scrollZoom=False` (page scroll has priority)
- When released: `scrollZoom=True` + `yaxis.fixedrange=True` (zoom only along the X axis)

**Dash scrollZoom constraint**: `config.scrollZoom` accepts only `boolean` (strings such as `"x"` are not allowed).
Hence, Y-axis zoom is suppressed via `yaxis.fixedrange=True`.

### 8-3. Numeric Y-axis range entry

A `dcc.Input(type="number")` for min / max is placed next to each row's dropdown.
On "Update", they are passed to the `ymin` / `ymax` arguments of `make_row_fig()`.

- Both specified: `yaxis.range=[ymin, ymax]`, `autorange=False`
- Only one specified: `range=[ymin, None]` or `[None, ymax]`
- Both blank: autorange (default)

### 8-4. SI-prefix Y-axis tick labels

**Problem**: If the number of digits in Y-axis ticks varies between rows, the left edges of the graphs become misaligned.

**Solution**: `yaxis.tickformat=".3s"` + `yaxis.automargin=False` + `margin.l=60px` (fixed)

- Using SI prefixes (k, M, m, μ, etc.) keeps the digit count consistent, so the left edges of all graphs line up
- Matches the convention used by power-electronics measurement tools (oscilloscopes, etc.)

### 8-5. Keyboard shortcut (Ctrl+Enter)

A clientside callback registers `document.addEventListener('keydown')`.
On `Ctrl+Enter` (Mac: `Cmd+Enter`), it calls `click()` on `update-btn`.

```javascript
document.addEventListener('keydown', function(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        document.getElementById('update-btn').click();
    }
});
```

---

## 9. Implementation Patterns Carried Over from #1

| Pattern | Description |
|---------|-------------|
| Independent Graph approach | Each row is its own `go.Figure()`. `make_subplots` is not used |
| `hover-x-store` sync | hoverData from any Graph → Store → value display + spikelines on every row |
| `xaxis-range-store` sync | relayoutData from any Graph → Store → range applied to every row |
| `Patch()` partial updates | Update only `shapes` / `xaxis.range` instead of returning the full figure |
| `yref="paper"` shapes | Vertical lines (spikes / cursors) that span the full height of every graph |
| Alternating cursors | A → B → A updated (B cleared) → B → ... |
| Dark theme | `template="plotly_dark"`, `paper/plot_bgcolor` |
| Server-side df | Large DataFrames are held in server memory |

---

## 10. Store Reference

| Store ID | Type | Purpose |
|----------|------|---------|
| `row-count` | int | Row counter (sequential IDs for additions) |
| `row-groups-store` | list[list[str]] | Current row grouping |
| `hover-x-store` | float \| None | Current hover X coordinate |
| `xaxis-range-store` | [float, float] \| None | Synchronized X-axis range |
| `data-store` | dict | Loaded file path + list of channel names |
| `cursor-a-store` | float \| None | Cursor A X coordinate |
| `cursor-b-store` | float \| None | Cursor B X coordinate |
| `measure-mode` | bool | Measurement mode ON/OFF |
| `scroll-zoom-store` | bool | Scroll zoom ON/OFF (default OFF) |

---

## 11. Summary of Changes from #1 to #2.0

| Area | #1 | #2.0 |
|------|----|------|
| Channel selection | Single header dropdown (1 ch = 1 row) | Per-row dropdown (multi-channel overlay) |
| Row management | Auto-generated by channel selection | Add / delete buttons + Update button |
| pattern-matching key | `{type:"wf-graph", channel:ch}` | `{type:"wf-graph", row:N}` |
| scrollZoom | Always ON | Default OFF, toggle to ON (X axis only) |
| Y-axis range | Fixed to autorange | Per-row numeric min / max entry |
| Y-axis ticks | Default formatting | SI-prefix formatting (`.3s`) |
| venv | Per proto | Shared venv at panel-y root |

---

## 12. Reference Documents

- [Proto #1 Design Document](./proto_1_設計書.md) — base architecture and callback flow
- [Proto #0.4 Design Document](./proto_0_4_設計書.md) — original implementation of the dropdown-row UI
- [Waveform Display Feature Roadmap](./wave_viewer_roadmap.md) — Phase 1-5 plan
- [bug_report_0_1.md](./bug_report_0_1.md) — reasons for rejecting make_subplots
- [bug_report_2_0.md](./bug_report_2_0.md) — issues encountered during #2.0 development and their resolutions
