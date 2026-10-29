---
project: Panel_y
doc_type: Bug Report
target: Proto #2.0
created: "2026-03-24"
status: All Fixed
---

# Bug Report — Issues and Fixes During Proto #2.0 Development

---

## Issue 1: With `make_subplots`, hover and spike line do not sync across rows

### Symptom

Early on we used `make_subplots(shared_xaxes=True)` together with `hoversubplots="axis"`,
but hover values only appeared in the subplot the user was interacting with, and the
spike line never crossed subplot boundaries.

### Root cause

Identical to the known issue documented in bug_report_0_1.md.
With `make_subplots` each trace references a different X-axis object (`x`, `x2`, ...),
so `hoversubplots="axis"` does not behave as expected.

### Fix

Abandon `make_subplots` and switch to the same **independent-Graph approach** used in Proto #1
(one `go.Figure()` per row). Adopt the Store-based synchronization pattern through
`hover-x-store` and `xaxis-range-store`.

### Lesson learned

The hover-sync limitation of `make_subplots` is a fundamental constraint rooted in Plotly's architecture.
For a multi-row waveform viewer, the independent-Graph approach is the only viable solution.
Past bug reports let us pivot quickly.

---

## Issue 2: Passing the string `"x"` to `config.scrollZoom` raises an error

### Symptom

To restrict scroll-zoom to the X-axis we set `config={"scrollZoom": "x"}`,
which made Dash return a validation error.

```
Invalid argument `config.scrollZoom` passed into Graph with ID "{"row":1,"type":"wf-graph"}".
Expected `boolean`. Was supplied type `string`.
```

### Root cause

Plotly.js itself accepts `scrollZoom: "x"`, but Dash's `dcc.Graph` component prop
validation only allows `boolean`.

### Fix

Use only `True`/`False` (boolean) for `scrollZoom`,
and constrain Y-axis zoom via `yaxis.fixedrange=True`.

```python
# Before (errors out)
config={"scrollZoom": "x"}

# After (fixed)
config={"scrollZoom": True}  # boolean only
yaxis=dict(fixedrange=True)  # disable Y-axis zoom
```

### Lesson learned

Plotly.js and Dash (its Python wrapper) sometimes accept different types for the same option.
Always check Dash's documentation for the type constraints.

---

## Issue 3: Branching on `ctx.triggered_id` prevents zoom sync from firing

### Symptom

Scroll-zooming in one row did not propagate the X-axis range to the other rows.

### Root cause

The `update_graphs` callback used `ctx.triggered_id` to identify the event kind and
branch between updating shapes and syncing the range.

```python
# Before
if triggered in ("hover-x-store", "cursor-a-store", "cursor-b-store"):
    p["layout"]["shapes"] = shapes
if triggered == "xaxis-range-store":
    p["layout"]["xaxis"]["range"] = xaxis_range
```

When multiple Inputs fired together or Dash's callback execution order shifted,
`triggered_id` could differ from the expected value and range sync never ran.

### Fix

Drop the conditional branching on `ctx.triggered_id` and always apply both shapes and range.

```python
# After — always apply every update
p["layout"]["shapes"] = shapes
if xaxis_range:
    p["layout"]["xaxis"]["range"] = xaxis_range
    p["layout"]["xaxis"]["autorange"] = False
else:
    p["layout"]["xaxis"]["autorange"] = True
```

### Lesson learned

For callbacks with multiple Inputs, avoid branching on `ctx.triggered_id` and prefer an
idempotent "apply everything" approach. With `Patch()`-based diff updates the cost of
sending unnecessary fields is negligible.

---

## Issue 4: The left edge of graphs drifts when Y-axis tick label widths differ

### Symptom

When the numeric range of Y-axis values varies across rows (for example, -10 to 10 vs.
-1000 to 1000), Plotly auto-adjusts the Y-axis label width, so the left edge of the
drawing area drifts row by row. Spike lines and cursor lines line up correctly with
the time axis but appear visually offset.

### Fix

Applied a three-step countermeasure:

1. **`yaxis.automargin=False`**: disable Plotly's automatic margin adjustment
2. **`margin.l=60`**: fix the left margin at a constant value
3. **`yaxis.tickformat=".3s"`**: SI-prefix notation (1k, 1M, 1m, etc.) to keep digit count constant

Because SI-prefix notation produces fixed-width labels regardless of value magnitude,
a fixed margin handles both very large and very small values.

### Lesson learned

Plotly's `automargin` is convenient but breaks alignment between graphs stacked vertically.
The combination of a fixed margin and a fixed-width tickformat is the most stable.
