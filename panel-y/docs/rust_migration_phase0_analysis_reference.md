---
project: Panel_y
doc_type: Analysis Reference
target: Rust Migration Phase 0
created: "2026-04-27"
updated: "2026-04-27"
status: Complete
source_proto: "archive/proto_3_1b"
---

# Rust Migration Phase 0 Analysis Reference

---

## 1. Purpose

This document defines reference values based on fixed data, fixed cursors, and fixed parameters so that we can verify whether the Rust version's Cursor / range statistics / FFT produce results equivalent to `proto_3_1b`.

---

## 2. Reference Data

| Item | Value |
|------|-------|
| File | `data/test_100k.parquet` |
| Rows | 100,000 |
| time range | 0.0 s to 0.99999 s |
| Ts | 0.00001 s |
| Fs | 100 kHz |
| Channels | `sine_50Hz`, `pwm_1kHz`, `noise`, `chirp_1_500Hz` |

---

## 3. Range Statistics Reference

Fixed cursors:

- Cursor A: `0.1 s`
- Cursor B: `0.3 s`

Calculation specification for the current Dash version:

- `idx_a = (df["time"] - cursor_a).abs().idxmin()`
- `idx_b = (df["time"] - cursor_b).abs().idxmin()`
- `i_lo, i_hi = min(idx_a, idx_b), max(idx_a, idx_b) + 1`
- The range statistics are inclusive on both ends
- `Slope = (B - A) / (cursor_b - cursor_a)`

Reference indices:

| Item | Value |
|------|-------|
| idx_a | 10000 |
| idx_b | 30000 |
| time_a | 0.1 |
| time_b | 0.30000000000000004 |
| Sample count | 20001 |

Reference values:

| ch | A | B | Delta | Mean | Max | Min | P-P | RMS | Slope/s |
|----|---|---|-------|------|-----|-----|-----|-----|---------|
| sine_50Hz | 2.328066879653148e-14 | 1.7642341675360946e-13 | 1.53142747957078e-13 | -4.547246146557313e-17 | 10.0 | -10.0 | 20.0 | 7.070891041799028 | 7.6571373978539e-13 |
| pwm_1kHz | 5.0 | 5.0 | 0.0 | 1.5001749912504374 | 5.0 | 0.0 | 5.0 | 2.7387725272925074 | 0.0 |
| noise | 0.17627703572818604 | -1.0114276118671959 | -1.1877046475953819 | 0.017334500301483497 | 4.151241397039601 | -3.887804341619664 | 8.039045738659265 | 1.0031410348802567 | -5.93852323797691 |
| chirp_1_500Hz | -4.135402871372795 | 0.15705379539082578 | 4.292456666763621 | 0.015371182337173656 | 4.999999948213196 | -4.999999998144212 | 9.999999946357407 | 3.5224359648687225 | 21.462283333818107 |

Tolerances:

- A/B/Delta/Mean/Max/Min/P-P/RMS/Slope: `1e-6` as the baseline
- Comparisons made after conversion to float32 are allowed up to `1e-5`

---

## 4. FFT Reference

Fixed cursors:

- Cursor A: `0.1 s`
- Cursor B: `0.9 s`

FFT conditions:

- Target range: inclusive on both A and B ends
- Window function: Hanning
- DC removal: applied via `seg - seg.mean()`
- Amplitude calculation: `abs(rfft(windowed)) * 2.0 / N`
- dB calculation: `20 * log10(max(amplitude, 1e-12))`

Reference information:

| Item | Value |
|------|-------|
| idx_a | 10000 |
| idx_b | 90000 |
| N | 80001 |
| dt | 9.999999999996123e-06 |

Peak reference values:

| ch | peak_freq [Hz] | peak_amp | peak_dB |
|----|----------------|----------|---------|
| sine_50Hz | 49.99937500783179 | 4.9999366897603945 | 13.979290104873243 |
| pwm_1kHz | 999.9875001566357 | 1.2867577545787694 | 2.1899358836646003 |
| chirp_1_500Hz | 249.99687503915894 | 0.2797771990412082 | -11.063753640634989 |

Tolerances:

- peak_freq: `0.1 Hz`
- peak_amp: `1e-4`
- peak_dB: `1e-3 dB`

---

## 5. Behaviors the Rust Version Must Match

- Pick the nearest sample to the cursor time
- Include both ends of the range
- Compute statistics on the raw data
- Compute FFT on the raw data as well
- Do not use display-purpose LOD or min/max envelope data for analysis
- Subtract the range mean before FFT
- Match Hanning / Hamming / Rectangular definitions to NumPy's

---

## 6. Open Items

- Analysis of non-uniformly sampled data in the Rust version
- Handling of data containing NaN / Inf
- Upper limit on extremely long FFT ranges
- Whether analysis precision should be based on float32 or float64 storage

These items will be specified separately in Phase 3.
