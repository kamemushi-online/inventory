---
project: Panel_y
doc_type: Design Spec
target: Proto #0.3
created: "2026-03-24"
updated: "2026-03-24"
status: Complete & Verified
---

# Proto #0.3 Design Document

---

## 1. Purpose

Enable multiple channels to be overlaid on a single row.
This is a validation prototype for the textarea-driven row-grouping approach (option E).

---

## 2. Feature Overview

| # | Feature | Difference vs #0.2 |
|---|---------|--------------------|
| 1 | **Overlay multiple channels per row** | **NEW** — comma-separate channel names to overlay them on the same row |
| 2 | **Textarea-based layout definition** | **NEW** — one line per row, comma-separated grouping |
| 3 | **Per-channel color coding** | **NEW** — automatic assignment from an 8-color palette |
| 4 | **List of available channels** | **NEW** — shows channel names auto-detected from the data |
| 5 | Time-axis synchronization (shared_xaxes) | Carried over from #0.1 |
| 6 | Hover (x unified) | Based on #0.1 (not the row-layout approach from #0.2) |
| 7 | Scroll zoom | Carried over from #0.1 |

---

## 3. Usage

### Launch

```bash
cd proto_0_3
source .venv/bin/activate
python app.py
# → http://localhost:8050
```

### Sample Data Generation

```bash
python generate_sample.py
```

Generates sample data simulating a motor-control step response.

| Channel | Description |
|---------|-------------|
| id_ref | d-axis current reference (5 A step input @ t=0.02 s) |
| id | d-axis current (first-order response, τ=5 ms + noise) |
| iq_ref | q-axis current reference (10 A step input @ t=0.01 s) |
| iq | q-axis current (first-order response, τ=5 ms + noise) |
| voltage_u | U-phase voltage (200 V / 50 Hz sine wave) |
| voltage_v | V-phase voltage (200 V / 50 Hz sine wave, -120° phase) |

Parameters: 10 kHz sampling, 100 ms, 1,000 points.

### Controls

| Action | Behavior |
|--------|----------|
| Edit the textarea | Change the row grouping (one line per row, comma-separate for overlay) |
| "Update" button | Apply the edited layout to the waveforms |
| Drag | Pan the time axis |
| Mouse wheel | Zoom the time axis |
| Double click | Reset zoom |

### Layout Syntax

```
id_ref, id        ← overlay id_ref and id on the same row
iq_ref, iq        ← overlay iq_ref and iq on the same row
voltage_u         ← show voltage_u on its own row
voltage_v         ← show voltage_v on its own row
```

- One line = one row
- Comma-separate to group channels
- Three or more channels can be overlaid (e.g. `id_ref, id, iq`)
- Channel names that do not exist in the data are ignored

---

## 4. Implementation Design

### File Layout

```
proto_0_3/
├── app.py                 # Dash application entry point (184 lines)
├── generate_sample.py     # Sample waveform generator (66 lines)
├── sample_data/
│   └── sample_waveform.parquet
└── .venv/
```

### UI Layout

```
┌────────────────────────────────────────────────────┐
│ Panel_y — Proto #0.3                                │
├───────────────────────────┬────────────────────────┤
│ Layout definition          │ Available channels      │
│ ┌───────────────────────┐ │ id_ref, id, iq_ref,   │
│ │ id_ref, id            │ │ iq, voltage_u,        │
│ │ iq_ref, iq            │ │ voltage_v             │
│ │ voltage_u             │ │                        │
│ │ voltage_v             │ │                        │
│ └───────────────────────┘ │                        │
│ [Update]                   │                        │
├───────────────────────────┴────────────────────────┤
│  Row 1: id_ref (blue) / id (orange) ── overlaid     │
│  ─────────────────────────────────────────────────  │
│  Row 2: iq_ref (green) / iq (red) ── overlaid       │
│  ─────────────────────────────────────────────────  │
│  Row 3: voltage_u ── standalone                     │
│  ─────────────────────────────────────────────────  │
│  Row 4: voltage_v ── standalone                     │
└────────────────────────────────────────────────────┘
```

### Key Functions

**`parse_layout(text: str) -> list[list[str]]`**

Parses the row grouping from textarea input.

- Skip blank lines
- Split on commas and trim whitespace
- Keep only channels that exist in the data

```python
parse_layout("id_ref, id\niq_ref, iq\nvoltage_u")
# → [["id_ref", "id"], ["iq_ref", "iq"], ["voltage_u"]]
```

**`build_figure(row_groups: list[list[str]]) -> go.Figure`**

Builds a figure with `make_subplots` according to the row grouping.

- `shared_xaxes=True` keeps the time axes in sync
- Multiple channels on the same row share a subplot via `add_trace`
- The Y-axis label is `" / ".join(group)` (e.g. "id_ref / id")
- Colors are assigned automatically via `TRACE_COLORS[j % 8]`

### Callback Flow

```
["Update" button click]
      │
      ▼
  update_graph(n_clicks, layout_text)
    Input:  update-btn.n_clicks
    State:  layout-input.value
      │
      ├── parse_layout(layout_text) → row_groups
      ├── build_figure(row_groups)  → fig
      └── Output: graph-container.children ← dcc.Graph(figure=fig)
```

A simple single-callback design. The hover sync and zoom sync from #0.2 are not included (this is based on #0.1).

### Color Palette

```python
TRACE_COLORS = [
    "#1f77b4",  # blue
    "#ff7f0e",  # orange
    "#2ca02c",  # green
    "#d62728",  # red
    "#9467bd",  # purple
    "#8c564b",  # brown
    "#e377c2",  # pink
    "#7f7f7f",  # gray
]
```

Colors are assigned by index within each row and reset across rows.

---

## 5. Verification Results (2026-03-24)

| # | Test item | Result |
|---|-----------|--------|
| 1 | Overlay of two channels (id_ref, id) | OK |
| 2 | Overlay of three or more channels | OK |
| 3 | Standalone row display | OK |
| 4 | Edit text → apply via Update button | OK |
| 5 | Time-axis zoom/pan | OK |
| 6 | Non-existent channel names are ignored | OK |

---

## 6. Findings and Next Steps

The textarea approach works but has the following drawbacks:

- Channel names must be typed by hand, which invites typos
- Managing the text gets cumbersome with many channels

→ **Proto #0.4** validates a dropdown-based UI (completed).
→ Once the UI approach is settled, port it into **Proto #1**.

---

## 7. Related Documents

- [Proto #0.2 Design Document](./proto_0_2_設計書.md) — Row layout and hover sync (baseline)
- [Proto #1 Design Document](./proto_1_設計書.md) — Production viewer (port target)
- [Waveform Viewer Roadmap](./wave_viewer_roadmap.md) — Future development plan
