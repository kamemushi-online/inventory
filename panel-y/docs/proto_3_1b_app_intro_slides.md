---
marp: true
theme: default
paginate: true
size: 16:9
title: Panel_y proto_3_1b
description: proto_3_1b app introduction
---

# Panel_y `proto_3_1b`
## App Introduction

- Scope: waveform viewer (Dash + Plotly + Parquet)
- Goal: visualize and analyze large datasets without sacrificing responsiveness

---

# Overview

`proto_3_1b` retains the functionality of `proto_3_1` while
**strengthening support for large datasets** as a waveform viewer.

- Mitigations against hangs on large logs (roughly hundreds of thousands to millions of points)
- Waveform display, measurement, FFT, and plot-setting save/restore in a single screen
- Analysis references raw data while rendering uses optimized drawing — both at once

---

# Technical Highlights

- Vectorized `minmax_envelope` to speed up render-time preprocessing
- `np.searchsorted` to lighten index extraction for the visible range
- `float32` conversion at load time to reduce memory footprint
- `df_overview` cache for the full-range view to accelerate initial display

---

# Features 1/5: Waveform Display and Row Layout

- Freely assign channels per row (multi-channel overlay per row)
- Add / delete rows, specify Y-axis range (min/max)
- Toggle `line` / `step` display (per channel)
- Per-channel customization of color and line width

---

# Features 2/5: Interaction Sync and Display Control

- Sync hover position across all rows (spikeline display)
- Sync zoom / pan X-axis range across all rows
- Scale-lock toggle (scroll zoom ON/OFF)
- Light / dark theme toggle
- Pinned header, update shortcut (`Ctrl+Enter`)

---

# Features 3/5: Measurement Mode (Cursor A/B)

- With measurement mode ON, clicks place cursors A / B alternately
- Display `Δt` (time difference) and its frequency equivalent
- Cross-channel range statistics:
  - A value / B value / Δ
  - Mean / Max / Min / P-P / RMS / Slope

---

# Features 4/5: FFT Analysis

- Run FFT over the interval between the two cursors
- Up to 3 target channels can be selected
- Window functions: Hanning / Hamming / Rectangular
- Display: Amplitude / dB, frequency range entry (min/max)

---

# Features 5/5: Save / Load Plot Settings

- Save and restore plot state with `.pyc.json`
- Saved items:
  - Row composition (channels, step, ymin/ymax)
  - Colors and line widths
  - Theme
  - Scroll zoom state
- On load, the data, UI, and waveforms are restored together

---

# Planned Additions (Roadmap)

## Phase 3: Cursor Analysis Expansion
- Manage multiple cursors (T1/T2/T3 ...)
- UI for comparing arbitrary cursor pairs

## Phase 4: Signal Processing Expansion
- Apply preprocessing such as LPF (`filtfilt`) from the UI
- Richer FFT (harmonics / THD, etc.)

## Phase 5: Command Invocation
- Analysis-command input to complement GUI operations

---

# Summary

- `proto_3_1b` combines **large-dataset support** with **practical analysis features**
- Today it already covers "visualization, measurement, FFT, and settings restore" end to end
- Next phases focus on "multi-cursor", "more advanced signal processing", and "operation automation"

If you'd like, I can polish this further into a company-style template (colors, fonts, logo placement).
