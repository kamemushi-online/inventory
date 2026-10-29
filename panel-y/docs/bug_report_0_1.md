---
project: Panel_y
doc_type: Bug Report
target: Proto #0.1 / #0.2
created: "2026-03-15"
status: Fixed (addressed in Proto #0.2b)
---

# Bug Report — Hover Values and Spike Lines Do Not Synchronize Across All Channels

## Problem

1. **Hover values appear only for the channel under the cursor**
   - Only the value of the panel the mouse is over is displayed
   - Values of other channels at the same time cannot be checked

2. **Spike line appears only in the hovered panel**
   - The vertical line is drawn only inside a single panel
   - It does not become a vertical line that spans all panels

---

## Root Cause

### Axis-reference structure of `make_subplots`

When you use `make_subplots(shared_xaxes=True)`, each trace references a **different X-axis object**.

```python
# Actual reference state (confirmed with python -c)
trace=ch1, xaxis=x,  yaxis=y
trace=ch2, xaxis=x2, yaxis=y2
xaxis2.matches: None   ← no matches link either
```

`hoversubplots="axis"` only works **between traces that reference the same xaxis object**.
Because `x` and `x2` are different objects, hover does not synchronize.

### Spike line limitation

Plotly's spike lines (`showspikes=True`) **cannot cross subplot boundaries**.
A spike whose `xref` is `x` is only rendered within that subplot's drawing area.

---

## Fix and Modified Files

**Modified file**: `proto_0_2/app.py` (effectively reimplemented as #0.2b)

### Fix 1: Hover values — unify every trace under `xaxis="x"` with a manual layout

Drop `make_subplots` and assign Y-axis domains directly on `go.Figure()`.
Because every trace then references `xaxis="x"`, `hovermode="x unified"` works across all channels.

```python
# Before (make_subplots)
fig.add_trace(go.Scatter(...), row=1, col=1)  # → xaxis="x"
fig.add_trace(go.Scatter(...), row=2, col=1)  # → xaxis="x2" ← problem

# After (manual layout)
fig.add_trace(go.Scatter(x=..., y=..., xaxis="x", yaxis="y"))   # ch1
fig.add_trace(go.Scatter(x=..., y=..., xaxis="x", yaxis="y2"))  # ch2
# → All traces reference xaxis="x" → hover is synced across all channels
```

### Fix 2: Spike line — add `yref="paper"` shapes dynamically via a Dash callback

Receive `hoverData` in a callback and add a vertical-line shape spanning the entire figure (`yref="paper"`).

```python
from dash import Patch

@app.callback(
    Output("waveform", "figure"),
    Input("waveform", "hoverData"),
    prevent_initial_call=True,
)
def update_spike(hoverData):
    patched = Patch()
    if hoverData is None:
        patched["layout"]["shapes"] = []
    else:
        x_val = hoverData["points"][0]["x"]
        patched["layout"]["shapes"] = [{
            "type": "line",
            "x0": x_val, "x1": x_val,
            "y0": 0, "y1": 1,
            "xref": "x",
            "yref": "paper",   # ← vertical line spanning the full figure height
            "line": {"color": "rgba(150,150,150,0.8)", "width": 1},
        }]
    return patched
```
