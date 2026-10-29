# Rust Migration Phase 1 — Benchmark Results

## Scope

Using the vertical-slice prototype in `code/rust_phase1`, we measured Parquet loading, full-range envelope generation, and visible-range envelope re-extraction for a single-channel display.

The following GUI operations were verified manually:

- Drag: pan
- Wheel / pinch: zoom centered on the cursor position
- Horizontal scroll: pan
- Double click or Reset X Range: full view

## Measurement Method

We used the Rust release-build CLI benchmark.

```bash
cd code/rust_phase1
cargo run --release -- --bench-channel ../proto_3_1b/data/panely_medium_2m_8ch.parquet sine_50Hz
cargo run --release -- --bench-channel ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet iu
cargo run --release -- --stress-channel ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet iu 1000
```

`--bench-channel` measures the following:

- Time to read only `time` and the selected channel from Parquet
- Time to extract the full-range min/max envelope
- Time to re-extract min/max envelopes for 24 different visible ranges corresponding to pan/zoom operations

The visible-range envelopes were requested with 1200 buckets, matching the GUI display width.

`--stress-channel` repeats the same 1200-bucket visible-range envelope re-extraction for the specified number of iterations, recording RSS at each block.

## Results

| dataset | channel | rows | array memory | read | full envelope | visible envelope 24 runs | avg | max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `panely_medium_2m_8ch.parquet` | `sine_50Hz` | 2,000,000 | 22.9 MiB | 0.039 s | 0.003 s | 0.012 s | 0.0005 s | 0.0013 s |
| `panely_large_10s_1mhz_9ch.parquet` | `iu` | 10,000,000 | 114.4 MiB | 0.278 s | 0.013 s | 0.059 s | 0.0025 s | 0.0065 s |

## Stress Results

We re-extracted visible-range envelopes 1000 times on `panely_large_10s_1mhz_9ch.parquet` / `iu`.

| runs | wall | measured total | avg | max | RSS before load | RSS after load | RSS after full envelope | RSS after stress |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1000 | 3.179 s | 3.160 s | 0.0032 s | 0.0090 s | 9.0 MiB | 130.0 MiB | 130.0 MiB | 130.0 MiB |

RSS block samples:

| completed runs | wall | RSS |
|---:|---:|---:|
| 200 | 0.635 s | 130.0 MiB |
| 400 | 1.271 s | 130.0 MiB |
| 600 | 1.903 s | 130.0 MiB |
| 800 | 2.540 s | 130.0 MiB |
| 1000 | 3.175 s | 130.0 MiB |

RSS did not grow during continuous re-extraction.

## GUI Subjective Evaluation

We loaded `panely_large_10s_1mhz_9ch.parquet` in the GUI, and the perceived latency from pressing `Load Selected Channel` to seeing the waveform was roughly 0.5 s. Compared to the smaller datasets we have used so far, the time to first display is noticeably longer.

On the other hand, interaction after the waveform was displayed felt good.

- Panning kept up without issues
- Zoom operations worked correctly
- X range reset worked correctly
- For waveform inspection tasks, the interaction is practically stress-free

A first-display latency of around 0.5 s is roughly consistent with the 0.278 s CLI load time plus the GUI's initial envelope extraction and render update, so we consider it within an acceptable range.

## Large-Dataset Details

`panely_large_10s_1mhz_9ch.parquet` / `iu`

- samples: 10,000,000
- time range: `0.000000 .. 9.999999`
- projected columns: `[0, 1]`
- full envelope: 4096 buckets requested, 4096 buckets built, bucket size 2442, draw points 8192
- visible envelope: 1200 buckets requested
- visible envelope draw points: approximately 2400

Per-range re-extraction took at most 0.0065 s even on the wide 5,000,000-sample range. For narrow ranges around 100,000 samples it was roughly 0.0001-0.0002 s.

## Assessment

P1-08 and P1-09 are complete.

- X-axis pan/zoom has been verified manually in the GUI
- Range-aware min/max envelope re-extraction has been verified via the CLI benchmark
- The number of draw points scales with the requested bucket count rather than the source sample count of the visible range
- For 10M-row data, pan/zoom-equivalent processing after loading is sufficiently light
- RSS did not grow across 1000 visible-range re-extractions
- For large-data GUI interaction, the post-first-display experience is good

The main Phase 1 risk verification is complete. Next, as part of the Phase 1 evaluation, we will summarize the go/no-go decision and the implementation policies to carry into Phase 2.
