---
project: Panel_y
doc_type: M5 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M5"
created: "2026-07-26"
updated: "2026-07-26"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
source_design: "docs/rust_rfq_1_multi_source_overlay_design.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m4_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M5 実行結果

## 1. 目的とスコープ

M4で完成したmulti-time Line / Step描画を、Hover、Cursor A/B、区間統計、FFTへ接続した。

M5専用の実行指示ファイルは開始時点で存在しなかったため、工程表の
`M5 — Hover, Cursor, Statistics, and FFT Safety`、overlay設計書のStep 5、
M4結果文書の申し送りを実装条件として使用した。

M5では時間offset編集UIを追加していない。`TimeTransform` 境界を通る解析実装と
non-zero offsetのUnit testまでを対象とした。

## 2. 開始条件

- 実行日: 2026-07-26
- branch: `codex/rfq1-multi-source-phase1-docs`
- HEAD: `fa9253a`
- HEAD subject: `Refine application behavior and structure`
- 開始時worktree: clean
- M4までcommit済み
- ユーザー確認済みM4 GUI smoke: 合格

commit、push、PR作成は行っていない。

## 3. Source別Hover

Hover位置は既存どおり `ViewState.hover_x` にdisplay timeとして保持する。

各visible traceは次の順序で値を解決する。

1. traceの `ChannelKey` から所属sourceを確定
2. `TimeTransform::to_raw()` でdisplay hover timeをsource固有raw timeへ変換
3. 所属sourceのtime/value配列だけを参照
4. Lineはnearest sample、Stepはheld sampleを返す
5. raw time range外ではendpointへclampせず `None` を返す

active sourceによるM3のanalysis filterは削除した。同名pathでも `ChannelKey` が異なる
ため、`iu #1` と `iu #2` は独立した値を表示する。

## 4. Source別Cursor A/B

Cursor A/Bは共通display timeのまま保持し、チャンネルごとにraw timeへ逆変換する。

- visible channelはfull `ChannelKey` で一意化
- 各sourceのtime/value配列でnearest sampleを検索
- AまたはBがsource range外なら、そのreadoutだけ `None`
- A/Bの両方がrange外なら、そのsource channelのreadout行を表示しない
- 表示ラベルとtooltipはsource番号・ファイル名・channel pathを維持

これにより、異なるsample rateとsample countを持つ同名channelでも、各sourceの
実サンプルを参照する。

## 5. Source別区間統計

区間統計はvisible channelごとに次を行う。

1. display Cursor A/Bを所属sourceのraw A/Bへ変換
2. 両端がsource raw range内にあることを検証
3. source固有time配列からnearest indexを求める
4. source固有channel valuesのinclusive sample intervalで統計を計算

Mean、Max、Min、Peak-to-Peak、RMS、Delta、Slope/sは従来の計算を維持する。
constant offsetではraw B-Aとdisplay B-Aが等しいため、Slope/sの時間差は変化しない。

片方のCursorがsource range外の場合、そのsourceの区間統計は生成しない。

## 6. Source-aware statistics cache

区間統計cache keyをworkspace全体へ拡張した。

共通部分:

- display Cursor A bits
- display Cursor B bits

channelごとの部分:

- full `ChannelKey`
- source generation
- time offset bits
- time sample count
- raw time start/end bits
- channel sample count

source path変更、reload、time identity変更、将来のoffset変更、visible channel構成変更で
古い統計cacheを再利用しない。

## 7. FFT mixed-source guard

FFTの利用可能targetを全visible sourceへ拡張し、target identityはfull `ChannelKey` を
維持した。

- 1ジョブのtargetがすべて同一sourceか、input copy前に検証
- mixed-source選択時は警告を表示し、`Run FFT` を無効化
- API側のinput builderでも同じguardを実行
- 同一sourceの複数channel FFTは従来どおり許可
- Cursor A/Bはtarget sourceのraw timeへ変換してsample intervalを確定
- FFTの `dt` はraw time差から計算し、constant offsetの影響を受けない

guard message:

```text
FFT targets must belong to exactly one data source; mixed-source FFT was not started.
```

## 8. FFT request/result identity

`FftRequestKey` に次を追加した。

- source id
- source generation
- time offset bits

既存のtime sample count、raw time start/end、index range、window、full channel keys、
channel sample countsも継続して含める。

background result適用時はjob id/key一致だけでなく、現在のworkspaceに対して次を再検証する。

- sourceが存在する
- source generationが一致する
- time transformが一致する
- time fingerprintが一致する
- target channelが同じsourceに存在し、sample countが一致する

source path変更、reload、unload、transform変更後のstale FFT resultは表示しない。

## 9. 追加・更新テスト

M5で5件を追加し、M3 safety boundary test 1件をM5の全source解析契約へ更新した。

1. `multi_source_hover_and_cursor_use_each_source_time_transform_and_range`
   - source別raw lookup
   - non-zero `TimeTransform`
   - `iu #1` / `iu #2` readout
   - source range外 `None`
2. `interval_statistics_use_each_sources_raw_cursor_range`
   - 異なるraw time/sample interval
   - source別値とMean
3. `interval_statistics_cache_key_tracks_source_generation_and_time_transform`
   - generation変更
   - transform変更
4. `mixed_source_fft_is_rejected_before_input_copy`
   - mixed-source target拒否
5. `same_source_fft_uses_raw_cursor_times_without_changing_dt`
   - 同一source複数channel
   - display/raw Cursor変換
   - constant offsetで `dt` 不変

既存stale FFT testもworkspace generation変更を含む判定へ強化した。
Line / Step hover testへsource range外の非clamp条件を追加した。

## 10. 自動検証

`code/rust_phase1` で実行した。

```text
cargo fmt --check
cargo check
cargo test
cargo build --release
git diff --check
```

最終結果:

```text
test result: ok. 158 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Phase 0 FFT reference fixtureが存在する環境で、既存のFFT reference peak testも成功した。
release profileのbuildも成功した。

## 11. M5 GUI smoke checklist

M0 fixtureを使用する。

1. Source #1 / Source #2の `iu` を同じRowへ表示する
2. overlap区間でHoverし、`iu #1` と `iu #2` の両方に値が表示される
3. `0.0 .. 0.5 s` と `1.75 .. 2.0 s` でSource #2のHover値が表示されない
4. overlap区間へCursor A/Bを置き、両sourceのCursor値と区間統計が番号付きで表示される
5. Cursor端点をsource range外へ移し、端点値へclampした統計が表示されない
6. Source #1の `iu` と `torque` をFFT targetに選び、FFTを実行できる
7. Source #2の `iu` と `speed` をFFT targetに選び、FFTを実行できる
8. `iu #1` と `iu #2` を同時にFFT targetへ選び、mixed-source警告とRun無効を確認する
9. FFT結果表示後に対象sourceのpathを変更し、古い結果が消えて再表示されない
10. Source #2削除後、単一sourceのHover、Cursor、統計、FFTが回帰していない

### 確認結果

2026-07-26にユーザーが上記M5 GUI smoke checklistを実施し、全10項目の完了を報告した。

- 2source同名channelのHover値とsource-only区間: 合格
- source別Cursor A/B値と区間統計: 合格
- source range外のendpoint clamp抑止: 合格
- Source #1 / Source #2の同一source複数channel FFT: 合格
- mixed-source FFTの警告表示とRun無効化: 合格
- source path変更後のstale FFT結果破棄: 合格
- Source #2削除後の単一source回帰: 合格

## 12. 変更ファイル

- `code/rust_phase1/src/main.rs`
- `code/rust_phase1/src/fft_analysis.rs`
- `code/rust_phase1/README.md`
- `docs/rust_rfq_1_multi_source_phase1_m5_results.md`

Cargo依存関係、fixture generator、Parquet生成物は変更していない。

## 13. M6への申し送り

- multi-source状態を欠落させる保存操作を有効化しない
- source pathとRow channelはfull `ChannelKey` の対応を維持する
- 将来offset UIを追加する場合も、raw cacheとFFT `dt` へoffsetを焼き込まない
- config適用でsource generationまたはtransformが変わる場合はanalysis cacheを無効化する
- M6ではmulti-source永続化のguardとoperational polishを実施する

## 14. M5完了判定

実装、自動検証、M5 GUI smoke checklistがすべて成功した。
M5を `complete` と判定する。
