---
project: Panel_y
doc_type: Rock Industrial Mode Work Plan
target: Rust viewer visual mode
created: "2026-05-31"
updated: "2026-05-31"
status: draft
source_design: "docs/rock_industrial_mode_design.md"
implementation: "code/rust_phase1"
---

# Rock Industrial Mode Work Plan

---

## 1. Purpose

`docs/rock_industrial_mode_design.md` に基づき、Rust viewer に内蔵 `Plain` / `Rock` 表示モードを追加するための工程表を定義する。

この工程表では、外部 skin ロード、画像テクスチャ、procedural texture、大規模な独自 widget 化は扱わない。First Cut では、表示モード状態、Rock palette、global visuals、waveform grid、About dialog を完成させる。

---

## 2. Milestone Overview

```text
M0 Preparation
  -> M1 State model and selector
  -> M2 Palette and global visuals
  -> M3 Waveform plot grid
  -> M4 About dialog
  -> M5 Plot config persistence and polish
```

M1 から M4 までが First Cut の必須範囲である。M5 は First Cut 後に続けて実施する推奨範囲とする。

---

## 3. M0 Preparation

Goal:

- 現行テーマ実装の変更点を最小化できる編集範囲を確定する
- Rock mode で触る描画箇所を明確にする

Tasks:

- `ThemeMode`, `ViewState`, `draw_app_brand()`, `draw_plot_frame()` の現状確認
- FFT plot frame が waveform plot frame とどの程度共通化できるか確認
- Plot config の `theme` 保存・復元箇所を確認
- Visual verification に使う dataset を決める

Deliverables:

- 実装対象関数の短いメモ
- Visual check 用の起動手順

Completion criteria:

- M1 以降で編集するファイルと関数が明確
- `journal/` に触らない運用が確認済み

Expected files:

- `code/rust_phase1/src/main.rs`
- `code/rust_phase1/src/plot_config.rs`
- `docs/rock_industrial_mode_design.md`

---

## 4. M1 State Model and Selector

Goal:

- `Plain Dark` / `Plain Light` / `Rock` を UI から切り替えられるようにする
- Rock mode の状態を loaded data, row layout, cache と独立させる

Tasks:

- `VisualMode` を追加する
- `ViewState` に `visual_mode` を追加する
- `Default` では `theme = Dark`, `visual_mode = Plain` とする
- View section の `Theme` selector を combined selector に変更する
- UI 表示ラベルを以下にする
  - `Plain Dark`
  - `Plain Light`
  - `Rock`
- `Rock` 選択時は内部的に `theme = Dark`, `visual_mode = Rock` にする

Deliverables:

- 表示モードを切り替える UI
- `Rock` 選択状態を保持する `ViewState`

Completion criteria:

- `Plain Dark` / `Plain Light` / `Rock` を切り替えられる
- 切り替えで dataset, loaded channels, rows, x range が reset されない
- 既存の dark / light 操作が退化しない

Verification:

- `cargo fmt`
- `cargo test`
- Manual: default dataset を読み、チャンネル追加後に 3 モードを切り替える

Risk:

- `ThemeMode` と `VisualMode` の責務が混ざる

Mitigation:

- UI は combined selector、内部状態は `ThemeMode + VisualMode` に分ける

---

## 5. M2 Palette and Global Visuals

Goal:

- Rock mode の global egui visuals を適用する
- UI 全体に dark hardware-like な基調と cyan accent を与える

Tasks:

- `VisualPalette` または同等の palette helper を追加する
- `plain_palette()` / `rock_palette()` を用意する
- `apply_visuals()` を追加または `ThemeMode::visuals()` 呼び出しを拡張する
- Rock visuals で以下を調整する
  - app background
  - panel fill
  - text color
  - selection fill / stroke
  - widgets inactive / hovered / active
  - warning color
- `draw_app_brand()` を Rock aware にする

Deliverables:

- Rock mode の app-wide base styling
- cyan accent の brand / selection / widget styling

Completion criteria:

- Rock mode 選択時に left panel, top bar, controls の色が変わる
- Button / input / combo box が視認しやすい
- Plain mode の見た目が大きく変わらない

Verification:

- `cargo fmt`
- `cargo test`
- Manual: left panel を狭めても text / input / button が読める

Risk:

- global visuals の変更で細部の contrast が落ちる

Mitigation:

- text primary / secondary を強めに設定する
- disabled 色は低すぎない値にする

---

## 6. M3 Waveform Plot Grid

Goal:

- 実用的で見やすい Rock mode 用 grid を waveform plot に適用する
- 画像案の青系雰囲気は色で表現し、透視・歪み・texture は入れない

Tasks:

- `draw_plot_frame()` を palette-aware にする
- Plot background を描画する
- minor grid を 10 分割で描画する
- major grid を 5 分割または中央線で強調する
- plot border を Rock accent にする
- `draw_selected_plot_frame()` の Rock selected stroke を調整する
- Trace 色との競合を確認する

Deliverables:

- Rock mode の深い青黒 plot background
- cyan 系の実用 grid
- 見やすい selected row frame

Completion criteria:

- cyan / green / yellow / magenta traces が grid より明確
- axis labels が読める
- hover line / cursor line / selected frame が grid と区別できる
- 複数 row でも視覚的にうるさくない

Verification:

- `cargo fmt`
- `cargo test`
- Manual:
  - `test_100k.parquet`
  - 4 traces 以上
  - 1 row / multiple rows
  - zoom / pan / hover

Risk:

- grid が強すぎて波形を邪魔する

Mitigation:

- minor grid alpha を低くする
- major grid は線幅ではなく alpha で控えめに強調する

---

## 7. M4 About Dialog

Goal:

- 山本製作所クレジットを常時表示せず、About から確認できるようにする
- パンクな要素を操作 UI から分離する

Tasks:

- `PanelYApp` に `show_about` を追加する
- Top bar に `About` button を追加する
- `draw_about_dialog()` を追加する
- About dialog に以下を表示する
  - `YAMAMOTO Mfg.`
  - `山本製作所`
  - `EST. 197X`
  - `NO FUTURE!`
  - `当製作所ではDIY精神と反体制的なカオスなアプリケーション開発を推進しています`
- 金属銘板風の flat style を painter / frame で表現する
- corner screw marks は small circles で表現する

Deliverables:

- Top bar の `About` button
- About dialog / modal
- 山本製作所クレジットと説明文

Completion criteria:

- About は button click で開く
- close できる
- About を閉じた後に通常操作へ戻れる
- 操作画面にクレジットが常時表示されない

Verification:

- `cargo fmt`
- `cargo test`
- Manual: open / close / mode switching / normal waveform interaction

Risk:

- About button が top bar の status 表示を圧迫する
- GUI 確認の結果、右寄せ配置では status / perf 表示が長い場合に About button が見切れる

Mitigation:

- About button は `Panel-y` brand / version の直後に置く
- status text を wrap させない
- この調整は M1-M4 完了後の polish として扱い、M5 plot config persistence とは分離する

---

## 8. M5 Plot Config Persistence and Polish

Goal:

- Rust plot config で Rock mode を保存・復元できるようにする
- Plain 互換と `.pyc.json` version 1 互換を保つ

Tasks:

- `RustPlotConfigExt` に `visual_mode` を追加する
- JSON read で `rust.visual_mode` を読む
- JSON write で `rust.visual_mode` を保存する
- top-level `theme` は `dark` / `light` のまま維持する
- Rock mode 保存時は `theme: "dark"` と `rust.visual_mode: "rock"` にする
- invalid value は Plain fallback と warning にする
- Unit tests を追加する

Deliverables:

- Plot config の Rock mode persistence
- Backward-compatible read path
- Tests

Completion criteria:

- 古い config で Plain として開ける
- Rock で保存した config を Rock として復元できる
- invalid `visual_mode` で panic しない
- top-level `theme: "rock"` を作らない

Verification:

- `cargo fmt`
- `cargo test`
- `cargo check`
- Manual: save config -> restart -> load config

Risk:

- Dash 互換 top-level `theme` を壊す

Mitigation:

- Rock は Rust-specific namespace のみに保存する
- Existing `theme` parser を壊さず拡張する

---

## 9. Release Gate

First Cut release gate:

- M1 complete
- M2 complete
- M3 complete
- M4 complete
- `cargo fmt` complete
- `cargo test` complete
- Manual visual check complete

Full design release gate:

- First Cut release gate
- M5 complete
- `cargo check` complete
- Plot config save/load manual check complete

---

## 10. Suggested PR Split

PR 1: State and selector

- M1 only
- Smallest behavior change
- Easy rollback

PR 2: Rock visual base and plot grid

- M2 + M3
- Main visual review target

PR 3: About dialog

- M4
- Isolated UI addition

PR 4: Plot config persistence

- M5
- Includes tests

If speed matters more than review granularity, PR 1 and PR 2 can be merged into one implementation branch, but M5 should remain separate until the visual behavior is accepted.

---

## 11. Deferred Items

Do not include these in the initial Rock mode work:

- External skin loading
- Image-based metal plate or background
- Procedural scratch/noise texture
- Perspective grid
- Rotary knob widgets
- Toggle switch widgets
- Custom font packaging
- Animated glow
- Per-user skin directory

These can be revisited only after the built-in Rock mode is stable and useful.

---

## 12. Progress

### 2026-05-31 M0 Preparation

Status: Completed.

Checks performed:

- Confirmed current theme state is `ThemeMode::{Dark, Light}` in `code/rust_phase1/src/main.rs`.
- Confirmed `ViewState` currently stores `theme: ThemeMode` but has no separate visual/skin mode.
- Confirmed app-wide visuals are applied each frame through `ui.ctx().set_visuals(self.view.theme.visuals())`.
- Confirmed the current View section exposes a `Theme` ComboBox backed directly by `ThemeMode::ALL`.
- Confirmed `draw_app_brand()` is already theme-aware and is the right entry point for Rock-aware brand colors.
- Confirmed waveform grid is drawn in `draw_plot_frame()` with `visuals.faint_bg_color` and no explicit plot background fill.
- Confirmed selected waveform frame is drawn separately in `draw_selected_plot_frame()` with `visuals.selection.stroke.color`.
- Confirmed FFT plot frame is separate in `code/rust_phase1/src/fft_analysis.rs` as `draw_fft_plot_frame()`, so waveform grid can be changed first and FFT can follow later or share a helper after M3 stabilizes.
- Confirmed plot config reads/writes top-level `theme` as `dark` / `light`.
- Confirmed Rust-specific plot config namespace currently contains row/channel extension data only, so `rust.visual_mode` can be added in M5 without changing the compatible top-level `theme`.

Implementation target memo:

- M1 primary file: `code/rust_phase1/src/main.rs`
- M1 target areas: `ThemeMode`, `ViewState`, default app state, View section ComboBox
- M2 primary file: `code/rust_phase1/src/main.rs`
- M2 target areas: `ThemeMode::visuals()` replacement or wrapper, palette helper, `draw_app_brand()`
- M3 primary file: `code/rust_phase1/src/main.rs`
- M3 target areas: `draw_plot_frame()`, `draw_selected_plot_frame()`, call sites that pass `ui.visuals()`
- M4 primary file: `code/rust_phase1/src/main.rs`
- M4 target areas: `PanelYApp` state, top bar, new `draw_about_dialog()` helper
- M5 primary files: `code/rust_phase1/src/main.rs`, `code/rust_phase1/src/plot_config.rs`
- M5 target areas: `RustPlotConfigExt`, `parse_rust_ext()`, plot config capture/write/read/apply tests

Visual verification dataset:

- Use `code/proto_3_1b/data/test_100k.parquet` for First Cut visual checks.
- Schema CLI verified the file is readable.
- Dataset summary: 100000 rows, 5 columns, 4 channels.
- Channels: `sine_50Hz`, `pwm_1kHz`, `noise`, `chirp_1_500Hz`.
- This is enough to check cyan / green / yellow / magenta trace contrast, multiple rows, pan/zoom, and hover behavior.

Command run:

```bash
cd code/rust_phase1
cargo run --release -- --schema ../proto_3_1b/data/test_100k.parquet
```

Result:

- Build completed.
- Schema read succeeded.
- No code changes were made during M0.

Notes for M1:

- Prefer adding `VisualMode::{Plain, Rock}` instead of expanding `ThemeMode` into `PlainDark`, `PlainLight`, and `Rock`.
- UI should present combined choices: `Plain Dark`, `Plain Light`, `Rock`.
- Selecting `Rock` should set `theme = Dark` and `visual_mode = Rock`.
- Mode switching must not reset dataset state, loaded channels, rows, x range, or caches.
