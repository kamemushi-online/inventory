# Review Request: Integrating the Plot-Config File Feature into proto_3_1b

- **Date**: 2026-04-13
- **Branch**: `proto_3_1b`
- **Target file**: `code/proto_3_1b/app.py` (+392 lines / -9 lines)
- **Review focus**: validity of the feature merge, possible bug introduction, consistency with proto_3_1b-specific processing

---

## 1. Background and Purpose

proto_3_1 and proto_3_1b are sibling branches that diverged from the same codebase.

| Branch | Added features |
|----------|----------|
| proto_3_1 | Plot-config save/load (.pyc.json) + 2 bug fixes |
| proto_3_1b | Large-dataset support (minmax vectorize, searchsorted, float32, df_overview) |

The goal is to merge both feature sets into proto_3_1b — i.e., fold the proto_3_1 code into 3_1b.

---

## 2. List of Changes

### 2.1 New additions (ported from proto_3_1)

| # | Change | Lines (approx.) | Description |
|---|------|-----------|------|
| A1 | `import json` | 1 line | Required for reading/writing plot-config files |
| A2 | `list_config_suggestions()` | 40 lines | Generates path-completion suggestions for .pyc.json files |
| A3 | Layout: plot-config UI | 25 lines | config-path-input, save (💾) button, load (📂) button, config-status |
| A4 | `suggest_config_files()` callback | 15 lines | Displays file suggestions in response to config-path-input |
| A5 | `on_config_suggestion_click()` callback | 10 lines | Updates config-path-input on suggestion click |
| A6 | `save_plotconfig()` callback | 55 lines | Saves row layout, styles, theme, etc. as JSON |
| A7 | `load_plotconfig()` callback | 160 lines | Restores everything from JSON in one shot (data load + UI + waveform render) |

### 2.2 Modifications to existing code

| # | Change | Description |
|---|------|------|
| B1 | `make_ch_settings()` signature change | Added `ch_styles: dict \| None = None` argument. Applies saved color and line width |
| B2 | Added `Output` to `load_file()` | Returns a default plot-config path (`{stem}.pyc.json`) to `config-path-input`. Return values: 7 → 8 |
| B3 | `manage_rows()` spurious-trigger guard | Added a check that the delete button's `n_clicks` is set |
| B4 | `update_waveform_rows()` style propagation fix | `make_ch_settings(used_chs)` → `make_ch_settings(used_chs, ch_styles)` |

### 2.3 Aligning with proto_3_1b-specific processing (adaptations during the port)

The data-loading logic inside `load_plotconfig()` was adapted to the proto_3_1b spec:

| Item | proto_3_1 (source) | proto_3_1b (after merge) |
|------|-------------------|---------------------|
| Global variables | `df`, `channels` | `df`, `df_overview`, `time_arr_cache`, `channels` |
| Type cast | none | downcast to float32 |
| Time axis | `.values` each time | cached in `time_arr_cache` |
| Non-monotonic time | unsupported | sort-corrected via `np.argsort` |
| Downsampling | none | thinned to `OVERVIEW_POINTS` → `df_overview` |
| Status display | basic info only | also displays memory usage (MB) |
| Cleanup on error | resets only `df`, `channels` | also resets `df_overview`, `time_arr_cache` |

---

## 3. Points That Need Special Attention in Review

### 3.1 Consistency of `load_plotconfig()` (highest priority)

- Replicates the same data-initialization logic as proto_3_1b's `load_file()` inside `load_plotconfig()`
- This is duplicated code, but the two callbacks have different Inputs/Outputs, making sharing difficult
- **Please check**: no missed initialization for float32 cast, time_arr_cache, or df_overview

### 3.2 Spurious-trigger prevention in `manage_rows()`

- Workaround for a known bug: when `load_plotconfig` updates `rows-container`, the pattern-matching callback's `row-delete` Input changes and falsely triggers `manage_rows`
- **Please check**: whether the `not any(n for n in (delete_clicks or []) if n)` guard is sufficient, and that it does not block legitimate delete operations

### 3.3 Backward compatibility of `make_ch_settings()`

- The new argument keeps backward compatibility through its default value `ch_styles=None`
- Calls from `load_file()` continue to use `make_ch_settings(initial_channels)` (no styles)
- Calls from `update_waveform_rows()` and `load_plotconfig()` pass `ch_styles`
- **Please check**: when called without the argument, `saved.get()` returns an empty dict, which should be fine — please confirm

### 3.4 Return-tuple length

- `load_file`: 7 → 8 elements (`config-path-input` added)
- `load_plotconfig`: 17 elements
- **Please check**: every error path returns the right number of `no_update` values to match the Output count

---

## 4. Intentionally Untouched

- `assets/style.css`: the styles needed for the plot-config UI are already covered by existing classes (`input-path`, `btn`, `btn-accent`, `status-text`, `suggestion-item`)
- Anything under `utils/`: unchanged
- `generate_test_data.py`, `multiconverter.py`: unchanged

---

## 5. Test Plan

| # | Test | Procedure |
|---|--------|------|
| T1 | Normal data load | Load Parquet → confirm config-path-input is populated with the default path |
| T2 | Save plot config | After changing row layout and styles, click 💾 → confirm the .pyc.json is generated correctly |
| T3 | Load plot config | Click 📂 → confirm data + row layout + styles + theme are all restored in one shot |
| T4 | Delete a row after load | Confirm that row 1 is not removed immediately after load (verifies the spurious-trigger guard) |
| T5 | Change styles after load | Change line width → click Update → values must not revert |
| T6 | Load plot config with large data | With multi-million-point data, load the plot config → confirm df_overview is built and the app does not hang |
| T7 | Plot-config path completion | Type into config-path-input → confirm .pyc.json candidates are shown |




---

I performed a static review of `code/proto_3_1b/app.py` along the lines of the review request.
(No execution or testing was done.)

Findings
1) `update_waveform_rows()` return-value count is inconsistent with its Output definition
`update_waveform_rows()` defines three Outputs, but in the `if not row_groups or df is None:` branch it returns only two elements (around lines 1228–1232 of app.py).
When this branch is taken, Dash raises a callback exception (return-length mismatch).

Recommended task
Fix `update_waveform_rows` so every branch returns three elements.

Modify `update_waveform_rows` in `code/proto_3_1b/app.py`.

1. Confirm the `@app.callback` defines three Outputs (`waveform-container`, `row-groups-store`, `ch-settings-container`).
2. Make the `if not row_groups or df is None:` branch return three elements.\
   For example: `(empty-state message Div, [], no_update)` or `(empty-state message Div, [], [])`.
3. Audit the other branches of the same function and confirm every return is a 3-tuple.
4. To prevent recurrence, add a comment stating the intent "Output count = 3".



2) `load_plotconfig()` has no exception handling around data loading, leading directly to UI errors
In `load_plotconfig()`, `pd.read_parquet(data_path)` is not protected by try/except (around line 837).
If `data_path` points to a corrupted file, lacks permissions, or is wrong (including the case where `data_path` is empty and becomes `Path(".")`), the callback simply crashes.

Recommended task
Strengthen `read_parquet` and `data_path` validation in `load_plotconfig`.

In `load_plotconfig` in `code/proto_3_1b/app.py`, add `data_path` validation and recovery on load failure.

1. Immediately after obtaining `data_path_str`, check for empty strings; if unset, return an explicit error to `config-status`.
2. Verify not just `data_path.exists()` but also `is_file()` and `.suffix == ".parquet"`.
3. Wrap `pd.read_parquet(data_path)` in try/except, and on failure return an error tuple of length 17 to keep Output count consistent.
4. On failure, decide and apply a consistent policy: either reset global state (`df`, `df_overview`, `time_arr_cache`, `channels`) to a safe baseline, or keep existing state (and document the chosen policy).



3) Sampling-frequency computation in `load_file()` / `load_plotconfig()` breaks on single-point data
Both functions assume `ts = df["time"].iloc[1] - df["time"].iloc[0]`, which raises `IndexError` when the row count is ≤ 1 (around line 868 in `load_plotconfig` and line 1045 in `load_file`).
Also, `1/ts` is invalid when `ts == 0`.

Recommended task
Make the status display robust against insufficient rows and zero step in the time column.

Harden the frequency-display calculation in `load_file` and `load_plotconfig` in `code/proto_3_1b/app.py`.

1. Verify `len(df) >= 2` before computing `ts`.
2. When `len(df) < 2` or `ts == 0`, fall back to displaying Hz as `"N/A"` (or similar).
3. Centralize the `load_status` string format so the two sites cannot diverge if only one is updated.
4. If appropriate, add a `time`-column existence check and return an explicit error when it is missing.
