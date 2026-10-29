---
project: Panel_y
doc_type: M2 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M2"
created: "2026-07-26"
updated: "2026-07-26"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m1_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M2 実行結果

## 1. 目的とスコープ

M2では、M1の `SourceId` / `ChannelKey` / `WorkspaceDataState` をschemaと
channelのロード境界へ接続し、sourceごとのtime、raw channel、描画cache、
非同期job/resultを分離した。

実施範囲は非UI source lifecycle API、source-aware schema/channel load、
generation付きjob/result、stale result拒否、source単位のreset/failure/remove
隔離、workspace全体のcache/memory集計、およびUnit testである。

2ソースUI、`#1/#2` presentation、multi-time rendering、source別
Hover/Cursor/interval statistics、mixed-source FFT、複数source永続化には
着手していない。

## 2. Git baselineと開始時worktree

実行日: 2026-07-26

| Item | Measured value |
|---|---|
| Current branch | `codex/rfq1-multi-source-phase1-docs` |
| HEAD | `a6a35418a1321399924522559e76f91570d53a7a` |
| `main` | `2ba81459519735003838d61d24d4efa2f8359b38` |
| `origin/main` | `2ba81459519735003838d61d24d4efa2f8359b38` |
| HEAD relation | `main` / `origin/main`を祖先とする3 commits ahead |
| HEAD subject | `a6a3541 Add RFQ multi-source Phase 1 M2 execution prompt` |
| Initial worktree | clean |
| Initial `git diff --check` | success |
| Rust | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| Cargo | `cargo 1.96.0 (30a34c682 2026-05-25)` |

M0/M1結果文書のstatusがともに `complete` であり、Parquet path手入力が
`set_parquet_path()` 相当の共通mutation境界を通ることを確認して開始した。

## 3. 変更前 `cargo test`

`code/rust_phase1` で実行した結果:

```text
test result: ok. 117 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

開始時blockerはなかった。

## 4. Source lifecycle API

`WorkspaceDataState` に次の非UI APIを追加した。

- `add_source(parquet_path) -> SourceId`
- `source()` / `source_mut()`
- `set_source_parquet_path()`
- `reset_source()`
- `remove_source()`
- source指定のchannel unload / 全channel unload / draw cache clear

`add_source()` はmonotonic counterを使い、remove済みIDを再利用しない。
`sources` と `source_order` を同時に更新し、selected sourceをremoveした場合は
残った先頭source、最後のsourceをremoveした場合は `None` へfallbackする。
内部source数は2へ固定していない。

## 5. Generation / reset semantics

`DataSourceState::reset()` はgenerationをchecked incrementし、対象sourceの
schema、shared time、raw channel、Envelope、Line tile、Step edge、
fallback hint、直近cache統計をclearする。

path変更はlabel/path更新とresetを1つのmutationとして実行する。同じpathの
明示的schema reloadもresetを行うためgenerationが進む。これにより
path A → B → Aの後でも、最初のAで開始したjobはgeneration不一致となる。

他sourceのgeneration、schema、time、raw data、draw cacheは変更しない。

## 6. Source-aware schema load

`load_schema()` はselected Source #1用の薄いwrapperとし、実処理を
`load_schema_for_source(SourceId)` へ移した。

- path、generation、schema格納先を指定sourceから解決する
- source単位でactive loadを判定する
- 成功時は対象sourceだけをreloadしてschemaを格納する
- validation/read failure時は対象sourceだけをresetする
- missing sourceはpanicせずstatus/errorを返す
- time columnがないschemaは保持するがchannel loadを開始しない
- 他sourceのactive job/progressとdataを保持する

M0の2fixtureを使うUnit testで、100,001 rowsと80,001 rowsの異なるschemaを
同じworkspaceへ保持し、Source #2 schema failure後もSource #1を保持することを
確認した。

## 7. Source-aware channel job / result

`request_channel_load()` は `ChannelKey` からsourceを解決し、同じsourceから
schema、Parquet path、generation、shared time、`ChannelStore` を取得する。
selected sourceによる格納先推測とSource #1限定guardを除去した。

`ActiveLoadJob` は次を保持する。

- `source_id`
- `source_generation`
- full `channel_key`
- target row / cache状態 / elapsed計測用の既存metadata

`LoadJobResult` はjob開始時の `source_id`、`source_generation`、
`channel_path` をbackground threadからそのまま返す。

loading判定は `ChannelKey` 単位であり、different source / same pathの
`iu` jobを同時に保持できる。同じfull keyの重複開始は拒否する。

## 8. Stale result validation

result適用前に次を検証する。

1. `job_id` がactiveである
2. resultのsource / generation / channel pathがactive jobと一致する
3. `channel_key.source_id == job.source_id`
4. sourceが現在も存在する
5. 現在のsource generationがjob/result generationと一致する
6. loaded channel pathとjobのfull keyが一致する
7. time/value sample countが対象source内で一致する

適用先はselected sourceではなくjobのsource idで決定する。不一致時は
time、channel、cache、Rowへ書き込まない。

Unit testでcurrent result、generation mismatch、removed source、result source
mismatch、result path mismatch、path A → B → A、sample count mismatchを確認した。

## 9. Per-source cacheとmemory aggregation

`ChannelStore` は引き続き各 `DataSourceState` が所有する。store内部のkeyは
source内で一意な元の `channel_path` のままであり、workspace-level flat cacheは
追加していない。

`WorkspaceCacheStats` は全sourceについて次を集計する。

- source count
- loaded channel count
- raw time memory
- raw channel memory / total raw memory
- Envelope count
- Line tile cache count / memory
- Step edge cache count / memory
- fallback hint count

Cache UIにはworkspace aggregateを明示し、selected sourceのcache値と区別した。

## 10. Failure / reset / remove isolation

Unit testで次を確認した。

- 異なるsample countの2source timeを同時保持できる
- `(Source #1, "iu")` と `(Source #2, "iu")` が別値として共存する
- Source #2 reset後もSource #1のschema/time/raw/draw cacheを保持する
- Source #2 channel failure後もSource #1 dataを保持する
- Source #2 remove後もSource #1 data/cacheを保持する
- source指定unload/clearが他sourceを変更しない
- source単位job cancelが他sourceのjob/progressを保持する

## 11. 追加・更新したテスト

変更後の最終結果:

```text
test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

M2で16 testsを追加した。source lifecycle、schema/time/cache分離、
loading identity、result validation、failure/reset/remove isolation、
aggregate memory/cacheを対象とする。

既存のRow、Line、Step、Hover、Cursor、interval statistics、FFT、
Session preset、Plot config version 1テストは削除・緩和せず成功した。

## 12. Fixture / CLI確認

`code/rust_phase1` からM0 fixtureを確認した。

| Check | Source #1 | Source #2 |
|---|---:|---:|
| Schema rows | `100001` | `80001` |
| Time column | `time` (`DOUBLE`) | `time` (`DOUBLE`) |
| Channels | `iu`, `torque`, `gate_pwm` | `iu`, `speed`, `gate_pwm` |
| Loaded channel | `iu` | `iu` |
| Samples | `100001` | `80001` |
| Time range | `0.000000 .. 2.000000` | `0.500000 .. 1.750000` |
| Value range | `-10.000000 .. 10.000000` | `-7.850000 .. 8.650000` |
| Result | success | success |

両CLI出力のchannel pathは `iu` のままで、`#1/#2` suffixは混入していない。

単一ソース `test_100k.parquet` でもschemaと `sine_50Hz` loadを確認した。

- rows: `100000`
- samples: `100000`
- time range: `0.000000 .. 0.999990`
- value range: `-10.000000 .. 10.000000`
- result: success

## 13. GUI単一ソース回帰

GUI手動確認は未実施である。M2のSource #2 GUI操作はスコープ外であり、
2ソースUIは追加していない。単一ソースのデータ経路はCLIと133 Unit testsで
確認した。

## 14. 既知の制約

- UIから登録・選択・removeできるsourceは引き続きSource #1だけである。
- 描画、Hover、Cursor、FFTはselected Source #1のtimeを使う暫定境界のままである。
- `TimeTransform` はmulti-time renderingへ未接続である。
- source remove後のRow/selection/analysis cleanupはM3以降のUI工程へ残る。
- Plot config version 1とSession presetのformatは単一sourceのままである。
- mixed-source FFT guardと複数source persistence guardは未実装である。

## 15. M3への申し送り

- 非UI `add_source()` / `remove_source()` / `source_number()` をData Sources UIへ接続する。
- 初期UI上限2sourceを画面側だけで適用する。
- schema/channel browserをsource単位で表示し、loading keyをfull
  `ChannelKey` のまま使用する。
- `ChannelPresentation` を追加し、すべての表示へ常時 `#1/#2` を付ける。
- source removal時のRow、selected channel、Hover/Cursor、FFT cleanupを実装する。
- M4のmulti-time renderingまではSource #2 channelを描画経路へ流さない。

## 16. M2完了判定

M2はcompleteと判定する。source lifecycle、generation/reset、source-aware
schema/channel load、full-key loading判定、job/result identity、stale rejection、
per-source cache/failure isolation、workspace aggregate statsを実装した。

品質確認:

| Command | Result |
|---|---|
| `cargo fmt --check` | success |
| `cargo check` | success、warningなし |
| `cargo test` | 133 passed |
| fixture / single-source CLI | success |
| `git diff --check` | success |

Cargo依存関係、M0 fixture generator、fixture仕様、FFT、Plot config format、
Session preset formatは変更していない。commit / pushは行っていない。
