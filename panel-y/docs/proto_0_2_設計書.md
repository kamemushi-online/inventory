---
project: Panel_y
doc_type: Design Spec
target: Proto #0.2
created: "2026-03-15"
updated: "2026-03-15"
status: Complete & Verified
---

# Proto #0.2 Design Document

---

## 1. Feature Overview

| # | Feature | Difference vs #0.1 |
|---|---------|--------------------|
| 1 | **Row layout** `[channel name/value] [waveform graph]` | **NEW** |
| 2 | **Simultaneous hover values for every channel** | **NEW** (not in #0.1) |
| 3 | **Full-height spike line across all graphs** | **FIX** (the first #0.1/0.2 drafts drew it on one panel only) |
| 4 | **Inter-graph zoom/pan sync** | **NEW** |
| 5 | Parquet data loading | Carried over from #0.1 |
| 6 | Automatic channel detection | Carried over from #0.1 |
| 7 | Scroll zoom | Carried over from #0.1 |
| 8 | Toolbar (hover display) | **FIX** (restored after going missing in the first #0.2 draft) |

---

## 2. Usage

### Launch

```bash
cd proto_0_2
source ../proto_0_1/.venv/bin/activate   # reuse the proto_0_1 venv
python app.py
# → http://localhost:8050
```

### Controls

| Action | Behavior |
|--------|----------|
| Mouse hover on a graph | Shows every channel's value in the left column; draws a full-height spike line across all graphs |
| Drag | Pans the time axis; all graphs stay in sync |
| Mouse wheel | Zooms the time axis; all graphs stay in sync |
| Double click | Resets zoom (returns to the full-range view) |
| Toolbar (appears on hover) | Standard Plotly actions: zoom, pan, PNG export, and so on |

---

## 3. Implementation Design

### File Layout

```
proto_0_2/
└── app.py      # Dash application entry point (references proto_0_1's sample_data)
```

> `sample_data/` and `.venv/` are shared with `proto_0_1/`. No `requirements.txt` is needed.

### Overall Architecture

```
┌─────────────────────────────────────────────────────┐
│ app.layout                                          │
│  ┌──────────────┬──────────────────────────────┐   │
│  │ Channel/Value│ dcc.Graph(id="graph-voltage") │   │  ← channel_row()
│  │  "voltage_u" │                              │   │
│  │  id=val-volt │    [waveform graph]           │   │
│  ├──────────────┼──────────────────────────────┤   │
│  │ "current_u"  │ dcc.Graph(id="graph-current") │   │  ← channel_row()
│  │  id=val-curr │    [waveform graph]           │   │
│  └──────────────┴──────────────────────────────┘   │
│                                                     │
│  dcc.Store(id="hover-x-store")                      │
│  dcc.Store(id="xaxis-range-store")                  │
└─────────────────────────────────────────────────────┘
```

### Callback Flow

```
[User action]
      │
      ├─ Hover over a graph
      │       │
      │       ▼
      │  Callback 1: store_hover_x
      │    Input:  graph-{ch}.hoverData (all channels)
      │    Output: hover-x-store.data (X coordinate)
      │       │
      │       ├──▶ Callback 2: update_values
      │       │      Input:  hover-x-store.data
      │       │      Output: val-{ch}.children (all channels)
      │       │      Logic:  use idxmin on df["time"] to find the nearest index
      │       │
      │       └──▶ Callback 4: update_graphs (triggered="hover-x-store")
      │              → Patch a vertical-line shape with yref="paper" into every graph
      │
      └─ Zoom / pan
              │
              ▼
         Callback 3: store_xaxis_range
           Input:  graph-{ch}.relayoutData (all channels)
           Output: xaxis-range-store.data ([x_min, x_max] or None)
              │
              ▼
         Callback 4: update_graphs (triggered="xaxis-range-store")
           → Patch xaxis.range across every graph to keep them in sync
```

### Implementation Highlights

**Per-channel figures (`make_channel_fig`)**

```python
hoverinfo="none"    # hide the tooltip / still fire hoverData events
hovermode="x"       # enable X-axis hover detection
```

> `hoverinfo="skip"` disables the event itself and cannot be used (see bug_report_0_2).

**Hover values: reverse lookup in the DataFrame**

```python
idx = (df["time"] - x_val).abs().idxmin()   # nearest index
return [f"{df[ch].iloc[idx]:.4f}" for ch in channels]
```

**Spike line: span every graph with `yref="paper"`**

```python
{"type": "line", "xref": "x", "yref": "paper", "y0": 0, "y1": 1, ...}
```

> Plotly's native spike line only renders inside a single graph, so we add it dynamically as a shape via a Dash callback (see bug_report_0_1).

**Incremental updates: `Patch()`**

```python
p = Patch()
p["layout"]["shapes"] = [...]   # update only the relevant part of the figure
```

> Returning the entire figure is expensive on the wire, so we use Dash 2.9+'s `Patch()`.

**Zoom synchronization: dispatch on `ctx.triggered_id`**

```python
triggered = ctx.triggered_id
if triggered == "hover-x-store":   # hover origin → update only the spike line
if triggered == "xaxis-range-store": # zoom origin → update only xaxis.range
```

> Handling both events in one callback avoids Output conflicts (Dash disallows multiple callbacks writing to the same Output).

**Toolbar configuration**

```python
config={
    "scrollZoom": True,
    "displayModeBar": "hover",                            # show only on hover
    "modeBarButtonsToRemove": ["select2d", "lasso2d"],    # drop unused buttons
}
```

---

## 4. Improvements

> Items captured here are candidates for Proto #1 and beyond.

| # | Improvement | Rationale |
|---|-------------|-----------|
| 1 | **No file selection UI** | The data path is hard-coded; arbitrary files cannot be loaded |
| 2 | **No units on the Y axis** | Only the channel name is shown — units like V or A are missing |
| 3 | **Cannot toggle channel visibility** | With many channels there is no way to hide specific ones |
| 4 | **No cursor diff calculations** | No support for Δt, ΔV, or ΔA |
| 5 | **Hover values appear only in the left column, not on the graph** | An inline-on-graph view is sometimes preferred |
| 6 | **Values persist after the cursor leaves the graph** | They do not reset to `---` on mouse-out |
| 7 | **No data-preprocessing utilities** | No CSV/MATLAB → Parquet conversion |

---

## 5. Referenced Bug Reports

- [bug_report_0_1.md](./bug_report_0_1.md) — Spike line not propagating across panels (make_subplots axis-reference issue)
- [bug_report_0_2.md](./bug_report_0_2.md) — Hover values, spike line, and toolbar disappearing (hoverinfo / displayModeBar misconfiguration)
