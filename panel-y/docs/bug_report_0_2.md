---
project: Panel_y
doc_type: Bug Report
target: Proto #0.2
created: "2026-03-15"
status: Open
---

# Bug Report — Three Issues in Proto #0.2

---

## Bug #1: Spike line does not appear

### Problem
Hovering over a graph does not display the cross-panel vertical spike line.

### Root cause
The spike line is drawn by Callback 4 (`update_graphs`) when it receives an update from `hover-x-store`.
However, `hover-x-store` is only updated when Callback 1 (`store_hover_x`) receives `hoverData`.

The reason `hoverData` is not fired lies in Bug #2.
→ **A chained failure caused by Bug #2.**

### Fix
Fixing Bug #2 resolves this as a side effect.

---

## Bug #2: Hover values are not displayed

### Problem
Hovering over a graph leaves the values in the left column (`val-{ch}`) as `---` without updating.

### Root cause
Traces are created with `hoverinfo="skip"`.

```python
# In app.py: make_channel_fig()
fig.add_trace(go.Scatter(
    ...
    hoverinfo="skip",   ← root cause
))
```

Behavior of Plotly's `hoverinfo`:

| Value | Tooltip | hoverData event |
|----|-------------|-------------------|
| `"skip"` | Hidden | **Does not fire** ← problem |
| `"none"` | Hidden | Fires |
| `"x+y"`, etc. | Shown | Fires |

`hoverinfo="skip"` disables the hover event itself, so
Dash's `hoverData` callback property is never updated.
→ Callback 1 never runs, and neither the value display nor the spike line works.

### Fix
```python
# Before
hoverinfo="skip"

# After (hide the tooltip but still fire the event)
hoverinfo="none"
```

---

## Bug #3: Zoom, pan, and reset buttons have disappeared

### Problem
The Plotly standard toolbar (zoom, pan, axis reset, save as PNG) that was present in #0.1 is no longer shown.

### Root cause
`dcc.Graph`'s `config` specifies `displayModeBar: False`.

```python
# In app.py: channel_row()
dcc.Graph(
    config={"scrollZoom": True, "displayModeBar": False},  ← toolbar completely hidden
)
```

Behavior of `displayModeBar`:

| Value | Behavior |
|----|------|
| `True` | Always shown |
| `False` | Completely hidden ← problem |
| `"hover"` | Shown only on mouse hover (same as #0.1) |

`displayModeBar: False` was set because each graph is small in the row layout and the bar felt obtrusive.
But this also removed zoom and reset operations entirely.

### Fix
```python
# After (show only on hover, and remove unneeded buttons)
config={
    "scrollZoom": True,
    "displayModeBar": "hover",
    "modeBarButtonsToRemove": ["select2d", "lasso2d", "autoScale2d"],
}
```

---

## Modified files

- `proto_0_2/app.py`
  - Inside `make_channel_fig()`: `hoverinfo="skip"` → `hoverinfo="none"`
  - Inside `channel_row()`: `displayModeBar: False` → `displayModeBar: "hover"`
