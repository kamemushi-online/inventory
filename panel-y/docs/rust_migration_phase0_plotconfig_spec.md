---
project: Panel_y
doc_type: Plot Config Spec
target: Rust Migration Phase 0
created: "2026-04-27"
updated: "2026-04-27"
status: Complete
source_proto: "archive/proto_3_1b"
---

# Rust Migration Phase 0 Plot Configuration Specification

---

## 1. Purpose

The Rust version must read the `.pyc.json` files written by the current `proto_3_1b` as a compatibility target.
This document freezes the current format and defines the compatibility policy for the Rust version.

---

## 2. Current Format

The current save routine is `save_plotconfig()` in `archive/proto_3_1b/app.py`.

Top-level structure:

```json
{
  "version": 1,
  "data_path": "/path/to/data.parquet",
  "rows": [],
  "ch_styles": {},
  "theme": "dark",
  "scroll_zoom": false
}
```

---

## 3. Top-Level Fields

| Key | Type | Required | Current default | Description |
|------|----|------|----------------|------|
| `version` | number | Required | `1` | Version of the plot configuration format |
| `data_path` | string | Required | `""` | Path to the Parquet file to load |
| `rows` | array | Required | `[]` | Row composition |
| `ch_styles` | object | Required | `{}` | Per-channel color and line width |
| `theme` | string | Optional | `"dark"` | `"dark"` or `"light"` |
| `scroll_zoom` | boolean | Optional | `false` | Equivalent to Plotly's scrollZoom |

Rust loader policy:

- If `version` is missing, it may be treated as `1`
- If `data_path` is empty, does not exist, or is not a Parquet file, raise an explicit error
- Unknown keys are ignored
- For type mismatches, fall back on the safe side and drop only the unrecoverable fields

---

## 4. The `rows` Field

Structure of each row:

```json
{
  "channels": ["sine_50Hz", "noise"],
  "ymin": null,
  "ymax": null,
  "step_channels": ["pwm_1kHz"]
}
```

| Key | Type | Required | Description |
|------|----|------|------|
| `channels` | string array | Required | Channels overlaid in this row |
| `ymin` | number/null | Optional | Lower Y-axis bound |
| `ymax` | number/null | Optional | Upper Y-axis bound |
| `step_channels` | string array | Optional | Channels rendered as a step plot |

Rust loader policy:

- Ignore channels that are not present in the Parquet file
- Exclude any row whose `channels` becomes empty from display
- Ignore channels that are listed in `step_channels` but not in `channels`
- If `ymin > ymax`, disable that range specification
- Allow `ymin`-only or `ymax`-only specifications

---

## 5. The `ch_styles` Field

Structure:

```json
{
  "sine_50Hz": {
    "color": "#1f77b4",
    "width": 1
  }
}
```

| Key | Type | Required | Description |
|------|----|------|------|
| `color` | string | Optional | `#RRGGBB` format by default |
| `width` | number | Optional | Line width. Current UI: 0.5 to 5, step 0.5 |

Rust loader policy:

- Invalid colors fall back to the default color
- Invalid line widths fall back to `1`
- Styles for channels that do not exist in the Parquet file may either be retained or ignored
- On save, it is acceptable to save styles only for channels currently in use

---

## 6. Theme

Current values:

- `dark`
- `light`

Rust policy:

- Make `dark` / `light` compatibility-target values for loading
- Unknown values fall back to `dark`
- Even if the Rust version adds new themes, keep the compatibility field intact on save

---

## 7. scroll_zoom

Current semantics:

- `false`: Scroll zoom disabled. UI label reads `スケール固定` (scale locked)
- `true`: Scroll zoom enabled. UI label reads `スケール解除` (scale unlocked)

Rust policy:

- Even if the input-device interaction is redesigned, the setting value is still loaded
- Whether the Rust UI uses the same labels is decided in Phase 2

---

## 8. Sample

```json
{
  "version": 1,
  "data_path": "/Users/user/panel-y/data/test_100k.parquet",
  "rows": [
    {
      "channels": ["sine_50Hz", "chirp_1_500Hz"],
      "ymin": -12,
      "ymax": 12,
      "step_channels": []
    },
    {
      "channels": ["pwm_1kHz"],
      "ymin": -1,
      "ymax": 6,
      "step_channels": ["pwm_1kHz"]
    }
  ],
  "ch_styles": {
    "sine_50Hz": {"color": "#1f77b4", "width": 1},
    "chirp_1_500Hz": {"color": "#ff7f0e", "width": 1},
    "pwm_1kHz": {"color": "#2ca02c", "width": 1.5}
  },
  "theme": "dark",
  "scroll_zoom": false
}
```

---

## 9. Rust Save Format

Up through Phase 4, prioritize compatibility with the current format.

When Rust-specific fields become necessary:

- Bump to `version: 2`
- Continue to support reading `version: 1`
- Place added fields at the top level or under a `rust` namespace
- Do not persist rendering caches or LOD information that compatibility does not require

Future candidates:

- Switch the extension to `.panely.json`
- Treat `.pyc.json` as a read-only compatibility format

---

## 10. Compatibility Test Items

- Configurations saved by the current Dash version load successfully in the Rust version
- Configurations saved by the Rust version reload successfully in the Rust version
- Configurations that include nonexistent channels do not crash the app
- Configurations that include empty rows do not crash the app
- If `data_path` does not exist, an explicit error is shown
- Invalid theme values do not crash the app
- Invalid color values do not crash the app
- Invalid line-width values do not crash the app
