---
project: Panel_y
doc_type: Phase 2 Performance Improvement Memo
target: Rust Migration Phase 2
created: "2026-04-30"
updated: "2026-05-01"
status: Phase A Implemented
source: "docs/rust_migration_phase2_benchmark_results.md"
implementation: "code/rust_phase1"
---

# Rust Migration Phase 2 Performance Improvement Notes

---

## 1. Purpose

This note organizes mitigation candidates, risks, alternatives, and ordering for the issue surfaced by the P2-12 quantitative benchmark: minor stutter during fast pan/zoom when displaying multiple channels of large data.

The targets are mainly:

- range cache
- tile / LOD
- earlier fallback decisions for Step over wide ranges
- other alternatives

---

## 2. Current Understanding

P2-12 results:

| case | layout | visible avg | visible max | RSS after bench | hover |
|---|---:|---:|---:|---:|---:|
| medium all 8ch | 3 rows / 8 ch | 8.0 ms | 22.1 ms | 91.6 MiB | 0.264 us/lookup |
| large all 9ch | 3 rows / 9 ch | 31.3 ms | 92.5 ms | 435.4 MiB | 0.439 us/lookup |
| large representative 4ch | 2 rows / 4 ch | 14.4 ms | 38.2 ms | 244.4 MiB | 0.504 us/lookup |

Assessment:

- Hover value lookup is sufficiently lightweight and is not the current primary bottleneck.
- RSS sits around 435 MiB even with the full per-channel raw cache for 10M rows x 9ch, so it is not the main near-term risk.
- The problem lies in the path that cold-generates display data for multiple channels over wide display ranges.
- For large data with all 9 channels, a wide range corresponding to about 5M samples reaches a max of 92.5 ms, exceeding the 16 ms/frame budget.
- The slight stutter observed during fast zoom in GUI testing can be explained by this visible trace generation time.

Characteristics of the current implementation:

- `visible_row_traces()` generates display data within the frame.
- Line builds a range-aware min/max envelope for each display range.
- Step tries `raw step` -> `change-point step` -> `envelope fallback` in order.
- The envelope cache assumes an exact range match.
- Because pan/zoom changes the x range and clears the cache, the current cache is ineffective for continuous pan/zoom.

---

## 3. Evaluation of Mitigation Candidates

| Mitigation | Expected effect | Risk | Priority | Notes |
|---|---|---|---:|---|
| GUI frame timing instrumentation | Identify the real bottleneck | Low | 1 | Verify whether anything beyond `visible_row_traces()`, such as `egui::Painter` shape generation and drawing, is congested. |
| Preview / debounce during interaction | Improves perceived UI responsiveness | Low to medium | 2 | While dragging/zooming, use lower buckets or the previous display, then regenerate precisely after the interaction stops. |
| Earlier Step wide-range fallback | Reduces wasted scans on Step channels | Low | 3 | Limited effect. For full 9ch over wide ranges, the dominant cause is more likely the Line envelope side. |
| Per-channel envelope parallelization | Effective for many channels over wide ranges | Medium | 4 | Memory bandwidth is the ceiling. Implementation is lighter than LOD. |
| Range cache improvements | Effective for returning to or near previously seen ranges | Medium | 5 | Exact LRU alone is weak for continuous interaction. Overscan cache is the real candidate. |
| tile / LOD | Large effect for wide-range Line display | High | 6 | Memory, build time, Step edge preservation, and cache invalidation are design risks. |
| Bucket-boundary sharing / batched generation | Moderate effect with many channels | Medium | Additional candidate | Avoid duplicating computation per channel over the same time/index range. |

---

## 4. Outlook and Risks for Each Option

### 4.1 GUI Frame Timing Instrumentation

Goal: measure on the GUI side

- `visible_row_traces()` generation time
- draw shape generation time
- total frame time
- the difference between active interaction and idle

Outlook:

- Low cost, and a prerequisite for subsequent mitigation choices.
- Reveals Painter-side cost that CLI benchmarks cannot see.

Risk:

- Almost none.
- Make sure the measurement display itself does not get in the way of the UI; treat it as a status indicator or a debug toggle.

Done criteria:

- For large all 9ch, the per-frame breakdown during pan/zoom is observable.
- It is clear whether `visible_row_traces()` or the Painter side dominates.

### 4.2 Preview / Debounce During Interaction

Proposal:

- Lower the bucket count during drag/zoom.
- Or hold the previous display data to keep the UI responsive, and regenerate at normal bucket count after the interaction stops.
- Thin out heavy regeneration while wheel/drag events are streaming.

Outlook:

- Most likely to improve perceived performance quickly.
- Even without LOD, we can achieve a "no freeze during interaction" state.

Risk:

- The waveform looks coarser only during interaction.
- Mistiming the switch back to the precise display after interaction stops causes flicker.
- If Cursor/analysis features are added in Phase 3, the design must avoid mixing up the on-screen preview and the raw data used for analysis.

Done criteria:

- For large all 9ch, UI interaction does not stall during fast pan/zoom.
- The normal-quality display returns after interaction stops.
- The preview state can be surfaced in the status area if needed.

### 4.3 Earlier Step Wide-Range Fallback Decision

Current state:

- For Step, when raw samples are many, change-point extraction is attempted.
- If there are too many change points, it falls back to the envelope.
- Displaying high-frequency PWM over a wide range scans a fair amount before the fallback triggers.

Proposal:

- If the display range has a large sample count and is at least as wide as a previously fallback-triggering span for the same channel, route it to the envelope early.
- Keep lightweight per-channel meta information on "step change density".
- For low-density and mid-zoom ranges, preserve the current edge-preserving behavior.

Outlook:

- Effective for wide-range display of Step channels.
- Implementation is relatively small.

Risk:

- Falling back too eagerly can lose edges that should be visible.
- Misestimating channel characteristics undermines the benefits of low-density Step.

Done criteria:

- Unnecessary change-point scans are reduced for wide-range display of `gate_pwm`.
- The 1-sample early/late verification with `step_validation_100k.parquet` is preserved.

### 4.4 Per-Channel Envelope Parallelization

Proposal:

- Parallelize envelope generation across multiple Line channels at the per-channel level.
- Restrict the scope initially to the display data generation path.
- Consider introducing `rayon` if necessary, but experimentation is possible with standard threads.

Outlook:

- May be effective when there are many channels, such as large all 9ch.
- Smaller design change than tile / LOD.

Risk:

- Memory bandwidth becomes the ceiling, so it does not scale to the number of CPU cores.
- Creating threads per frame is counterproductive.
- Shared cache updates and borrow design become more complex.

Done criteria:

- The wide-range visible max for large all 9ch drops noticeably.
- Per-frame alloc/thread overhead does not increase.

### 4.5 Range Cache Improvements

Current state:

- The envelope cache assumes an exact range match.
- Pan/zoom changes the range and clears the cache.
- Exact LRU alone is weak for the new ranges produced by continuous pan/zoom.

Proposal:

- Cache an overscan range slightly wider than the display range rather than caching exact ranges.
- During pan, skip re-extraction if the display range is within the overscan range.
- Align bucket density per zoom step, so nearby ranges land on the same cache.

Outlook:

- Effective for pan.
- Less effective for zoom, but stepping helps partially.
- Lower implementation difficulty than LOD.

Risk:

- Since bucket boundaries and the display range do not match exactly, the rendering may look slightly coarser.
- Sloppy cache key design balloons memory.
- If Y auto-range is derived from the overscan range, values outside the display range may pull it.

Done criteria:

- The number of visible-trace regenerations during pan decreases.
- Y auto-range retains the meaning of the display range.
- The cache has a memory cap.

### 4.6 tile / LOD

Proposal:

- Rather than a full tile pyramid, start with a coarse fixed-sample-width min/max tile built lazily.
- Target Line display.
- Treat Step separately, preferring raw / change-point in zoom ranges where edge preservation is required.

Outlook:

- Fundamental remedy for wide-range Line display.
- Most likely to address the 92.5 ms max for large all 9ch.

Risk:

- Large implementation scope.
- Tile build time and memory must be bounded.
- Cache invalidation on channel addition, file reload, style change, and range change is complex.
- Using only min/max tiles for Step signals makes edge positions ambiguous.

Done criteria:

- A 16 ms/frame target is realistic for wide-range display of large all 9ch.
- Tile memory is acceptable relative to the raw cache.
- The Step display's edge-preservation spec is not broken.

---

## 5. Recommended Order

### Phase A: Measurement and Low-Risk Mitigations

1. Add GUI frame timing instrumentation.
2. Add preview / debounce during interaction.
3. Bring forward the Step wide-range fallback decision.

Intent:

- Reduce perceived stutter first.
- Confirm the true dominant factor before moving on to LOD.

Pass criteria:

- The UI does not stall during fast pan/zoom for large all 9ch.
- It redraws at normal quality after interaction stops.
- Hover updates remain as lightweight as today.

### Phase B: Mid-Scale Optimization

4. Validate per-channel envelope parallelization.
5. Validate the overscan range cache.

Intent:

- Reduce cold generation time for wide range x multi channel.
- Reduce wasted regeneration during pan.

Pass criteria:

- The visible max for large all 9ch drops noticeably.
- RSS growth is controlled.
- No cache-induced display inconsistencies.

### Phase C: Introducing LOD

6. Introduce a lazily built coarse tile / LOD.
7. Add tile levels if necessary.

Intent:

- Reliably keep wide-range display of around 90M points lightweight.
- Decouple from Phase 3 onward analysis features and treat it as a drawing-only optimization.

Pass criteria:

- Wide-range display for large all 9ch is consistently lightweight.
- The UI does not freeze during tile construction.
- The raw-data-based analysis policy is preserved.

---

## 6. Options Not Pursued Right Now

### Exact LRU cache only

It helps with going back / resetting, but is weak because each continuous pan/zoom step yields a new range. If we do range caching, pair it with overscan or range stepping.

### Building a full tile pyramid from the start

The effect is large, but build time, memory, and the risks to the Step display spec are also large. Validate with a lazy coarse tile first.

### Always rendering Step as an envelope

It is faster, but breaks the edge-preservation value validated in Phase 2. Wide-range fallback is fine, but keep raw / change-point Step for low-density and mid-zoom ranges.

---

## 7. Conclusion at This Point

The first thing to tackle is not tile / LOD, but GUI frame timing instrumentation and preview / debounce during interaction.

Reasons:

- CLI benchmarks alone do not reveal the real Painter-side cost.
- The perceived problem is not only "computation is slow" but also "the UI thread is being blocked".
- Preview / debounce is smaller than LOD and lets us confirm the effect sooner.
- tile / LOD is a candidate root-cause remedy, but it is safer to introduce after Phase A/B clarify what is still missing.

Recommended order:

```text
GUI frame timing
  -> Preview / debounce during interaction
  -> Earlier Step wide-range fallback
  -> Channel parallelization / overscan range cache
  -> Coarse tile / LOD
```

---

## 8. Phase A Implementation Results

Date: 2026-05-01

Target implementation: `code/rust_phase1/src/main.rs`

Work done:

- Added GUI frame timing instrumentation. The top bar exposes frame / visible trace generation / draw shape generation / input handling / rows / channels / draw points / bucket count.
- Added a preview mode during pan/zoom interaction. It does not trigger for medium-grade data or small display ranges; only when there are at least 8M samples and the display load is large does it limit the bucket count to a quarter of normal (max 512) for the 140 ms following a range change.
- Brought the Step wide-range fallback forward. Per-channel records of change-point cap overruns let later requests with the same or greater span/sample count skip change-point scanning and go straight to the envelope.
- For wide-range Step during preview, if raw exact display does not fit, skip change-point scanning and fall back to the envelope display.

Verification:

```text
cargo check
cargo test
cargo run -- --bench-phase2 ../proto_3_1b/data/step_validation_100k.parquet
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_medium_2m_8ch.parquet
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet
```

Results:

- `cargo test`: 32 passed
- `step_validation_100k.parquet`: Step-related CLI benchmarks, including the 1-sample early/late check, pass.
- `panely_medium_2m_8ch.parquet` release: visible avg 0.0053 s, max 0.0161 s, hover avg 0.210 us/lookup
- `panely_large_10s_1mhz_9ch.parquet` release: visible avg 0.0283 s, max 0.0784 s, hover avg 0.352 us/lookup

Remaining tasks:

- Confirm frame timing during GUI interaction for large all 9ch.
- As Phase B, validate per-channel envelope parallelization and the overscan range cache.
