---
project: Panel_y
doc_type: M4 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M4"
created: "2026-07-26"
updated: "2026-07-26"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
source_design: "docs/rust_rfq_1_multi_source_overlay_design.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m3_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M4 実行結果

## 1. 目的とスコープ

M3でRowへstage可能になった追加source traceを実描画へ接続し、異なるraw time、
sample count、sample rateを持つ2sourceのLine / Stepを、共通display time X軸へ描画する
M4を実装した。

M4専用の実行指示ファイルは開始時点で存在しなかったため、工程表の
`M4 — Multi-Time-Axis Line / Step Rendering`、overlay設計書のXレンジ・描画契約、
M3結果文書の申し送りを実装条件として使用した。

Hover / Cursor / interval statistics / FFTはM5の対象である。M4ではM3のactive-source
analysis境界を維持し、描画・Auto Y・X軸操作だけをmulti-source化した。

## 2. 開始条件

- 実行日: 2026-07-26
- branch: `codex/rfq1-multi-source-phase1-docs`
- HEAD: `52ef6723320a79d366004effadece5792da9ccc7`
- HEAD subject: `52ef672 Clarify single-source channel numbering`
- `main`: `2ba81459519735003838d61d24d4efa2f8359b38`
- `origin/main`: `2ba81459519735003838d61d24d4efa2f8359b38`
- rustc: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`
- 開始時worktree: clean
- M3までcommit済み
- ユーザー確認済みM3 GUI smoke:
  - Source #2追加
  - Parquet schema load
  - Source #2 channelのRow登録
  - M4前のcross-source描画抑止

commit、push、PR作成は行っていない。

## 3. Display time workspace range

`WorkspaceDataState` に全ロード済みsourceを集約する描画用helperを追加した。

- source raw rangeへ `TimeTransform::display_range()` を適用
- 全source display rangeの和集合をworkspace full rangeとして計算
- source別最小zoom幅の最小値をworkspace最小zoom幅として採用
- interaction感度用にsource別sample countを集約
- source削除、path変更、schema failure時は残存source unionへview rangeを再評価
- 最後のtime sourceが消えた場合は `ViewState.x_range` を `None` へ戻す

UIの `Reset X Range` は `Fit All` に改名し、全ロード済みsourceのdisplay range和集合へ
移動する。

## 4. Source別raw render plan

各frameで共通 `ViewState.x_range` をdisplay timeとして確定し、sourceごとに次を行う。

1. source display rangeと現在のdisplay view rangeの共通区間を計算
2. 共通区間がなければ、そのsourceのtrace生成をskip
3. `TimeTransform::raw_range()` でsource固有raw view rangeへ変換
4. source固有 `shared_time` と `ChannelStore` でEnvelope / Step planを準備
5. raw cacheを変更せず、描画時だけraw timeをdisplay timeへ変換

sourceごとの短いmutable borrowを使用し、Row snapshotのchannel順を維持したまま、
各traceを所属sourceのtime/cacheで生成する。

## 5. `VisibleTrace` と描画API

`VisibleTrace` は次を保持する。

- full `ChannelKey`
- source番号付きdisplay label
- source固有 `TimeTransform`
- raw sample count
- Line / Step dataと既存style

`draw_waveform_traces()` は最初のtraceからX rangeを推測せず、共通
`display_time_range` を明示的に受け取る。

- Envelope bucketのraw timeは描画時に `to_display()` する
- Raw Step / cached change pointのraw timeも描画時に `to_display()` する
- Stepの左右端延長はsource固有raw visible range内だけで行う
- axis、Hover line、Cursor lineの画面X座標は共通display rangeを使用する

## 6. Line overlay

Lineはsource固有raw time上で既存range-aware min/max Envelopeを生成する。

- source別Envelope context/cacheを維持
- source別Line tile cacheを維持
- offsetをraw timeやcache keyへ焼き込まない
- 異なるsample count/rateのtraceを補間・resampleせず同一Rowへ描画
- 既存spike/min-max保持経路をそのまま再利用

## 7. Step overlay

Stepはsource固有raw timeとsource別 `ChannelStore` を使う。

- low-density raw Step
- cached exact change points
- dense Step Envelope fallback
- fallback hint

上記の既存選択条件とedge保持を維持し、画面Xへの変換だけをsourceの
`TimeTransform` 経由にした。同名pathのStep edge cacheはsource別store内で分離される。

## 8. Auto Yとsource範囲外

Auto Yは現在のdisplay viewと交差するsource traceだけを統合する。

- source display rangeとviewが非交差ならtraceを生成しない
- 非交差sourceの値はRow Auto Yへ含めない
- Row内の他sourceにtraceがあれば、そのtraceだけを描画
- 全visible channelがsource範囲外なら
  `No source data in the current X range` を表示
- 空trace、非交差区間、unloaded channelをpanicなしで分離

## 9. 共通X軸操作

次の操作はworkspace display range、workspace最小zoom幅を使用する。

- Fit All
- double click reset
- horizontal pan
- horizontal scroll pan
- wheel / pinch zoom
- Shift + drag box zoom
- Hover X / Cursor lineの画面位置

全Rowと全sourceは同じ `ViewState.x_range` を参照するため、Pan/Zoomで同期して移動する。

## 10. Cache / performance表示

frame開始時に全sourceのEnvelope統計をresetし、frame終了時に次をworkspace全体で集約する。

- Envelope context hit/miss
- Envelope hit/miss/clip
- Line tile hit/build
- Step edge hit/build
- Envelope / Step edge / fallback hint cache count

既存performance panelとstatusは、選択中sourceだけでなく描画に参加した全sourceの値を
表示する。

## 11. M5境界

M4では次を意図的に実装していない。

- source別Hover value lookup
- source別Cursor A/B raw range変換
- source別interval statistics
- mixed-source FFT guardの完成
- time offset編集UI

`VisibleTrace` とX座標はfull `ChannelKey` / `TimeTransform` を保持するが、value readoutと
analysisはM3のactive-source filterを継続する。M5でsource別raw lookupへ置き換える。

`Fit Overlap` はPhase 1延期項目であり、M4の `Fit All` には含めていない。

## 12. 追加テスト

M4で3件の回帰テストを追加した。

1. `workspace_display_range_and_min_zoom_span_cover_every_loaded_source`
   - offset適用後の全source range union
   - 高sample-rate側を採用するworkspace最小zoom幅
   - `Fit All`
2. `visible_rows_overlay_different_rate_line_and_step_sources`
   - 異なるsample count/rateの同名 `iu #1` / `iu #2`
   - Line + Step混在Row
   - source #2 raw Step rangeとdisplay range変換
   - 両sourceを統合したAuto Y
3. `source_without_data_in_display_window_is_excluded_from_auto_y`
   - 異名 `torque #1` / `speed #2` overlay
   - source-only time region
   - 非交差sourceを除いたAuto Y
   - disjoint rangeを誤って反転・交差扱いしないこと

既存Envelope、Line tile、raw Step、change point、dense fallback、Pan/Zoomテストも継続して
成功する。

## 13. 自動検証

`code/rust_phase1` で実行した。

```text
cargo fmt --check
cargo check
cargo test
git diff --check
```

最終結果:

```text
test result: ok. 153 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo build --release` は成功した。生成したrelease binaryでM0 fixtureの代表channelを
ロードし、すべてexit code 0を確認した。

| Fixture / channel | Samples | Time range |
|---|---:|---|
| Source #1 / `iu` | 100,001 | `0.0 .. 2.0` |
| Source #2 / `iu` | 80,001 | `0.5 .. 1.75` |
| Source #1 / `torque` | 100,001 | `0.0 .. 2.0` |
| Source #2 / `speed` | 80,001 | `0.5 .. 1.75` |

各実行でtime/channel projection、range-aware Envelope生成、有限value rangeを確認した。

## 14. M4 GUI smoke checklist

M0 fixtureを使う。

1. Source #1へ `multi_source_validation_1.parquet` を登録し、`iu` をRow 1へLine追加
2. Source #2へ `multi_source_validation_2.parquet` を登録し、`iu` を同じRowへLine追加
3. `iu #1` と `iu #2` が同時描画される
4. `torque #1` と `speed #2` を同じRowへ追加し、同時描画される
5. `gate_pwm #1` と `gate_pwm #2` をStepへ変更し、異なるedge位置が保持される
6. `Fit All` でX rangeが `0.0 .. 2.0 s` になる
7. Pan、wheel/pinch zoom、Shift + drag box zoomで両sourceが同期する
8. `0.0 .. 0.5 s` と `1.75 .. 2.0 s` でSource #1だけが表示され、Auto Yが壊れない
9. sourceごとのLine / Step設定を混在させてもpanicしない
10. Source #2削除後、Source #1の単一source Line / StepとFit Allが動く

### 確認結果

2026-07-26にユーザーが上記M4 GUI smoke checklistを実施し、全項目について
「問題なし」と報告した。

- 2sourceのLine / Step重ね描き: 合格
- Fit All: 合格
- 共通X軸Pan / Zoom / box zoom: 合格
- source-only時間区間とAuto Y: 合格
- Source #2削除後の単一source回帰: 合格

Fit Allは各sourceの元の時間位置を維持したまま全source rangeの和集合を表示する。
各sourceの先頭時刻を0秒へ揃える `Start at zero` と時間offset編集UIはM4に含まず、
後続機能として未実装であることも確認した。

## 15. 変更ファイル

- `code/rust_phase1/src/main.rs`
- `code/rust_phase1/src/workspace_data.rs`
- `code/rust_phase1/README.md`
- `docs/rust_rfq_1_multi_source_phase1_m4_results.md`

Cargo依存関係、fixture generator、Parquet生成物は変更していない。

## 16. M5への申し送り

- `ViewState.hover_x` とCursor A/Bはdisplay timeのまま維持する
- traceごとに `TimeTransform::to_raw()` し、所属sourceのtime/valueだけを検索する
- source display range外ではendpoint clampせず `None` を返す
- interval statistics cache keyはfull `ChannelKey` とsource time identityを含める
- FFT targetはfull `ChannelKey` を維持し、mixed-source inputを開始前に拒否する
- M4のsource別raw cacheとdisplay-time draw境界をpath-onlyへ戻さない

## 17. M4完了判定

実装、Unit/Integration検証、release fixture確認、M4 GUI smoke checklistがすべて成功した。
M4を `complete` と判定する。
