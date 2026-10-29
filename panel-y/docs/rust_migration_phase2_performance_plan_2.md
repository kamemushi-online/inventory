---
project: Panel_y
doc_type: Phase 2 Performance Plan, Part 2
target: Rust Migration Phase 2
created: "2026-05-02"
updated: "2026-05-02"
status: Phase 2 closeout complete
source: "docs/rust_migration_phase2_performance_plan.md"
implementation: "code/rust_phase1"
---

# Rust Migration Phase 2 Performance Improvement Plan, Part 2

---

## 1. Purpose

Switch the main strategy of Phase 2 performance improvement from interaction-time preview to "drawing optimization that does not hide events".

Assumed usage:

- Use events on the waveform as landmarks to find what to zoom into.
- Pan/zoom toward spikes, edges, steps, and outliers.
- If the appearance differs between interaction and idle, landmarks are easy to lose.

For this usage, a preview that lowers rendering quality only during interaction is a poor fit. From now on, the baseline is to make normal rendering itself lighter, so the same information remains visible both during and after pan/zoom.

### 1.1 Closeout Summary

As of 2026-05-02, Phase 2 performance improvement reaches a closing point.

Implemented:

- Demoting preview to a debug / emergency toggle
- Frame timing and cache statistics display
- Overscan range cache
- Bucket boundary sharing / batched generation
- Line lazy tile / LOD
- Ripple verification data and benchmarks
- Step edge cache / dense LOD

Deferred:

- Background build for Line tile / Step edge cache
- Multiple LOD levels
- Channel parallelization

Judgment:

- Response for large all 9ch on pan cache hits is good and does not block moving on to Phase 3.
- The initial build cost remains a known perceived risk, but we are not pursuing it further at this time.
- Going forward, the basis remains preserving min/max for Line and preserving edges for Step, rather than simple decimation.

---

## 2. Baseline Decisions

Policies we adopt:

- Do not adopt simple uniform-interval decimation.
- Adopt only information-preserving methods for reducing the number of draw points.
- For Line, use pixel-aware min/max envelope as the baseline.
- For Step / digital signals, preserve edges / change points as the baseline.
- Remove interaction-time preview from the main strategy. If kept, demote it to a debug / emergency toggle.

Policies we do not adopt:

- Drawing only one of every two points
- Drawing only a representative value every N samples
- Dropping the bucket count drastically only during interaction
- Always routing Step signals to a min/max envelope

Reasons:

- Uniform-interval decimation can erase short spikes and edges.
- Preview changes the appearance of landmarks between interaction and idle.
- For search-oriented usage, "events disappearing or being deformed" is a worse problem than some stutter.

---

## 3. Display Quality Requirements

### 3.1 Line

Requirements:

- When multiple samples fall within roughly 1 pixel, preserve min/max.
- Do not erase spikes, outliers, or steep peaks.
- Even when reducing the envelope bucket count, preserve min/max within each bucket.
- Take hover values from raw data, not from the rendering envelope.

Notes:

- A min/max envelope can preserve the existence of spikes, but cannot represent the exact timestamps within a bucket.
- For precise timing, zoom in or inspect raw data via hover / cursor features.

### 3.2 Step / Digital

Requirements:

- From low to medium density, preserve change points.
- At the left edge of the display range, carry over the state of the sample immediately before.
- At high density, where edges exceed pixel density, do not decimate edges at uniform intervals.
- At high density, consider dense rendering, edge density rendering, or a Step-specific LOD.

Notes:

- Drawing Step as a plain min/max envelope makes edge positions ambiguous.
- For dense signals such as PWM, in wide-range views it can be more accurate to show "density / occupied band" rather than individual edges.

---

## 4. Current Bottlenecks

Current implementation:

- `visible_row_traces()` generates display traces inside the frame.
- Line generates a min/max envelope per display range.
- Step uses `raw step` -> `change-point step` -> `envelope fallback`.
- The envelope cache is close to requiring an exact range match, and is ineffective for pan/zoom.

Problems:

- Cold generation is heavy for wide range x multi channel.
- Even a small range shift during pan triggers regeneration.
- Even after dropping the bucket count, the visible-sample scan cost in the Line envelope remains.
- Preview breaks visual consistency and conflicts with search-oriented usage.

---

## 5. Mitigation Policies

### 5.1 Change How Preview Is Handled

Policy:

- Remove preview from the main performance strategy.
- Default it to off, or demote it to a debug / emergency toggle.
- Keep normal quality for medium-grade data, small display ranges, and during event search.

Done criteria:

- Preview does not trigger during pan/zoom of medium-grade data.
- Existing basic operations work with preview off.
- The preview state does not mix into the UI unintentionally.

Implementation result:

- Set the default of `ViewState::large_preview_enabled` to `false`.
- Renamed the UI label to `Emergency preview`.
- Kept preview as a debug / emergency toggle rather than a main strategy.

### 5.2 Overscan Range Cache

Goal:

- Avoid regenerating the envelope every time the range shifts slightly during pan.
- Hold an envelope for a range wider than the display range, and reuse it as long as the display range is inside.

Design:

- Build a cache range that adds an overscan margin on each side of the view range.
- During pan, reuse the cache when the view range fits within the cache range.
- Fix the cache bucket time width or sample width so that bucket boundaries do not shift much during pan.
- When rendering, clip buckets outside the view range.

Notes:

- Compute Y auto-range from the actual view range, not the overscan range.
- Prevent overscan-induced outliers from pulling the Y axis.
- Cap cache memory.

Done criteria:

- Envelope regeneration count drops during pan for large all 9ch.
- The appearance does not switch between interaction and idle.
- Y auto-range is not pulled by values outside the display range.

Implementation result:

- Changed the envelope cache in `ChannelStore` to use a cache range with overscan.
- Reused the cache during pan when the view range is within the cache range.
- Clipped outside-of-overscan regions before drawing, and recomputed boundary buckets with raw values to avoid contaminating Y auto-range.
- Added a pan cache benchmark to `--bench-phase2`.

### 5.3 Bucket Boundary Sharing / Batched Generation

Goal:

- Avoid recomputing the same time/index range per channel.
- Reduce fixed-cost Line envelope generation for multi-channel display.

Design:

- Compute the sample index range from the view range once.
- Build bucket boundaries once.
- Each Line channel takes min/max over the same bucket boundaries.
- Share boundaries across rows and channels when the x range / bucket conditions match.

Done criteria:

- Visible trace generation time drops for medium / large multi-channel displays.
- The displayed point count and Y range meanings remain consistent with the current behavior.

Implementation result:

- Added `parquet_waveform::MinMaxEnvelopePlan`.
- Built the sample index range and bucket boundaries once from the view range and shared them across multiple Line channels.
- Gave `EnvelopeBucket` `start_index` / `end_index` so that overscan-cache clipping handles boundary buckets correctly.

### 5.4 Lazy Tile / LOD for Line

Goal:

- Avoid scanning raw samples widely every frame in 10M-class wide-range Line display.
- Reduce draw point count and scan volume while preserving min/max.

Design:

- Lazily build min/max tiles of fixed sample width per channel.
- Initially build only one or a few coarse levels.
- Choose between raw scan and tile scan based on the display range and pixel width.
- Keep tiles as a rendering-only cache, separate from raw data used for analysis.

Memory estimate:

- 10M samples at 256 samples/tile yields about 39,063 tiles.
- min/max as f32 x 2 is about 0.3 MiB/channel/level.
- Even at 9ch x multiple levels, this stays small relative to the raw cache.

Notes:

- For irregular time, allow looking up the time range from the tile index.
- Tile construction must not block the UI thread.
- Discard the tile cache on channel reload.

Done criteria:

- Visible generation time stays consistently lower for wide-range Line display of large all 9ch.
- Spikes / outliers do not disappear.
- RSS growth stays within acceptable bounds.

Implementation result:

- Added `parquet_waveform::LineTileCache`.
- Lazily built min/max tiles of `256 samples/tile` per channel.
- Used tile LOD only for wide-range Line display, and kept it separated from Step fallback.
- Verified through spike / ripple checks that the min/max envelope does not drop important components.
- Initial tile builds run synchronously, so background build is deferred.

### 5.5 Step Edge Cache / Dense LOD

Goal:

- Avoid running a change-point scan every time for wide-range Step display.
- Maintain the state changes needed for search, without uniformly decimating edges.

Design:

- Lazily build a change-point index/time/value list per Step channel.
- For a display range, binary-search the edge list to extract edges.
- When edge count significantly exceeds pixel density, switch to a dense display.
- Dense display does not "smooth away edges"; it indicates "this range has high-density transitions".

Candidates:

- High/low occupancy band
- Per-pixel edge density
- min/max ribbon + dense marker

Done criteria:

- The 1-sample early/late check with `step_validation_100k.parquet` is preserved.
- Wasted change-point scans for wide-range display of gate_pwm are reduced.
- Even in dense display, the view returns to accurate Step rendering in regions where edges are few.

Implementation result:

- Added an edge cache per Step channel to `ChannelStore`.
- The cache lazily builds the `index/time/value` change-point list on the first access.
- For a display range, the cache binary-searches the edge list to extract the state just before the left edge plus the edges inside the range.
- When the edge count exceeds `MAX_STEP_CHANGE_POINTS`, it falls back to a Step-specific dense envelope.
- The dense envelope does not use the Line tile LOD; it draws a min/max ribbon as a Step fallback.
- Added Step edge cache hit/build and edge memory to the frame timing / benchmark output.

### 5.6 Channel Parallelization

Positioning:

- An additional measure after cache / tile / boundary sharing.
- Validated only if cold generation remains.

Notes:

- Memory bandwidth tends to be the ceiling.
- Do not spawn threads per frame.
- Cache updates and borrow design become more complex.

Done criteria:

- The cold generation max for large all 9ch drops noticeably.
- No additional overhead on small / medium.

Closeout decision:

- Overscan cache, bucket boundary sharing, Line tile, and Step edge cache provided improvements sufficient for Phase 2, so channel parallelization is deferred.
- Revisit it if initial build cost or cold generation max becomes a real-world problem.

---

## 6. Recommended Order

```text
Set preview to default off / debug-only
  -> Add cache hit/miss to frame timing
  -> Overscan range cache
  -> Bucket boundary sharing / batched generation
  -> Line lazy tile / LOD
  -> Step edge cache / dense LOD
  -> Channel parallelization if necessary
```

What to do first:

1. Remove preview from the main path.
2. Reduce regeneration during pan with an overscan range cache.
3. Reduce duplicate computation across multi channels with bucket boundary sharing.

tile / LOD has a large effect but also large design impact, so introduce it after measuring the effect of the overscan cache and bucket boundary sharing.

---

## 7. Pass Criteria

Quality:

- Waveform landmarks do not shift between pan/zoom and idle.
- Spikes / edges / steps / outliers do not vanish under simple decimation.
- Hover values continue to be raw-data based.

Performance:

- medium all 8ch can be operated smoothly without preview.
- Frame stalls during wide-range pan/zoom of large all 9ch are reduced noticeably.
- The visible max for large all 9ch drops noticeably from P2-12's 92.5 ms.

Memory:

- Tile / cache growth is acceptable relative to the raw cache.
- Cache caps and eviction conditions are in place.

Regression:

- The Step check with `step_validation_100k.parquet` is preserved.
- Spikes do not disappear in `spike_event` or other spike-type channels.
- Y auto-range is not pulled by the overscan range.

---

## 8. Verification Plan

CLI:

```text
cargo check
cargo test
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_medium_2m_8ch.parquet
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet
cargo run --release -- --bench-phase2 ../proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet iu iv iw gate_pwm
```

GUI:

- medium all 8ch is smooth during pan/zoom without preview.
- Frame timing during wide-range pan/zoom for large all 9ch is observed.
- When zooming into spikes / edges as landmarks, the landmarks do not vanish between interaction and idle.

Additional measurements to add:

- Envelope cache hit/miss
- Overscan hit/miss
- Tile build time
- Tile hit/miss
- Visible trace generation breakdown: index search / bucket boundary / channel minmax / step edge extraction

---

## 9. Risks

Overscan range cache:

- Values originating from the cache range mixing into Y auto-range can break the appearance.
- Sloppy handling of bucket boundaries can make the waveform appear to jitter during pan.

Line tile / LOD:

- min/max preserves the existence of spikes, but not the exact timestamps within a bucket.
- Running tile construction on the UI thread freezes loading.

Step dense LOD:

- A poorly designed dense display can make digital signals look analog.
- Switching conditions between edge-density display and accurate edge display are required.

Parallelization:

- The effect may be limited by memory-bandwidth bound.
- More complex cache updates can cause inconsistencies.

---

## 10. Conclusion

At the Phase 2 closeout, the main strategy is fixed as follows, not preview:

```text
Line: keep min/max while making it lighter with cache / tile
Step: preserve edges, and only fall back to a dedicated display in dense regions
pan: avoid regeneration with an overscan cache
multi channel: reduce duplicate computation with bucket boundary sharing
```

We do not adopt simple decimation of draw points. When reducing the draw count, we adopt only forms that preserve the information needed for search, such as min/max, edges, and density.

Phase 2 performance improvement closes here. The remaining initial build cost is treated as a known deferred item; background build for Line tile / Step edge cache, multi-level LOD, and channel parallelization will be revisited if needed around Phase 3.
