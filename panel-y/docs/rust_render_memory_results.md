---
project: Panel_y
created: "2026-10-25"
status: measured; warm rendering and macOS release improved; cold construction remains
---

# 36ch描画遅延とソース削除後のメモリ保持

追補: [2026-10-26の初回描画調査](rust_cold_render_results.md)では、更新後のmacOSで改善前後を再比較し、初回CPUフレームを226.33→165.61 msへ短縮した。以下は2026-10-25時点の記録。

36chの計算待機中CPUフレームp95は53.43→11.56 msへ短縮した。macOS用launcherでは全削除後RSSが1,642→75 MiB、physical footprintが1,633→13 MiBへ低下した。初回の約0.4秒の描画準備は引き続き課題として残る。

PR #53（`bcc7763`）で残った描画遅延とRSS保持を調査した。性能値は同じ4ソース・合計36M行相当・36ch、Cursor A=0.25 s / B=9.5 sで比較する。入力ファイル・配置・操作・試行回数は[前回の測定](rust_interval_statistics_multi_source_results.md)と同じ。

## 描画の原因と改善

- Pan後の有効な表示範囲をclamp処理で再構成すると、例えば`(1.91, 7.91)`が`(1.9100000000000001, 7.91)`になる。この差を範囲修正と判断して全ソースのenvelope cacheを破棄していた。有効な範囲は端点をそのまま返し、Panや小幅Zoomでキャッシュを再利用するよう修正した。
- 密なStep波形をmin/max envelopeへ縮小する経路では、既存の値タイルを利用せずraw配列を再走査していた。Lineと同じ正確なmin/maxタイルを利用するよう変更した。境界の部分区間はrawから再計算し、拡大時には従来の正確なStep表示へ戻る。
- envelopeを表示範囲へ切り出す前の全体コピーを除去した。
- 測定CLIにUI全体、波形準備、描画命令生成、tessellation、キャッシュ再構築回数を追加した。eguiがサイズ計算のためにUIを再実行する場合も全passを集計する。波形準備と描画命令生成はUI時間の内数。

## RSS保持の原因

全rawバッファの強参照が0になっても、macOSの大きな割り当て用キャッシュが解放済みページを保持していた。この環境ではRSSだけでなくMach physical footprintも高いままだったため、RSS表示だけの問題とは判断しない。

アプリから独立した256 MiBのmalloc/free実験でも、解放後のmalloc live bytesは約14 KiBに戻る一方、RSSとphysical footprintは約257 MiBに残った。`vmmap`は`MALLOC_LARGE (empty)`に256 MiBのresident/dirtyページを報告した。3回の再割り当て・解放では同じメモリを再利用し、累積的な増加はなかった。

`MallocLargeCache=0`をプロセス起動前に設定したA/B実験では、解放直後のRSSが約1.42 MiB、physical footprintが約0.89 MiBへ低下した。`malloc_zone_pressure_relief(NULL, 0)`は解放0 bytesで効果がなく、別のdeferred reclaim設定でも変化しなかった。特定の内部reclaim方式を原因とは断定しない。Apple実装の[起動時設定](https://github.com/apple-oss-distributions/libmalloc/blob/main/src/malloc.c)と[large allocation cache](https://github.com/apple-oss-distributions/libmalloc/blob/main/src/magazine_large.c)も参照した。

## macOSでの起動方法

`run_macos.sh`を追加した。既にビルドした実行ファイルを、既定で`MallocLargeCache=0`にして起動する。設定はこの子プロセスだけに適用し、呼出し側の明示設定を尊重する。

```sh
cargo build --release --offline --locked --manifest-path code/rust_phase1/Cargo.toml
./code/rust_phase1/run_macos.sh
# 比較のためmacOS側のlarge cacheを有効にする場合
MallocLargeCache=1 ./code/rust_phase1/run_macos.sh
```

この対策はmacOSの起動時設定であり、バイナリや`cargo run`を直接起動した場合のallocator動作は変更しない。他OSの挙動も変更しない。現在のmacOS 26.6.2 / arm64で確認した結果であり、Appleのallocator実装が変わった場合は再確認する。キャッシュの再利用を抑えるため、再読込コストも併せて記録する。

## 再測定

同一の計測コードを改善前の本体（`bcc7763`）にも適用して比較した。macOS 26.6.2 / arm64 / 16 GiB、Rust 1.96.0、release。各列は独立プロセスの1実行で、初回フレーム値は分布ではない。

| 指標 | 改善前・既定allocator | 改善後・既定allocator | 改善後・macOS launcher |
|---|---:|---:|---:|
| 計算待機中フレーム p95 ms | 53.43 | 11.56 | 11.97 |
| 計算待機中フレーム 最大 ms | 96.79 | 14.67 | 17.05 |
| 再計算なしフレーム p95 ms | 64.32 | 13.09 | 12.50 |
| 波形準備 中央値 ms（再計算なし） | 43.209 | 0.494 | 0.506 |
| tessellation 中央値 ms（再計算なし） | 9.87 | 9.18 | 8.36 |
| 60 idle frames中のenvelope再構築フレーム数 | 40 | 0 | 0 |
| 初回CPUフレーム ms（各1回） | 459.97 | 373.56 | 401.80 |
| 30回統計パス RSS増加 MiB | 0.125 | 0.172 | 0.172 |
| 全削除約2秒後 RSS MiB | 1,656.75 | 1,642.19 | 75.19 |
| 全削除約2秒後 physical footprint MiB | 1,647.41 | 1,632.83 | 12.63 |
| 全削除約2秒後 malloc live MiB | 2.039 | 2.036 | 2.036 |
| 計算中の末尾ソース削除 UI処理 ms | 0.075 | 0.114 | 0.617 |
| 同削除からworker終了受信 ms | 0.080 | 0.120 | 7.151 |

描画改善だけで待機中p95は約78%、再計算なしp95は約80%短縮した。再計算なしの60フレームは同じ入力列で比較し、envelope再構築が40フレームから0へ減った。待機中は処理速度に応じてフレーム数・Pan/Zoomイベント数が変わるため、統計worker自体の高速化とは解釈しない。launcher使用時のworker中央値は576.23 msであり、全36chの統計計算は引き続き別workerで行う。

初回は全データのタイル・Step edge構築が必要で、約0.4秒の停止が残る。warm frameの残りは主に約8〜9 msのtessellationで、3万件超のShape・約57万頂点を処理している。今回の改善は表示密度を減らさず、意図しない再計算を除去したもの。cold timeの単発値の差を安定した改善率として扱わない。

macOS launcherでは削除後RSSが約95%、physical footprintが約99%低下した。残るRSSには約52.6 MiBのreusable pagesも含まれる。malloc liveは両起動方法とも約2 MiBであり、元の1.6 GiBはアプリが使用中のraw配列ではなかった。pressure-relief前の数値で比較しており、通常viewerに強制trimは追加していない。

### 1・2・4ソースの改善後確認

| ソース数 / ch | 待機中p95 ms | 再計算なしp95 ms | 全削除約2秒後RSS MiB | 同footprint MiB |
|---|---:|---:|---:|---:|
| 1 / 9 | 3.20 | 3.32 | 42.53 | 8.30 |
| 2 / 18 | 7.29 | 6.69 | 53.53 | 9.98 |
| 4 / 36 | 11.97 | 12.50 | 75.19 | 12.63 |

### 同一プロセスでの3回raw再読込・解放

| 起動方法 | 再読込時間 s（1 / 2 / 3回目） | raw解放時間 ms（1 / 2 / 3回目） | 解放後RSS MiB（1 / 2 / 3回目） |
|---|---|---|---|
| 既定allocator | 4.03 / 2.69 / 2.63 | 0.013 / 0.009 / 0.011 | 1595.86 / 1596.23 / 1596.47 |
| macOS launcher | 3.79 / 3.35 / 3.53 | 39.443 / 30.960 / 29.143 | 24.44 / 25.28 / 24.11 |

全サイクルで1,510.62 MiBのrawを再読込し、強参照0を確認した。既定allocatorは保持メモリを再利用し、単調な大容量増加はなかった。launcherは毎回OSへ返す分、解放に約29〜39 msを要した。3サイクルの再読込中央値も2.69→3.53 sとなったが、少数回・OSキャッシュ依存の観測であり一般的な速度差ではない。大容量メモリを長く保持しないこととのトレードオフとして記録する。表の削除UI時間0.617 msはworkerがraw snapshotを保持している計算中のケースであり、通常削除全般の保証ではない。workerが保持していない場合は解放コストがUIスレッドへ乗る可能性があり、待機中の削除操作については別途計測が必要。


## 検証範囲

- 同じ1280×900・3行・36ch、Emergency preview OFF、10回の統計再計算、60 idle frames、30回の統計のみメモリ測定。
- 各ケースは別プロセスで順番に実行し、ビルド・他の測定と重ねない。CPU時間にはeguiの全UI処理とtessellationを含み、OS入力配送・GPU・画面提示は含まない。
- 診断は`PANEL_Y_MEMORY_DIAGNOSTICS=1`で有効化。RSS、malloc live/reserved bytes、Mach resident/footprint/reusable/compressedを別々に記録する。APIは通常のviewer経路では呼ばない。
- `PANEL_Y_MEMORY_RELOADS=3`は、測定とソース削除・pressure-relief診断の終了後、同じParquetをraw配列として3回再読込・解放する追加実験。各ページに触れ、全raw強参照の解放を確認する。GUI全体の再ロードや非同期ロード処理を検証するものではない。
- 初回の全rawタイル・Step edge構築、OS/GPU時間、実計測ログと長時間利用は引き続き別の検証範囲。

## 数値と回帰検証

228テスト、fmt、debug/release build、差分チェックを実行した。新しい3テストは密なStepの極値保持・キャッシュ再利用・拡大時の復帰、有効範囲の端点保持、4ソースのPan/小幅Zoomでの再利用を検証する。範囲・Panの2テストは修正前に失敗し、修正後に成功した。

全ケースの非同期統計は同期参照と完全一致し、全削除時のraw強参照は0。改善前4ソース、改善後既定4ソース、launcherの1/2/4ソースそれぞれを独立NumPy参照でも照合した。許容誤差は従来どおり`rtol=1e-10, atol=1e-10`。

## 証跡と再現

- [環境・実行ファイルハッシュ](../study/diary/assets/2026-10-25-render-memory/environment.json)
- [改善前4ソース](../study/diary/assets/2026-10-25-render-memory/baseline-4-sources.json)、[改善後・既定4ソース](../study/diary/assets/2026-10-25-render-memory/improved-default-4-sources.json)
- [launcher 1ソース](../study/diary/assets/2026-10-25-render-memory/improved-release-1-sources.json)、[2ソース](../study/diary/assets/2026-10-25-render-memory/improved-release-2-sources.json)、[4ソース](../study/diary/assets/2026-10-25-render-memory/improved-release-4-sources.json)
- NumPy照合JSONは同じフォルダの各ファイル名に`-numpy`を付加したもの。
- 独立実験: [Cソース](../study/diary/assets/2026-10-25-render-memory/malloc-probe.c)、[既定設定](../study/diary/assets/2026-10-25-render-memory/malloc-default.txt)、[large cache無効](../study/diary/assets/2026-10-25-render-memory/malloc-cache-disabled.txt)、[vmmap](../study/diary/assets/2026-10-25-render-memory/malloc-vmmap.txt)。

リポジトリルートから、前回の生成データを用いて実行する。RSS取得には`ps`を実行できる環境が必要。

```sh
PANEL_Y_MEMORY_DIAGNOSTICS=1 PANEL_Y_MEMORY_RELOADS=3 \
  ./code/rust_phase1/run_macos.sh --bench-interval-stats-multi \
  study/diary/assets/2026-10-25-multi-source-stats/config-4-sources.json \
  > /tmp/render-memory-4.json
python3 code/proto_3_1b/validate_interval_statistics.py /tmp/render-memory-4.json
```

既定allocatorとの比較はlauncher部分を`code/rust_phase1/target/release/rust_phase1`へ置き換え、`MallocLargeCache`を未設定にして別プロセスで実行する。1/2ソースはconfig名を変更する。診断の有無・起動設定はJSONに記録する。
