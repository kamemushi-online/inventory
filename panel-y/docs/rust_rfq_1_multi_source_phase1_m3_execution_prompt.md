---
project: Panel_y
doc_type: Execution Prompt
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M3"
target: Two-source UI and channel presentation
created: "2026-07-26"
updated: "2026-07-26"
status: ready to use
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m2_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — マルチデータ読み込み対応 Phase 1 / M3 実行指示プロンプト

以下の指示に従い、Panel-y Rust版の
「M3 — Two-Source UI and Channel Presentation」だけを実行してください。

---

## 1. 役割

あなたは `/Users/ochiba/newspace/life/factory/panel-y` を対象に作業する
実装エージェントです。

今回の目的は、M2で完成したsource-aware lifecycle / schema load /
channel load / per-source cache APIを画面へ接続し、UIから最大2つの
Parquet sourceを登録・ロード・削除できるようにすることです。

さらに、チャンネルを表示するすべての既存UIで、内部の `ChannelKey` を
失わずに `channel_name #1` / `channel_name #2` を一貫して表示してください。

このターンではM3を完了させてください。`Data Sources` UI、source別Channel
browser、任意sourceからのRow追加、source削除cleanup、表示番号の再計算、
統一presentation helperを実装し、Unit testと可能な範囲のGUI確認で検証します。

M3はUIとpresentationの工程です。異なるtime配列を同じplotへ同時描画する処理、
source別Hover/Cursor計算、mixed-source FFT、複数source persistenceは
実装しません。

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
8. `docs/rust_rfq_1_multi_source_phase1_m2_execution_prompt.md`
9. `docs/rust_rfq_1_multi_source_phase1_m2_results.md`
10. `code/rust_phase1/README.md`
11. `code/rust_phase1/src/main.rs`
12. `code/rust_phase1/src/workspace_data.rs`
13. `code/rust_phase1/src/parquet_schema.rs`
14. `code/rust_phase1/src/parquet_waveform.rs`
15. `code/rust_phase1/src/fft_analysis.rs`
16. `code/rust_phase1/src/plot_config.rs`

`journal/` は参照・編集しないでください。

特に設計書の次を確認してください。

- `4.1 チャンネルの同一性と表示名を分離する`
- `5. 識別子と状態モデル`
- `6. チャンネル表示名`
- `7. データソースUI`
- `8. ロード処理`
- `14. Plot configとSession preset`
- `15. エラー処理`
- `17. Step 3: 2ソースUI`
- `18. テスト計画`

工程表では次を重点的に確認してください。

- `4.1 ソース番号`
- `4.3 ソース変更時の扱い`
- `9. M3 — Two-Source UI and Channel Presentation`
- `16.1 Core identity`
- `16.2 Load and lifecycle`
- `16.5 Persistence safety`

---

## 3. M3開始条件

次を確認してから実装してください。

- M0、M1、M2結果文書のstatusがすべて `complete` である
- M2レビュー指摘である同一path明示reload時のgeneration更新とcache clearが
  コードへ反映済みである
- `WorkspaceDataState` がsource collection、`source_order`、
  `selected_source_id`、monotonic source id counterを所有する
- `add_source()` / `remove_source()` / `source_number()` が存在する
- source別path change / reset / schema load APIが存在する
- channel load request、active job、result、loading判定が `ChannelKey` 対応済みである
- schema、shared time、raw channel、draw cacheがsource単位で分離済みである
- `RowChannel` と `selected_channel` が `ChannelKey` を保持する
- workspace全体のcache/memory集計が全source対応済みである
- M0の2ソースfixtureが存在する、または再生成手順が確認できる
- 現行 `cargo test` が成功する
- M3対象コードに、M2完了後の所有者不明変更が重なっていない

M0〜M2成果物が未コミットで残っていること自体は停止理由ではありません。
既存成果をM3の前提として保持し、巻き戻し・再実装・削除しないでください。

開始時に既存差分と今回のM3差分を区別できない場合、または安全に統合できない
所有者不明変更がある場合は、実装を開始せず停止条件に従って報告してください。

---

## 4. M3のゴール

次の状態を作ってください。

- 初期状態は従来どおりdefault pathを持つSource `#1` だけで起動する
- `Data Sources` UIから空のSource `#2` を追加できる
- UI上のsource数は最大2つに制限する
- 各sourceにPath、Browse、Load Schema、Remove、状態表示がある
- sourceごとの危険操作は、そのsourceのchannel load中だけ無効になる
- #1のjob中でも、jobがない#2のpath/schema操作は可能である
- #1と#2のschema/channel一覧を同時に確認できる
- Channel browserがsource単位にグループ化される
- browser filterが元のname/path、`name #n`、source番号、source label、
  file nameを検索できる
- 同じpathの `(source #1, "iu")` と `(source #2, "iu")` を区別して
  選択・ロード・Row追加できる
- 異なるpathのチャンネルも同じRowへ追加できる
- 単一source時を含め、チャンネル表示名へ常時 `#n` を付ける
- source番号は `source_order` の表示位置から計算し、内部IDとして使わない
- source削除後、残ったsourceを先頭から再採番する
- source削除・path変更後にdanglingなRow/selection/analysis参照が残らない
- internal channel name/path、`ChannelKey.channel_path` へ `#n` を混入させない
- M2のload/cache/failure isolationを維持する
- Plot config version 1とSession presetの単一source互換を維持する
- M4より前に2sourceの異なるtime配列を同じplotへ同時描画しない
- M5より前にsource #2をsource #1のtimeでHover/Cursor/FFT計算しない

---

## 5. 厳守するスコープ

### 実施すること

- Git状態、M2 baseline、toolchainの確認
- M3変更前の `cargo test`
- 最大2sourceの `Data Sources` UI
- source別Path edit / Browse / Load Schema / Remove
- source別loading guardと状態表示
- source別Channel browserとfull-key load state
- source number / file nameを含むChannel browser filter
- `ChannelPresentation` または同等の統一presentation helper
- 既存の全チャンネル表示への常時 `#n` 付与
- 任意 `ChannelKey` の選択RowへのLoad / Add
- same-key重複拒否とdifferent-source/same-path許可
- matching channel検索と、全sourceから同一Rowへ追加する操作
- source別Unload / Clear cache action
- source path change / remove時の対象RowChannel cleanup
- selected channel、Hover、Cursor、interval statistics、FFT stateの安全なcleanup
- source削除後の表示再採番
- M4/M5前のcross-source time misuse防止
- pure presentation / filter / cleanup / matching helperのUnit test
- M0 fixtureを使ったCLIおよび可能な範囲のGUI確認
- M3実行結果文書の作成
- `cargo fmt --check`、`cargo check`、`cargo test`、`git diff --check`

### 実施しないこと

- 3つ以上のsourceを追加できるUI
- source順の並び替えUI
- source番号の永続化や内部ID化
- `#1/#2` suffixをschema、channel name、channel pathへ書き込むこと
- 2source timeを同じplotへ同時描画する処理
- display range union、`Fit All`、`Fit Overlap`
- source別display/raw time変換
- `TimeTransform` の描画接続やoffset編集UI
- 複数source Line / Step / Auto Y / Reset X Range
- source別Hover sample lookup
- source別Cursor / interval statistics計算
- mixed-source FFT実行またはmixed-source FFT guardの完成
- source別FFT time切り出し
- 複数source Plot config / Session preset
- Plot config format/version変更
- persistence guardの完成
- Cargo依存関係追加
- M0 fixture仕様やgenerator変更
- 大規模な `main.rs` 再編
- 既存テストの削除・緩和
- `journal/` の参照・編集
- branch作成、commit、push、PR作成

M3ではSource #2のchannelをロードし、Row modelへ追加して構いません。
ただし、2source同時描画はM4へ残してください。

---

## 6. M3とM4/M5の境界

M3で2source UIを有効にすると、Source #2のloaded channelとRow参照が実データとして
存在します。既存単一sourceの描画・解析helperがchannel pathだけ、または
selected Source #1のtimeだけを使ってSource #2の値を処理すると、見かけ上は動作しても
誤った結果になります。

M3では次を厳守してください。

- source identityを削除して単一pathへflattenしない
- 既存の単一source描画境界は、同時に1sourceのtime/storeだけを扱う
- Rowに別sourceのchannelがあっても、別sourceのtime/valueを組み合わせない
- Source #2のloaded channelをSource #1のtimeでHover/Cursor/statistics/FFTへ渡さない
- M3で表示されるHover、Cursor、statistics、FFTのチャンネルラベルには
  presentation helperを使用する
- M3で計算対象外のsource channelは、解析候補から安全に除外する
- source #2をRowへ追加済みだがM4前で同時描画できないことを、UI上の情報表示または
  結果文書の既知の制約として明確にする
- source削除時は、削除sourceを参照する解析target/result/jobを無効化する
- stale background resultが削除・変更後のsourceへ適用されないM2保証を維持する

M3の目的は「UIとidentityを完成させる」ことであり、M4/M5の数値計算を先取りする
ことではありません。

---

## 7. 作業開始時の確認

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

M3対象ファイルとCargo差分を確認してください。

```bash
git diff -- code/rust_phase1/src/main.rs
git diff -- code/rust_phase1/src/workspace_data.rs
git diff -- code/rust_phase1/src/fft_analysis.rs
git diff -- code/rust_phase1/src/plot_config.rs
git diff -- code/rust_phase1/Cargo.toml
git diff -- code/rust_phase1/Cargo.lock
```

ルール:

- M0〜M2の未コミット成果物を削除・上書きしない
- 既存変更の所有者を推測しない
- M2のsource-aware load/cache APIを単一source APIへ戻さない
- `git fetch`、`git pull`、remote writeを行わない
- M0 fixtureをforce addしない
- baseline SHA、branch、開始時worktreeをM3結果へ記録する

---

## 8. 変更前ベースライン

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

M2結果では実行時点のtest数を記録していますが、数を決め打ちしないでください。
レビュー対応や他の正当な変更で増減している場合は、開始時の実測値を使用します。

変更前から再現性のある失敗がある場合は、M3で無関係な修正をせず停止して
報告してください。

---

## 9. Data Sources UI

既存の単一 `Dataset` セクションを、source collectionを表示する
`Data Sources` セクションへ置き換えてください。

表示例:

```text
Data Sources
  #1  multi_source_validation_1.parquet
      Path [...]
      [Browse] [Load Schema] [Remove]
      schema: ready, time: ready, channels: 2 loaded

  #2  multi_source_validation_2.parquet
      Path [...]
      [Browse] [Load Schema] [Remove]
      schema: ready, time: ready, channels: 1 loaded

  [Add Source]
```

要件:

- `source_order` の順で表示する
- 表示番号はposition + 1であり、`SourceId` の値を表示番号として使わない
- source見出しに `#n` とfile nameを表示する
- 空pathは `(no file)` など明確なlabelにする
- 初期状態はdefault pathを持つsource #1だけ
- `Add Source` は空pathのsourceを追加する
- sourceが2つある場合、`Add Source` を無効化する
- UI上限だけを2にし、workspace modelの `add_source()` 自体を2固定へ変更しない
- 各sourceのwidget IDは表示番号やpathではなくstableな `SourceId` を含める
- 同じfile name、同じchannel pathでもegui IDが衝突しない
- sourceごとにschema ready / time ready / loaded channel count / loadingを表示する
- memory/cache表示を行う場合はsource値とworkspace aggregateを混同しない
- global `load.is_busy()` で全sourceのPath/Browse/Removeを一括無効化しない
- `load.is_source_loading(source_id)` で対象sourceの危険操作だけをguardする

最後の1sourceをRemove可能にするか、常に1sourceを残すかは既存UXと設計の整合を
確認して決めて構いません。ただし、zero source stateでpanicせず、`Add Source` で
再開できることをUnit testしてください。

---

## 10. Source別Path / Browse / Load Schema

既存のselected Source #1前提のPath edit、Browse、Load Schemaを、明示的な
`SourceId` を受け取る境界へ接続してください。

例:

```rust
fn select_parquet_path_for_source(&mut self, source_id: SourceId, path: String);
fn browse_parquet_path_for_source(&mut self, source_id: SourceId);
fn load_schema_for_source(&mut self, source_id: SourceId);
```

要件:

- Path editは対象sourceの `set_source_parquet_path()` を通る
- 明示的な再読込はM2のreload/generation境界を通る
- Browse結果は開始時または操作時に指定したsourceへ適用する
- source numberやcurrent selected channelから適用先を推測しない
- source path変更時、対象sourceのschema/time/raw/draw cacheだけをresetする
- source path変更時、対象sourceを参照するRowChannelを削除する
- 対象sourceがselected channelならselectionをclearまたは有効なfallbackへ変更する
- 対象sourceに関係するanalysis stateを安全にinvalidateする
- 他sourceのschema/time/cache/RowChannel/selectionを保持する
- #2のpath変更で#1のPlot config pathを不用意に書き換えない
- recent pathを再利用する場合、選択したsourceへ適用する
- schema failure時、対象sourceだけをfailure/reset状態へする
- status/error textにpresentation上のsource labelを含める
- missing source、空path、invalid pathでpanicしない

既存のglobal `reset_dataset_after_path_change()` のように、全View/Analysisを
無条件clearするhelperをsource #2の変更へ流用しないでください。

---

## 11. Source別Channel browser

Channel browserは、schemaがロード済みのsourceを `source_order` 順に
グループ表示してください。

```text
#1 multi_source_validation_1.parquet
  iu #1
  torque #1

#2 multi_source_validation_2.parquet
  iu #2
  speed #2
```

要件:

- 両sourceのschema一覧を同じ画面で確認できる
- schema未ロードsourceも見出しと状態を表示する
- 各channel rowのidentityは `ChannelKey`
- selected判定はfull key一致
- loaded / loading / unloaded判定はfull key一致
- same source / same pathの二重loadを抑止する
- different source / same pathは別々にloadできる
- Load / Add actionは選択中Row IDとfull `ChannelKey` を保持する
- schema順、名前順、loaded firstなど既存sortをsource group内で維持する
- channel widget IDに `SourceId` とchannel pathを含める
- selected channel detailsも対象sourceのschema/cacheから取得する
- source #2のdetailsへsource #1のschema/cacheを使わない
- pathしか保持しない一時actionやresponse型を残さない

既存filterは次をcase-insensitiveで検索対象にしてください。

- 元の `channel_name`
- 元の `channel_path`
- presentation label、例 `iu #1`
- source number、例 `#1`
- source label
- Parquet file name

状態filterは同名pathをsource間で衝突させないでください。

---

## 12. 統一Channel Presentation API

チャンネル表示文字列を各UIで個別連結せず、一箇所で生成してください。

設計例:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
struct ChannelPresentation {
    short_label: String, // "iu #1"
    source_label: String, // "#1 run_A.parquet"
    tooltip: String,     // "run_A.parquet / iu"
}

fn channel_presentation(
    workspace: &WorkspaceDataState,
    key: &ChannelKey,
) -> Option<ChannelPresentation>;
```

型名やfield名は既存コードへ合わせて構いません。

要件:

- source番号は呼出時の `source_order` から取得する
- source削除後は残ったsourceのlabelが自動的に再採番される
- `short_label` は単一source時も必ず `#1` を含む
- display nameが空の場合はchannel pathへfallbackする
- tooltipにはfile nameまたはsource labelと元のchannel pathを含める
- internal `channel_name` / `channel_path` / `ChannelKey` を変更しない
- presentation labelをload/cache keyとして使わない
- sourceがremove済みのdangling keyでpanicしない
- source/channelがschemaにない場合も、安全なfallbackまたは `None` を返す
- `#n` 連結をUIごとに複製しない

source見出し用にも、同じ番号規則を使う統一helperを用意してください。

---

## 13. `#n` を適用する表示箇所

少なくとも次の既存表示を調査し、チャンネルを表す文字列にはpresentation helperを
使用してください。

- Data Sources内のsource status
- Channel browser
- selected channel details
- Load / Add target status
- load progress / completion / error status
- Row controls
- plot axis / plot title / trace label
- Hover readout
- Cursor A/B value table
- interval statistics table
- FFT target choices
- FFT running/result/spectrum/hover labels
- Cache controls
- Unload / Clear status
- empty rowやselected channelを示す案内文

ルール:

- 常に `iu #1` / `iu #2` の形式を使う
- 重複がない場合もsuffixを省略しない
- status内でsourceを表す場合も、可能なら `#n file.parquet` を使う
- tooltipは元pathを確認できるようにする
- channel pathへsuffixを追加してreader/cacheへ渡さない
- plot/hover/cursor/FFTの数値処理をM3でmulti-time化しない

`VisibleTrace`、Cursor readout、interval statistics、FFT spectrumが既に
`ChannelKey` を保持している場合は、そのkeyからlabelを生成してください。
保持していない場合は、M4/M5でidentityを失わない最小変更として
`ChannelKey` またはephemeral presentation labelを追加して構いません。

---

## 14. Row追加とmatching channel

通常のLoad / Addは、任意sourceの任意 `ChannelKey` を選択中Rowへ追加できるように
してください。

要件:

- same `ChannelKey` の同一Row重複を拒否する
- different source / same pathは同一Rowへ追加できる
- different source / different pathも同一Rowへ追加できる
- cached channelはsource別storeから再利用する
- load完了時はjobが保持した `ChannelKey` をRowへ追加する
- UI操作後にselected sourceやselected channelからkeyを再構築しない
- Row controlsのRemoveもfull keyで削除する
- Row membership表示もfull keyで判定する
- source #2のloaded channelをsource #1 cacheとして扱わない

`Add matching channel from all sources` 操作を追加してください。

matching規則:

1. 選択channelと完全な `channel_path` が一致する候補を各sourceで探す
2. path一致がないsourceでは、元の `display_name` が一致する候補を探す
3. 1source内でdisplay name一致が複数ある場合は自動選択しない
4. schema未ロード、候補なし、ambiguousなsourceはskipする
5. suffix付きpresentation labelでは比較しない

matching操作は見つかったfull keyを同一Rowへload/addし、追加・既存・skip・ambiguousを
statusへ要約してください。

matchingの照合処理はUIから分離したpure helperとしてUnit testしてください。

---

## 15. Source path change / remove cleanup

sourceのpath変更またはremove時は、対象sourceを参照するUI/View/Analysis stateを
整理してください。

最低限:

- 全Rowから対象 `source_id` の `RowChannel` を削除する
- 空Rowを許容する既存仕様を維持する
- 他sourceのRowChannelは保持する
- `selected_channel.source_id` が対象ならclearまたは有効なfallbackへ変更する
- Hover値やtrace cacheが対象sourceを参照し続けないようinvalidateする
- Cursor interval statistics cacheをinvalidateする
- FFT targetから対象source keyを削除する
- 対象sourceを含むFFT result / running jobを無効化する
- source-specific load jobが残る操作はdisableするか、M2のstale扱いにする
- source-specific raw/draw cacheをremove/resetする
- 他sourceのload job/cacheを保持する
- load/cache/status表示を再計算する

source remove後:

- `source_order` を更新する
- 残ったsourceの表示番号をpositionから再計算する
- SourceId自体は変更しない
- removeしたSourceIdを再利用しない
- old #2がnew #1になった場合、すべてのpresentation labelがnew #1になる
- dangling `ChannelKey` がRow、selection、analysis targetへ残らない
- zero sourceになってもpanicしない

cleanupは可能な限り `source_id` を引数に取るpureまたは小さなhelperへ分離し、
Unit testしてください。

---

## 16. Source別Cache controlsとstatus

M2で追加したsource-aware unload / clear APIをUIへ接続してください。

要件:

- cache一覧をsource単位に識別できる
- 同名channelは `iu #1` / `iu #2` と表示する
- `UnloadChannel` actionは `ChannelKey` を保持する
- `Unload all` は対象 `SourceId` を明示する
- `Clear draw caches` は対象 `SourceId` を明示する
- #2のUnload/Clearで#1 cacheを変更しない
- workspace aggregate memoryは全source合計のまま
- source別値をworkspace合計として表示しない
- channel load中は対象key/sourceの危険操作だけを無効化する
- global load progressで同名channelがある場合もsourceを識別できる
- success/failure/stale statusへ可能な範囲でsource/channel presentationを含める
- remove済みsourceのstale result statusでpanicしない

`CacheControlAction::UnloadAllChannels` や `ClearDrawCaches` がsourceを持たない場合は、
M3でsource identityを追加してください。

---

## 17. Persistence互換とguard境界

M3ではPlot configとSession presetのformatを変更しません。

要件:

- Plot config version 1のload/apply/save回帰を維持する
- Session presetの既存単一source round tripを維持する
- 保存値へpresentation suffixを混入させない
- 1source時は従来どおり操作できる
- 2source stateで既存formatへ誤って複数source参照をflattenしない
- M6で正式なpersistence guardを実装するまで、少なくとも破損・panic・silent
  cross-source mappingを起こさない

2source時のSave / Applyを全面的に無効化する正式guardはM6の責務です。
ただしM3で2source Rowを既存v1 formatへ誤保存する危険が顕在化する場合は、
最小の暫定guardと理由表示を追加し、M3結果文書へ記録してください。
format version追加や複数source保存は行わないでください。

---

## 18. 必須Unit test

既存テストを削除・緩和せず、少なくとも次を追加または更新してください。

### Source UI model

- default workspaceはSource #1だけ
- Add Sourceでmonotonic IDのSource #2を追加する
- UI上限判定は2sourceでadd不可、workspace API自体は2固定でない
- zero sourceからaddしてdisplay #1になる
- source orderがUI順になる
- same path/file nameでもstable IDが異なる

### Presentation

- 単一sourceのlabelが `iu #1`
- 2source同名pathが `iu #1` / `iu #2`
- 異名channelも正しいsuffixを持つ
- display name欠損時はpathへfallbackする
- tooltipがfile nameと元pathを含む
- internal channel name/path/keyにsuffixが入らない
- Source #1 remove後、old #2が `#1` と表示される
- missing/removed source keyでpanicしない

### Browser / filter

- sourceごとのschema entriesを保持する
- same path / different sourceのload stateが衝突しない
- filterが元name/pathへ一致する
- filterが `name #n` / `#n` へ一致する
- filterがsource label/file nameへ一致する
- same pathのwidget/list identityがsource間で衝突しない

### Row membership / matching

- same keyの同一Row重複を拒否する
- different source / same pathを同一Rowへ追加できる
- different source / different pathを同一Rowへ追加できる
- Row removeがfull keyだけを削除する
- matchingは完全path一致を優先する
- pathなしの場合にunique display nameへfallbackする
- ambiguous display nameを自動選択しない
- missing/schema未ロードsourceをskipする

### Lifecycle cleanup

- source #2 path changeで#2 RowChannelだけを削除する
- source #2 removeで#2 RowChannelだけを削除する
- #2 cleanup後も#1 selection/cache/RowChannelを保持する
- removed sourceがselected channelならselectionを安全にclearする
- removed sourceのFFT target/result/jobをinvalidateする
- interval statistics cacheをinvalidateする
- source remove後にdangling keyが残らない
- remove後の再採番が全presentationで一致する

### Cache / loading / safety

- source #1 loading中でもsource #2の安全な操作可否を維持する
- source #2 loading中は#2 path/removeを無効化する
- source別Unload/Clearが他sourceへ影響しない
- Source #2 channelをSource #1 timeでHover/Cursor/statistics/FFTへ渡さない
- M2のsame path / different source cache isolation
- M2のgeneration mismatch / stale result rejection

### Regression

- Source #1 schema/channel load
- Row / Line / Step
- plot/hover/cursor/statisticsの既存数値仕様
- FFT reference peak / stale result
- Plot config version 1
- Session preset
- Plain / Rock表示

テストの都合だけで不要なpublic APIを増やさないでください。binary crate内の
private pure helperとUnit testを優先してください。

---

## 19. Fixture / CLI / GUI確認

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

CLI確認:

- Source #1: 100,001 rows、time `0.0 .. 2.0`、50,000 Hz相当
- Source #2: 80,001 rows、time `0.5 .. 1.75`、64,000 Hz相当
- 両方の `iu` がロード可能
- internal channel pathがどちらも `iu`
- CLI出力やreaderへ `#1/#2` suffixが混入していない

GUIを利用できる場合は、少なくとも次を確認してください。

1. 初期状態でSource #1にdefault pathが表示される
2. Add Sourceで空のSource #2が追加される
3. 2source時にAdd Sourceが無効になる
4. #1/#2へ別々のfixture pathを設定する
5. #1 loading中に#2の安全な操作が不必要にlockされない
6. 両sourceのschemaをロードし、一覧を同時表示する
7. `iu #1` と `iu #2` を同じRowへLoad / Addする
8. `torque #1` と `speed #2` など異名channelも同じRowへ追加する
9. browser、Row、cache、statusでsuffixが一貫する
10. source/file/filter検索が機能する
11. #1 remove後にold #2がnew #1と表示される
12. remove後にdangling RowChannelや解析参照がない
13. 1sourceへ戻した後もschema/channel loadと既存単一source描画が動く
14. 2source同時描画がM3で実装されたと誤認しない

GUIを操作できない場合は未実施と明記し、実施したと記録しないでください。

---

## 20. M3実行結果文書

次のファイルを新規作成してください。

```text
docs/rust_rfq_1_multi_source_phase1_m3_results.md
```

front matter:

```yaml
---
project: Panel_y
doc_type: M3 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M3"
created: "<実行日>"
updated: "<実行日>"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m2_results.md"
implementation: "code/rust_phase1"
---
```

本文へ最低限記録する内容:

1. M3の目的とスコープ
2. Git baselineと開始時worktree
3. 変更前 `cargo test`
4. Data Sources UI
5. source別Path / Browse / Schema lifecycle
6. source別Channel browserとfilter
7. Channel presentation API
8. `#n` を反映した表示箇所
9. Row追加とmatching channel
10. source path change / remove cleanup
11. source別cache/loading/status
12. M4/M5前のcross-source time safety
13. 追加・更新したテスト
14. fixture / CLI確認
15. GUI確認の実施・未実施
16. persistence互換と暫定guard
17. 既知の制約
18. M4への申し送り
19. M3完了判定

テスト数、commit SHA、CLI結果、fixture値、GUI確認を推測で記載しないでください。

---

## 21. 検証と品質確認

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

- M0〜M2成果物を意図せず変更していない
- M0 fixture generatorとfixture仕様を変更していない
- Cargo.toml / Cargo.lockを変更していない
- source数の上限2がUIだけに適用されている
- internal name/path/keyへsuffixを追加していない
- `source_order` と表示番号を混同していない
- widget IDへ表示番号だけを使っていない
- Channel browser/action/loading/cacheがfull `ChannelKey` 対応である
- source path change/removeが他source Row/cacheを削除しない
- source remove後にdangling Row/selection/analysis keyがない
- #1 remove後にold #2がnew #1と表示される
- presentation連結が画面ごとに分散していない
- 単一source時もすべてのchannel labelに `#1` が付く
- 2source timeを同じplotへ同時描画していない
- Source #2 valueをSource #1 timeで解析していない
- Plot config / Session preset formatを変更していない
- 既存テストを削除・緩和していない
- 最終テスト結果をM3結果文書へ反映した

残存するpath-only presentationやglobal busy guardを `rg` で確認してください。

```bash
rg -n \
  "channel_display_name|trace_label|fft_channel_label|channel_name|channel_path|load\\.is_busy\\(\\)|selected_source_id|UnloadAllChannels|ClearDrawCaches" \
  code/rust_phase1/src/main.rs \
  code/rust_phase1/src/fft_analysis.rs
```

検索結果が存在すること自体は失敗ではありません。reader/cache/internal identityとして
元pathを使う箇所と、ユーザー表示でpresentation helperを使う箇所を区別して
確認してください。

---

## 22. 想定する変更ファイル

原則として変更は次に限定してください。

```text
code/rust_phase1/src/main.rs
code/rust_phase1/src/fft_analysis.rs
docs/rust_rfq_1_multi_source_phase1_m3_results.md
```

pure presentation / filter / cleanup helperを既存方針に従って分離する場合だけ、
次を変更して構いません。

```text
code/rust_phase1/src/workspace_data.rs
```

M3ではユーザー向けUIが変わりますが、Phase 1途中の制約を正式な利用仕様として
固定しないため、README変更は必須にしません。変更する場合はM3の実装事実と
M4/M5前の制約だけを記載してください。

次は変更しないでください。

```text
code/rust_phase1/Cargo.toml
code/rust_phase1/Cargo.lock
code/rust_phase1/src/parquet_waveform.rs
code/rust_phase1/src/plot_config.rs
code/proto_3_1b/generate_multi_source_validation_data.py
```

`plot_config.rs` の追従が避けられない場合も、format/version/外部挙動を変更せず、
M3結果文書へ理由を記録してください。

---

## 23. 停止条件

次の場合は、勝手にスコープを広げず停止して報告してください。

- M2結果がcompleteでない
- M3開始前から `cargo test` が再現性をもって失敗する
- M3対象コードに所有者不明の競合変更がある
- 2source UIを完成するためにM4のmulti-time renderingが必要になる
- presentationを完成するためにM5のsource別Hover/Cursor/FFT計算が必要になる
- source remove safetyのためにpersistence format変更が必要になる
- Plot config version追加や複数source保存が必要になる
- M0 fixtureやCargo依存関係変更が必要になる
- 既存単一source回帰を維持できない
- 既存テストを削除・緩和しないと完了できない
- M3の範囲を超える大規模なmodule再編が必要になる

停止報告には次を含めてください。

- 実行したコマンド
- 観測したエラー
- 原因の切り分け
- 変更済みファイル
- 未完了項目
- M4以降へ送るべき内容

---

## 24. 完了条件

次をすべて満たした場合だけM3をcompleteとしてください。

- [ ] M0 / M1 / M2がcompleteであることを確認した
- [ ] Git/toolchain/test baselineを記録した
- [ ] Data Sources UIを追加した
- [ ] default Source #1から起動する
- [ ] Add Sourceで空のSource #2を追加できる
- [ ] UI上限を2sourceにした
- [ ] 各sourceにPath / Browse / Load Schema / Remove / statusがある
- [ ] source-specific loading guardを実装した
- [ ] Channel browserをsource別にグループ表示した
- [ ] filterがsource number / label / file nameへ対応した
- [ ] presentation helperを一箇所に実装した
- [ ] 単一source時もchannel labelへ `#1` を付けた
- [ ] 全既存channel表示箇所へpresentationを適用した
- [ ] internal name/path/keyへsuffixを混入させていない
- [ ] 任意sourceのchannelを任意Rowへ追加できる
- [ ] same key重複を拒否した
- [ ] different source / same pathを許可した
- [ ] matching channelのpath優先/unique name fallbackを実装した
- [ ] source別Unload / Clearを実装した
- [ ] source path change/removeで対象RowChannelだけを削除した
- [ ] selected channel / statistics / FFT stateを安全にcleanupした
- [ ] source remove後にdangling keyが残らない
- [ ] source remove後の再採番が全表示で一致する
- [ ] Source #2をSource #1 timeで解析しないguardを追加した
- [ ] 2source同時描画へ着手していない
- [ ] Plot config / Session preset formatを変更していない
- [ ] Source #1の単一source利用が回帰していない
- [ ] M2のload/cache/stale/failure isolationが回帰していない
- [ ] Cargo依存関係を変更していない
- [ ] `cargo fmt --check` が成功した
- [ ] `cargo check` が成功した
- [ ] `cargo test` が成功した
- [ ] `git diff --check` が成功した
- [ ] M3実行結果文書を実測値で作成した

---

## 25. 最終報告フォーマット

最終応答は次の順序で簡潔に報告してください。

1. M3完了 / blocker
2. 作成・変更したファイル
3. Git baseline
4. 変更前・変更後の `cargo test` 結果
5. Data Sources UI
6. source別Channel browser / filter
7. presentation helperと `#n` 適用範囲
8. Row追加 / matching / remove cleanup
9. source別loading / cache / status
10. M4/M5前のcross-source time safety
11. fixture CLI / GUI確認結果
12. persistence互換と既知の制約
13. M4への申し送り
14. commit / pushを行っていないこと

成果物へのローカルファイルリンクを付けてください。
