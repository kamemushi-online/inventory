---
project: Panel_y
doc_type: M7 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M7"
created: "2026-08-21"
updated: "2026-09-08"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
source_design: "docs/rust_rfq_1_multi_source_overlay_design.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m6_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M7 実行結果

## 1. 目的と結論

M0–M6で実装したmulti-source Phase 1を統合し、機能、安全性、将来拡張境界、品質、
性能のRelease Gateを確認した。

167件の自動test、Debug/Release GUIプロセス起動、2source integration/performance
benchmark、M4–M6でユーザー確認済みのGUI smokeを統合判定し、全Release Gateを通過した。
blocker級の既知不具合はない。M7およびRFQ #1 Multi-source Phase 1を `complete` とする。

本書の検証結果は2026-08-21のM7完了時点の記録である。その後、Phase 2 Time Alignmentは
実装・自動テストまで完了し、GUI smoke待ちとなった。現在の進捗と未実装範囲は
[工程表](./rust_rfq_1_multi_source_phase1_work_plan.md)を参照する。

## 2. 開始条件

- 実行日: 2026-08-21
- branch: `codex/rfq1-multi-source-phase1-docs`
- HEAD: `44e2774`
- HEAD subject: `Record M6 GUI smoke completion`
- 開始時worktree: clean
- M0–M6: complete
- M4–M6 GUI smoke: ユーザー確認済み

commit、push、PR作成は行っていない。

## 3. M7 multi-source benchmark

単一source用の既存benchmarkとは別に、2sourceを同時に同じworkspaceへロードするCLIを
追加した。

```text
cargo run --release -- --bench-multi-source \
  ../proto_3_1b/data/multi_source_validation_1.parquet \
  ../proto_3_1b/data/multi_source_validation_2.parquet
```

処理内容:

1. 両sourceのschemaとtimeを独立してロード
2. fixture内の全6channelをsource別 `ChannelStore` へロード
3. 同一pathの `iu` と `gate_pwm` を同一Rowへoverlay
4. `gate_pwm` はStep、その他はLineとして4Rowを構築
5. display range union上で24個の可視範囲を生成
6. 24回の連続Pan範囲でcache hit/missをaggregateおよびsource別に集計
7. 1,000 display time位置、6,000 lookupのHover性能を測定
8. raw/draw cache memoryとprocess RSSを出力

既存の単一source `--bench-phase2` も、共通の可視trace benchmark helperと
source-aware Hover helperを使用するよう整理した。

## 4. Benchmark dataset

| Source | Time range | Sample rate | Rows | Channels |
|---|---:|---:|---:|---|
| #1 | 0.0–2.0 s | 50,000 Hz | 100,001 | `iu`, `torque`, `gate_pwm` |
| #2 | 0.5–1.75 s | 64,000 Hz | 80,001 | `iu`, `speed`, `gate_pwm` |

benchmarkは両sourceの全channelをロードする。raw cacheは合計3.4 MiB、6channelである。
display rangeは `0.0 .. 2.0 s`、layoutは4Row、Line 4trace、Step 2traceとなる。

## 5. Performance results

環境:

- macOS 26.6.1 / arm64
- rustc 1.96.0
- cargo 1.96.0
- requested visible envelope buckets: 1,200
- visible ranges: 24
- Pan ranges: 24
- Hover positions: 1,000

### 5.1 Summary

| Metric | Debug | Release |
|---|---:|---:|
| 2source full load wall | 0.070 s | 0.009 s |
| visible trace total / 24 | 0.070 s | 0.004 s |
| visible trace average | 2.9 ms | 0.2 ms |
| visible trace maximum | 7.9 ms | 0.7 ms |
| Pan trace average | 2.1 ms | 0.1 ms |
| Pan trace maximum | 3.6 ms | 0.3 ms |
| Hover average / lookup | 0.867 µs | 0.075 µs |
| RSS before load | 10.2 MiB | 9.0 MiB |
| RSS after load | 20.7 MiB | 15.4 MiB |
| RSS after visible/Pan/Hover | 22.3 MiB | 16.5 MiB |

Releaseでは24範囲すべてのvisible trace生成が1 ms未満、Pan範囲生成が0.3 ms以下だった。
Debugでも最大7.9 msであり、fixture上のwide-range Pan/Zoom backend workloadに
blockerはない。

Hoverは6,000 lookup中4,875 hitだった。残りは `0.0 .. 0.5 s` および
`1.75 .. 2.0 s` でSource #2がrange外となる意図したmissingであり、endpoint clampを
行っていない。

### 5.2 Source-specific cache results

24 Pan frameで両sourceは同じcache遷移を示した。

| Source | Envelope hit/miss | Context hit/miss | Step-edge hit/build |
|---|---:|---:|---:|
| #1 | 20 / 28 | 10 / 14 | 24 / 0 |
| #2 | 20 / 28 | 10 / 14 | 24 / 0 |

最初のvisible benchmarkで構築したStep-edge cacheをPan benchmarkでsource別に再利用した。
最終draw cacheはEnvelope 4件、Step-edge cache 2件、edge memory約0.1 MiBだった。

fixtureはLine tile build閾値未満のため、Line tile hit/buildは0件である。Line tileの
source分離と再利用は既存Unit test
`unloading_and_clearing_caches_are_source_isolated`、
`line_tile_lod_only_applies_to_wide_line_buckets`で継続確認した。

## 6. Lifecycle stress

`workspace_source_lifecycle_stress_preserves_source_one_and_monotonic_ids` を追加した。

- Source #2 add
- waveform cache追加
- path reload
- Source #2 remove

を64回繰り返し、各cycleで次を確認した。

- Source IDが再利用されず単調増加する
- Source #1のtime/channel cacheが保持される
- `source_order` とselected sourceがSource #1へ戻る
- recent path履歴が上限内に保たれる

Row selection、Hover/Cursor、FFT cleanupは既存の
`source_remove_cleans_only_its_row_selection_and_analysis_references` と
`source_two_path_change_preserves_source_one_row_selection_and_cache`で確認した。

## 7. Stale job safety

次の既存testをRelease Gateへ再採用した。

- `removed_source_and_mismatched_result_identity_are_rejected`
- `path_a_b_a_rejects_the_old_generation_result`
- `current_generation_result_applies_only_to_job_source`
- `stale_fft_job_results_are_rejected`
- `source_cleanup_removes_targets_and_invalidates_running_job`

loadはsource IDとgeneration、FFTはsource generation、time fingerprint、transform、
channel sample identityを検証する。remove/reload後の古いbackground resultは適用されない。

## 8. 追加テスト

M7で3件を追加した。

1. `multi_source_benchmark_rows_overlay_same_paths_and_keep_unique_paths`
2. `workspace_source_lifecycle_stress_preserves_source_one_and_monotonic_ids`
3. `multi_source_benchmark_reports_each_source_if_fixtures_exist`

総test数はM6の164件から167件となった。

## 9. 自動検証

`code/rust_phase1` で実行した。

```text
cargo fmt --check
cargo check
cargo test
cargo build --release
cargo clippy --all-targets
git diff --check
```

結果:

```text
test result: ok. 167 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo clippy --all-targets` はerrorなしで完了した。既存UI/FFT関数のargument数、
manual clamp、test fixture初期化などに非blockerのstyle warningが残るが、M7の機能・
安全性・性能に影響するwarningはない。

## 10. Debug / Release startup

`target/debug/rust_phase1` と `target/release/rust_phase1` をそれぞれGUI modeで起動し、
プロセスが継続動作することを確認後に終了した。

GUI操作の機能確認は次のユーザー確認済み結果を再利用した。

- M4: 2source Line/Step、Fit All、共通Pan/Zoom/box zoom、source-only区間
- M5: source別Hover/Cursor/統計、same-source FFT、mixed-source guard、stale FFT
- M6: persistence button guard、v1 Load後のSource #1置換

## 11. Release Gate

工程表のRelease Gateをすべて完了へ更新した。

- Functional: 合格
- Safety: 合格
- Future readiness: 合格
- Quality: 合格

M7完了時点では `TimeTransform` をraw/display boundaryで使用し、offsetは0.0であった。
時間offset UIをraw time arrayやsource-owned cacheの所有構造を変更せず追加できることを
確認した。

## 12. Deferred scope

M7完了時点で後続工程へ残した項目は次のとおり。

- 時間offset編集UI
- `Start at zero`
- `Fit Overlap`
- multi-source Plot config `rust.version: 2`
- multi-source Session preset
- source別FFT result overlay
- 3source以上のUI
- cross-source resampling / alignment analysis

2026-09-08の進捗整理で、`Add matching channel from all sources` の延期表記を訂正した。
この機能は[M3で実装済み](./rust_rfq_1_multi_source_phase1_m3_results.md)である。
時間offset編集UIと `Start at zero` は、その後のPhase 2で実装済みとなり、GUI smokeが残る。

## 13. 変更ファイル

- `code/rust_phase1/src/main.rs`
- `code/rust_phase1/README.md`
- `docs/rust_rfq_1_multi_source_phase1_work_plan.md`
- `docs/rust_rfq_1_multi_source_phase1_m7_results.md`

Cargo dependency、Parquet fixture、generatorは変更していない。

## 14. 完了判定

M7のintegration、performance、stress、品質検証が成功し、Phase 1 Release Gateを
すべて通過した。RFQ #1 Multi-source Phase 1を `complete` と判定する。

M7完了時の次工程だった時間offset編集UIと `Start at zero` は、Phase 2で実装済みである。
現在の次工程は[Phase 2 Time AlignmentのGUI smoke](./rust_rfq_1_multi_source_phase2_time_alignment_results.md)。
