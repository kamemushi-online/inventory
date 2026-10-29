---
project: Panel_y
doc_type: Phase 2 Performance Results
target: Rust Migration Phase 2
created: "2026-04-30"
updated: "2026-05-02"
status: P2-12 measurement complete
implementation: "code/rust_phase1"
---

# Rust Migration Phase 2 Performance Measurement Results

---

## 1. Purpose

This document quantitatively verifies display-data generation and memory usage based on the state reached after Phase 2, which added multiple rows, multiple channels, line/step rendering, hover synchronization, and asynchronous loading.

These measurements do not target the GUI rendering itself; instead, the display-data generation equivalent to `visible_row_traces()` (which the GUI calls every frame) is exercised from the CLI.

What the measurement covers:

- Individual reads of the time column and each channel column
- Memory for the shared time and the channel raw cache
- Display-data generation for multiple rows and multiple channels
- Range-aware min/max envelope generation for line channels
- Raw step / change-point step / envelope fallback decisions for step channels
- Value lookup at the hover position

What the measurement does not cover:

- Actual shape rendering through `egui::Painter`
- OS event handling
- Asynchronous worker startup overhead on the GUI side

---

## 2. Measurement Method

The CLI that was added:

```bash
cd code/rust_phase1
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_medium_2m_8ch.parquet
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet iu iv iw gate_pwm
```

Layout produced by `--bench-phase2`:

- If no channels are specified, all channels are targeted
- Channels are arranged three per row
- Channels whose names contain `pwm`, `gate`, or `step` are measured as Step; the rest are measured as Line
- 24 display ranges are evaluated
- The requested envelope bucket count is 1200
- The hover measurement uses 1000 positions

---

## 3. Target Datasets

| dataset | rows | channels | total channel points | notes |
|---|---:|---:|---:|---|
| `panely_medium_2m_8ch.parquet` | 2,000,000 | 8 | 16,000,000 | 100 kHz, about 20 s |
| `panely_large_10s_1mhz_9ch.parquet` | 10,000,000 | 9 | 90,000,000 | 1 MHz, about 10 s |
| `ripple_validation_2_4m.parquet` | 2,400,000 | 5 | 12,000,000 | 1 MHz, about 2.4 s, ripple envelope validation |

---

## 4. Summary of Results

| case | layout | time col read | total channel read | raw cache | RSS after load | RSS after bench | visible avg | visible max | hover |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| medium, all 8 ch | 3 rows / 8 ch | 0.040 s | 0.085 s | 76.3 MiB | 91.0 MiB | 91.6 MiB | 0.0080 s | 0.0221 s | 0.264 us/lookup |
| large, all 9 ch | 3 rows / 9 ch | 0.215 s | 0.941 s | 419.6 MiB | 434.9 MiB | 435.4 MiB | 0.0313 s | 0.0925 s | 0.439 us/lookup |
| large, representative 4 ch | 2 rows / 4 ch | 0.153 s | 0.236 s | 228.9 MiB | 244.1 MiB | 244.4 MiB | 0.0144 s | 0.0382 s | 0.504 us/lookup |

RSS barely increased even after display-data generation and the hover measurement. The added envelope cache contributed only about 0.5 MiB even for the all-9-channel large dataset.

---

## 5. Display-Data Generation

| case | ranges | buckets | avg | max | max draw points | max raw step samples |
|---|---:|---:|---:|---:|---:|---:|
| medium, all 8 ch | 24 | 1200 | 0.0080 s | 0.0221 s | 30,396 | 8,010 |
| large, all 9 ch | 24 | 1200 | 0.0313 s | 0.0925 s | 27,058 | 4,001 |
| large, representative 4 ch | 24 | 1200 | 0.0144 s | 0.0382 s | 15,148 | 4,001 |

For the all-9-channel large dataset, a wide display range corresponding to 5,000,000 samples peaked at 92.5 ms, while a range covering about 2,000,000 samples took roughly 32-33 ms. The mild stutter observed during rapid zooming in the GUI is explained by the cold multi-channel envelope generation time measured here.

In contrast, a range of about 500,000 samples completes in roughly 8.6-9.0 ms even across all 9 channels, and a range of about 100,000 samples drops further to roughly 1.5-1.9 ms.

---

## 6. Hover

| case | positions | lookups | hits | total | avg |
|---|---:|---:|---:|---:|---:|
| medium, all 8 ch | 1000 | 8000 | 8000 | 0.0021 s | 0.264 us/lookup |
| large, all 9 ch | 1000 | 9000 | 9000 | 0.0040 s | 0.439 us/lookup |
| large, representative 4 ch | 1000 | 4000 | 4000 | 0.0020 s | 0.504 us/lookup |

Hover value lookup is sufficiently lightweight and is not a primary bottleneck at this time.

---

## 7. Assessment

As a quantitative Phase 2 benchmark, we confirmed full-channel raw caching and multi-row display-data generation for 10M rows / 9 channels, totaling 90M channel points.

Assessment:

- Phase 2's basic viewer functionality has reached a level where the Rust version alone can support routine waveform inspection
- RSS for the all-9-channel large dataset is about 435 MiB, which is acceptable on a development machine with 16 GB-class memory
- Hover synchronization is not a performance risk
- Cases where a cold envelope for 5M samples across many channels is rebuilt on every rapid pan/zoom can exceed 16 ms/frame, producing mild stutter

Candidate optimizations to consider next:

- Envelope reuse or neighbor-range caching for the display range during pan/zoom
- Tile / LOD pyramid
- Earlier fallback decisions for wide-range step channels
- GUI-side measurement of `egui::Painter` shape generation time

The mitigation plan is documented in [rust_migration_phase2_performance_plan.md](rust_migration_phase2_performance_plan.md).

P2-12, the Phase 2 completion gate, is treated as measurement-complete. The optimizations above will be handled as Phase 2 optional work or as performance-improvement tasks before Phase 3.

---

## 8. Additional Measurement: Ripple Envelope Validation

On 2026-05-02 we added a dedicated dataset to verify whether fine ripples remain visible as a band, rather than being dropped, under the Line tile / min-max envelope.

Generation:

```bash
cd code
.venv/bin/python proto_3_1b/generate_ripple_validation_data.py --force
```

Output:

- `code/proto_3_1b/data/ripple_validation_2_4m.parquet`
- 2,400,000 rows / 5 channels
- 39,063,837 bytes
- sha256: `0125ad1bf06cee22a4281d0b86134f87b043a170f63181d17f77140b1289cc39`

channels:

- `ripple_low_amp_80kHz`
- `ripple_am_60kHz`
- `ripple_noise_70kHz`
- `ripple_bucket_near_2048samples`
- `ripple_on_sine_80kHz`

Rust schema / load verification:

- schema: 2,400,000 rows / 10 row groups / 5 numeric channels
- `ripple_low_amp_80kHz`
  - samples: 2,400,000
  - time range: `0.000000 .. 2.399999`
  - value range: `-0.024951 .. 0.024951`
  - full envelope: 4,096 buckets / bucket size 586 / 8,192 draw points

Phase 2 bench:

```bash
cd code/rust_phase1
cargo run --release -- --bench-phase2 ../proto_3_1b/data/ripple_validation_2_4m.parquet ripple_low_amp_80kHz ripple_am_60kHz ripple_noise_70kHz ripple_bucket_near_2048samples ripple_on_sine_80kHz
```

Results:

| case | layout | raw cache | RSS after bench | visible avg | visible max | tile hits/builds | hover |
|---|---:|---:|---:|---:|---:|---:|---:|
| ripple 5 ch | 2 rows / 5 line ch | 64.1 MiB | 79.5 MiB | 0.0040 s | 0.0250 s | 25 / 5 | 0.306 us/lookup |

Assessment:

- All 5 channels were measured as Line, so the Step path was not mixed in
- During the first wide-range view, Line tiles for all 5 channels were built, and subsequent wide ranges produced tile hits
- The low-amplitude 80 kHz ripple retains its value range as a band of roughly +/-0.025 in the full envelope
- Display-data generation in the bench is sufficiently lightweight, and the ripple-validation dataset can be reused for future visual GUI checks

---

## 9. Additional Measurement: Step Edge Cache / Dense LOD

On 2026-05-02 we added an edge cache and a dense fallback for Step channels.

Implementation:

- For each Step channel, a change-point list of `index/time/value` is built lazily
- For a given display range, the edge list is queried via binary search to extract the state just before the left edge and the edges that fall within the range
- If there are too many edges, we fall back to a Step-specific dense envelope
- Step dense envelopes do not use the Line tile LOD
- Step edge cache hits/builds, edge count, and edge memory were added to the frame timing and bench output

Verification:

```bash
cd code/rust_phase1
cargo run --release -- --bench-phase2 ../proto_3_1b/data/step_validation_100k.parquet
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet gate_pwm
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet
```

Results:

| case | layout | visible avg | visible max | edge hits/builds | edge count | edge memory | dense / exact behavior |
|---|---:|---:|---:|---:|---:|---:|---|
| `step_validation_100k.parquet` | 1 row / 3 step ch | 0.0001 s | 0.0005 s | 33 / 3 | - | 0.1 MiB | 1-sample lead/lag paths preserved |
| `gate_pwm` only | 1 row / 1 step ch | 0.0064 s | 0.0309 s | 8 / 1 | 400,000 | 9.2 MiB | dense for wide ranges, edge-step at the 100k-sample scale |
| large, all 9 ch | 3 rows / 8 line + 1 step | 0.0194 s | 0.1857 s | 8 / 1 | 400,000 | 9.2 MiB | Line tile + Step dense/edge mixing works |

Assessment:

- The 1-sample lead/lag verification on `step_validation_100k.parquet` is preserved
- For `gate_pwm`, the first call builds a cache of 400,000 edges; subsequent ranges can be extracted from the edge list
- Wide ranges fall back to the dense envelope, while ranges with few edges (around the 100k-sample scale) return to exact edge-step rendering
- For the all-9-channel large dataset, the first range incurs both Line tile build and Step edge cache build, so the max is somewhat higher, but the pan cache is sustained at avg 0.0079 s / max 0.0135 s
