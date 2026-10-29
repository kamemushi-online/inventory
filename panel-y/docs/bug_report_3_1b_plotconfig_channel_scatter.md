# Bug Investigation Report: Channels Scattered After Loading a Plot Config in proto_3_1b

- Investigation date: 2026-04-16
- Target: `code/proto_3_1b/app.py`
- Symptom: When loading a `.pyc.json` that was saved with multiple channels on a single row, the overlay rendering breaks and channels appear scattered.

## Conclusion (root cause)

The primary cause is a **callback race during plot-config loading**.

Immediately after `load_plotconfig()` restores the waveform, an update to `scroll-zoom-store`
re-triggers `update_waveform_rows()`, which overwrites `waveform-container` again.
Because both callbacks write to the same Output, the later one wins and the display breaks.

## Evidence (code)

1. On restore, `load_plotconfig()` updates the following at the same time:
- `waveform-container` (direct restore render)
- `row-groups-store`
- `scroll-zoom-store`

Locations: `code/proto_3_1b/app.py:804-825`, `code/proto_3_1b/app.py:984-1002`

2. `update_waveform_rows()` takes `scroll-zoom-store` as an **Input** and writes to the same `waveform-container`

Location: `code/proto_3_1b/app.py:1196-1211`

3. `update_waveform_rows()` rebuilds the row groups from the `ALL` State of `row-dropdown` and overwrites the container

Locations: `code/proto_3_1b/app.py:1232-1244`, `code/proto_3_1b/app.py:1270-1292`

## How the layout breaks

- Even when `load_plotconfig()` restores the figures correctly, `update_waveform_rows()` runs next
- It rebuilds the rows from the current `ALL` State (the old UI state, or a remount-in-progress state), so the rendered layout differs from the saved row groups
- As a result, "multiple channels on one row" collapses and channels appear scattered

## Affected scope

- Waveform rendering when loading a plot config via `📂`
- Particularly visible when using the "multiple channels per row" pattern

## Additional note

`journal/2026-04-13.md` also records that pattern-matching callbacks are prone to firing as side effects on load (a related design risk).
