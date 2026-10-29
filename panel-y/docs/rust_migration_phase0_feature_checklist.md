---
project: Panel_y
doc_type: Feature Checklist
target: Rust Migration Phase 0
created: "2026-04-27"
updated: "2026-04-27"
status: Complete
source_proto: "archive/proto_3_1b"
---

# Rust Migration Phase 0 Feature Checklist

---

## 1. Premise

The spec reference for the Rust version is `archive/proto_3_1b`.

`proto_3_1` includes the multiconverter-to-exe work, but for fixing the viewer spec itself we use `proto_3_1b` — which includes the large-data support and integrated plot configuration — as the source of truth.

---

## 2. Priority Definitions

| Priority | Meaning |
|----------|---------|
| Phase 1 | Required in the Rust vertical-slice prototype |
| Phase 2 | Required as a basic viewer |
| Phase 3 | Required as analysis functionality |
| Phase 4 | Required for operations / persistence / distribution |
| Deferred | Will not be reproduced in the initial Rust version |

---

## 3. Data Loading

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| Parquet file path input | Working | Phase 1 | Spec may change | Manual |
| File candidate suggestions | Working | Phase 2 | Spec may change | Manual |
| `.parquet` extension check | Working | Phase 1 | Full reproduction | Negative-case check |
| Treat non-`time` columns as channels | Working | Phase 1 | Full reproduction | Load test data |
| Error when 0 channels | Working | Phase 1 | Full reproduction | Negative-case check |
| Stable sort when `time` is non-monotonic | Working | Phase 1 | Full reproduction | Test data |
| Cast waveform columns to float32 | Working | Phase 1 | Spec may change | Memory check |
| `df_overview` for full-range view | Working | Phase 1 | Design may change | Performance measurement |

---

## 4. Waveform Rendering / LOD

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| One channel per row | Working | Phase 1 | Full reproduction | Manual |
| Multiple channels overlaid in one row | Working | Phase 2 | Full reproduction | Manual |
| Multi-row display | Working | Phase 2 | Full reproduction | Manual |
| X-axis ticks only on the last row | Working | Phase 2 | Spec may change | Screenshot |
| Min-Max envelope | Working | Phase 1 | Design may change | Performance measurement |
| Raw / LOD switching on zoom | Working | Phase 1 | Design may change | Performance measurement |
| `np.searchsorted`-equivalent range extraction | Working | Phase 1 | Roughly full reproduction | Performance measurement |
| Step display | Working | Phase 2 | Full reproduction | Manual |
| Edge preservation in step display | Needs check | Phase 2 | Spec improvable | Waveform comparison |

---

## 5. Row Layout / Channel Settings

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| Auto-create up to 8 rows on load | Working | Phase 2 | Spec may change | Manual |
| Add row | Working | Phase 2 | Full reproduction | Manual |
| Remove row | Working | Phase 2 | Full reproduction | Manual |
| Select multiple channels per row | Working | Phase 2 | Full reproduction | Manual |
| Select step channels per row | Working | Phase 2 | Full reproduction | Manual |
| Specify Y min/max per row | Working | Phase 2 | Full reproduction | Manual |
| Channel color setting | Working | Phase 2 | Full reproduction | Manual |
| Channel line-width setting | Working | Phase 2 | Full reproduction | Manual |

---

## 6. Interaction Synchronization

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| Hover X-coordinate sync | Working | Phase 2 | Full reproduction | Manual |
| All-row spike line | Working | Phase 2 | Full reproduction | Manual |
| Hover value display | Working | Phase 2 | Full reproduction | Numeric comparison |
| X-axis zoom sync | Working | Phase 1/2 | Full reproduction | Manual |
| X-axis pan sync | Working | Phase 1/2 | Full reproduction | Manual |
| Double-click to restore full range | Plotly built-in | Phase 2 | Spec may change | Manual |
| Scroll-zoom on/off | Working | Phase 2 | Full reproduction | Manual |

---

## 7. Measurement / Analysis

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| Measurement mode on/off | Working | Phase 3 | Full reproduction | Manual |
| Alternating Cursor A/B placement | Working | Phase 3 | Full reproduction | Manual |
| Next click after B updates A and clears B | Working | Phase 3 | Full reproduction | Manual |
| Cursor line display | Working | Phase 3 | Full reproduction | Screenshot |
| Cursor A/B value display | Working | Phase 3 | Full reproduction | Numeric comparison |
| Δt display | Working | Phase 3 | Full reproduction | Numeric comparison |
| Frequency conversion | Working | Phase 3 | Full reproduction | Numeric comparison |
| Range statistic Mean | Working | Phase 3 | Full reproduction | Numeric comparison |
| Range statistic Max/Min | Working | Phase 3 | Full reproduction | Numeric comparison |
| Range statistic P-P | Working | Phase 3 | Full reproduction | Numeric comparison |
| Range statistic RMS | Working | Phase 3 | Full reproduction | Numeric comparison |
| Range statistic Slope | Working | Phase 3 | Full reproduction | Numeric comparison |
| FFT | Working | Phase 3 | Full reproduction | Numeric comparison |
| Up to 3 FFT target channels | Working | Phase 3 | Full reproduction | Manual |
| FFT window: Hanning/Hamming/Rectangular | Working | Phase 3 | Full reproduction | Numeric comparison |
| FFT Amplitude/dB toggle | Working | Phase 3 | Full reproduction | Numeric comparison |
| FFT frequency-range specification | Working | Phase 3 | Full reproduction | Manual |

---

## 8. Plot Configuration

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| `.pyc.json` save | Working | Phase 4 | Read-compat required | File comparison |
| `.pyc.json` load | Working | Phase 4 | Read-compat required | Manual |
| Data-path restoration | Working | Phase 4 | Full reproduction | Manual |
| Row-layout restoration | Working | Phase 4 | Full reproduction | Manual |
| Step-setting restoration | Working | Phase 4 | Full reproduction | Manual |
| Y-axis range restoration | Working | Phase 4 | Full reproduction | Manual |
| Color / line-width restoration | Working | Phase 4 | Full reproduction | Manual |
| Theme restoration | Working | Phase 4 | Full reproduction | Manual |
| Scroll-zoom state restoration | Working | Phase 4 | Full reproduction | Manual |
| Channel scatter after plot-config load | Known risk | Phase 4 | Needs check | `bug_report_3_1b_plotconfig_channel_scatter.md` |

---

## 9. Theme / Auxiliary UI

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| Dark theme | Working | Phase 2 | Full reproduction | Screenshot |
| Light theme | Working | Phase 2 | Full reproduction | Screenshot |
| Theme toggle button | Working | Phase 2 | Full reproduction | Manual |
| Pin / unpin header | Working | Phase 4 | Spec may change | Manual |
| Ctrl+Enter update | Working | Phase 2 | Full reproduction | Manual |
| Plotly modebar equivalent | Plotly built-in | Deferred | Spec may change | Necessity TBD |

---

## 10. Preprocessing / Conversion Tools

| Feature | Current state | Rust priority | Reproduction | Verification |
|---------|--------------|---------------|--------------|--------------|
| CSV -> Parquet | Working | Deferred | Keep using Python | CLI check |
| MAT pre-7.2 -> Parquet | Working | Deferred | Keep using Python | CLI check |
| MAT 7.3+ -> Parquet | Working | Deferred | Keep using Python | CLI check |
| Time column detection | Working | Deferred | Keep using Python | CLI check |
| Time unit inference | Working | Deferred | Keep using Python | CLI check |
| Non-uniform resampling | Working | Deferred | Keep using Python | CLI check |
| `multiconverter.py` batch conversion | Working | Deferred | Keep using Python | CLI check |
| Exe packaging | Exists in `proto_3_1` | Deferred | Separate task | Diff check |

---

## 11. Minimum Feature Set Carried into Phase 1

- Parquet loading
- Recognize the `time` column and channel columns
- Single-channel selection
- Single-row waveform display
- pan / zoom
- min/max envelope keyed to the visible range
- Confirmation that large data does not freeze the UI

In Phase 1 we will not implement Cursor, FFT, plot configuration, or the multi-row UI.
