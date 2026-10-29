---
project: Panel_y
doc_type: Execution Prompt
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M1"
target: Identity and single-source-compatible state model
created: "2026-07-25"
updated: "2026-07-25"
status: ready to use
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m0_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — マルチデータ読み込み対応 Phase 1 / M1 実行指示プロンプト

以下の指示に従い、Panel-y Rust版の
「M1 — Identity and Single-Source-Compatible State Model」だけを実行してください。

---

## 1. 役割

あなたは `/Users/ochiba/newspace/life/factory/panel-y` を対象に作業する
実装エージェントです。

今回の目的は、画面やロード対象を2ソースへ増やすことではありません。
現行の単一ソース動作を維持したまま、内部のチャンネル参照を
`(SourceId, channel_path)` で一意に識別できる状態モデルへ移行することです。

このターンではM1を完了させてください。調査や型定義の追加だけで終了せず、
既存のRow、選択状態、解析、Session preset、Plot config境界を単一ソース互換の
新しいidentity modelへ接続し、テストで検証してください。

---

## 2. 最初に読む資料

作業前に次を最後まで読んでください。

1. `AGENTS.md`
2. `docs/rust_rfq_1_multi_source_overlay_design.md`
3. `docs/rust_rfq_1_multi_source_phase1_work_plan.md`
4. `docs/rust_rfq_1_multi_source_phase1_m0_execution_prompt.md`
5. `docs/rust_rfq_1_multi_source_phase1_m0_results.md`
6. `code/rust_phase1/README.md`
7. `code/rust_phase1/src/main.rs`
8. `code/rust_phase1/src/fft_analysis.rs`
9. `code/rust_phase1/src/plot_config.rs`

`journal/` は参照・編集しないでください。

---

## 3. M1開始条件

次を確認してから実装してください。

- M0結果文書のstatusが `complete` である
- 2ソースfixture generatorが存在する
- 現行 `cargo test` が成功する
- `main.rs`、`fft_analysis.rs`、`plot_config.rs` に、今回のM1変更と競合する
  既存未コミット変更がない
- M0の未コミット成果物が残っている場合、それらを上書き・削除しない

M0成果物が未コミットであること自体は停止理由ではありません。ただし、
M1対象コードと既存変更が重なり、安全に所有範囲を分離できない場合は停止して
報告してください。

---

## 4. M1のゴール

次の状態を作ってください。

- `SourceId` がファイルpathや画面番号から独立した内部identityとして存在する
- `ChannelKey { source_id, channel_path }` がView/Row内のチャンネルidentityになる
- `TimeTransform` がraw/display time変換を表現し、初期値 `0.0` を持つ
- `WorkspaceDataState` がソースcollectionと表示順を保持する
- 起動時は既存default Parquetを持つSource #1だけが存在する
- `RowChannel` と `ViewState.selected_channel` が `ChannelKey` を使用する
- 同じ `ChannelKey` の行内重複を拒否し、異なるsourceの同じpathは区別できる
- 既存単一ソースのschema、load、Row、Line/Step、Hover、Cursor、FFT、
  Session preset、Plot config version 1が回帰しない
- 実際に登録・表示できるソース数はまだ1つのままである

---

## 5. 厳守するスコープ

### 実施すること

- Git状態とM0 baselineの確認
- M1変更前の `cargo test`
- `SourceId`、`ChannelKey`、`TimeTransform` の実装
- `DataSourceState`、`WorkspaceDataState` の実装
- default datasetをSource #1として初期化
- `PanelYApp` のデータ所有をworkspace modelへ移行
- `RowChannel.channel_path` を `RowChannel.key` へ置換
- `ViewState.selected_channel: String` を `Option<ChannelKey>` へ置換
- 行内重複判定とView helperを `ChannelKey` 対応
- workspace/source/channel lookup accessorの追加
- 現行単一ソースのUI、描画、Hover、Cursor、統計、FFT呼出箇所の追従
- Session presetとPlot config version 1の単一ソース変換境界の追従
- pure modelと単一ソース回帰のUnit test追加
- M1実行結果文書の作成
- `cargo fmt --check`、`cargo check`、`cargo test`、`git diff --check`

### 実施しないこと

- `Add Source`、Source #2 path、Remove、並び替えなどの2ソースUI
- 実データを2ソース同時に登録・ロードする処理
- load job/resultへのsource id / generation追加
- source-aware stale load result判定
- 複数sourceの並列schema/channel load
- 複数sourceのtime/channel/cache動作
- `ChannelPresentation` や常時 `#1/#2` 表示
- `Add matching channel from all sources`
- 複数時刻配列のLine/Step描画
- display range union、`Fit All`、`Fit Overlap`
- Hover/Cursorのsource別raw time変換
- 異なるsourceを混在したFFTの拒否処理
- 複数ソースPlot config / Session preset
- Plot config formatやversionの変更
- 時間オフセット編集UI
- Cargo依存関係の追加
- M0 fixture仕様やgeneratorの変更
- 既存テストの削除・緩和
- `journal/` の参照・編集
- branch作成、commit、push、PR作成

M2以降の変更が必要に見えても、このターンでは実装せず、M1結果文書の
「M2への申し送り」へ記録してください。

---

## 6. 作業開始時の確認

リポジトリルートで、少なくとも次を確認してください。

```bash
git status --short --branch
git branch -vv
git rev-parse HEAD
git rev-parse main
git rev-parse origin/main
git log -1 --oneline --decorate
git diff --check
rustc --version
cargo --version
```

さらにM1対象ファイルの既存差分を確認してください。

```bash
git diff -- code/rust_phase1/src/main.rs
git diff -- code/rust_phase1/src/fft_analysis.rs
git diff -- code/rust_phase1/src/plot_config.rs
```

ルール:

- 既存の未コミット変更の所有者を推測して上書きしない
- `git fetch`、`git pull`、remote writeは行わない
- M0成果物を削除、再生成、force addしない
- baselineのSHA、branch、worktree状態をM1結果へ記録する

---

## 7. 変更前ベースライン

`code/rust_phase1` で実行してください。

```bash
cargo test
```

実際の出力を正として、次を記録してください。

- 成功・失敗
- passed / failed / ignored / filtered out
- 実行日
- HEAD commit
- rustc / cargo version

工程表とM0では106テストを記録していますが、数を決め打ちしないでください。

変更前から再現性のある失敗がある場合は、M1コードで無関係な修正をせず、
停止条件に従って報告してください。

---

## 8. Identity model

### 8.1 `SourceId`

設計書に従い、安定した内部IDを追加してください。

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SourceId(u64);
```

必要な箇所で `Hash` などのderiveを追加して構いませんが、次を守ってください。

- Parquet pathをIDにしない
- `#1` の表示番号をIDにしない
- collection内のindexを永続的なIDとして扱わない
- default sourceは安定した1つの `SourceId` を持つ
- 次のsource idを再利用しない単調なcounter構造にする

### 8.2 `ChannelKey`

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ChannelKey {
    source_id: SourceId,
    channel_path: String,
}
```

共通constructorまたはhelperを設け、UIやテストで組み立て方を分散させないで
ください。

ルール:

- `channel_path` はParquet内の元pathを保持する
- `#1` / `#2` を `channel_path` へ付加しない
- 同じsource + 同じpathは同じkey
- 異なるsource + 同じpathは異なるkey
- `parquet_schema::ChannelSummary` やParquet reader内部のpathは変更しない

### 8.3 `TimeTransform`

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
struct TimeTransform {
    offset_seconds: f64,
}

impl TimeTransform {
    fn to_display(self, raw_time: f64) -> f64;
    fn to_raw(self, display_time: f64) -> f64;
    fn raw_range(self, display_range: (f64, f64)) -> (f64, f64);
    fn display_range(self, raw_range: (f64, f64)) -> (f64, f64);
}
```

変換式:

```text
display_time = raw_time + offset_seconds
raw_time     = display_time - offset_seconds
```

要件:

- defaultは `offset_seconds = 0.0`
- raw/displayの往復が成立する
- range変換は両端へ同じ変換を適用する
- 正負のoffsetをUnit testする
- M1ではoffset編集UIを追加しない
- M1では既存描画へ多ソース時刻分岐を追加しない
- 時刻配列そのものを書き換えない

---

## 9. Workspace state model

設計書の所有関係を基本としてください。

```rust
struct DataSourceState {
    id: SourceId,
    label: String,
    parquet_path: String,
    schema: Option<SchemaSummary>,
    shared_time: Option<TimeData>,
    loaded_channels: ChannelStore,
    time_transform: TimeTransform,
    generation: u64,
}

struct WorkspaceDataState {
    sources: BTreeMap<SourceId, DataSourceState>,
    source_order: Vec<SourceId>,
    selected_source_id: Option<SourceId>,
    next_source_id: u64,
    recent_parquet_paths: Vec<String>,
}
```

型名やprivate helperの細部は現行コードに合わせて調整して構いませんが、
責務を変えないでください。

### 9.1 M1での初期状態

- workspaceにはSource #1だけを作る
- Source #1のpathは現行 `default_parquet_path()`
- `source_order` はSource #1のIDを1件だけ持つ
- `selected_source_id` はSource #1
- `next_source_id` は次に未使用の値
- Source #1の `TimeTransform` はdefault 0.0
- 現行のschema、shared time、ChannelStore、recent pathの単一ソース動作を維持
- UIからsource collectionを増減するAPIはまだ公開しない

`schema`、`shared_time`、`loaded_channels` をSource #1の
`DataSourceState`へ移す場合も、M1ではロードjob/resultをsource-aware化しないで
ください。現行jobは常に選択中の唯一のSource #1へ適用する暫定境界とし、
実際の複数source loadとgeneration検証はM2へ残してください。

### 9.2 必須accessor

少なくとも次と同等の共通accessorを用意してください。

```rust
fn source(&self, source_id: SourceId) -> Option<&DataSourceState>;
fn source_mut(&mut self, source_id: SourceId) -> Option<&mut DataSourceState>;
fn selected_source(&self) -> Option<&DataSourceState>;
fn selected_source_mut(&mut self) -> Option<&mut DataSourceState>;
fn channel(&self, key: &ChannelKey) -> Option<&ChannelData>;
fn has_channel(&self, key: &ChannelKey) -> bool;
fn source_number(&self, source_id: SourceId) -> Option<usize>;
```

ルール:

- 呼出側で `sources.get(...).loaded_channels...` を反復しない
- `source_number()` は `source_order` に基づく1-originの表示番号を返す
- 存在しないsource/keyはpanicせず `None` / `false`
- raw channelのflatなworkspace-level cacheを新設しない
- 同一source内の `ChannelStore` は引き続きpathを内部keyにしてよい

---

## 10. View / Row migration

### 10.1 `ViewState.selected_channel`

次へ移行してください。

```rust
selected_channel: Option<ChannelKey>
```

要件:

- schema load時はSource #1の最初のchannelをkeyとして選択
- schemaが空なら `None`
- dataset reset時は `None`
- schema filter/sortそのものは元path/nameを使用
- UIの表示文言はM1では現行相当を維持し、`#1` suffix追加はM3へ延期
- stringのempty sentinelを残さない

### 10.2 `RowChannel`

次へ移行してください。

```rust
struct RowChannel {
    key: ChannelKey,
    // existing style fields...
}
```

要件:

- 同じ行への同じ `ChannelKey` の重複追加を拒否
- 異なるSourceIdで同じpathのkeyは同じ行へ追加可能
- color index、Line/Step、visible、override color、line widthを維持
- 行追加、削除、選択の既存挙動を維持
- loaded data参照はworkspace accessorを通してkeyから解決
- pathだけが必要なParquet/ChannelStore境界では、解決済みsource内で
  `key.channel_path` を渡す

### 10.3 helper signature

View helperは可能な限り `&ChannelKey` または `ChannelKey` を受け取る形へ
変更してください。

対象例:

- `add_channel_to_row`
- `add_channel_to_selected_row`
- duplicate判定
- visible channelの一意化
- Cursor readout / interval statistics用channel収集
- unload/cache controlへ渡す選択identity

schemaの検索やParquet readerのprojectionなど、単一ファイル内部pathが責務の
helperまで無理に `ChannelKey` 化しないでください。

---

## 11. Load / rendering / analysisのM1境界

M1では実source数を増やしません。既存処理を次の境界で追従させてください。

### 11.1 Load

- UIの選択とRow追加要求は `ChannelKey`
- Parquet readerへ渡す直前にSource #1とpathを解決
- load完了後のChannelStore格納は解決済みSource #1内でpathを使用
- `ActiveLoadJob` / `LoadJobResult` のsource id / generation追加はM2へ延期
- 同時にSource #2をロードできるようにしない

M1の暫定single-source boundaryはコード上で明確にし、複数箇所へ
`source_order[0]` の直接参照を散らさないでください。

### 11.2 Rendering / Hover / Cursor / statistics

- RowChannelのkeyからSource #1のtime/channel/cacheを解決する
- 既存のraw time動作と結果を維持する
- M1ではdisplay/raw range変換やrange unionを描画へ追加しない
- source range外の欠損処理などM4/M5の仕様を先行実装しない
- cache keyをworkspace全体の `ChannelKey` に作り替える作業はM2以降へ残す

### 11.3 FFT

内部の選択identityやrequest identityがViewのpath文字列だけに依存している箇所は
`ChannelKey` 対応へ移行してください。ただしM1では全対象がSource #1である。

- FFTの数値計算、window、limit、描画仕様を変更しない
- 同一ソースFFTの既存結果を維持する
- mixed-source validation/errorはM5へ延期
- serializedされない内部keyにはsource identityを保持する
- spectrumの元channel name/pathは表示・診断用に維持してよい
- stale FFT resultの既存テストを回帰させない

---

## 12. Session preset / Plot config境界

### 12.1 基本方針

M1では外部保存形式を変更しません。

- Plot config top-level `version: 1` を維持
- Rust extension versionを変更しない
- JSONのchannel pathは従来どおり文字列
- Source #1以外を保存する仕様を追加しない
- `#1` suffixを保存値へ入れない

### 12.2 Load

legacy pathを読み込むときは、現在または新規のSource #1の `SourceId` を明示的に
渡し、`ChannelKey` / `RowChannel` へ変換してください。

変換を `SourceId(1)` の直書きとして複数箇所へ散らさず、workspaceまたは
apply planの境界で一元化してください。

### 12.3 Save

内部の `ChannelKey` からSource #1であることを確認し、元の
`channel_path` だけを既存version 1形式へ出力してください。

M1は1ソースだけなので既存save/load round tripを維持します。
複数source時のsave guardはM6で実装するため、M1でformat version 2や
部分的なmulti-source保存を追加しないでください。

### 12.4 Session preset

Session presetの内部channel参照をkeyへ移行するか、Source #1との変換境界を
明示してください。いずれの場合も次を守ってください。

- 現行単一ソースのcapture/applyを維持
- schemaにないchannelのskip/reportを維持
- selected channelとRow channelのSource #1 identityを失わない
- multi-source presetは実装しない

---

## 13. 必須Unit test

既存テストを削除・緩和せず、少なくとも次を追加または更新してください。

### Identity

- 同じsource + 同じpathの `ChannelKey` は等しい
- 異なるsource + 同じpathの `ChannelKey` は異なる
- 同じsource + 異なるpathの `ChannelKey` は異なる
- pathや表示番号をSourceIdとして使用していない

### TimeTransform

- offset 0.0のraw/display変換
- 正のoffsetのto_display / to_raw
- 負のoffsetのto_display / to_raw
- raw/displayの往復
- raw/display rangeの往復

### Workspace

- default workspaceはSource #1を1件だけ持つ
- source orderと `source_number()` が一致する
- 存在しないsourceは `None`
- default sourceのtime transformが0.0
- channel accessorがsourceを誤って横断検索しない

### View / Row

- 同じ `ChannelKey` の行内重複を拒否
- 異なるsourceの同じpathを同じ行へ追加できる
- selected channelは `Option<ChannelKey>`
- Row追加・削除・選択の既存テストが成功
- visible channelの一意化がkey単位

### Compatibility

- Session presetの単一ソースround trip
- Plot config version 1のloadでSource #1 keyへ変換
- Plot config version 1のsaveで元pathだけを出力
- existing FFT reference peak / stale result tests

テストの都合だけで本番APIへ不要なpublic surfaceを増やさないでください。

---

## 14. 単一ソース回帰確認

CLIで少なくとも次を実行してください。

```bash
cargo run -- --schema ../proto_3_1b/data/test_100k.parquet
cargo run -- --load-channel ../proto_3_1b/data/test_100k.parquet sine_50Hz
```

GUIを利用できる場合は次を確認してください。

1. Rustアプリを起動する
2. `test_100k.parquet` のschemaをロードする
3. 複数チャンネルを1行へ追加する
4. 同じチャンネルの行内重複が拒否される
5. Line / Stepを切り替える
6. Row追加・削除を行う
7. Pan / Zoom / Box zoom / Reset X Rangeを行う
8. Hover値を確認する
9. Cursor A/Bと区間統計を確認する
10. 同一ソース内FFTを実行する
11. Session presetのcapture/applyを確認する
12. Plot config version 1のsave/loadを確認する

GUIを操作できない場合は未実施と明記し、実施したと記録しないでください。
M1ではSource #2のGUI確認を行いません。

---

## 15. M1実行結果文書

次のファイルを新規作成してください。

```text
docs/rust_rfq_1_multi_source_phase1_m1_results.md
```

front matter:

```yaml
---
project: Panel_y
doc_type: M1 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M1"
created: "<実行日>"
updated: "<実行日>"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m0_results.md"
implementation: "code/rust_phase1"
---
```

本文へ最低限記録する内容:

1. M1の目的とスコープ
2. Git baselineと開始時worktree
3. 変更前 `cargo test`
4. 実装したidentity model
5. workspace / Source #1初期化
6. View / Row移行
7. Load / rendering / analysisのsingle-source boundary
8. Session preset / Plot config version 1互換
9. 追加・更新したテスト
10. CLI確認結果
11. GUI手動確認の実施・未実施
12. 既知の制約
13. M2への申し送り
14. M1完了判定

テスト数、commit SHA、CLI結果を推測で記載しないでください。

---

## 16. 検証と品質確認

編集後に `code/rust_phase1` で実行してください。

```bash
cargo fmt --check
cargo check
cargo test
```

リポジトリルートで実行してください。

```bash
git diff --check
git status --short
```

さらに次を確認してください。

- M0成果物とfixtureを変更していない
- Cargo依存関係を変更していない
- 実際のSource #2を追加していない
- 2ソースUIや `#1/#2` 表示を追加していない
- load job/resultをM2仕様へ先行変更していない
- Plot configのversion/JSON shapeを変更していない
- `channel_path` だけでRow/Viewの所有sourceを識別する箇所が残っていない
- Parquet/schema/ChannelStore内部の正当なpath使用まで誤ってkey化していない
- `TimeTransform` のUnit testがある
- 既存テストを削除・緩和していない
- 最終テスト結果をM1結果文書へ反映した

`rg` で残存参照を確認してください。

```bash
rg -n "selected_channel: String|struct RowChannel|channel_path" \
  code/rust_phase1/src/main.rs \
  code/rust_phase1/src/fft_analysis.rs \
  code/rust_phase1/src/plot_config.rs
```

`channel_path` 自体はParquet、schema、ChannelStore、legacy config境界で正当に
残ります。機械的に0件を目標にせず、各残存箇所の責務を確認してください。

---

## 17. 想定する変更ファイル

原則として変更は次に限定してください。

```text
code/rust_phase1/src/main.rs
code/rust_phase1/src/fft_analysis.rs
code/rust_phase1/src/plot_config.rs
docs/rust_rfq_1_multi_source_phase1_m1_results.md
```

pure modelを分離する場合は、次を追加して構いません。

```text
code/rust_phase1/src/workspace_data.rs
```

`workspace_data.rs` を追加する場合は、`SourceId`、`ChannelKey`、
`TimeTransform`、source numberingなど、依存の少ないmodel/helperを優先して
ください。`main.rs` 全体の再編やChannelStoreの全面的なmodule移動を
M1の完了条件にしないでください。

README変更は、M1の単一ソース内部移行にユーザー向け操作変更がない限り不要です。
追加変更が必要な場合はM1結果文書で理由を説明してください。

次は変更しないでください。

```text
code/rust_phase1/Cargo.toml
code/rust_phase1/Cargo.lock
code/rust_phase1/src/parquet_schema.rs
code/rust_phase1/src/parquet_waveform.rs
code/proto_3_1b/generate_multi_source_validation_data.py
```

---

## 18. 停止条件

次の場合は、勝手にスコープを広げず停止して報告してください。

- M1開始前から `cargo test` が再現性をもって失敗する
- M1対象コードに所有者不明の既存変更があり、安全に統合できない
- 単一ソース互換を維持するためにM2のsource-aware load job実装が必要になる
- M1完了に2ソースUIまたは複数時刻描画が必要になる
- Plot config version 1互換の維持にformat変更が必要になる
- M0 fixtureやCargo依存関係の変更が必要になる
- `ChannelKey` 化により既存FFT、Session preset、Plot configの回帰を解消できない
- 既存テストを削除・緩和しないと完了できない

停止報告には次を含めてください。

- 実行したコマンド
- 観測したエラー
- 原因の切り分け
- 変更済みファイル
- 未完了項目
- M2以降へ送るべき内容

---

## 19. 完了条件

次をすべて満たした場合だけM1をcompleteとしてください。

- [ ] M0がcompleteであることを確認した
- [ ] Git/toolchain/test baselineを記録した
- [ ] `SourceId` を追加した
- [ ] `ChannelKey` を追加した
- [ ] `TimeTransform` とraw/display変換を追加した
- [ ] `DataSourceState` / `WorkspaceDataState` を追加した
- [ ] default datasetをSource #1として初期化した
- [ ] `ViewState.selected_channel` を `Option<ChannelKey>` へ移行した
- [ ] `RowChannel` を `ChannelKey` 対応にした
- [ ] 同じkeyの重複拒否と異なるsourceの同path許可をテストした
- [ ] workspace accessorを追加した
- [ ] 単一ソースのLoad / Line / Step / Hover / Cursor / FFTが回帰していない
- [ ] Session presetが単一ソースで回帰していない
- [ ] Plot config version 1のsave/loadが回帰していない
- [ ] 2ソースUI・複数source loadへ着手していない
- [ ] Cargo依存関係を変更していない
- [ ] `cargo fmt --check` が成功した
- [ ] `cargo check` が成功した
- [ ] `cargo test` が成功した
- [ ] `git diff --check` が成功した
- [ ] M1実行結果文書を実測値で作成した

---

## 20. 最終報告フォーマット

最終応答は次の順序で簡潔に報告してください。

1. M1完了 / blocker
2. 作成・変更したファイル
3. Git baseline
4. 変更前・変更後の `cargo test` 結果
5. 実装した `SourceId` / `ChannelKey` / `TimeTransform`
6. workspaceとSource #1初期化
7. View / Row / FFT / config互換の結果
8. CLI / GUI確認結果
9. M2への申し送り
10. commit / pushを行っていないこと

成果物へのローカルファイルリンクを付けてください。
