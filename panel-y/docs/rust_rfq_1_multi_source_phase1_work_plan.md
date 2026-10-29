---
project: Panel_y
doc_type: RFQ Work Plan
rfq: "RFQ #1"
phase: "Multi-source Phase 1"
target: Rust multi-source load and overlay first cut
created: "2026-07-25"
updated: "2026-10-28"
status: complete
source_design: "docs/rust_rfq_1_multi_source_overlay_design.md"
implementation: "apps/panel-y"
---

# RFQ #1 — マルチデータ読み込み対応 Phase 1 実装工程表

---

## 1. 目的

[RFQ #1 — 複数時系列データ重ね描き設計書](./rust_rfq_1_multi_source_overlay_design.md) から、マルチデータ読み込み対応の最初の実装範囲を選定し、依存関係、作業順序、確認方法、完了条件を定義する。

Phase 1では、次の最小の縦切りを完成させる。

> 2つのParquetデータソースを独立してロードし、それぞれの任意チャンネルを同一プロット行へ安全に追加し、Line/Step、Pan/Zoom、Hover、Cursor統計を使って比較できる。

単に2つ目のファイルを読めるだけでは完了としない。チャンネル識別、キャッシュ、非同期結果、時刻軸、表示ラベルが衝突せず、既存の単一ソース利用を回帰させないところまでをPhase 1に含める。

### 1.1 現在の進捗（2026-10-24確認）

| 範囲 | 状態 | 記録 |
|---|---|---|
| Phase 1 M0–M7 | 完了。2ソース読込・重ね描き・解析安全対策・保存guard・Release Gateを通過 | [M7実行結果](./rust_rfq_1_multi_source_phase1_m7_results.md) |
| matching channel一括追加 | M3で実装済み。当初の延期候補からPhase 1へ取り込み | [M3実行結果](./rust_rfq_1_multi_source_phase1_m3_results.md) |
| Phase 2 Time Alignment | 完了。Offset、Reset、Start at zero、Start all at zeroの実装・自動テストとGUI smoke全10項目が合格 | [Phase 2実行結果](./rust_rfq_1_multi_source_phase2_time_alignment_results.md) |
| Fit Overlap | 完了。共通区間の表示・無効化・offset変更・Source削除をテストとGUIで確認 | [Fit Overlap実行結果](./rust_rfq_1_multi_source_fit_overlap_results.md) |
| Plot config v2 | 完了。複数ソース・offset・全行設定の保存/復元とv1互換を確認 | [v2実行結果](./rust_rfq_1_multi_source_plot_config_v2_results.md) |
| 複数ソースSession preset | 完了。194件の自動テスト・ビルド・GUI全10項目が合格 | [Session preset実行結果](./rust_rfq_1_multi_source_session_preset_results.md) |
| ソース別FFT結果の重ね表示 | 完了。205件の自動テスト・GUI全10項目が合格 | [FFT overlay実行結果](./rust_rfq_1_multi_source_fft_overlay_results.md) |
| 3ソース以上のUI | 実装・208件の自動テスト・fmt/build/diff・GUI全10項目完了 | [3ソース以上のUI実行結果](./rust_rfq_1_multi_source_ui_results.md) |

実装対象は `563da91`（`Implement Phase 2 time alignment controls`）。Phase 1完了時は
167テスト、2026-09-08の再確認では170テストが成功した。2026-10-23に `23be017` の
debugビルドでGUI smoke全10項目が合格した。2026-10-24にFit Overlapを追加し、
175テストとGUI確認が成功した。同日にPlot config v2を追加し、185テストとGUI確認が成功した。複数ソースSession presetも194テスト・ビルド・GUI全10項目が成功した。ソース別FFT重ね表示も205テスト・GUI全10項目が成功した。3ソース以上のUIも208テスト・fmt/build/diff・GUI全10項目が成功した。未実装機能は19節にまとめる。
以下のM0–M7各節は、完了済み工程の実施内容と完了条件として保持する。

2026-10-28にRust本体を`code/rust_phase1/`から`apps/panel-y/`へ移動し、
Cargo packageと実行ファイル名を`panel-y`へ変更した。本工程表の実装参照先は
現行配置へ更新した。各実行結果・測定証跡のパスとrevisionは当時の記録である。

---

## 2. Phase 1の位置づけ

RFQ #1全体を一度に実装せず、次の境界で分割する。

```text
Phase 1 (complete)
  Core identity and workspace model
  -> independent two-source loading
  -> arbitrary channel overlay
  -> matching-channel convenience action (implemented in M3)
  -> multi-time-axis rendering
  -> Hover / Cursor statistics
  -> FFT safety boundary
  -> regression and release gate

Phase 2 Time Alignment (complete)
  Time offset controls
  -> per-source Reset / Start at zero
  -> Start all at zero

Fit Overlap (complete)
  Common display-time range of loaded sources

Plot config v2 (complete)
  Multi-source plot-config persistence including offsets

Multi-source Session preset (complete)
  In-memory workspace snapshot including offsets and view state

Per-source FFT result overlay (complete)
  Independent source jobs and shared spectrum axes

3+ source UI (complete: 208 tests and ten GUI checks passed)
  Uncapped source registration and bounded source-card scrolling

Remaining scope
  Automatic alignment
```

Phase 1の表示・比較基盤とPhase 2の時間合わせ操作は完成している。
Plot config v2の複数ソース・時間offset永続化も完成した。複数ソースSession presetは実装・自動テスト・ビルド・GUI確認完了。自動整列などは未実装である。

---

## 3. Phase 1採用機能

### 3.1 必須機能

以下はPhase 1当初の採用範囲。現在の3ソース以上のUIは19.6節を参照する。

| Area | Phase 1 selection |
|---|---|
| 内部モデル | `SourceId`、`ChannelKey`、`TimeTransform` |
| ソース管理 | 内部Nソース対応、初期UIは最大2ソース |
| スキーマ | ソースごとの独立読込とエラー隔離 |
| 時刻データ | ソースごとの `shared_time` |
| チャンネルデータ | ソースごとの `ChannelStore` |
| 非同期ロード | source id / generation付きジョブ |
| Row構成 | 任意ソースの任意チャンネルを同一行へ追加 |
| matching channel | M3で一括追加を実装。完全path一致を優先し、一意のname一致へfallback |
| 重複判定 | `ChannelKey` 単位 |
| 表示名 | 単一ソース時を含め常に `channel_name #n` |
| X軸 | 共通display time、全ソース範囲の和集合 |
| 時間変換 | `TimeTransform` を実処理へ通す。初期offsetは `0.0` |
| Line | ソース固有time/Envelopeによる重ね描き |
| Step | ソース固有time/Step cacheによる重ね描き |
| Y軸 | 行内の全ソース・全表示トレースを統合 |
| 操作 | 同期Pan/Zoom、Reset/Fit All |
| Hover | チャンネル所属ソースのtimeで値検索 |
| Cursor | display timeで保持し、ソースごとにraw timeへ変換 |
| 区間統計 | ソースごとのsample区間で計算 |
| FFT | 1ジョブ1ソース制限と誤実行防止 |
| Source removal | 対象Row参照、キャッシュ、解析結果の整理 |
| 性能表示 | 全ソース分のメモリ・キャッシュ統計を集計 |
| 回帰 | 現行単一ソース機能と既存106テストを維持 |

### 3.2 Phase 1の安全対策

未対応機能が誤った結果や不完全な保存を行わないよう、以下をPhase 1に含めた。
保存guardは当時の記録であり、Plot config v2と複数ソースSession presetの実装で解除した。
現在の保存・適用条件は19節と各実行結果を参照する。

- 複数ソース状態では、複数ソース未対応のPlot config保存を無効化する
- 複数ソース状態では、複数ソース未対応のSession preset保存・適用を無効化する
- 既存version 1 Plot configのロードは、ワークスペースを単一ソース `#1` として開く
- `Apply to Current Data` は1ソース時だけ有効にする
- 異なるソースのチャンネルを同一FFTジョブへ渡さない
- ソース表示範囲外のHover/Cursor値を端点へクランプせず、欠損として扱う
- stale load resultはsource idとgenerationで破棄する

### 3.3 Phase 1外の機能と現在の状態

| Item | 現在の状態 | 境界・理由 |
|---|---|---|
| 時間オフセット編集UI / Reset | Phase 2で実装・GUI確認完了 | raw timeを変更せずdisplay transformを更新 |
| Start at zero / Start all at zero | Phase 2で実装・GUI確認完了 | ロード済みSourceの先頭時刻を0秒へ合わせる |
| トリガー・相互相関による自動整列 | 未実装 | 独立した解析要求 |
| `Fit Overlap` | 実装・テスト・GUI確認完了 | time読込済みSourceの共通display区間。重なりなし・端点接触では無効 |
| ソース別名編集 | 未実装 | 現在はファイル名を補助表示に使用 |
| ソース順並び替え | 未実装 | 現在は追加順を使用 |
| 3ソース以上のUI | 実装・208件の自動テスト・GUI確認完了 | 固定上限なし。カード一覧を高さ制限付きでスクロール |
| 複数ソースPlot config `rust.version: 2` | 完了 | 2026-10-24、保存・復元・Apply・欠損処理・v1互換を検証 |
| 複数ソースSession preset | 実装・自動テスト・ビルド・GUI確認完了 | メモリ内snapshot。offset・全行設定・選択・X範囲を復元し、欠損ソースを警告 |
| ソース別FFT結果の重ね表示 | 実装・自動テスト・GUI確認完了 | 合計最大3対象をソース別の独立ジョブへ分割。1ジョブ1ソースguardは維持 |
| 異なるソース間のリサンプリング | 未実装 | 表示には不要。解析仕様として別途検討 |

`Add matching channel from all sources` はM3で実装済みのため、延期項目には含めない。

---

## 4. Phase 1の前提と固定判断

### 4.1 ソース番号

- `source_order[0]` を画面上の `#1` とする
- `source_order[1]` を画面上の `#2` とする
- 番号は表示用であり内部IDではない
- ソース削除後は残ったソースを先頭から再採番する
- 初期UIではソース順の並び替えを提供しない

### 4.2 時刻

- Parquetのtime値は秒単位、有限値、単調非減少を前提とする
- `ViewState.x_range`、Hover、Cursorはdisplay timeで保持する
- sample lookup、Envelope、Step、FFTはraw timeを使用する
- `TimeTransform.offset_seconds` は初期値 `0.0`
- Phase 1でも変換APIを通し、将来offset UIを追加しても呼出境界を変更しない

### 4.3 ソース変更時の扱い

- ソースのファイルパス変更は対象ソースのgenerationを進める
- 対象ソースのschema、time、raw channel、draw cacheをクリアする
- 対象ソースを参照するRowChannelを削除する
- 他ソースのtime、raw channel、RowChannelは保持する
- Xレンジは残ったロード済みソースの和集合へ再評価する

### 4.4 対応する描画モード

Phase 1でLineとStepの両方を対応する。

理由:

- 現行Rust版の既存機能であり、マルチソース化で回帰させない
- Stepは電力・モータ制御ログで主要な比較対象である
- `ChannelStore` をソース単位へ分離すれば、既存Step edge cacheを再利用できる

---

## 5. マイルストーン概要

```text
M0 Scope lock and validation fixtures
  -> M1 Identity and single-source-compatible state model
  -> M2 Source-aware load jobs and per-source caches
  -> M3 Two-source UI and channel presentation
  -> M4 Multi-time-axis Line / Step rendering
  -> M5 Hover / Cursor / statistics / FFT safety
  -> M6 Unsupported persistence guards and operational polish
  -> M7 Integration, performance check, and release gate
```

### 5.1 工数目安

以下は1名で既存コードを把握している前提の実装目安であり、カレンダー上の確約ではない。

| Milestone | Size | Estimate |
|---|---:|---:|
| M0 Scope / fixtures | S | 0.5–1.0日 |
| M1 Identity / model | L | 1.5–2.5日 |
| M2 Loading / caches | L | 1.5–2.5日 |
| M3 UI / labels | M | 1.0–2.0日 |
| M4 Rendering | L | 2.0–3.0日 |
| M5 Interaction / analysis safety | L | 1.5–2.5日 |
| M6 Persistence guards / polish | M | 0.5–1.5日 |
| M7 QA / performance / docs | M | 1.0–2.0日 |
| Total |  | 9.5–17.0日 |

不確実性が大きい箇所は、`main.rs` 全体へ広がっている `channel_path: String` 前提の置換と、Rustの可変借用を保った複数 `ChannelStore` の描画処理である。

---

## 6. M0 — Scope Lock and Validation Fixtures

[M0実行指示プロンプト](./rust_rfq_1_multi_source_phase1_m0_execution_prompt.md)

### Goal

- Phase 1の対象・延期境界を実装開始前に固定する
- 2ソース動作を再現できる検証データを用意する
- 現行ベースラインを記録する

### Tasks

- 現行 `main` / `origin/main` と作業ツリーを確認する
- `cargo test` で既存106テストの成功を確認する
- 単一ソースの手動確認手順を記録する
- 2ソース検証データの生成方法を決定する
- 次の差を持つ2ファイルを用意する
  - 同名チャンネル `iu`
  - #1だけに存在する `torque`
  - #2だけに存在する `speed`
  - 異なるsample count
  - 異なるsample rate
  - 異なるtime range
- Load、Pan/Zoom、Hover、Cursor、Stepを確認するシナリオを作る

### Deliverables

- 2ソース検証用Parquetまたは再生成スクリプト
- 手動確認チェックリスト
- ベースラインテスト結果

### Completion criteria

- 同名と異名の両方を検証できる
- 異なるtime/sample条件を検証できる
- 既存テストが変更前に成功している

### Expected files

- `tools/data_generation/generate_multi_source_validation_data.py`、または同等のfixture generator
- `data/` の生成物はリポジトリ方針に従う
- `apps/panel-y/README.md`

---

## 7. M1 — Identity and Single-Source-Compatible State Model

### Goal

- すべてのチャンネル参照をソース修飾可能にする
- UI上はまだ1ソースでも、内部を複数ソースモデルへ移行する
- 後続工程前に単一ソース回帰を止める

### Tasks

- `SourceId` を追加する
- `ChannelKey { source_id, channel_path }` を追加する
- `TimeTransform { offset_seconds }` とraw/display変換を追加する
- `DataSourceState` を追加する
- `WorkspaceDataState` を追加する
- 既存default Parquet pathをソース `#1` として初期化する
- `RowChannel.channel_path` を `RowChannel.key` へ置換する
- `ViewState.selected_channel` を `Option<ChannelKey>` へ置換する
- 行の重複判定を `ChannelKey` 単位へ変更する
- channel lookup用の共通accessorを追加する
  - `source(source_id)`
  - `source_mut(source_id)`
  - `channel(&ChannelKey)`
  - `has_channel(&ChannelKey)`
  - `source_number(source_id)`
- Cursor/FFT/configを含む既存の参照箇所をコンパイル可能な形へ移行する
- この工程では実データソース数を増やさない

### Deliverables

- 複数ソース対応可能な状態モデル
- `ChannelKey` を使うRow/View状態
- 初期値 `0.0` の `TimeTransform`
- 単一ソース互換動作

### Completion criteria

- `channel_path` だけでデータ所有者を識別するView参照が残っていない
- 単一ソースでSchema、Load、Row追加、Line/Step、Hover、Cursor、FFTが動作する
- 表示名を除き、既存ユーザー操作が回帰しない
- 既存テストが成功する

### Verification

- `cargo fmt --check`
- `cargo test`
- `cargo check`
- Manual: default datasetで既存操作一式

### Risk

- 一度に広範囲の型エラーが発生する

### Mitigation

- `ChannelKey` とworkspace accessorを先に導入する
- 直接 `BTreeMap` をたどるコードを増やさない
- M1中はUIを1ソースのまま維持する

### Expected files

- `apps/panel-y/src/main.rs`
- `apps/panel-y/src/fft_analysis.rs`
- 必要に応じて `apps/panel-y/src/workspace_data.rs`

---

## 8. M2 — Source-Aware Load Jobs and Per-Source Caches

### Goal

- 2つのソースを同名チャンネル衝突なしでロードできるデータ層を完成させる
- ロード失敗・再ロード・stale resultをソース単位で隔離する

### Tasks

- `ChannelStore` を各 `DataSourceState` の所有物にする
- `shared_time` を各 `DataSourceState` の所有物にする
- schema loadを `SourceId` 指定に変更する
- channel load requestを `ChannelKey` 指定に変更する
- `ActiveLoadJob` へsource id / source generationを追加する
- `LoadJobResult` へsource id / source generationを追加する
- `LoadState::is_channel_loading()` を `ChannelKey` 対応にする
- 結果適用時にsource存在、generation、channel pathを検証する
- 一方のソースのload failureで他方をクリアしない
- 一方のsource resetで他方のraw data/cacheを保持する
- memory/cache統計を全ソース集計へ変更する
- source追加・削除に必要な非UI APIを追加する

### Deliverables

- 2ソース独立ロードAPI
- ソース別time/channel/cache
- stale result防止
- ソース単位のreset/remove

### Completion criteria

- `(source #1, "iu")` と `(source #2, "iu")` を同時にキャッシュできる
- 2ソースで異なるsample countを保持できる
- #2のロード失敗後も#1を表示できる
- #2のパス変更で#1のcacheが消えない
- stale background resultが新しいソース状態へ適用されない

### Verification

- Unit: same path / different source cache isolation
- Unit: source generation mismatch rejection
- Unit: source reset isolation
- `cargo fmt --check`
- `cargo test`
- `cargo check`

### Risk

- 描画キャッシュが現行の単一 `shared_time` と暗黙に結びついている

### Mitigation

- `ChannelStore` と `shared_time` を必ず同じ `DataSourceState` から取得する
- workspace levelにraw channelのflat cacheを作らない

### Expected files

- `apps/panel-y/src/main.rs`
- `apps/panel-y/src/parquet_waveform.rs`
- 必要に応じて `apps/panel-y/src/workspace_data.rs`

---

## 9. M3 — Two-Source UI and Channel Presentation

### Goal

- UIから2つのデータソースを登録・選択・ロードできる
- すべてのチャンネル表示でデータソースを識別できる
- 任意ソースの任意チャンネルを選択行へ追加できる

### Tasks

- `Data Sources` セクションを追加する
- 初期状態ではソース `#1` を既存default pathで表示する
- `Add Source` で空のソース `#2` を追加する
- 初期UI上限を2ソースにする
- ソースごとにPath、Browse、Load Schema、Removeを表示する
- 対象ソースのロード中だけ、そのソースの危険操作を無効化する
- Channel browserをソース単位でグループ表示する
- browser filterへsource number / file name検索を追加する
- `ChannelPresentation` または統一label helperを追加する
- 次の全表示へ常時 `#n` を付ける
  - Channel browser
  - Row controls
  - Plot axis/title
  - Hover readout
  - Cursor values / interval statistics
  - FFT target/result labels
  - Cache controls
  - Load status
- Row追加・Remove・Unloadを `ChannelKey` 対応にする
- `Add matching channel from all sources` を追加する（M3で採用）
  - 完全channel path一致を優先し、一意の元channel name一致へfallbackする
  - schema未ロード、候補なし、曖昧な一致はskipしてstatusへ要約する
- ソース削除時に対象RowChannelを削除する
- 削除後に表示番号を再計算する

### Deliverables

- 最大2ソースのUI
- `channel_name #1` / `channel_name #2` 表示
- 任意チャンネルのRow追加
- ソース削除と表示再採番

### Completion criteria

- #1と#2のschema/channel一覧を同時に確認できる
- 同じ名前のチャンネルを区別して選択できる
- 異なる名前のチャンネルも同一行へ追加できる
- 1ソース時もチャンネル表示名に `#1` が付く
- 内部channel name/pathには `#n` が入らない
- source remove後にdangling RowChannelが残らない

### Verification

- Unit: presentation labels
- Unit: source renumbering
- Unit: `ChannelKey` row membership
- Unit: matchingのpath優先、一意name fallback、missing / ambiguousのskip
- Manual: #1/#2から同名・異名チャンネルを追加
- `cargo fmt --check`
- `cargo test`

### Risk

- ラベル生成が画面ごとに分散し、一部だけ `#n` が欠ける

### Mitigation

- 文字列連結を各UIで行わず、統一presentation helperを使う
- 受入テストで全表示箇所を列挙する

### Expected files

- `apps/panel-y/src/main.rs`
- `apps/panel-y/src/fft_analysis.rs`

---

## 10. M4 — Multi-Time-Axis Line / Step Rendering

### Goal

- 異なる時刻配列を持つ2ソースのトレースを、共通display time上へ正しく描画する
- 既存Line/Stepの情報保持特性を維持する

### Tasks

- `ViewState.x_range` をdisplay timeとして扱う
- 全ロード済みソースのdisplay range和集合を計算する
- `Reset X Range` を `Fit All` 相当へ変更する
- workspaceの最小ズーム幅をソース別値の最小として計算する
- `visible_row_traces()` をソース別time/cache対応にする
- 各ソースでdisplay rangeをraw rangeへ変換する
- ソースごとにEnvelope context/planを準備する
- `VisibleTrace` へ `ChannelKey`、display label、`TimeTransform` を追加する
- `draw_waveform_traces()` へ共通display rangeを明示的に渡す
- Envelope bucket timeを描画時にdisplay timeへ変換する
- Raw Step / change pointsのtimeを描画時にdisplay timeへ変換する
- 表示範囲外のソースで空traceを安全に扱う
- Auto Yは現在のdisplay range内に存在する値だけを統合する
- Pan/Zoom/Box zoom/Double clickを共通display rangeへ適用する
- Line tile cache、Step edge cache、Envelope cacheをソースごとに再利用する

### Deliverables

- 2ソースLine overlay
- 2ソースStep overlay
- 共通X軸Pan/Zoom
- 全ソース範囲のFit All
- ソース別描画キャッシュ

### Completion criteria

- 異なるsample count / sample rateでも描画できる
- `iu #1` と `iu #2` を同一行へ表示できる
- `torque #1` と `speed #2` を同一行へ表示できる
- Lineのspike/min-max保持を回帰させない
- Stepのedge位置とdense fallbackを回帰させない
- 一方のソースにデータがないX区間でpanic・不正Y rangeが起きない
- Pan/Zoomで両ソースが同じdisplay time範囲を使用する

### Verification

- Existing envelope/Step tests
- New unit: source display/raw range conversion
- New integration: different sample count/rate overlay
- Manual:
  - same-name Line overlay
  - different-name overlay
  - Line + Step mixed row
  - full-range pan/zoom
  - one-source-only time region
- `cargo fmt --check`
- `cargo test`
- `cargo run --release`

### Risk

- Rustのborrow checkerにより、rows走査中に複数source cacheを可変参照しづらい

### Mitigation

- View rowsを描画用snapshotとしてcloneする現行方針を維持する
- source idごとに処理をまとめる、または短い可変借用スコープを使う
- cache contextをworkspace全体で共有せず、source内で完結させる

### Expected files

- `apps/panel-y/src/main.rs`
- `apps/panel-y/src/parquet_waveform.rs`

---

## 11. M5 — Hover, Cursor, Statistics, and FFT Safety

この節はPhase 1時点の記録。異なるソースの選択をRun前に拒否する条件は、後続のFFT重ね表示で
ソース別独立ジョブへの分割に置き換える。計算1ジョブのmixed-source guardは維持する。
現在の仕様と検証状況は19.5節と[FFT overlay実行結果](./rust_rfq_1_multi_source_fft_overlay_results.md)を参照。

### Goal

- 複数ソース表示時にも値読出しと区間統計を正しく行う
- FFTが異なるtime/dtのデータを暗黙に混在させない

### Tasks

- Hover位置をdisplay timeで保持する
- チャンネルごとにHoverをraw timeへ逆変換する
- 所属ソースのtime/valueでnearest/held valueを計算する
- ソースのdisplay range外では `None` を返す
- Cursor A/Bをdisplay timeで保持する
- Cursor readoutを `ChannelKey` で一意化する
- 区間統計をソースごとのraw A/B sample rangeで計算する
- 統計キャッシュキーへ `ChannelKey` とsource generation/time fingerprintを含める
- FFT targetを `ChannelKey` 化する
- FFT対象がすべて同一sourceか検証する
- 異なるsource混在時はジョブを開始せずエラー表示する
- FFT request/result keyへsource identityを含める
- constant offsetがFFTの `dt` に影響しない構造を維持する
- stale FFT resultをsource変更後に適用しない

### Deliverables

- ソース別Hover
- ソース別Cursor値
- ソース別区間統計
- 1ジョブ1ソースのFFT guard
- source-aware analysis cache

### Completion criteria

- 同名チャンネルのHover/Cursor値を `#1/#2` で区別できる
- 異なるsample rateでも各ソースの正しいsampleを参照する
- ソース範囲外で端点値を誤表示しない
- 同一ソース複数チャンネルFFTは従来どおり実行できる
- 異なるソースのFFT混在は明確に拒否される
- source path変更後に古いFFT結果を表示しない

### Verification

- Unit: multi-source unique visible channels
- Unit: source-specific hover lookup
- Unit: source-specific interval statistics
- Unit: mixed-source FFT rejection
- Existing FFT reference peak tests
- Manual: A/B Cursorを同名2トレースへ適用
- `cargo fmt --check`
- `cargo test`

### Risk

- 既存解析helperが `&DatasetState` と単一 `shared_time` を広く前提としている

### Mitigation

- 解析関数へworkspace全体を渡して内部で曖昧に探索せず、`ChannelKey` からsource/time/channelを明示的に解決する
- FFT入力構築時にsource検証を最初に行う

### Expected files

- `apps/panel-y/src/main.rs`
- `apps/panel-y/src/fft_analysis.rs`

---

## 12. M6 — Persistence Guards and Operational Polish

### Goal

- Phase 1で未対応の永続化機能が、複数ソース状態を欠落させて保存しないようにする
- ソース追加・削除・失敗時のUI状態を実用レベルへ整える

### Tasks

- Plot config version 1 loadを単一ソースworkspace作成として維持する
- multi-source時はPlot config Save / Save Asを無効化する
- multi-source時は `Apply to Current Data` を無効化する
- Plot config Loadを実行する場合は、現在のworkspaceが単一ソースへ置き換わることをUIで明示する
- multi-source時はSession preset Save / Applyを無効化する
- 無効化理由をhover/statusで説明する
- multi-source状態をsource #1だけで暗黙保存しない
- source remove/path change後に次を再評価する
  - selected source
  - selected channel
  - selected row
  - x range
  - Hover/Cursor
  - FFT result
  - load status
- status/progressへsource番号を表示する
- Perf表示をaggregate値へ更新する
- READMEへPhase 1の制約を記載する

### Deliverables

- 不完全保存の防止
- version 1単一ソース互換
- 操作状態の整理
- Phase 1制約のUI説明

### Completion criteria

- 2ソース状態を1ソース設定として誤保存できない
- version 1 configはsource #1としてロードできる
- source削除後に無効なselection/analysis stateが残らない
- UI上で未対応理由が分かる
- 単一ソース時のPlot config機能は回帰しない

### Verification

- Existing plot_config tests
- New unit: multi-source persistence action guards
- Manual:
  - two-source -> Save disabled
  - two-source -> Load v1 -> one-source workspace
  - one-source -> Save/load round trip
- `cargo fmt --check`
- `cargo test`

### Risk

- 既存config操作が意図せず全ソースを消す

### Mitigation

- action可否を1つのhelperで判定する
- workspace置換操作とcurrent-data適用操作を明確に分ける

### Expected files

- `apps/panel-y/src/main.rs`
- `apps/panel-y/src/plot_config.rs`
- `apps/panel-y/README.md`

---

## 13. M7 — Integration, Performance Check, and Release Gate

### Goal

- Phase 1の受入条件を通し、実装を次フェーズの安定したベースにする

### Tasks

- 追加Unit testsを整理する
- 2ソースintegration scenarioを実行する
- 単一ソースregression scenarioを実行する
- DebugとReleaseの両方で起動確認する
- 2ソースのロード時間とメモリを確認する
- 広いX範囲でLine/Stepのpan/zoom応答を確認する
- cache hit/missがソース別に破綻していないことを確認する
- source add/remove/reloadを繰り返すstress操作を行う
- stale jobを再現するテストまたはUnit testを確認する
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- 必要に応じて `cargo clippy --all-targets`
- 実装後のREADMEと工程表statusを更新する

### Deliverables

- Phase 1 test results
- 性能確認結果
- 更新済みREADME
- Phase 1 closeout記録

### Completion criteria

- Release gateをすべて通過する
- blocker級の既知不具合がない
- 延期項目が暗黙対応や部分対応になっていない
- 後続の時間オフセットUIが `TimeTransform` 上へ追加できる

### Expected files

- `apps/panel-y/src/main.rs`
- `apps/panel-y/src/fft_analysis.rs`
- `apps/panel-y/src/plot_config.rs`
- `apps/panel-y/src/parquet_waveform.rs`
- `apps/panel-y/README.md`
- test fixture generator
- 本工程表

### Result

- Status: `complete` (2026-08-21)
- [M7実行結果](./rust_rfq_1_multi_source_phase1_m7_results.md)
- 2source integration/performance benchmark、167件の自動test、Debug/Release起動、
  M4–M6 GUI smokeの統合判定によりPhase 1 Release Gateを通過した

---

## 14. 工程依存関係

| Work item | Depends on | Blocks |
|---|---|---|
| Validation fixtures | none | M4, M5, M7 |
| `SourceId` / `ChannelKey` | none | all implementation |
| `TimeTransform` | identity model | M4, M5 |
| Workspace state | identity model | M2, M3 |
| Source-aware load jobs | workspace state | M3, M4 |
| Per-source ChannelStore | workspace state | M4, M5 |
| Unified channel labels | ChannelKey + source order | M3, M5 |
| Two-source UI | workspace/load API | manual integration |
| Multi-time rendering | load/cache + TimeTransform | M5 |
| Hover/Cursor/statistics | rendering/time model | release |
| FFT source guard | ChannelKey + source time | release |
| Persistence guards | source count/state | release |
| Release QA | all milestones | Phase 1 closeout |

原則としてM1とM2を完了する前にUI・描画へ2ソース分岐を追加しない。型と所有構造が不安定なままUIから着手すると、同じ変換を複数回やり直す可能性が高い。

---

## 15. 実装対象マップ

| File / area | Expected Phase 1 changes |
|---|---|
| `src/main.rs` state | workspace/source/channel identity |
| `src/main.rs` load | source-aware schema/channel jobs |
| `src/main.rs` UI | Data Sources、channel browser、labels、guards |
| `src/main.rs` rendering | source-specific time/cache、display range |
| `src/main.rs` analysis | Hover、Cursor、statistics cache |
| `src/fft_analysis.rs` | `ChannelKey`、same-source validation、stale key |
| `src/parquet_waveform.rs` | 原則再利用。必要ならtime/range helper追加 |
| `src/plot_config.rs` | version 1単一ソース互換の追従。v2保存は延期 |
| `src/workspace_data.rs` | 必要に応じてpure model/helperを分離 |
| `README.md` | multi-source Phase 1の利用方法と制約 |
| validation generator | 2ソースfixture生成 |

### 15.1 モジュール分割方針

Phase 1の主目的はマルチソース機能であり、`main.rs` 全体の再編を同時に完了条件へしない。

ただし、次のpure modelは新規モジュールへ分離してよい。

- `SourceId`
- `ChannelKey`
- `TimeTransform`
- display range union helper
- source numbering helper

`ChannelStore` の全面移動は、依存型が多く変更リスクが高い場合、Phase 1中は `main.rs` に残してよい。データ所有関係が `DataSourceState -> ChannelStore` になっていることを優先する。

---

## 16. テストマトリクス

### 16.1 Core identity

| Case | Expected |
|---|---|
| same source + same path | same `ChannelKey` |
| different source + same path | different `ChannelKey` |
| same key added twice to row | rejected |
| same path from #1 and #2 | accepted |

### 16.2 Load and lifecycle

| Case | Expected |
|---|---|
| #1 success, #2 success | both available |
| #1 success, #2 schema failure | #1 preserved |
| #2 channel failure | #1 and other #2 channels preserved |
| #2 path change | only #2 reset |
| #1 remove | #1 refs removed, old #2 becomes display #1 |
| stale #2 load result | ignored |

### 16.3 Rendering

| Case | Expected |
|---|---|
| same name / same sample rate | overlay |
| same name / different sample rate | overlay without resampling |
| different names / different time range | overlay on common display axis |
| current range outside one source | other source remains visible |
| Line + Line | min/max preserved |
| Step + Step | edges preserved |
| Line + Step | both modes render correctly |

### 16.4 Interaction and analysis

以下はPhase 1 M5の受入条件。FFT #1 + #2のRun前拒否は当時の仕様であり、
後続実装では独立ジョブへ分割して重ね表示する。1ジョブ内の混在入力は引き続き拒否する。

| Case | Expected |
|---|---|
| Hover in both ranges | both source values |
| Hover only in #1 range | #1 value, #2 missing |
| Cursor A/B | source-specific sample intervals |
| interval stats | distinct `#1/#2` rows |
| FFT targets all #1 | runs |
| FFT targets #1 + #2 | rejected before job start |

### 16.5 Persistence safety

| Case | Expected |
|---|---|
| one source Save | existing v1 behavior |
| two sources Save | disabled with reason |
| two sources Apply to Current Data | disabled with reason |
| load v1 config | workspace becomes source #1 |
| two sources Session preset Save | disabled with reason |

---

## 17. リスク一覧

| Risk | Impact | Mitigation |
|---|---|---|
| `channel_path`前提が広範囲 | 大規模な型変更 | M1で一括し、1ソース回帰を先に通す |
| 複数mutable source cache | borrow設計が複雑 | source単位の短い借用、描画用snapshot |
| 同名cache衝突 | 誤表示・データ上書き | ChannelStoreをsource内へ配置 |
| stale load result | 別ファイルへ古い値適用 | source generation検証 |
| X範囲を先頭traceから推測 | 空区間・異範囲で誤描画 | display rangeを描画APIへ明示 |
| Hoverの端点クランプ | 範囲外で偽の値 | source range外は `None` |
| FFTの異なるdt混在 | 誤解析 | same-source guard |
| configの部分保存 | 次回復元でデータ欠落 | multi-source SaveをPhase 1では無効化 |
| `#n`表示漏れ | ソース誤認 | presentation helperへ集約 |
| 2ソースで性能低下 | Pan/Zoom操作性低下 | source別cache再利用、M7計測 |

---

## 18. Phase 1 Release Gate

以下をすべて満たした時点でPhase 1を完了とする。

### Functional

- [x] 2つのParquetファイルを独立して登録できる
- [x] 各ソースのschemaを独立してロードできる
- [x] 各ソースからチャンネルをオンデマンドロードできる
- [x] 同名チャンネルを同一行へ追加できる
- [x] 異名チャンネルを同一行へ追加できる
- [x] すべてのチャンネル表示に常時 `#1/#2` が付く
- [x] 内部name/pathに `#1/#2` が混入しない
- [x] Line/Stepを2ソースで使用できる
- [x] 異なるsample count/sample rateを表示できる
- [x] Pan/Zoomが共通display timeで同期する
- [x] Fit Allが全ソース範囲の和集合を使用する
- [x] Hover/Cursor/統計がsource固有timeを使う
- [x] FFTが異なるsource混在を拒否する
- [x] source remove/path changeが他sourceを破壊しない
- [x] stale load/FFT resultが適用されない

### Safety

- [x] multi-source Plot config Saveが誤って部分保存しない
- [x] multi-source Session presetが誤って部分保存しない
- [x] version 1 Plot configの単一ソース互換を維持する
- [x] source range外の値を偽表示しない
- [x] 一方のsource failureが他方へ伝播しない

### Future readiness

- [x] 全time boundaryで `TimeTransform` を使用し、初期offsetは `0.0` とする
- [x] offset UI追加時にraw time配列を書き換える必要がない
- [x] 内部モデルが3ソース以上を表現できる
- [x] Row/Cache/Analysisのキーが `ChannelKey` で統一されている

### Quality

- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo test`
- [x] 既存106テスト相当が成功
- [x] Phase 1追加テストが成功
- [x] Debug/Releaseの手動確認が成功
- [x] 2ソースwide-range Pan/Zoomが実用上操作可能
- [x] `git diff --check`

---

## 19. 現在の次工程と未実装範囲

### 19.1 Phase 2 Time Alignment完了

時間オフセットUIは2026-08-21に実装し、2026-10-23にGUI smoke全10項目を完了した。
Phase 1で導入済みの `TimeTransform` を更新し、raw time、ChannelKey、ChannelStoreの
所有構造は変更しない。

- [x] Source別の時間offset編集UI
- [x] Source別 `Reset`
- [x] Source別 `Start at zero`
- [x] ロード済みSourceの `Start all at zero`
- [x] raw time / ChannelStore / source generationを変更しない自動テスト
- [x] Cursor表示時刻保持、統計再計算、FFT結果無効化
- [x] GUI smoke 10項目の確認と完了記録

実装・自動テスト結果とGUI smoke項目は
[Phase 2 Time Alignment実行結果](./rust_rfq_1_multi_source_phase2_time_alignment_results.md)
に記録する。

### 19.2 Fit Overlap完了

2026-10-24に実装・自動テスト・GUI確認を完了した。time読込済みSourceのoffset適用後の
共通区間へ表示範囲を合わせる。重なりなし・端点接触は無効化し、1ソース時は全範囲を表示する。
詳細は[Fit Overlap実行結果](./rust_rfq_1_multi_source_fit_overlap_results.md)を参照。

### 19.3 Plot config v2完了

2026-10-24に実装・185件の自動テスト・GUI確認を完了した。複数ソース、offset、全行の
表示設定を保存・復元し、v1互換、欠損部分復元、全欠損時のworkspace保持を確認した。
詳細は[Plot config v2実行結果](./rust_rfq_1_multi_source_plot_config_v2_results.md)を参照。
M6節のPlot config保存guardはPhase 1時点の記録であり、v2実装で解除済み。

### 19.4 複数ソースSession preset

2026-10-24に実装し、194件の自動テスト・ビルド・GUI全10項目が成功した。
ソース順・パス・ラベル・offset、
全行の表示設定、選択状態、X範囲、大規模データpreview設定を実行中のメモリ内へ保存する。
同ID・同パスでschemaを持つソースはrawデータを再利用し、削除済み/パス変更済みソースは
schemaを事前確認してIDを再割当する。欠損は警告付き部分復元、全欠損は現状保持とする。
未読込の表示チャンネルを非同期ロードした後、保存したX範囲へ戻す。
繰り返しApplyで復元済みソースのID対応を保持し、全非表示時も保存X範囲を維持する。
この2点の回帰修正も自動テストで確認した。アプリ終了後の保持は対象外。
詳細と検証結果は[Session preset実行結果](./rust_rfq_1_multi_source_session_preset_results.md)を参照。
M6節のSession preset guardはPhase 1時点の記録であり、複数ソース対応で解除済み。

### 19.5 ソース別FFT結果の重ね表示

2026-10-24に実装し、205件の自動テスト、fmt/build/diffチェック、GUI全10項目が成功した。
全ソース合計で最大3対象をソースごとの独立ジョブへ分割し、
各ソースのraw time・dt・native frequency binsを保持して共通Hz/振幅/dB軸へ重ねる。
Cursor A/Bは各ソースのoffsetでraw区間へ変換する。ソース別N・dt・df・peakと色凡例を表示し、
Hoverは各ソースの近傍binを使い、Nyquist外の値を補わない。

片方のソースが失敗しても他方の成功結果は表示する。Cursor/offset/window/targets変更、
Cursorを消すSource削除/path変更では全結果を無効化し、Cancelとrequest identityでstale結果を防ぐ。
M5の計算1ジョブのmixed-source guardは維持する。Cancelの競合は決定的な自動テストで確認した。
自動テスト・GUI結果は
[FFT overlay実行結果](./rust_rfq_1_multi_source_fft_overlay_results.md)に記録した。

### 19.6 3ソース以上のUI

2026-10-24に実装し、208件の自動テストとfmt/build/diffチェックが成功した。
GUI全10項目が合格し、画面証跡11枚を保存した。3ソースFFTでは各ソースのnative binsとdtを保持した重ね表示・Hoverを確認した。
初期UIの2ソース制限を撤廃し、固定のソース数上限を設けずに追加できる。
`Add Source` と `Start all at zero` をソース一覧の上に置き、カード一覧には高さを制限した
縦スクロール領域を使う。

カード・コントロールは安定した `SourceId` を使って識別する。中間ソースの削除後は
表示番号を詰め、残ったソースのIDとチャンネル参照を保持する。
FFT対象数は登録ソース数にかかわらず全ソース合計で最大3チャンネルを維持する。
詳細は[3ソース以上のUI実行結果](./rust_rfq_1_multi_source_ui_results.md)を参照。

### 19.7 次の実装候補

自動整列（トリガー・相互相関）、ソース別名編集、ソース順並び替え、異なるソース間の
リサンプリングも未実装であり、実装順序・解析仕様は別途検討する。

---

## 20. 関連資料

- [RFQ #1 — 複数時系列データ重ね描き設計書](./rust_rfq_1_multi_source_overlay_design.md)
- [Rust Migration Phase 2 Design](./rust_migration_phase2_design.md)
- [Rust Migration Phase 3 — Plot Config Detailed Design](./rust_migration_phase3_plotconfig_design.md)
- [Native viewer README](../apps/panel-y/README.md)
