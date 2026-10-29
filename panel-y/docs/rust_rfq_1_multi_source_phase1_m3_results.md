---
project: Panel_y
doc_type: M3 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M3"
created: "2026-07-26"
updated: "2026-07-26"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m2_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M3 実行結果

## 1. 目的とスコープ

M2で完成したsource-aware lifecycle、schema/channel load、per-source cacheをUIへ接続し、
最大2sourceを登録・ロード・削除できる `Data Sources` UIを実装した。

内部identityは引き続き `SourceId` と元の `channel_path` からなる `ChannelKey` とし、
表示だけを `channel_name #n` へ統一した。異なるtime配列の同時描画、source別
Hover/Cursor/statistics、mixed-source FFT、複数source persistenceはM3の対象外とした。

M0、M1、M2結果文書の `status: complete`、M2の同一path明示reload時のgeneration更新と
cache clear、M0 fixtureの存在を実装前に確認した。

## 2. Git baselineと開始時worktree

- 実行日: 2026-07-26
- branch: `codex/rfq1-multi-source-phase1-docs`
- HEAD: `a5a43664ac4e780d531a61de4817b93f469c3299`
- HEAD subject: `a5a4366 Refactor implementation`
- `main`: `2ba81459519735003838d61d24d4efa2f8359b38`
- `origin/main`: `2ba81459519735003838d61d24d4efa2f8359b38`
- rustc: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`
- 開始時worktree: `docs/rust_rfq_1_multi_source_phase1_m3_execution_prompt.md` のみuntracked
- M3対象source、`Cargo.toml`、`Cargo.lock`に開始前差分なし
- 開始時 `git diff --check`: 成功

`git fetch`、`git pull`、branch作成、commit、push、PR作成は行っていない。

## 3. 変更前cargo test

`code/rust_phase1` で2026-07-26に実行した。

```text
test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

実行時HEADとtoolchainは上記baselineのとおり。

## 4. Data Sources UI

既存の単一 `Dataset` セクションを、`source_order` 順に全sourceを表示する
`Data Sources` セクションへ置き換えた。

- 起動時は従来のdefault pathを持つSource #1のみ
- `Add Source` は空pathのSource #2を追加
- 2source時は `Add Source` を無効化
- UI上限だけを2とし、workspaceの `add_source()` は3source以上も保持可能
- 各sourceに `#n file`、Path、Browse、Load Schema、Remove、schema/time/cache/loading状態を表示
- 空pathは `#n (no file)` と表示
- widget IDはstableな `SourceId` を含み、pathや表示番号をidentityに使用しない
- 最後のsourceも削除可能。zero-source状態から `Add Source` で新しい表示 #1へ復帰可能

表示番号は常に `source_order` のposition + 1から計算する。Source #1削除後は旧 #2を
同じ `SourceId` のまま新 #1として表示する。

## 5. Source別Path / Browse / Schema lifecycle

Path edit、Browse、Load Schema、recent path、Removeを明示的な `SourceId` 境界へ接続した。

- path変更と明示reloadはM2のgeneration/reset境界を使用
- 対象sourceのschema/time/raw/draw cacheだけをreset
- 対象sourceのRowChannel、selected channel、Hover、cursor/statistics cache、FFT参照をcleanup
- 他sourceのschema/time/cache/RowChannel/selectionを保持
- Source #2のpath変更ではSource #1用default Plot config pathを変更しない
- Path欄の編集中はrecent historyへ追加せず、Browse / Recent選択 / schema load成功など
  path確定時だけ履歴へ追加
- schema failureは対象sourceだけをreset
- missing source、空path、invalid pathはstatus/errorで処理しpanicしない
- status/errorへ `#n file` のsource presentationを付与

## 6. Source別Channel browserとfilter

Channel browserを `source_order` 順のsource groupへ変更し、schema未ロードsourceも見出しと
状態を表示する。

- channel row、selection、loaded/loading/unloaded、action、Row membershipはfull `ChannelKey`
- source group内の既存sort/state filterを維持
- widget IDは `(SourceId, channel_path)` を含む
- selected detailsは選択keyのsource schema/cacheだけを参照
- same source / same pathの二重loadを抑止
- different source / same pathは別channelとして選択・load可能

case-insensitive filterは元のchannel name/path、presentation label、`#n`、source label、
Parquet file nameを検索する。

## 7. Channel presentation API

表示用の `SourcePresentation` と `ChannelPresentation`、および次の統一helperを
`main.rs` に追加した。

```text
source_presentation(workspace, source_id)
channel_presentation(workspace, channel_key)
channel_label_or_fallback(workspace, channel_key)
channel_tooltip_or_fallback(workspace, channel_key)
```

- short labelは単一source時も常に `channel_name #1`
- 元nameが空なら元pathへfallback
- tooltipは `file name / original channel path`
- source番号は呼出時の `source_order` から計算
- removed/missing source keyはpanicせず `None` または安全なfallback
- internal `channel_name`、`channel_path`、`ChannelKey` へsuffixを保存しない

## 8. `#n` を反映した表示箇所

統一presentationを次へ接続した。

- Data Sourcesとsource status
- Channel browser、selected details、filter
- Load / Add target、progress、completion、failure、stale status
- Row controlsとempty-row案内
- plot axis/title相当のtrace labels
- Hover readout
- Cursor A/B value table
- interval statistics table
- FFT target、result summary、spectrum、hover、error labels
- Cache controls、Unload/Clear status

Raw columns詳細、selected detailsの `Path:`、CLI report、reader/cache keyは元pathを確認する
ため意図的にsuffixなしの内部値を表示・使用する。

## 9. Row追加とmatching channel

任意sourceのfull `ChannelKey` を選択RowへLoad/Addする経路へ変更した。

- same keyの同一Row重複を拒否
- different source / same pathを同一Rowへ追加可能
- different source / different pathを同一Rowへ追加可能
- cached channelはkeyのsource storeから再利用
- load完了時はactive jobが保持したfull keyをRowへ追加
- Row controlsのRemoveとmembershipはfull key比較

`Add matching channel from all sources` を追加した。pure helperで各sourceを走査し、完全な
channel path一致を優先し、見つからない場合だけ元channel nameの一意一致へfallbackする。
schema未ロード、候補なし、ambiguousはskipし、request/existing/skip/ambiguous数をstatusへ
要約する。

## 10. Source path change / remove cleanup

小さなsource-aware cleanup境界を追加した。

- 全Rowから対象sourceのRowChannelだけを削除
- 他sourceのRowChannelとcacheを保持
- 対象sourceのselected channelをclear
- Hoverとcursor A/Bをclear
- interval statistics cacheをinvalidate
- FFT targetから対象keyを削除
- 対象sourceを含むFFT result/running jobをinvalidate
- workspaceからsourceをremoveし、表示番号を動的に再計算
- active sourceの削除で旧 #2が新 #1になった場合、既定のPlot config pathも新しい
  active sourceへ追従。ただしユーザー指定のcustom pathは保持
- source ID counterはmonotonicのままでremoved IDを再利用しない

## 11. Source別cache / loading / status

- `UnloadChannel` はfull `ChannelKey`
- `Unload all` と `Clear draw caches` は明示的な `SourceId`
- cache一覧はsource group別に `name #n` を表示
- source値とworkspace aggregate memoryを分けて表示
- Path/Browse/Load Schema/Remove/Unload/Clearは対象sourceの
  `load.is_source_loading(source_id)` だけでguard
- Source #1のjob中もSource #2のpath/schema操作は可能
- active load jobは表示名を保存せずfull `ChannelKey` を保持し、global progress/status/errorを
  描画・適用時の最新presentationから生成
- source削除による再採番中もロード中ラベルがnew `#n`へ即時追従
- M2のcache/load/failure/stale isolationを維持

## 12. M4/M5前のcross-source time safety

M3ではplotとanalysisが同時に扱うsourceをworkspaceのactive/selected sourceだけへ限定した。

- Rowに別source keyを保持・loadできるが、別source traceは描画計算から除外
- UIには `Additional-source traces are staged; simultaneous rendering starts in M4` と表示
- Hover/Cursor/statistics/FFT候補はactive sourceのloaded channelだけ
- FFT job inputは単一source keyだけを受け、timeはそのsourceから明示取得
- source cleanupでFFT target/result/jobを無効化

したがってSource #2 valueをSource #1 timeで計算せず、M4のmulti-time renderingとM5の
source別analysisを先取りしていない。

## 13. 追加・更新したテスト

既存テストを削除・緩和せず、次を追加または拡張した。

- UI上限2と3source以上を保持できるworkspace model
- zero-sourceから新しい表示 #1への復帰
- 常時suffix、同名/異名、空name fallback、tooltip、内部identity不変、削除後再採番
- source number/label/file nameを含むbrowser filter
- exact path優先、unique name fallback、missing/ambiguous matching
- same key重複拒否、different sourceのsame/different path Row追加、full-key Remove
- Source #2 path change/removeの対象限定cleanup
- selection、cursor、interval statistics cache、FFT target/result/job cleanup
- source-specific loading guard
- Source #2 valueをSource #1 timeのanalysisへ渡さないM3 boundary
- FFT presentation tooltipとsource cleanup
- Path編集中のrecent history非登録と、path確定時の登録
- active source削除後の既定Plot config path追従、custom path保持、zero-source復帰
- active load progress / failure labelのsource再採番追従

最終実測:

```text
test result: ok. 150 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 14. Fixture / CLI確認

次のM0 fixtureが存在することを確認した。

```text
../proto_3_1b/data/multi_source_validation_1.parquet  586K
../proto_3_1b/data/multi_source_validation_2.parquet  359K
```

指定4コマンドはいずれも成功した。

| Fixture | Schema rows / channels | `iu` samples | time range | sample rate相当 | value range |
|---|---:|---:|---|---:|---|
| `multi_source_validation_1.parquet` | 100,001 / `iu, torque, gate_pwm` | 100,001 | `0.000000 .. 2.000000` | 50,000 Hz | `-10.000000 .. 10.000000` |
| `multi_source_validation_2.parquet` | 80,001 / `iu, speed, gate_pwm` | 80,001 | `0.500000 .. 1.750000` | 64,000 Hz | `-7.850000 .. 8.650000` |

両方のCLI出力は `channel: iu`、`channel path: iu` であり、reader/CLIへ
`#1` / `#2` suffixは混入していない。

## 15. GUI確認

GUI操作確認は未実施。

`cargo run` で `target/debug/rust_phase1` の起動自体は成功した。しかしこの未バンドル
binaryは利用可能なComputer Useアプリ一覧へ現れず、absolute binary path、`Panel-y`、
`rust_phase1`、`Panel-y v3.0` のいずれも操作対象として認識されなかった。このため、
実行指示19節の14項目を確認済みとは記録しない。

## 16. Persistence互換と暫定guard

- Plot config version 1とSession presetのformatを変更していない
- 保存するchannel pathへpresentation suffixを混入させていない
- 1source時の既存load/apply/saveとsession round tripテストを維持
- active source削除後は、既定パスを使用中の場合だけ新しいactive source由来の
  `.pyc.json` pathへ追従し、custom指定済みpathは変更しない
- 2source Rowをv1へflattenして誤保存しないため、M3の最小暫定guardとして
  multiple-source時のPlot config操作とSession preset Save/Applyを無効化し理由を表示

正式な複数source persistence/guardはM6へ残す。

## 17. 既知の制約

- 追加sourceのchannelはRow modelとsource別cacheへstageされるが、同時描画しない
- plot/Line/Step/Auto Y/X rangeはactive sourceだけ
- Hover/Cursor/statistics/FFTもactive sourceだけ
- `TimeTransform` とoffset UIは未接続
- 複数source Plot config / Session presetは暫定的に無効
- GUI手動シナリオは上記環境制約により未実施

## 18. M4への申し送り

- `ChannelKey` と `ChannelPresentation` を維持したまま、各traceが自身のsource
  time/storeを使うmulti-time renderingへ拡張する
- staged additional-source traceを実描画へ接続する
- source別Line / Step / Auto Y、display range union、Fit All / Fit Overlapを実装する
- axis、Hover、Cursor、FFTへ渡すidentityをpath-onlyへ戻さない
- M3のactive-source analysis filterはM5のsource別time lookupへ置き換える
- persistence暫定guardの正式化はM6で行う
- Phase 1では単一source時もチャンネル表示へ常に `#1` を付ける。単一source時だけ
  suffixを隠す案は、source追加・削除時に表示名が変わる影響を含め、M4とは分離した
  後続のUI polish候補として扱う

## 19. M3完了判定

M3のData Sources UI、source別browser/filter/load/cache、統一presentation、Row/matching、
cleanup、再採番、cross-source time safetyを実装した。指定fixture CLI、format/check/test、
diff checkは成功し、Cargo依存関係と対象外ファイルを変更していない。

GUI手動確認は環境上未実施だが、Unit testとCLIでM3のpure/model/load境界を検証したため、
M3を `complete` と判定する。
