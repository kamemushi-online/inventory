---
project: Panel_y
doc_type: Design Spec
target: Proto #0.1 - Hello Dash
created: "2026-03-15"
updated: "2026-03-15"
status: Complete & Verified
---

# Proto #0.1 Design Document — Hello Dash

---

## 1. Feature Overview

| # | Feature | Summary |
|---|---------|---------|
| 1 | **Parquet data loading** | Loads a `.parquet` file from a fixed path at application startup |
| 2 | **Automatic channel detection** | Treats every column other than `time` as a channel |
| 3 | **Multi-channel waveform display** | Dynamically generates subplots based on the channel count |
| 4 | **Time-axis synchronization** | Pan and zoom are linked across all panels (`shared_xaxes=True`) |
| 5 | **Hover values** | Shows values for all channels at the cursor position simultaneously (X unified mode) |
| 6 | **Spike lines** | Draws a vertical spike line across every panel on hover |
| 7 | **Scroll zoom** | Mouse wheel zooms the time axis in and out |
| 8 | **Sample data generator** | A script that generates a 2-channel waveform (voltage + current) for motor-control scenarios as Parquet |

---

## 2. Usage

### Environment Setup (first time only)

```bash
cd proto_0_1
python3 -m venv .venv
source .venv/bin/activate      # Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

### Sample Data Generation (first time only)

```bash
python generate_sample.py
# → generates sample_data/sample_waveform.parquet
```

### Launching the App

```bash
python app.py
# → open http://localhost:8050 in your browser
```

### Controls

| Action | Behavior |
|--------|----------|
| Mouse hover | Shows the values of all channels at the cursor position |
| Drag | Pans the time axis (horizontal scroll). All panels stay in sync |
| Mouse wheel | Zooms the time axis in/out. All panels stay in sync |
| Double click | Resets zoom (returns to the full-range view) |
| Top-right toolbar | Standard Plotly actions such as zoom, pan, and PNG export |

---

## 3. Implementation Design

### File Layout

```
proto_0_1/
├── app.py                        # Dash application entry point
├── generate_sample.py            # Script that generates the sample Parquet
├── requirements.txt              # Python dependencies
├── sample_data/
│   └── sample_waveform.parquet   # Generated sample data (git-ignored)
└── .venv/                        # Python virtual environment (git-ignored)
```

### Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `dash` | >=2.16.0 | Web app framework and UI |
| `plotly` | >=5.20.0 | Plotting |
| `pandas` | >=2.0.0 | Loading and manipulating Parquet data |
| `numpy` | >=1.26.0 | Sample data generation (sine wave math) |
| `pyarrow` | >=15.0.0 | Parquet engine (pandas dependency) |

### Data Specification

**Parquet format (`sample_waveform.parquet`)**

| Column | Type | Description |
|--------|------|-------------|
| `time` | float64 | Time [s] |
| `voltage_u` | float64 | U-phase voltage [V] |
| `current_u` | float64 | U-phase current [A] |

**Sample-data parameters (`generate_sample.py`)**

| Parameter | Value | Description |
|-----------|-------|-------------|
| `SAMPLE_RATE` | 10,000 Hz | Sampling frequency |
| `DURATION` | 0.1 s (100 ms) | Capture duration |
| `FREQ` | 50 Hz | Fundamental frequency |
| Voltage amplitude | 200 V | Sine wave |
| Current amplitude | 10 A | Sine wave, 30° lagging phase |
| Total samples | 1,000 points | |

### app.py Processing Flow

```
Startup
 │
 ├─ Load Parquet (pd.read_parquet)
 │   └─ Auto-detect every column except `time` as a channel
 │
 ├─ Build Plotly Figure
 │   ├─ make_subplots (n_channels × 1 column, shared_xaxes=True)
 │   ├─ Add a go.Scatter per channel
 │   ├─ X axis: spikemode="across"; only the bottom panel shows the "Time [s]" label
 │   └─ layout: hovermode="x unified", template="plotly_dark"
 │
 └─ Define Dash layout
     ├─ html.H2 (title)
     └─ dcc.Graph (figure=fig, scrollZoom=True)
```

### Design Highlights

**Automatic channel detection**
```python
channels = [col for col in df.columns if col != "time"]
```
Parquet column names are used as-is for channel names and Y-axis labels. `app.py` requires no edits when the channel count changes.

**Time-axis synchronization**
```python
make_subplots(shared_xaxes=True)
```
Plotly's `shared_xaxes` keeps pan and zoom in sync across every panel automatically — no callback required.

**Dynamic figure height**
```python
height=300 * len(channels)
```
The total figure height scales automatically with the channel count.

---

## 4. Improvements

> Items captured here are candidates for Proto #1 and beyond.

| # | Improvement | Rationale |
|---|-------------|-----------|
| 1 | **No file selection UI** | The data path is hard-coded; arbitrary Parquet files cannot be loaded |
| 2 | **Cannot swap data after launch** | No callbacks; changing data requires restarting the server |
| 3 | **Cannot toggle channel visibility** | Clicking the legend hides traces but the panel itself stays |
| 4 | **No cursor diff calculations** | No support for Δt, ΔV, or ΔA |
| 5 | **Units not shown on labels** | Y-axis labels carry only channel names (no units) |
| 6 | **No data-preprocessing utilities** | No conversion path from CSV or MATLAB data |
| 7 | **No handling for large datasets** | No downsampling; rendering may slow down with many points |
