---
project: Panel_y
doc_type: Design Spec
target: Proto #0.4
created: "2026-03-24"
updated: "2026-03-24"
status: Complete & Verified
---

# Proto #0.4 Design Document

---

## 1. Purpose

Validate a **dropdown-based UI** for row grouping as an alternative to the textarea approach in Proto #0.3.
Compare usability against #0.3 and decide how to port the design into #1.

---

## 2. Feature Overview

| # | Feature | Difference vs #0.3 |
|---|---------|--------------------|
| 1 | Overlay multiple channels per row | Carried over from #0.3 |
| 2 | **Dropdown-based layout definition** | **NEW** — one multi-select dropdown per row |
| 3 | **Dynamic add/remove of rows** | **NEW** — "+ Add row" button and per-row "×" delete button |
| 4 | **Pattern-Matching Callbacks** | **NEW** — bulk handling of dynamically added/removed components |
| 5 | Channel color coding (8-color palette) | Carried over from #0.3 |
| 6 | Time-axis synchronization (shared_xaxes) | Carried over from #0.1 |
| 7 | Hover (x unified) | Based on #0.1 |
| 8 | Scroll zoom | Carried over from #0.1 |

### Comparison with #0.3

| Aspect | #0.3 Textarea | #0.4 Dropdown |
|--------|---------------|---------------|
| Channel selection | Manual typing (typo risk) | Pick-from-list (no typos) |
| Adding/removing rows | Edit text | Button clicks |
| Operational feel | Programmer-friendly | End-user friendly |
| Flexibility | High (free-form) | Moderate (pick from options) |
| Portability to #1 | Low (clashes with the existing UI) | High (consistent with the dropdown idioms used in #1) |

---

## 3. Usage

### Launch

```bash
cd proto_0_4
source .venv/bin/activate
python app.py
# → http://localhost:8050
```

### Sample Data Generation

```bash
python generate_sample.py
```

The sample data is identical to #0.3 (motor-control step response, 6 channels, 10 kHz, 100 ms).

### Controls

| Action | Behavior |
|--------|----------|
| Row dropdown | Multi-select channels (they will be overlaid on the same row) |
| "+ Add row" button | Append an empty row at the end |
| "×" button | Delete that row (the last remaining row cannot be deleted) |
| "Update" button | Apply the dropdown selections to the waveforms |
| Drag | Pan the time axis |
| Mouse wheel | Zoom the time axis |
| Double click | Reset zoom |

---

## 4. Implementation Design

### File Layout

```
proto_0_4/
├── app.py                 # Dash application entry point (294 lines)
├── generate_sample.py     # Sample waveform generator (shared with #0.3)
├── sample_data/
│   └── sample_waveform.parquet
└── .venv/
```

### UI Layout

```
┌──────────────────────────────────────────────────┐
│ Panel_y — Proto #0.4                              │
│ Select channels per row via dropdowns / overlay   │
├──────────────────────────────────────────────────┤
│ [+ Add row]  [Update]                             │
│                                                    │
│ Row 1  [ id_ref ▼ | id ▼        ]  [×]            │
│ Row 2  [ iq_ref ▼ | iq ▼        ]  [×]            │
│ Row 3  [ voltage_u ▼             ]  [×]            │
│ Row 4  [ voltage_v ▼             ]  [×]            │
├──────────────────────────────────────────────────┤
│  Row 1: id_ref (blue) / id (orange) ── overlaid   │
│  ────────────────────────────────────────────── │
│  Row 2: iq_ref (green) / iq (red) ── overlaid     │
│  ────────────────────────────────────────────── │
│  Row 3: voltage_u ── standalone                   │
│  ────────────────────────────────────────────── │
│  Row 4: voltage_v ── standalone                   │
└──────────────────────────────────────────────────┘
```

### Component Structure

```
app.layout
├── dcc.Store(id="row-count")           ← monotonically increasing row-index counter
├── html.H2 — title
├── html.Div — row-definition area
│   ├── [+ Add row] [Update] buttons
│   └── html.Div(id="rows-container")   ← parent container for the row UI
│       ├── make_row_div(0, ["id_ref", "id"])
│       ├── make_row_div(1, ["iq_ref", "iq"])
│       ├── make_row_div(2, ["voltage_u"])
│       └── make_row_div(3, ["voltage_v"])
└── html.Div(id="graph-container")
    └── dcc.Graph(id="waveform")
```

### Row UI Structure (`make_row_div`)

Each row is composed of three elements:

```
html.Div (flex row)
├── html.Span("Row N")                                  ← label
├── dcc.Dropdown(id={"type":"row-dropdown","index":N})   ← multi-select
└── html.Button("×", id={"type":"row-delete","index":N}) ← delete button
```

- The `id` uses the Pattern-Matching dict form
- `index` is issued from the `row-count` Store (monotonic; never reused even after a row is deleted)

### Callback Flow

```
[Row operations]                    [Waveform update]
    │                                   │
    ├─ "+ Add row" click               "Update" click
    │       │                                │
    │       ▼                                ▼
    │  Callback 1: manage_rows          Callback 2: update_graph
    │    Input:  add-row-btn.n_clicks     Input:  update-btn.n_clicks
    │    Input:  {"type":"row-delete",    State:  {"type":"row-dropdown",
    │             "index":ALL}.n_clicks            "index":ALL}.value
    │    State:  rows-container.children     │
    │    State:  row-count.data              ├── Filter out empty values
    │       │                                ├── build_figure(row_groups)
    │       ├── Add: make_row_div(count)     └── Output: graph-container
    │       │   → append to children
    │       │   → row-count + 1
    │       │
    │       └── Delete: drop child elements
    │           that match triggered_id.index
    │           → guard against reaching 0 rows
    │
    ├─ "×" click
    │       │
    │       └── (same manage_rows)
```

### Implementation Highlights

**Pattern-Matching Callbacks (`ALL`)**

```python
Input({"type": "row-delete", "index": ALL}, "n_clicks")
State({"type": "row-dropdown", "index": ALL}, "value")
```

- `ALL` receives every component sharing the same `type` at once
- This is Dash's standard mechanism for handling dynamically added/removed rows

**Monotonic row indexing**

```python
dcc.Store(id="row-count", data=len(DEFAULT_ROWS))
```

- `row-count` is incremented when a row is added and used as the new row's `index`
- Indexes are never reused after deletion, guaranteeing unique IDs

**Filtering children on delete**

```python
row_div_children = child["props"]["children"]
dropdown = row_div_children[1]
dropdown_id = dropdown["props"]["id"]
if dropdown_id["index"] != delete_index:
    updated.append(child)
```

- From the dict-form `children` of `rows-container`, drop any element whose `index` matches the deleted row
- The last remaining row cannot be deleted (this prevents the row list from going empty)

**`build_figure` is shared with #0.3**

The two-stage `parse_layout` → `build_figure` from #0.3 collapses to a single stage here: the dropdown `value` list is fed directly into `build_figure`. `build_figure` itself is essentially identical to #0.3.

---

## 5. Verification Results (2026-03-24)

| # | Test item | Result |
|---|-----------|--------|
| 1 | Default 4 rows shown with dropdowns | OK |
| 2 | Select channels in a dropdown → Update → overlay rendered | OK |
| 3 | Add row button → empty row appended | OK |
| 4 | × button → row deleted | OK |
| 5 | Last remaining row cannot be deleted | OK |
| 6 | Two-channel overlay | OK |
| 7 | Three-or-more-channel overlay | OK |
| 8 | Time-axis zoom/pan | OK |

---

## 6. Findings and Next Steps

Compared with the textarea approach in #0.3, the dropdown approach offers:

- **Advantages**: no typos in channel names, more intuitive operation, and consistency with the existing UI idioms in #1
- **Drawbacks**: the UI grows tall when there are many rows (a collapsible row design may be needed)

→ **The port to #1 will adopt the dropdown UI** (Phase 1-1).

---

## 7. Related Documents

- [Proto #0.3 Design Document](./proto_0_3_設計書.md) — Textarea approach (comparison target)
- [Proto #1 Design Document](./proto_1_設計書.md) — Production viewer (port target)
- [Waveform Viewer Roadmap](./wave_viewer_roadmap.md) — Future development plan
