---
project: Panel_y
doc_type: Phase 2.5 UI polish plan
target: Rust migration Phase 2.5
created: "2026-05-02"
updated: "2026-05-02"
status: Prioritizing low-risk load UI
implementation: "code/rust_phase1"
---

# Rust Migration Phase 2.5 — On-the-Ground Verification UI Polish Plan

---

## 1. Purpose

Before entering Phase 3 analysis features, we polish the UI so that on-the-ground verification cycles run efficiently.

Phase 2 reached a stopping point on display performance and basic viewer features. The next requirement is not to add Cursor / statistics / FFT, but to make the chain of actions — opening real data, choosing the needed channels, setting up the display state, and pan/zooming to inspect — fast, unambiguous, and easy to recover from when something goes wrong.

In Phase 2.5 we prioritize operational convenience for accumulating real-world experience. The first implementation target is limited to low-risk data-load UI improvements.

---

## 2. Baseline Decisions

What we prioritize:

- Enhanced UI around data loading
- State awareness during on-the-ground verification
- Easy recovery from failures
- Lightweight session presets

What Phase 2.5 will not do:

- Cursor A/B
- Range statistics
- FFT
- Full `.pyc.json` compatible save/load
- Background build of Line tiles / Step edge caches
- Formal plot config specification

What we defer:

- Wheel-operation separation
- Box zoom

Reasons:

- Before stacking analysis features, we need a smooth path to repeatedly open real logs and verify them
- If interaction is heavy, it becomes hard to gather on-the-ground feedback about performance, display quality, and UI
- Direct typing of parquet paths, the distance between channel selection and load operation, and the unclear load-state directly affect the iteration rate of on-the-ground verification
- Wheel-operation separation and box zoom are useful, but should come after the data-load UI is tidied up

---

## 3. Priority Tasks

### P2.5-01 Parquet selection UI

Goals:

- Avoid direct typing of parquet paths and lower the friction of opening real logs
- Make it easy to return to recently used files
- Catch path mistakes before schema load

Candidates:

- File picker / Browse button
- In-session history of recent paths
- Existence check and extension check on the Parquet path
- Persistent display and clear control for load errors

Completion criteria:

- A Parquet file can be chosen from a file picker or recent paths
- Non-existent paths, wrong extensions, and schema-load failures are clearly displayed
- The flow from path selection to schema load is intuitive

### P2.5-02 Schema / channel list UI

Goals:

- After schema load, quickly grasp the data overview and channel candidates
- Shorten the distance between channel selection and the load operation
- Make it easy to find the target channel even in real logs with many channels

Candidates:

- Summary display: rows / time range / estimated sample rate / numeric channel count
- Channel search/filter
- Per-channel `loaded` / `loading` / `unloaded` status display
- Direct `Load` / `Add to row` from the channel list
- Place the target row selection near the channel list
- Multi-channel selection and `Load checked`

Completion criteria:

- Selection, status check, and load/add all complete within the channel list
- Loaded channels are easy to reuse and add to a different row
- The differences among loading, loaded, and unloaded are visible in the UI

### P2.5-03 Low-risk plot helper: session preset

Goals:

- Make it possible to return to display states often used during on-the-ground verification
- Get convenience without committing to a formal plot config compatibility
- Avoid confusion with `.pyc.json` compatibility

Direction:

- Call it `View preset` or `Session preset`
- The save target is restricted to lightweight Rust-internal state
- Do not claim full `.pyc.json` compatibility
- On schema mismatch, restore only channels that can be restored and warn about the missing ones

Save target candidates:

- parquet path
- row composition
- channel paths
- Line / Step
- visible
- color override
- line width
- Y Auto / Manual range
- X range

Completion criteria:

- The current display state can be saved as a session preset
- Major display states can be restored on the same schema
- The UI clearly distinguishes session presets from `.pyc.json` compatibility / formal plot config

### P2.5-04 Loaded cache operations

Goals:

- During on-the-ground verification, allow swapping channels while keeping a handle on memory state

Candidates:

- Make the loaded-channel list easy to read
- Per-channel unload
- Unload all channels
- Keep displaying cache memory
- Make it easy to add already-loaded channels to a different row

Completion criteria:

- Unused channels can be removed during trial-and-error to keep state tidy
- The difference between cache reuse and unload is clear

---

## 4. Recommended Order

```text
P2.5-01 Parquet selection UI
  -> P2.5-02 Schema / channel list UI
  -> P2.5-03 Low-risk plot helper: session preset
  -> P2.5-04 Loaded cache operations
  -> P2.5-later Wheel-operation separation
  -> P2.5-later Box zoom
```

The top priority is the data-load UI. Wheel-operation separation and box zoom are useful for tracing waveforms during on-the-ground verification, but are excluded from this first implementation target and handled after the load UI is tidied.

---

## 5. Conditions for Moving to Phase 3

- Opening real logs, checking schemas, and loading multiple channels are not practically stressful
- Load/add is available from the channel list, and the loading / loaded / unloaded states are visible
- A low-risk session preset can restore the display state used during on-the-ground verification
- Loading, panning, zooming, row addition, and channel addition are all manually verified in the GUI on representative data

Once these are met, we proceed to Phase 3 — early porting of the plot config.
