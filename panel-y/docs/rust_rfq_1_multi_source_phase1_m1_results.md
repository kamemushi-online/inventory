---
project: Panel_y
doc_type: M1 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M1"
created: "2026-07-25"
updated: "2026-07-25"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m0_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M1 実行結果

## 1. 目的とスコープ

M1では、実際の複数source loadや2ソースUIへ進まず、後続milestoneの土台となる
identity modelとsingle-source互換state modelをRust実装へ導入した。

実施範囲は `SourceId`、`ChannelKey`、`TimeTransform`、
`DataSourceState`、`WorkspaceDataState`、View / Row / analysis / FFTの内部key移行、
およびSession preset / Plot config version 1の単一ソース互換境界である。

Source #2の追加・削除UI、source-aware load job/result、複数time描画、
mixed-source FFT guard、複数source保存形式には着手していない。

## 2. Git baselineと開始時worktree

実行日: 2026-07-25

| Item | Measured value |
|---|---|
| Current branch | `codex/rfq1-multi-source-phase1-docs` |
| HEAD | `6a003e35d2133db219caf64dabd2323414f43bd8` |
| `main` | `2ba81459519735003838d61d24d4efa2f8359b38` |
| `origin/main` | `2ba81459519735003838d61d24d4efa2f8359b38` |
| Rust | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| Cargo | `cargo 1.96.0 (30a34c682 2026-05-25)` |
| Initial `git diff --check` | 成功 |

M1開始時のworktree:

```text
 M code/rust_phase1/README.md
 M docs/rust_rfq_1_multi_source_phase1_work_plan.md
?? code/proto_3_1b/generate_multi_source_validation_data.py
?? docs/rust_rfq_1_multi_source_phase1_m0_execution_prompt.md
?? docs/rust_rfq_1_multi_source_phase1_m0_results.md
?? docs/rust_rfq_1_multi_source_phase1_m1_execution_prompt.md
```

これらはM0または実行指示に由来する既存の未コミット変更として保持した。
M0結果文書のstatusが `complete` であることを確認してから開始した。

## 3. 変更前 `cargo test`

`code/rust_phase1` でM1実装前に実行した結果:

```text
test result: ok. 106 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 4. Identity model

- `SourceId(u64)` を安定した内部IDとして追加した。path、表示番号、
  collection indexはIDとして使用しない。
- `ChannelKey { source_id, channel_path }` を追加した。同じsource / pathは同一key、
  異なるsourceの同一pathは別keyとなる。
- `TimeTransform { offset_seconds }` と `to_display()`、`to_raw()`、
  `raw_range()`、`display_range()` を追加した。
- default offsetは0.0であり、raw time配列自体は変更しない。
- `next_source_id` は2から始まり、privateな単調counterで予約したIDを再利用しない。

## 5. Workspace / Source #1初期化

`PanelYApp` は単一の `DatasetState` ではなく `WorkspaceDataState` を所有する。
初期workspaceには安定ID 1のSource #1だけを作り、`source_order` も1件、
selected sourceもSource #1、`next_source_id` は2とした。

Source #1は従来のdefault Parquet path、schema、shared time、`ChannelStore`を所有し、
time transformは0.0、generationは0から始まる。path変更時はlabelを更新し、
generationを単調に進める。

次の共通accessorを実装した。

- `source()` / `source_mut()`
- `selected_source()` / `selected_source_mut()`
- `channel()` / `has_channel()`
- `source_number()`

存在しないsource/keyは `None` / `false` を返す。workspace-levelのflat raw cacheは
追加していない。M1の既存UI向けには、selected Source #1への
`Deref` / `DerefMut` をsingle-source互換境界として集中させた。

## 6. View / Row移行

- `ViewState.selected_channel` を `Option<ChannelKey>` へ変更した。
- `RowChannel` は文字列pathではなく `ChannelKey` を所有する。
- 行内重複判定、選択、追加、削除、cache action、rendering traceはfull keyを使う。
- 同じkeyの重複は拒否し、異なるsourceの同じpathは同じ行へ追加できる。
- visible loaded channelの一意化はpathではなくfull key単位で行う。

Parquet reader、schema、単一source内の `ChannelStore`、path-based load job/resultは
引き続き元のchannel pathを使用する。

## 7. Load / rendering / analysisのsingle-source boundary

UIからの `ChannelKey` はload要求境界でselected Source #1と照合し、既存の
Parquet path / channel pathへ解決する。`ActiveLoadJob` と `LoadJobResult` は
工程表どおりpath-basedのままであり、source id / generationは追加していない。

rendering、Hover、Cursor readout、区間統計は内部identityとして
`ChannelKey` を保持する。raw timeとLine / Stepの計算経路は変更していない。
区間統計cache keyもsource identityを保持する。

FFTのtarget、request key、job input、spectrumへ `ChannelKey` を通し、同じpathでも
sourceが異なれば別request identityとなる。数値計算、表示path、同一ソースの
reference peak、stale result判定は従来動作を維持した。mixed-source FFT拒否は
M2以降へ残した。

## 8. Session preset / Plot config version 1互換

Session presetは外部表現を元pathのまま保持し、capture時はselected Source #1の
keyだけをpathへ変換し、apply時はSource #1の `ChannelKey` を再構築する。

Plot config version 1のversionとJSON shapeは変更していない。

- load: string pathを現在のSource #1の `ChannelKey` へ変換する。
- save: `ChannelKey` から元の `channel_path` だけを出力する。
- `data_path`、rows、`ch_styles`、`rust.rows[].channels[].path` は既存形式を維持する。

## 9. 追加・更新したテスト

最終結果:

```text
test result: ok. 117 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

M1で追加・強化した範囲:

- `ChannelKey` のsource/path identity
- offset 0、正負offset、raw/displayとrangeの往復
- default Source #1、source order、表示番号、selected source、next ID
- missing source/key accessorとsourceを横断しないchannel lookup
- source path変更時のlabel / generation
- 同じkeyの重複拒否と異なるsourceの同path許可
- full key単位のvisible channel一意化
- Session presetのSource #1 round trip
- Plot config v1のSource #1 key変換と元path保存
- FFT request identityへのsource保持

既存のFFT reference peak / stale result、Line / Step、Hover、Cursor、区間統計、
Row、Plot configテストは削除・緩和せず成功した。

品質確認:

| Command | Result |
|---|---|
| `cargo fmt --check` | success |
| `cargo check` | success、warningなし |
| `cargo test` | 117 passed |
| `git diff --check` | success |

Cargo.toml / Cargo.lockは変更していない。M0 fixtureのSHA-256も記録値と一致した。

## 10. CLI確認結果

開始時は `test_100k.parquet` が作業ツリーに存在せず、指定した2コマンドは
`No such file or directory` で失敗した。このため、repositoryの依存関係や
source fileを変更せず、ローカルcache済みNumPy / PyArrowで既存generatorと
同じ100,000点・4channelのgitignore対象fixtureを生成して再実行した。

```bash
cargo run -- --schema ../proto_3_1b/data/test_100k.parquet
cargo run -- --load-channel ../proto_3_1b/data/test_100k.parquet sine_50Hz
```

| Check | Result |
|---|---|
| Schema rows / row groups / columns | `100000` / `1` / `5` |
| Time column | `time` (`DOUBLE`) |
| Channels | `sine_50Hz`, `pwm_1kHz`, `noise`, `chirp_1_500Hz` |
| Loaded channel | `sine_50Hz` |
| Samples | `100000` |
| Time range | `0.000000 .. 0.999990` |
| Value range | `-10.000000 .. 10.000000` |
| Projection | `[0, 1]` |
| Envelope | 4000 buckets / 8000 draw points |

両コマンドとも最終実行はsuccessである。生成したParquetは `*.parquet` の
gitignore対象であり、worktree statusには含まれない。

## 11. GUI手動確認

Rust GUIを起動し、Computer Useによる操作を試みたが、Macがロック中で
自動解除できなかったため、GUI手動確認は未実施である。起動プロセスは終了した。
Source #2のGUI確認はM1スコープ外のため試みていない。

## 12. 既知の制約

- 実際のworkspaceにはSource #1しか作らず、Source #2 UIは存在しない。
- load job/resultはpath-basedであり、source id / generationを持たない。
- load resultのsource-aware stale判定は未実装である。
- `TimeTransform` はidentity/state modelとUnit testまでで、描画へ未接続である。
- 複数source time range、Hover、Cursor、統計、FFT拒否は未実装である。
- Plot config v1とSession presetは現在のSource #1だけを保存・復元する。
- GUI単一ソース手動回帰はMacロックのため未実施である。

## 13. M2への申し送り

- Source追加・削除・選択をworkspace APIとUIへ接続する。
- schema/channel load API、`ActiveLoadJob`、`LoadJobResult` にsource idと
  generationを追加する。
- result適用時にsource存在、generation、channel pathを検証し、
  path一致だけに依存しないstale判定へ移行する。
- 既存のselected Source #1 `Deref` 互換境界を、明示的なsource指定へ段階的に
  置き換える。
- M0の2fixtureを使い、独立source stateと同path channelの分離を検証する。
- multi-time描画、display-time transform、mixed-source FFT guard、
  複数source保存guardは工程表の該当milestoneまで先行実装しない。

## 14. M1完了判定

M1はcompleteと判定する。M0 completeとbaselineを確認し、identity model、
Source #1 workspace、View / Row / analysis / FFTのkey移行、legacy config境界、
必須accessorとUnit testを実装した。最終的に `cargo fmt --check`、
`cargo check`、117 tests、指定CLI、`git diff --check` が成功した。

2ソースUI、実際の複数source load、source-aware job/result、複数time描画、
mixed-source FFT guard、保存形式変更、Cargo依存関係変更、commit、pushは
行っていない。
