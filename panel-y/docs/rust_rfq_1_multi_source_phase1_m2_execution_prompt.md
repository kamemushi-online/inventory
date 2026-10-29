---
project: Panel_y
doc_type: Execution Prompt
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M2"
target: Source-aware load jobs and per-source caches
created: "2026-07-25"
updated: "2026-07-25"
status: ready to use
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m1_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — マルチデータ読み込み対応 Phase 1 / M2 実行指示プロンプト

以下の指示に従い、Panel-y Rust版の
「M2 — Source-Aware Load Jobs and Per-Source Caches」だけを実行してください。

---

## 1. 役割

あなたは `/Users/ochiba/newspace/life/factory/panel-y` を対象に作業する
実装エージェントです。

今回の目的は、M1で導入した `SourceId` / `ChannelKey` /
`WorkspaceDataState` をロード処理へ接続し、複数ソースのschema、time、
channel、draw cache、非同期結果をデータ層で安全に分離することです。

このターンではM2を完了させてください。型定義やAPI追加だけで終了せず、
同じchannel pathを持つ2ソースの独立キャッシュ、source generationによる
stale result破棄、ソース単位reset/failure隔離をUnit testで検証してください。

M2はデータ層の工程です。2ソースUI、複数時刻描画、Hover/Cursorの
source別time変換、mixed-source FFT guardは実装しません。

---

## 2. 最初に読む資料

作業前に次を最後まで読んでください。

1. `AGENTS.md`
2. `docs/rust_rfq_1_multi_source_overlay_design.md`
3. `docs/rust_rfq_1_multi_source_phase1_work_plan.md`
4. `docs/rust_rfq_1_multi_source_phase1_m0_execution_prompt.md`
5. `docs/rust_rfq_1_multi_source_phase1_m0_results.md`
6. `docs/rust_rfq_1_multi_source_phase1_m1_execution_prompt.md`
7. `docs/rust_rfq_1_multi_source_phase1_m1_results.md`
8. `code/rust_phase1/README.md`
9. `code/rust_phase1/src/main.rs`
10. `code/rust_phase1/src/workspace_data.rs`
11. `code/rust_phase1/src/parquet_schema.rs`
12. `code/rust_phase1/src/parquet_waveform.rs`
13. `code/rust_phase1/src/fft_analysis.rs`
14. `code/rust_phase1/src/plot_config.rs`

`journal/` は参照・編集しないでください。

特に設計書の次を確認してください。

- `5.3 DataSourceState`
- `5.4 WorkspaceDataState`
- `8. ロード処理`
- `9. キャッシュ設計`
- `15. エラー処理`
- `16. パフォーマンス方針`
- `17. Step 2: ソース別ロードとキャッシュ`
- `18. テスト計画`

---

## 3. M2開始条件

次を確認してから実装してください。

- M0結果文書のstatusが `complete` である
- M1結果文書のstatusが `complete` である
- M1のレビュー指摘であるParquet path手入力時の
  `set_parquet_path()` 経由更新がコードへ反映済みである
- `SourceId`、`ChannelKey`、`TimeTransform` が存在する
- `DataSourceState` がschema、shared time、`ChannelStore`、generationを所有する
- `WorkspaceDataState` がsource collection、source order、selected source、
  monotonic source id counterを所有する
- M0の2ソースfixture generatorが存在する
- 現行 `cargo test` が成功する
- M2対象コードに、M1完了後の所有者不明変更が重なっていない

M0/M1成果物が未コミットで残っていること自体は停止理由ではありません。
M1の変更はM2の前提として保持し、巻き戻し・再実装・削除しないでください。

開始時にM1対象差分と今回の追加差分を区別できない場合、または安全に統合できない
所有者不明変更がある場合は、実装を開始せず停止条件に従って報告してください。

---

## 4. M2のゴール

次の状態を作ってください。

- workspaceの非UI APIで複数の `DataSourceState` を追加・参照・reset・removeできる
- schema load APIが `SourceId` を明示的に受け取る
- channel load requestが `ChannelKey` を明示的に受け取る
- `ActiveLoadJob` と `LoadJobResult` がsource idとsource generationを保持する
- loading判定がpathではなく `ChannelKey` 単位になる
- 同じpathの `(source #1, "iu")` と `(source #2, "iu")` を同時にロードできる
- 各sourceが異なるtime配列とsample countを同時に保持できる
- result適用時にjob、source、generation、channel pathを検証する
- pathを別値へ変更して元へ戻した場合も、古いgenerationのresultを破棄する
- 一方のsourceのschema/channel failure、path change、reset、removeが
  他方のsource data/cacheを破壊しない
- memory/cache集計がworkspace内の全sourceを対象にする
- 現行の単一ソースUI、描画、Hover、Cursor、FFT、Session preset、
  Plot config version 1が回帰しない
- UIから登録・操作できるsource数はまだ1つのままである

---

## 5. 厳守するスコープ

### 実施すること

- Git状態、M1 baseline、toolchainの確認
- M2変更前の `cargo test`
- source追加・参照・reset・removeに必要な非UI workspace API
- source id指定のschema load境界
- `ChannelKey` 指定のchannel load request
- source id / generation付きのactive jobとresult
- source-aware loading判定とloading key collection
- source単位のschema/time/channel/draw cache mutation
- stale result判定のsource/generation対応
- source単位のfailure/reset/remove隔離
- 全sourceのmemory/cache count集計helper
- M2のpure model / load lifecycle Unit test
- M0 fixtureを使ったCLI回帰確認
- 単一ソースの既存回帰確認
- M2実行結果文書の作成
- `cargo fmt --check`、`cargo check`、`cargo test`、`git diff --check`

### 実施しないこと

- `Data Sources` UI
- `Add Source` / Remove / source selectorなどの画面操作
- UI上限2ソースの実装
- Channel browserのsource別グループ表示
- `channel_name #1` / `channel_name #2` のpresentation
- Row controls、plot title、Hover、Cursor、FFT labelへの `#n` 追加
- Source #2のRow追加を可能にするUI
- 複数source timeを同時描画する処理
- display range union、`Fit All`、`Fit Overlap`
- `TimeTransform` の描画接続やoffset編集UI
- source別Hover / Cursor / interval statistics
- mixed-source FFT拒否
- source削除時の全View/Row/analysis cleanup
- 複数source Plot config / Session preset
- persistence guard
- Plot config format/version変更
- Cargo依存関係追加
- M0 fixture仕様やgenerator変更
- 既存テストの削除・緩和
- `journal/` の参照・編集
- branch作成、commit、push、PR作成

M2の非UI APIをテストから使用して2ソース状態を作ることはスコープ内です。
ただし、そのAPIを新しい画面操作へ接続する作業はM3へ残してください。

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

M2対象ファイルとCargo差分を確認してください。

```bash
git diff -- code/rust_phase1/src/main.rs
git diff -- code/rust_phase1/src/workspace_data.rs
git diff -- code/rust_phase1/src/parquet_waveform.rs
git diff -- code/rust_phase1/Cargo.toml
git diff -- code/rust_phase1/Cargo.lock
```

ルール:

- M0/M1の未コミット成果物を削除・上書きしない
- 既存変更の所有者を推測しない
- M1の差分を今回のM2変更と誤認して巻き戻さない
- `git fetch`、`git pull`、remote writeを行わない
- M0 fixtureをforce addしない
- baseline SHA、branch、開始時worktreeをM2結果へ記録する

---

## 7. 変更前ベースライン

`code/rust_phase1` で実行してください。

```bash
cargo test
```

実際の出力を正として次を記録してください。

- 成功・失敗
- passed / failed / ignored / filtered out
- 実行日
- HEAD commit
- rustc / cargo version

M1結果では117 testsを記録していますが、数を決め打ちしないでください。
レビュー対応や他の正当な変更で増減している場合は、開始時の実測値を使用します。

変更前から再現性のある失敗がある場合は、M2で無関係な修正をせず停止して
報告してください。

---

## 8. Workspace source lifecycle API

M3のUIから使用できる、source collectionの非UI APIを追加してください。
型名や戻り値の細部は既存コードへ合わせて構いませんが、少なくとも次と同等の
責務を持たせます。

```rust
fn add_source(&mut self, parquet_path: String) -> SourceId;
fn source(&self, source_id: SourceId) -> Option<&DataSourceState>;
fn source_mut(&mut self, source_id: SourceId) -> Option<&mut DataSourceState>;
fn reset_source(&mut self, source_id: SourceId) -> bool;
fn remove_source(&mut self, source_id: SourceId) -> Option<DataSourceState>;
```

要件:

- `add_source()` はmonotonic counterで新しい `SourceId` を予約する
- removeしたIDを再利用しない
- `sources` と `source_order` を一貫して更新する
- remove対象がselected sourceなら、残った先頭sourceまたは `None` を選択する
- source番号は `source_order` に基づく
- 存在しないsourceのreset/removeはpanicしない
- source追加数を2へ固定しない。UI上限2はM3の責務とする
- workspace-levelのflat raw channel cacheを追加しない
- `DataSourceState` の `ChannelStore` と `shared_time` の所有関係を維持する

M2ではsource remove後のRow、selected channel、Hover、Cursor、FFT cleanupを
完了条件にしません。画面上のsource removalとdangling View参照の整理はM3以降で
実装します。

---

## 9. Source path change / reset / generation

sourceのpath変更・再読込・resetを、source単位の一貫したmutationとして
扱ってください。

要件:

- path変更時に対象sourceのgenerationを進める
- 同じpathの明示的reload/resetでも、古いjobを無効化する場合はgenerationを進める
- generationは単調であり、通常のresetで0へ戻さない
- 対象sourceのschema、shared time、raw channel、Envelope、Line tile、
  Step edge、fallback hintをクリアする
- 他sourceのschema、time、raw channel、draw cache、generationを変更しない
- path変更時にlabelを更新する
- pathをAからBへ変更し、再びAへ戻しても、最初のAのjob resultは適用しない
- reset/remove済みsourceのbackground resultを安全に無視する

generationを更新するhelperとdata/cacheをクリアするhelperを分離しても構いません。
ただし、呼出側がgeneration更新を忘れやすいAPIにしないでください。

既存Source #1 UIのpath変更は、M2後もこの共通mutation境界を通してください。

---

## 10. Source-aware schema load

schema loadの対象を暗黙のselected sourceやDerefに依存させず、`SourceId` で
明示してください。

例:

```rust
fn load_schema_for_source(&mut self, source_id: SourceId);
```

要件:

- path、generation、schema格納先を同じsourceから取得する
- 成功時は対象sourceだけのschemaを更新する
- 対象sourceのshared time / channel / draw cache resetが必要な場合、
  他sourceへ影響させない
- schema failure時は対象sourceだけをfailure/reset状態へする
- sourceが存在しない場合はpanicせず、明確なstatus/errorを返す
- time columnがないsourceは、そのsourceのchannel loadを禁止する
- Source #1向け既存 `load_schema()` wrapperを残す場合も、内部で
  `load_schema_for_source(source_id)` を呼ぶ薄い互換境界にする
- schema read自体を無理にbackground job化しない

M2ではsource別schema UIやsource別エラー表示UIを追加しません。非UI APIと
内部状態の隔離をテストしてください。

---

## 11. Source-aware channel load request

channel load requestは `ChannelKey` を受け取り、sourceとpathをロード境界で
解決してください。

```rust
fn request_channel_load(
    &mut self,
    ctx: &egui::Context,
    key: &ChannelKey,
    target_row_id: u64,
    add_to_row: bool,
);
```

要件:

- `key.source_id` から対象sourceを取得する
- schema、Parquet path、generation、shared time、`ChannelStore` を
  同じsourceから取得する
- Parquet readerには解決済みsourceの `key.channel_path` を渡す
- sourceが存在しない、schemaがない、time columnがない場合はjobを開始しない
- 異なるsourceの同じpathを別jobとして同時実行できる
- 同じ `ChannelKey` の重複jobは抑止する
- timeが対象sourceにキャッシュ済みなら再利用する
- channelが対象sourceにキャッシュ済みなら再利用する
- 他sourceの同path cacheを誤って再利用しない
- load完了後のRow追加が必要な既存Source #1経路では、job開始時の
  `ChannelKey` を保持し、selected sourceから再構築しない

M2ではSource #2のchannelをRowへ追加するUIや、複数source timeの描画は
追加しません。

---

## 12. ActiveLoadJob / LoadJobResult identity

設計書に従い、active jobとresultへsource identityとgenerationを追加してください。

最低限、次と同等の情報を保持します。

```rust
struct ActiveLoadJob {
    source_id: SourceId,
    source_generation: u64,
    channel_key: ChannelKey,
    target_row_id: u64,
    // existing fields...
}

struct LoadJobResult {
    job_id: u64,
    source_id: SourceId,
    source_generation: u64,
    channel_path: String,
    result: Result<LoadedChannelData, String>,
}
```

要件:

- `channel_key.source_id == source_id` が成立する
- background threadへsource id / generationを値として渡す
- resultはjob開始時のsource id / generationをそのまま返す
- `ActiveLoadJob` のchannel identityをpathだけへ戻さない
- target rowやstatus用の既存情報は維持する
- job idは引き続き一意である

`LoadedChannelData` 自体は、active/result metadataでsourceが確定するなら
source idを重複保持しなくても構いません。

---

## 13. Result validation and stale rejection

`apply_load_result()` は値を適用する前に、少なくとも次を順番に検証してください。

1. resultの `job_id` がactive jobとして存在する
2. resultのsource id / generation / channel pathがactive jobと一致する
3. source idがworkspaceに現在も存在する
4. 現在のsource generationがjob/result generationと一致する
5. `channel_key.source_id` とsource idが一致する
6. result channel pathと `channel_key.channel_path` が一致する

不一致のresultはstaleまたはmismatchedとして破棄し、どのsourceのtime、
channel、cache、Rowにも適用しないでください。

要件:

- path一致だけをstale判定に使用しない
- selected sourceやworkspace Derefを格納先決定に使用しない
- `apply_loaded_channel()` はjobのsource idから `source_mut()` を取得する
- time/value sample countは同じsource内で比較する
- target source以外のshared time / channel cacheを変更しない
- missing source、generation mismatch、path mismatchでpanicしない
- stale resultを破棄した後、active job/progress状態を一貫させる
- stale resultのstatus/errorが、正常な別sourceのdataをクリアしない

同じpathへ戻したケースを必ずUnit testしてください。

```text
generation 1: path Aでjob開始
generation 2: path Bへ変更
generation 3: path Aへ戻す
generation 1のresult到着
=> pathは一致してもgeneration不一致なので破棄
```

---

## 14. LoadStateのChannelKey対応

path-basedのloading判定をfull keyへ移行してください。

```rust
fn is_channel_loading(&self, key: &ChannelKey) -> bool;
fn loading_channel_keys(&self) -> BTreeSet<ChannelKey>;
fn is_source_loading(&self, source_id: SourceId) -> bool;
```

要件:

- `(source #1, "iu")` loading中でも `(source #2, "iu")` を開始できる
- 同じ `ChannelKey` の二重開始は抑止する
- source単位のreset/remove時に、対象sourceのactive jobだけを
  cancel扱いまたはstale扱いにできる
- 他sourceのactive jobとprogressは保持する
- M1の `loading_channel_paths()` 残存箇所を調査する
- Source #1だけを表示する既存UIへpath setが必要な場合は、
  sourceを明示して `ChannelKey` setからprojectionする
- workspace全体のloading stateをpathだけへflattenしない

globalな `is_busy()` は既存UI互換のため残して構いません。M3でsourceごとの
操作可否を実装できるよう、source-specific busy判定を用意してください。

---

## 15. Per-source cache and memory aggregation

`ChannelStore` 内部のcache keyはsource内で一意なので、既存どおり
`channel_path: String` を使用して構いません。

次の所有関係を維持してください。

```text
WorkspaceDataState
  Source #1
    shared_time #1
    ChannelStore #1
      raw_by_channel["iu"]
      envelope / line tile / step edge cache
  Source #2
    shared_time #2
    ChannelStore #2
      raw_by_channel["iu"]
      envelope / line tile / step edge cache
```

要件:

- 同名pathを別sourceのstoreへ同時格納できる
- time配列はsourceごとに1コピー
- channel値は `(SourceId, channel_path)` ごとに1コピー
- 同source同channelを複数Rowで使ってもraw配列を複製しない
- source間のデータ重複排除は行わない
- reset/unload/clear cacheは対象sourceを明示できる
- source単位操作で他sourceのdraw cacheを消さない

workspace全体の集計helperを追加してください。名前は既存コードに合わせて
構いませんが、少なくとも次をUnit testします。

- source count
- loaded channel count across sources
- raw time memory across sources
- raw channel memory across sources
- total raw memory
- Line tile / Step edge cache countまたはmemory

既存Perf/Cache表示がM2で複数source stateを作った場合、selected sourceだけの値を
全体値として表示しないようにしてください。ただしper-source UI表示はM3へ
延期します。

---

## 16. Single-source compatibility boundary

M1で導入したselected Source #1への `Deref` / `DerefMut` は、既存単一ソースUIと
描画を維持する暫定境界です。

M2では少なくとも次のdata mutationから暗黙Deref依存を除いてください。

- schema load
- source path/reset
- channel load request
- cached channel判定
- load result適用
- unload / clear cacheのsource-aware API
- memory/cache aggregate

描画、Hover、Cursor、FFTなど、M3〜M5でsource-aware化する箇所は
selected Source #1互換のままでも構いません。

ルール:

- dataの書込先をselected sourceから推測しない
- load job開始後にselected sourceが変わってもjob identityを失わない
- Source #1だけの現行UI操作は従来どおり動作させる
- Row / Line / Step / Hover / Cursor / FFTの数値仕様を変更しない
- Session preset / Plot config version 1の外部形式を変更しない

---

## 17. 必須Unit test

既存テストを削除・緩和せず、少なくとも次を追加または更新してください。

### Source lifecycle

- source追加でmonotonicな新規IDを取得する
- source remove後もIDを再利用しない
- `sources` と `source_order` が一致する
- selected source remove後のfallbackが一貫する
- missing source reset/removeがpanicしない

### Independent source state

- 2sourceが異なるschemaを保持できる
- 2sourceが異なるtime sample countを保持できる
- 2sourceの同じpath `iu` が別の値として同時cacheされる
- source #2 resetでsource #1のschema/time/raw/draw cacheが保持される
- source #2 failureでsource #1のdataが保持される
- source #2 removeでsource #1が保持される

### Load identity

- `is_channel_loading()` が `ChannelKey` 単位で判定する
- 同じsource / same pathの二重jobを拒否する
- different source / same pathの同時jobを許可する
- active jobとresultがsource id / generation / full keyを保持する

### Result validation

- current generationのresultを対象sourceへ適用する
- generation mismatch resultを破棄する
- sourceがremove済みのresultを破棄する
- result source id mismatchを破棄する
- result channel path mismatchを破棄する
- path A → B → A後の古いA resultを破棄する
- stale #2 resultが#1 cacheへ入らない
- sample count mismatchが対象source以外へ影響しない

### Cache / memory

- same path / different source cache isolation
- source reset isolation
- unload / clear cacheのsource isolation
- aggregate source/channel/time/raw memoryが全sourceの合計になる

### Regression

- default workspaceは引き続きSource #1だけで起動する
- Source #1のschema/channel loadが従来どおり成功する
- existing Row / Line / Step / Hover / Cursor / interval statistics tests
- existing FFT reference peak / stale result tests
- existing Session preset tests
- existing Plot config version 1 tests

テストの都合だけで不要なpublic APIを増やさないでください。binary crate内の
private helperとUnit testを優先してください。

---

## 18. Fixture / CLI確認

M0 fixtureが存在することを確認してください。

```text
code/proto_3_1b/data/multi_source_validation_1.parquet
code/proto_3_1b/data/multi_source_validation_2.parquet
```

存在しない場合はM0結果とREADMEの手順に従ってgeneratorを実行して構いません。
generator source、fixture仕様、Cargo依存関係は変更しないでください。
生成ParquetはGitへ追加しません。

`code/rust_phase1` から少なくとも次を実行してください。

```bash
cargo run -- --schema ../proto_3_1b/data/multi_source_validation_1.parquet
cargo run -- --schema ../proto_3_1b/data/multi_source_validation_2.parquet
cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_1.parquet iu
cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_2.parquet iu
```

確認する内容:

- Source #1: 100,001 rows、time `0.0 .. 2.0`、50,000 Hz相当
- Source #2: 80,001 rows、time `0.5 .. 1.75`、64,000 Hz相当
- 両方の `iu` がロード可能
- source間でsample countが異なる
- channel pathへ `#1/#2` suffixが混入していない

CLIは単一ファイルコマンドなので、2source同時cacheの検証はUnit testで行います。

GUIを利用できる場合は、現行Source #1のschema/channel load、Row、Line/Step、
Hover、Cursor、FFT、Session preset、Plot config v1の単一ソース回帰を確認して
ください。

M2ではSource #2のGUI操作を実施しません。2ソースUIは存在しないため、
実施したと記録しないでください。GUIを操作できない場合も未実施と明記します。

---

## 19. M2実行結果文書

次のファイルを新規作成してください。

```text
docs/rust_rfq_1_multi_source_phase1_m2_results.md
```

front matter:

```yaml
---
project: Panel_y
doc_type: M2 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M2"
created: "<実行日>"
updated: "<実行日>"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m1_results.md"
implementation: "code/rust_phase1"
---
```

本文へ最低限記録する内容:

1. M2の目的とスコープ
2. Git baselineと開始時worktree
3. 変更前 `cargo test`
4. source lifecycle API
5. generation / reset semantics
6. source-aware schema load
7. source-aware channel job / result
8. stale result validation
9. per-source cacheとmemory aggregation
10. failure / reset / remove isolation
11. 追加・更新したテスト
12. fixture / CLI確認
13. GUI単一ソース回帰の実施・未実施
14. 既知の制約
15. M3への申し送り
16. M2完了判定

テスト数、commit SHA、CLI結果、fixture値を推測で記載しないでください。

---

## 20. 検証と品質確認

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

- M0/M1成果物を意図せず変更していない
- M0 fixture generatorとfixture仕様を変更していない
- Cargo.toml / Cargo.lockを変更していない
- 2ソースUIや `#1/#2` presentationを追加していない
- multi-time renderingへ着手していない
- Plot config / Session preset formatを変更していない
- `ActiveLoadJob` がsource id / generation / full channel keyを保持する
- `LoadJobResult` がsource id / generationを保持する
- result適用先がselected sourceではなくjob sourceで決まる
- loading判定がpathだけへ戻っていない
- source reset/failureが他source cacheを消さない
- workspace-level flat raw cacheを追加していない
- memory/cache集計が全sourceを対象にする
- 既存テストを削除・緩和していない
- 最終テスト結果をM2結果文書へ反映した

残存する単一ソース境界を `rg` で確認してください。

```bash
rg -n \
  "struct ActiveLoadJob|struct LoadJobResult|is_channel_loading|loading_channel_paths|workspace\\.shared_time|workspace\\.loaded_channels" \
  code/rust_phase1/src/main.rs
```

`workspace.shared_time` / `workspace.loaded_channels` は、M2では既存Source #1の
描画・Hover・Cursor・FFT互換境界に残って構いません。
schema/channel load、result適用、source lifecycle、aggregate statsの書込先決定に
残っている場合は責務を確認し、暗黙selected source依存を除去してください。

---

## 21. 想定する変更ファイル

原則として変更は次に限定してください。

```text
code/rust_phase1/src/main.rs
code/rust_phase1/src/workspace_data.rs
docs/rust_rfq_1_multi_source_phase1_m2_results.md
```

reader/helperの小さな変更が本当に必要な場合だけ、次を変更して構いません。

```text
code/rust_phase1/src/parquet_waveform.rs
```

`ChannelStore` は依存が大きいため、M2完了のために `main.rs` から全面移動する
必要はありません。`DataSourceState -> ChannelStore` の所有関係とsource-awareな
アクセスを優先してください。

ユーザー向けUI操作は変わらないため、README変更は原則不要です。

次は変更しないでください。

```text
code/rust_phase1/Cargo.toml
code/rust_phase1/Cargo.lock
code/rust_phase1/src/fft_analysis.rs
code/rust_phase1/src/plot_config.rs
code/proto_3_1b/generate_multi_source_validation_data.py
```

既存互換のために `fft_analysis.rs` / `plot_config.rs` の追従が避けられない場合は、
format・数値仕様・M3以降の挙動を変更せず、M2結果文書へ理由を記録してください。

---

## 22. 停止条件

次の場合は、勝手にスコープを広げず停止して報告してください。

- M1結果がcompleteでない
- M2開始前から `cargo test` が再現性をもって失敗する
- M2対象コードに所有者不明の競合変更がある
- source-aware loadを完成するためにM3の2ソースUIが必要になる
- cache isolationを完成するためにM4のmulti-time renderingが必要になる
- stale load safetyのためにFFTやpersistence format変更が必要になる
- M0 fixtureやCargo依存関係変更が必要になる
- 既存単一ソース回帰を維持できない
- 既存テストを削除・緩和しないと完了できない
- M2の範囲を超える大規模なmodule再編が必要になる

停止報告には次を含めてください。

- 実行したコマンド
- 観測したエラー
- 原因の切り分け
- 変更済みファイル
- 未完了項目
- M3以降へ送るべき内容

---

## 23. 完了条件

次をすべて満たした場合だけM2をcompleteとしてください。

- [ ] M0 / M1がcompleteであることを確認した
- [ ] Git/toolchain/test baselineを記録した
- [ ] source追加・reset・removeの非UI APIを追加した
- [ ] source IDがremove後も再利用されない
- [ ] schema loadを `SourceId` 指定にした
- [ ] channel load requestを `ChannelKey` 指定にした
- [ ] `ActiveLoadJob` へsource id / generation / full keyを追加した
- [ ] `LoadJobResult` へsource id / generationを追加した
- [ ] loading判定を `ChannelKey` 対応にした
- [ ] different source / same pathの同時jobを許可した
- [ ] result適用時にjob/source/generation/pathを検証した
- [ ] path A → B → Aのstale resultを破棄した
- [ ] same path / different source cache isolationをテストした
- [ ] 異なるtime sample countを2sourceで保持した
- [ ] source failure/reset/remove isolationをテストした
- [ ] memory/cache aggregateを全source対応にした
- [ ] 既存Source #1のschema/channel loadが回帰していない
- [ ] Row / Line / Step / Hover / Cursor / FFTが回帰していない
- [ ] Session preset / Plot config version 1が回帰していない
- [ ] 2ソースUI・multi-time renderingへ着手していない
- [ ] Cargo依存関係を変更していない
- [ ] `cargo fmt --check` が成功した
- [ ] `cargo check` が成功した
- [ ] `cargo test` が成功した
- [ ] `git diff --check` が成功した
- [ ] M2実行結果文書を実測値で作成した

---

## 24. 最終報告フォーマット

最終応答は次の順序で簡潔に報告してください。

1. M2完了 / blocker
2. 作成・変更したファイル
3. Git baseline
4. 変更前・変更後の `cargo test` 結果
5. source lifecycle API
6. source-aware schema/channel load
7. job/result identityとstale rejection
8. per-source cache / failure isolation / aggregate stats
9. fixture CLI / GUI確認結果
10. M3への申し送り
11. commit / pushを行っていないこと

成果物へのローカルファイルリンクを付けてください。
