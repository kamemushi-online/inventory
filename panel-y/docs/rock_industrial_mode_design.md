---
project: Panel_y
doc_type: Rock Industrial Mode Design
target: Rust viewer visual mode
created: "2026-05-31"
updated: "2026-05-31"
status: draft
implementation: "code/rust_phase1"
---

# Rock Industrial Mode Design

---

## 1. Purpose

Rust viewer に、現行のすっきりしたプレーン表示を残したまま、よりハードでプロフェッショナルな印象の `Rock` 表示モードを追加する。

この設計では、外部 skin ロードや画像テクスチャには踏み込まない。背景色、グリッド、アクセントカラー、既存 UI 部品の描画調整、About 表示だけで、測定器・スタジオ機材・インダストリアル機材の雰囲気を作る。

最優先は波形ビューアとしての実用性である。装飾は読みやすさ、操作性、描画コストを悪化させない範囲に限定する。

---

## 2. Scope

In scope:

- 現行プレーン表示と `Rock` 表示の切り替え
- `Rock` 用の dark-only palette
- `Rock` 用の plot background / grid / selected frame
- `Rock` 用の button / input / combo box / checkbox 系アクセント
- top bar の `About` ボタン
- `About` ボタンから表示する山本製作所クレジット
- plot config で Rust 側表示モードを保存・復元する方針の定義

Out of scope:

- 外部 skin ファイルのロード
- PNG/JPEG などの画像テクスチャ読み込み
- 金属傷、汚れ、ノイズなどの procedural texture
- 透視投影・歪みのあるグリッド
- UI レイアウト全体の再設計
- ノブやスイッチの大規模な独自 widget 化
- About クレジットの常時表示

---

## 3. Baseline

現在の Rust viewer は `ThemeMode` が `Dark` / `Light` の 2 種類で、毎フレーム `ctx.set_visuals(self.view.theme.visuals())` により egui の visuals を反映している。

Plot config も top-level の `theme` を `dark` / `light` として保存・復元する前提になっている。

このため、最初の実装は「外部 skin システム」ではなく「内蔵 visual mode の追加」として扱うのが安全である。

---

## 4. Design Direction

### 4.1 Plain mode

Plain mode は現行 UI の性格を維持する。

- 既存の `Dark` / `Light` の見た目を大きく変えない
- ビジネスロジックや plot 操作に影響を与えない
- 既存 plot config との互換性を維持する

### 4.2 Rock mode

Rock mode は dark-only とする。Light 版 Rock は作らない。

狙う印象:

- 黒染め機材
- ネオンブルーの操作パネル
- オシロスコープやスタジオ機材に近い plot 面
- パンクな銘板を About に隔離

避ける印象:

- 読みにくい装飾 UI
- 過剰な glow
- 背景 texture による情報ノイズ
- おもちゃっぽいノブや無意味な装飾

---

## 5. Visual Tokens

初期値案:

```text
app_bg              #05090D
panel_bg            #10161B
panel_bg_alt        #151C22
input_bg            #070B0F
plot_bg             #031018
plot_bg_alt         #061822
text_primary        #D6E3EA
text_secondary      #8EA4AF
accent_blue         #45D8FF
accent_blue_deep    #18B7FF
accent_blue_dim     #0B5A75
grid_minor          #1B6D80 with low alpha
grid_major          #45D8FF with medium-low alpha
warning             #FFB84A
danger              #FF5C7A
plate_bg            #B8BEC3
plate_text          #111417
punk_magenta        #FF2AEF
```

実装上は `Color32::from_rgb()` と `Color32::from_rgba_unmultiplied()` で管理する。

---

## 6. Plot Design

### 6.1 Background

Plot 背景は深い青黒にする。

- 通常: `plot_bg`
- row hover / selected row: selected frame だけで示す
- 背景に texture は描かない

### 6.2 Grid

Grid は普通の直交グリッドにする。画像案のような青系の雰囲気は色で出し、透視・歪みは入れない。

Recommended grid:

- minor grid: 10 分割の細線
- major grid: 5 分割または中央線を少し強調
- color: cyan 系
- alpha: 実波形より明らかに弱くする

実用面の方針:

- grid は波形と競合しない
- Y 軸ラベル、X 軸ラベル、カーソル線の可読性を優先する
- selected frame は grid より強く、trace より弱くする

### 6.3 Traces

Rock mode でも trace palette は現在の識別性を維持する。初期段階では大きく変えず、必要なら彩度だけ少し強める。

Candidate Rock trace palette:

```text
cyan      #45D8FF
green     #76FF2A
yellow    #F8FF2D
magenta   #FF2AEF
violet    #B86CFF
teal      #30F0D0
orange    #FFB84A
white     #DCE7EC
```

---

## 7. UI Component Direction

### 7.1 Top bar

Top bar は機材のヘッダーとして扱う。

- `Panel-y` brand は cyan
- version badge は暗い青黒に薄い枠
- status text は通常の readable text
- `About` ボタンは brand / version の直後に置く

Top bar に常時クレジットや長文は置かない。

### 7.2 Left panel

Left panel は操作パネルとして扱う。

- 背景は `panel_bg`
- Section heading はやや明るい text
- Button border / active state は `accent_blue`
- Text input は `input_bg`
- ComboBox は現行構造を維持し、枠線と selection 色だけ変える

### 7.3 Buttons

Rock mode の button は、塗りつぶしより枠線を主役にする。

- normal: dark fill + dim cyan border
- hovered: slightly brighter fill + cyan border
- active: cyan border + subtle cyan fill
- disabled: low contrast gray

強い glow は使わない。必要なら 1px から 2px の追加 stroke に留める。

### 7.4 Toggles

初期実装では checkbox のままでよい。

将来、`Plain wheel X zoom` や `Emergency preview` は switch 風 widget にできるが、第一段階では必須ではない。操作意味が変わらないことを優先する。

---

## 8. About Credit

### 8.1 Placement

クレジットは常時表示しない。Top bar の `About` ボタンで dialog / modal window を開く。

理由:

- 波形表示領域を邪魔しない
- UI の情報密度を増やさない
- パンクな文言を本体操作 UI から分離できる

### 8.2 Content

About に表示する内容:

```text
YAMAMOTO Mfg.
山本製作所
EST. 197X
NO FUTURE!
当製作所ではDIY精神と反体制的なカオスなアプリケーション開発を推進しています
```

### 8.3 Visual style

About dialog は金属銘板風にするが、画像 texture は使わない。

- plate background: light gray metal-like flat color
- dark border
- corner screw marks: small circles drawn by egui painter
- gear mark: simple text/symbol or minimal painter shape
- `NO FUTURE!`: magenta or warning accent

第一段階では text と simple shapes だけで実装する。

---

## 9. State Model

### 9.1 Recommended model

`ThemeMode` を肥大化させるより、表示意図を分ける方が安全である。

Recommended:

```rust
enum ThemeMode {
    Dark,
    Light,
}

enum VisualMode {
    Plain,
    Rock,
}

struct ViewState {
    theme: ThemeMode,
    visual_mode: VisualMode,
    ...
}
```

Rules:

- `ThemeMode::Light + VisualMode::Rock` は許可しない
- UI では `Plain Dark`, `Plain Light`, `Rock` の 3 択として見せる
- 内部的には `Rock` 選択時に `theme = Dark`, `visual_mode = Rock` にする

Alternative:

```rust
enum ThemeMode {
    PlainDark,
    PlainLight,
    Rock,
}
```

この方が実装量は少ないが、plot config 互換や既存 docs の `theme` と意味が混ざる。長期的には `ThemeMode + VisualMode` の分離を推奨する。

### 9.2 Visual application

`visuals()` は `ThemeMode` だけでなく `VisualMode` を受け取る。

```rust
fn apply_visuals(ctx: &egui::Context, theme: ThemeMode, visual_mode: VisualMode) {
    let visuals = match visual_mode {
        VisualMode::Plain => theme.visuals(),
        VisualMode::Rock => rock_visuals(),
    };
    ctx.set_visuals(visuals);
}
```

Rock 固有の色は `SkinPalette` か `VisualPalette` として関数化する。

```rust
struct VisualPalette {
    app_bg: Color32,
    panel_bg: Color32,
    plot_bg: Color32,
    grid_minor: Color32,
    grid_major: Color32,
    accent: Color32,
}
```

---

## 10. Plot Config Strategy

### 10.1 Compatibility concern

Top-level `theme` は `.pyc.json` version 1 互換のフィールドであり、既存設計では `dark` / `light` を想定している。

Rock mode を `theme: "rock"` として保存すると、Dash 版や古い Rust 版での解釈が不明確になる。

### 10.2 Recommended persistence

Top-level `theme` は互換性のため `dark` / `light` のままにする。

Rock mode は Rust-specific namespace に保存する。

```json
{
  "version": 1,
  "theme": "dark",
  "rust": {
    "version": 1,
    "visual_mode": "rock"
  }
}
```

Read rules:

- `rust.visual_mode == "rock"` なら Rock mode
- `rust.visual_mode` がない場合は Plain mode
- top-level `theme` は `ThemeMode` として従来通り読む
- Rock mode は常に dark visuals を使う

Write rules:

- Plain dark: `theme: "dark"`, `rust.visual_mode: "plain"` or omit
- Plain light: `theme: "light"`, `rust.visual_mode: "plain"` or omit
- Rock: `theme: "dark"`, `rust.visual_mode: "rock"`

First implementation can omit plot config persistence if scope needs to stay smaller, but the model should not prevent this extension.

---

## 11. Implementation Plan

### Step 1: Add visual mode state

- Add `VisualMode`
- Add `visual_mode` to `ViewState`
- Add UI selector with labels:
  - `Plain Dark`
  - `Plain Light`
  - `Rock`
- Keep default as `Plain Dark`

### Step 2: Centralize palette

- Add helper functions:
  - `active_visual_mode()`
  - `visual_palette()`
  - `rock_visuals()`
- Keep plain visuals identical or near-identical to current behavior

### Step 3: Rock visuals

Adjust egui visuals:

- window / panel fill
- extreme background color
- text colors
- selection stroke / fill
- widget inactive / hovered / active colors
- hyperlink or accent colors if needed

### Step 4: Plot frame and grid

Update `draw_plot_frame()` to use visual palette.

- Fill plot background before drawing grid
- Draw minor grid
- Draw major grid
- Draw plot border
- Keep selected frame on top

FFT plot frame should either share the same helper or receive the same palette later, so waveform and FFT do not diverge visually.

### Step 5: About dialog

- Add `show_about: bool` to `PanelYApp`
- Add `About` button to top bar
- Add `draw_about_dialog()`
- Include YAMAMOTO Mfg. / 山本製作所 / EST. 197X / NO FUTURE! / description

### Step 6: Plot config persistence

- Extend Rust plot config namespace with `visual_mode`
- Read missing field as Plain
- Preserve top-level `theme` compatibility
- Add tests for:
  - old config without `visual_mode`
  - Rock config save/read
  - invalid `visual_mode` fallback

---

## 12. Verification Plan

Manual checks:

- App starts in Plain Dark
- Plain Light still works
- Rock mode can be selected from View controls
- Switching modes does not reset loaded data, rows, ranges, or caches
- Plot grid remains readable with cyan, green, yellow, magenta traces
- Axis labels remain readable
- Selected row frame is visible but not distracting
- About dialog opens and closes
- About dialog does not block normal operation after closing

Automated checks:

- `cargo fmt`
- `cargo test`
- `cargo check`
- Plot config normalization tests for `visual_mode`

Visual checks:

- Small default dataset
- Multiple rows
- At least 4 traces with different colors
- Dark room / high contrast scenario
- Narrow left panel width

---

## 13. Risks and Mitigations

Risk: Rock mode makes plot harder to read.

Mitigation:

- Keep grid alpha low
- Keep trace colors stronger than grid
- Avoid texture and glow
- Verify against cyan, green, yellow, magenta traces

Risk: Theme and visual mode become confused.

Mitigation:

- Separate `ThemeMode` and `VisualMode`
- Present combined choices in UI
- Save `theme` and Rust-only `visual_mode` separately

Risk: About content becomes visually noisy.

Mitigation:

- Hide it behind `About`
- Use a modal/dialog only when requested
- Keep top bar compact

Risk: FFT and waveform plots diverge visually.

Mitigation:

- Share palette helpers
- Apply Rock grid behavior to FFT plot frame after waveform plot is stable

---

## 14. Recommended First Cut

The first implementation should be intentionally small:

1. Add `Rock` as a selectable built-in mode
2. Add Rock color palette
3. Apply Rock visuals to global egui visuals
4. Apply Rock plot background and grid to waveform plots
5. Add About dialog

Do not implement external skin loading, image texture, custom knobs, or switch widgets in the first cut.

This gives the app a distinct Rock/industrial mode while keeping the viewer practical and maintainable.
