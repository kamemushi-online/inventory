---
project: Panel_y
created: "2026-10-26"
status: measured; initial Line tile construction parallelized
---

# 初回タイル構築の並列化と36ch再測定

4ソース・合計36M行・36chの初回CPUフレーム中央値を **168.18→95.24 ms（43.4%短縮）** に改善した。タイル構築の所要時間は **103.27→35.47 ms（65.7%短縮）**。既存の各チャネルのmin/max計算を最大4ワーカーで並行実行する。

比較対象はPR #55後の`b092a3d`。同じ環境・入力・計測コードで改善前後を測り直した。[前回のStep改善](rust_cold_render_results.md)で残った全chタイル構築を対象とする。

## 原因と変更

初回は36chの324M値を順番に走査し、256サンプル単位のmin/maxタイルを作っていた。1本の極値計算を変更する方法は前回の調査で遅くなったため、今回はチャネル間の独立した仕事を並行実行した。予備実験でも、32本のLineと4本のStepの全タイル構築は1/2/4ワーカーで97.62/53.24/32.33 msだった。

- 表示するLineのうち、raw読込済み、表示範囲内、既存のLOD条件を満たす、タイル未構築のチャネルを選ぶ。同一チャネルの複数行配置は`SourceId + channel_path`で重複排除する。
- 2チャネル以上・合計8Mサンプル以上の場合に並列化する。利用可能CPU数とジョブ数で上限を下げ、最大4ワーカーには呼出し側のUIスレッドも含める。入力サイズで仕事を配分し、結果を入力順へ戻す。
- rawスライスを借用し、すべてのワーカーが完了してからキャッシュを公開する。チャネル値のコピーや、次のフレームへ持ち越すジョブは作らない。スレッドを起動できない場合は同じ仕事を呼出し側で実行する。
- 小さな仕事・1チャネルのみ・Step波形は従来の遅延構築を使う。今回の36ch入力では32本のLineを並列構築し、4本のdense Stepは必要になった時点で逐次構築する。少数のチャネルが順次読み込まれる場合は逐次処理が残る。
- warm frameでは未構築ジョブがないため、新しいワーカーを起動しない。非表示・範囲外・未読込・Step専用・LOD条件未達のチャネルへ先行構築を行わない。

並列時の各タスクのelapsedは重なる。従来の`tile_build_ms`はその合計として維持し、新しい`tile_build_wall_ms`に「並列batchの所要時間＋逐次構築の所要時間」を記録する。前者をUI時間の内数として合算しない。`tile_build_workers`は呼出し側込みの最大実行数。先行構築したキャッシュは同フレームの描画時にhitも記録するが、構築数は実際の生成数を表す。

## 測定条件

macOS 27.0.1 / arm64 / 16 GiB / 10 logical CPUs、Rust 1.96.0、release、`MallocLargeCache=0`。前回と同じ10M・8M・12M・6M行、各9chのParquetを使った。rawは1,510.62 MiB。入力と実行ファイルのSHA-256、ソースhash、実行順を[環境記録](../study/diary/assets/2026-10-26-tile-build/environment.json)に保存した。

4ソースは前後各5独立プロセス、実行順を交互にして比較。各プロセス内で初回1回、warm 10回、同一contextで描画cache再構築5回、新規contextで再構築5回、中央60%→30%→10%→1%→全範囲のZoomを測った。ビルドや別の性能測定とは重ねていない。

1280×900・3行・analysis panel 360px、Emergency preview OFF。全36chのtrace準備・描画命令生成を確認する。最下行がviewportの下へ延び得るため、準備ch数は全行の可視性の保証ではない。高さ1200pxも補助測定した。

計時範囲はeguiの全UI passとtessellation。Parquet読込・同期統計参照は描画前に別計時し、初回前に描画cacheを作らない。OS入力配送・GPU・画面提示は含まない。RSSはフレーム外の段階境界で取得し、ワーカー実行中の瞬間的な最大値ではない。

## 4ソース・36chの結果

| 指標 | 改善前 | 改善後 |
|---|---:|---:|
| 初回CPUフレーム 中央値 ms（各5プロセス） | 168.18 | 95.24 |
| 初回CPUフレーム 最小〜最大 ms | 165.77〜190.94 | 91.67〜95.99 |
| 初回の波形準備 中央値 ms | 133.76 | 61.63 |
| タイル構築の所要時間 中央値 ms | 103.27 | 35.47 |
| 各タイル構築elapsedの合計 中央値 ms | 103.27 | 107.68 |
| 初回Step走査 中央値 ms | 3.21 | 2.90 |
| 初回tessellation 中央値 ms | 8.80 | 8.00 |
| 同一contextで再構築 中央値 ms（計25回） | 137.41 | 67.25 |
| 新規contextで再構築 中央値 ms（計25回） | 156.62 | 86.01 |
| warm全範囲フレーム p95 ms（計50回） | 13.44 | 12.21 |
| 初回後のタイルcache MiB | 38.62 | 38.62 |
| 初回描画に伴うRSS増加 中央値 MiB | 58.14 | 58.42 |
| 初回描画後RSS 中央値 MiB | 1,582.81 | 1,582.98 |

各タスクの時間合計が減ったわけではなく、並行実行によってUIが完了を待つ時間が短くなった。波形準備はUI時間の内数、タイル所要時間は波形準備の内数で、互いに足さない。再構築25回・warm50回は同じ5プロセス内の相関したサンプルである。

全8比較ペアで入力、寸法、準備ch数、描画点数、Shape数、タイル構築数、cache量が一致した。36chの初回は36個のタイルcache、描画点62,496、Shape31,990。有限値の極値、NaN/Inf除外、符号付き0、単発spike、端数タイルは既存の逐次計算と同じ結果を使う。

### 初回後のZoom

キャッシュを保持したまま表示範囲を直接変更した各段階の最初のフレーム中央値。OSイベントの配送時間は測っていない。

| 表示範囲 | 改善前 ms | 改善後 ms |
|---|---:|---:|
| 60% | 61.18 | 54.29 |
| 30% | 63.87 | 64.63 |
| 10% | 53.50 | 53.54 |
| 1% | 15.42 | 15.51 |
| 全範囲へ戻す | 38.45 | 38.43 |

これらの操作は既存タイルを再利用するため、今回の並列構築による一般的なZoom高速化とは解釈しない。30/10/1%などの操作時間はほぼ同程度で、前回のStep再判定とenvelope再構築のコストは残る。

### 補助測定

| 条件 | 初回・改善前 ms | 初回・改善後 ms |
|---|---:|---:|
| 1ソース / 9ch / 1280×900 | 57.06 | 36.33 |
| 2ソース / 18ch / 1280×900 | 94.81 | 56.16 |
| 4ソース / 36ch / 1280×1200 | 179.58 | 93.87 |

各条件・各列1独立プロセスの補助確認であり、分布ではない。

## 操作・メモリ・数値の回帰確認

既存の36ch統計ベンチを前後それぞれ実行した。計算待機中のHover/Pan/Zoom p95は10.61→10.65 ms、計算なしは11.41→11.56 ms。30回の統計パスのRSS増加は0.188→0.172 MiB、全ソース削除約2秒後のRSSは74.69→74.77 MiB、physical footprintは12.80→12.94 MiBだった。両方で全raw強参照の解放を確認し、統計値は同期参照と一致。改善後は独立NumPy参照でも照合した。

235テストが通過。追加4テストは、並列／逐次タイルのbit単位の一致、worker上限と入力順、重複・除外条件、複数ソースでの極値・再利用・raw置換と削除を確認する。debug/release build、fmt、差分チェックも実行した。

処理は同じフレーム内で完了を待つため、初回には約95 msが残る。タイル約35 msに加え、envelope準備・UI・tessellationにも時間がかかる。多くのチャネルが同時に未構築である場合の改善であり、単独チャネル、他OS、実測ログ、OS/GPU提示、長時間利用は別途確認する。

## 証跡と再現

- [環境・実行順・hash](../study/diary/assets/2026-10-26-tile-build/environment.json)、[集計JSON](../study/diary/assets/2026-10-26-tile-build/summary.json)、[再集計・整合チェック](../study/diary/assets/2026-10-26-tile-build/summarize.py)
- [改善前4ソース・第1回](../study/diary/assets/2026-10-26-tile-build/baseline-4-sources-1.json)、[改善後・第1回](../study/diary/assets/2026-10-26-tile-build/candidate-4-sources-1.json)。同フォルダに第2〜5回、1/2ソース、高さ1200pxを保存。
- [改善前の統計・メモリ測定](../study/diary/assets/2026-10-26-tile-build/baseline-statistics-4-sources.json)、[改善後](../study/diary/assets/2026-10-26-tile-build/candidate-statistics-4-sources.json)、[NumPy照合](../study/diary/assets/2026-10-26-tile-build/candidate-statistics-4-sources-numpy.json)
- [予備実験ソース](../study/diary/assets/2026-10-26-tile-build/tile-parallel-micro.rs)、[結果](../study/diary/assets/2026-10-26-tile-build/tile-parallel-micro.json)。正規の性能値は上記の実Parquet・egui測定を使う。
- [改善前計測用patch](../study/diary/assets/2026-10-26-tile-build/baseline-instrumentation.patch)。`b092a3d`の展開先へ`git apply --unidiff-zero`で適用し、今回の`cold_render_bench.rs`をコピーすると同じ計測ができる。

リポジトリルートで実行する。入力生成は[以前の再現手順](rust_interval_statistics_multi_source_results.md#保存した証跡と再現手順)を参照する。

```sh
cargo build --release --offline --locked --manifest-path code/rust_phase1/Cargo.toml
./code/rust_phase1/run_macos.sh --bench-cold-render \
  study/diary/assets/2026-10-25-multi-source-stats/config-4-sources.json \
  > /tmp/tile-build-4.json
python3 study/diary/assets/2026-10-26-tile-build/summarize.py
```

前後のビルドには別のtarget directoryを使うか、明示再コンパイルと異なる実行ファイルhashを確認して保存する。RSS取得には`ps`で自身を参照できる環境、NumPy照合にはnumpyとpyarrowが必要。
