---
project: Panel_y
created: "2026-10-26"
status: measured; dense Step cold preparation reduced; tile construction remains
---

# 36ch初回描画の調査と改善

追補: [タイル構築の並列化](rust_tile_build_results.md)で、PR #55後を測り直した初回CPUフレームは168.18→95.24 msへ短縮した。以下はStep改善時点の記録。

4ソース・合計36M行・36chの初回CPUフレーム中央値を **226.33→165.61 ms（26.8%短縮）** に改善した。密なStep波形で不要だった全変化点の構築を打ち切り、初回のStep走査を62.82→2.97 ms、保持するStepキャッシュを36.62→0 MiBに減らした。表示密度、min/maxタイル、統計の計算方法は変更していない。

[前回の描画・メモリ調査](rust_render_memory_results.md)では初回に約0.4秒かかっていた。今回はmacOSが26.6.2から27.0.1へ変わっているため、その単発値との直接比較はしない。PR #54をマージした`1f10930`を今回の環境で測り直し、同じ計測コードで改善後と比較した。

## 原因と実装

初回の波形準備には、36chの全rawを読むmin/maxタイル構築と、4本のStep波形の変化点構築が集中していた。Stepは表示上限12,000変化点を超えていても、以前は最後まで全変化点を保存してからdense envelopeへ切り替えていた。

- 表示範囲の変化点数が上限を超えた時点で走査を打ち切り、途中の配列を破棄して既存のdense envelopeへ切り替える。
- 疎なStepでは、要求範囲と直前の最後の有限値だけをキャッシュする。キャッシュ範囲内は再利用し、範囲外へPan/Zoomした場合は再構築する。これにより、初回後の拡大でも全データの変化点構築を後回しにして発生させない。
- NaN/Infをまたぐ前状態、重複時刻、端点、拡大時の正確なStep表示を維持する。直前に長い欠損区間がある場合、その有限値探索の逆走査は残る。
- タイルの複数accumulator化も試したが、この環境では既存のscalarループより遅くなったため採用していない。

## 測定条件

macOS 27.0.1 / arm64 / 16 GiB / 10 logical CPUs、Rust 1.96.0、release、`MallocLargeCache=0`。入力は前回と同じ10M・8M・12M・6M行、各9ch、独立したtime配列とoffsetを持つ4ファイル。合計324M値、rawは1,510.62 MiB。ファイルのSHA-256を[環境記録](../study/diary/assets/2026-10-26-cold-render/environment.json)に保存した。

新しい`--bench-cold-render`は全範囲の初回UIとtessellationを測り、全egui pass、波形準備、タイル・Step走査、描画命令、頂点数を記録する。Parquet読込と統計参照計算は描画より前に行い、それぞれ別計時する。最初の描画の前に描画キャッシュを作らない。これはファイルを開いてから表示までの総時間ではない。

既定は1280×900、3行、analysis panel 360px、Emergency preview OFF。36chのtrace準備と描画命令生成を確認するが、最下行がviewportの下へ延びるため、全行の可視性をこの数値だけで保証しない。高さ1200pxの補助測定も実施した。OS入力配送、GPU、画面提示、native GUIの実測は含まない。

4ソースは改善前後を交互の順序で各5独立プロセス実行した。各プロセス内では初回1回、warm 10回、描画キャッシュだけを消して同じcontextで5回、新規contextで5回再構築し、最後に段階Zoomを測定した。ビルド・他の性能測定とは重ねていない。RSSはフレーム外の段階境界で取得し、連続監視のピークではない。

## 4ソース・36chの結果

| 指標 | 改善前 | 改善後 |
|---|---:|---:|
| 初回CPUフレーム 中央値 ms（各5プロセス） | 226.33 | 165.61 |
| 初回CPUフレーム 最小〜最大 ms | 225.19〜235.84 | 164.26〜173.52 |
| 初回の波形準備 中央値 ms | 192.16 | 131.15 |
| 初回のタイル構築 中央値 ms | 103.83 | 101.69 |
| 初回のStep走査 中央値 ms | 62.82 | 2.97 |
| 初回tessellation 中央値 ms | 9.88 | 8.84 |
| 同じcontextでキャッシュ再構築 中央値 ms（計25回） | 192.46 | 136.90 |
| 新規contextでキャッシュ再構築 中央値 ms（計25回） | 211.92 | 155.55 |
| warm全範囲フレーム p95 ms（計50回） | 13.49 | 13.60 |
| 初回後に保持するStepキャッシュ MiB | 36.62 | 0.00 |
| 初回描画に伴うRSS増加 中央値 MiB | 94.81 | 58.11 |
| 初回描画後RSS 中央値 MiB | 1,619.38 | 1,582.89 |

波形準備はUI時間の内数、タイル構築・Step走査は波形準備の内数であり、合算しない。再構築・warmの分布は同じ5プロセス内の相関したサンプルをまとめたもので、25/50独立プロセスではない。初回の描画点数62,496とShape数31,990は前後で一致した。

### 初回後の大幅Zoom

キャッシュを保持したまま中央60%→30%→10%→1%→全範囲へ切り替えた。各段階の最初のフレームの中央値（各5回）は次のとおり。OSのZoomイベントではなく、表示範囲を直接変更した描画コストである。

| 表示範囲 | 改善前 ms | 改善後 ms |
|---|---:|---:|
| 60% | 54.85 | 60.68 |
| 30% | 59.91 | 64.94 |
| 10% | 50.91 | 53.90 |
| 1% | 14.90 | 15.72 |
| 全範囲へ戻す | 38.55 | 38.43 |

初回に全変化点を保持しなくなった分、密な範囲の再判定が入り、60/30/10%の切り替えでは約3〜6 ms増加した。初回短縮とキャッシュ削減のトレードオフであり、すべての操作が速くなったという結果ではない。狭い範囲の正確なStepは範囲キャッシュで再利用する。段階ごとの後続3フレームもJSONへ保存した。

### ソース数・ウィンドウ高さの補助測定

| 条件 | 初回・改善前 ms | 初回・改善後 ms |
|---|---:|---:|
| 1ソース / 9ch / 1280×900 | 74.10 | 56.15 |
| 2ソース / 18ch / 1280×900 | 127.36 | 91.58 |
| 4ソース / 36ch / 1280×1200 | 230.43 | 162.18 |

各列・条件は独立プロセス1回の補助確認であり、分布や一般的な改善率ではない。全8比較ペアについて、入力・寸法・準備ch数・描画点数・Shape数・タイル構築数が一致することを集計スクリプトで検証した。

## 回帰検証と残る課題

- 231テスト、release build、fmt、差分チェックを実行。追加3テストで、完全なStep cacheを参照とするdense/sparse・欠損値・重複時刻・境界の同値性、画面外のdense領域、部分cacheの再利用と左右Pan時の再構築を確認した。既存の単発spike保持・タイル再利用・拡大時のStep復帰テストも通過した。
- 既存の36ch統計ベンチも再実行。計算中のHover/Pan/Zoomフレームp95は10.99 ms、計算なしは12.42 ms。非同期値は同期参照と一致し、別のNumPy参照でも照合した。
- 30回の統計パスのRSS増加は0.203 MiB。全ソース削除後はraw強参照0、RSS 74.80 MiB、physical footprint 11.52 MiBを確認した。
- 初回CPU時間にはまだ約0.17秒が残り、主な残件は約0.10秒の全chタイル構築。今回の変更はUIで行う描画準備の短縮であり、描画準備全体を非同期化したものではない。実測ログ、長時間利用、他OS、OS/GPU提示時間は別途検証する。

## 証跡と再現

- [集計JSON](../study/diary/assets/2026-10-26-cold-render/summary.json)、[集計・前後整合チェック](../study/diary/assets/2026-10-26-cold-render/summarize.py)、[環境・実行順・ファイルハッシュ](../study/diary/assets/2026-10-26-cold-render/environment.json)
- [改善前4ソース・第1回](../study/diary/assets/2026-10-26-cold-render/baseline-4-sources-1.json)、[改善後・第1回](../study/diary/assets/2026-10-26-cold-render/candidate-4-sources-1.json)。同フォルダの末尾`-2`〜`-5`に残る独立試行を保存。
- 同フォルダの`baseline/candidate-1-sources.json`、`-2-sources.json`、`-4-sources-tall.json`に補助測定を保存。
- [統計・操作・メモリ回帰測定](../study/diary/assets/2026-10-26-cold-render/candidate-statistics-4-sources.json)、[NumPy照合](../study/diary/assets/2026-10-26-cold-render/candidate-statistics-4-sources-numpy.json)
- [改善前に追加した計測用差分](../study/diary/assets/2026-10-26-cold-render/baseline-instrumentation.patch)。`1f10930`の展開先で`git apply --unidiff-zero`により適用し、今回の`cold_render_bench.rs`をコピーすると同じ計測ができる。本体の描画アルゴリズムは変更しない。

リポジトリルートから実行する。入力の生成・再利用は[前回の再現手順](rust_interval_statistics_multi_source_results.md#保存した証跡と再現手順)を参照する。

```sh
cargo build --release --offline --locked --manifest-path code/rust_phase1/Cargo.toml
./code/rust_phase1/run_macos.sh --bench-cold-render \
  study/diary/assets/2026-10-25-multi-source-stats/config-4-sources.json \
  > /tmp/cold-render-4.json
# 高さ1200pxの補助ケース
./code/rust_phase1/run_macos.sh --bench-cold-render \
  study/diary/assets/2026-10-26-cold-render/config-4-sources-tall.json \
  > /tmp/cold-render-tall.json
python3 study/diary/assets/2026-10-26-cold-render/summarize.py
```

比較時は改善前後を別のtarget directoryでビルドするか、ソース再コンパイルと実行ファイルの異なるSHA-256を確認してから各実行ファイルを保存する。RSS取得には自身のプロセスを`ps`で参照できる環境が必要。NumPy照合にはnumpyとpyarrowを利用する。
