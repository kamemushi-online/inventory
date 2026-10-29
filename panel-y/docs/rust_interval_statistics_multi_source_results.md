---
project: Panel_y
created: "2026-10-25"
status: measured on 1, 2 and 4 large synthetic sources; rendering latency and retained RSS remain
---

# 複数ソース大規模データの区間統計・応答性・メモリ

区間統計は4ソース・36chでもUIから切り離して計算できた。開始処理は中央値0.112 ms、30回の再計算によるRSS増加は0.109 MiBだった。ただし、計算待機中のCPU側フレーム時間はp95 **56.89 ms**、初回描画は**417.14 ms**であり、36chの描画は滑らかとは言えない。全ソース削除後はrawバッファの参照がゼロになったが、2秒後もRSSは**1,679.55 MiB**残った。

## 測定条件

- 2026-10-25、macOS 26.6.2 / arm64 / 16 GiB、Rust 1.96.0、releaseビルド。
- ベースはPR #52マージ後の`69c4dbe`。変更は測定用CLI・データ生成・数値検証・回帰テストで、統計workerと通常の描画処理は変更していない。
- 独立した4ファイルを別ソースとして読込。時刻・値配列をソース間で共有せず、開始時刻・サンプル周期・振幅・DC成分も変えた。同名チャンネルの取り違えを検出できる。
- 1・2・4ソースをそれぞれ別プロセスで測定。ビルド・数値検証・他のベンチマークとは同時実行しない。
- 1280×900、3行に同じチャンネル群を配置し、ソースを重ねて表示。各ソース8 Line + 1 Step。カーソル情報パネル高さ360、Emergency previewは既定のOFF。
- 表示時刻を0秒に揃え、Cursor A=0.25 s、B=9.5 s。端点を含むnearest raw sample区間を計算。
- 初回描画を記録後、40フレームで描画キャッシュを準備。完了済み統計のキャッシュを表示する60フレームと、統計を10回再計算する間のフレームを測定。同じカーソル・パネル寸法でHover／Pan／Zoomイベントを実際の`PanelYApp::draw_ui`へ投入し、tessellationまで含める。
- フレーム値はCPU側処理の経過時間。OSの入力配送・GPU・画面提示は含めない。今回、新たなネイティブGUIでの実操作確認は行っていない。
- RSSの定期取得は別の30回再計算パスに限定し、`ps`起動の負荷をフレーム時間へ混ぜない。50 msのsleepと取得処理時間ごとの観測であり、OSの厳密な最大使用量ではない。

### データ

| ソース | 行数 | 周波数 | raw開始時刻 | 表示offset | 振幅倍率 / DC加算 |
|---|---:|---:|---:|---:|---:|
| #1 | 10,000,000 | 1 MHz | 0 s | 0 s | 1.00 / 0.00 |
| #2 | 8,000,000 | 0.8 MHz | 100 s | −100 s | 1.15 / 0.25 |
| #3 | 12,000,000 | 1.2 MHz | −20 s | +20 s | 1.30 / 0.50 |
| #4 | 6,000,000 | 0.6 MHz | 1,000 s | −1,000 s | 1.45 / 0.75 |

各ソース9ch。4ソースで合計36,000,000行相当、324,000,000値を保持し、1回の区間統計は299,700,036値を走査する。生成には既存Phase 1波形を利用した。ファイルサイズ・SHA-256・生成条件は[datasets.json](../study/diary/assets/2026-10-25-multi-source-stats/datasets.json)に保存。ParquetはGit対象外。

## 応答性

| 指標 | 1ソース・9ch | 2ソース・18ch | 4ソース・36ch |
|---|---:|---:|---:|
| ジョブ開始 中央値 / 最大 ms | 0.079 / 0.095 | 0.089 / 0.102 | 0.112 / 0.145 |
| worker計算 中央値 / 最大 ms | 159.93 / 160.25 | 287.83 / 288.12 | 577.23 / 598.87 |
| 計算待機中フレーム p95 / 最大 ms | **12.79 / 15.30** | **24.60 / 29.18** | **56.89 / 97.29** |
| 計算待機中の測定フレーム数 | 198 | 193 | 169 |
| 再計算なしフレーム p95 / 最大 ms | 14.52 / 14.72 | 28.86 / 29.18 | 62.80 / 74.87 |
| 初回描画 ms | 65.48 | 112.14 | 417.14 |
| 待機中に反映したPan / Zoom | 146 / 25 | 145 / 26 | 120 / 25 |
| Cancel通知→終了受信 ms（各1回） | 0.016 | 0.033 | 0.035 |
| 100回の連続カーソル変更 最大UI処理 ms | 0.032 | 0.048 | 0.073 |

4ソースの約577 msの統計計算中も操作イベントは処理でき、連続条件変更後の結果は最新条件の同期参照と一致した。一方、36chでは再計算なしでもp95 62.80 msであり、統計worker以外のUI・波形描画負荷がボトルネックであることを示唆する。今回の測定では各描画処理への時間配分までは調べていない。キャッシュ状態や操作の進み方も異なるため、再計算中の数値が低いことを速度改善と解釈しない。

## メモリ

単位はMiB。rawは保持配列の論理サイズ、RSSは`ps`で観測した常駐メモリであり、両者は別の指標。

| 指標 | 1ソース | 2ソース | 4ソース |
|---|---:|---:|---:|
| raw論理サイズ | 419.62 | 755.31 | 1,510.62 |
| 描画準備後RSS | 467.61 | 832.55 | 1,640.13 |
| UI付き統計10回後RSS | 467.75 | 832.63 | 1,707.94 |
| 統計のみ30回の開始→終了RSS | 467.891→467.922 | 832.766→832.844 | 1,708.141→1,708.250 |
| 30回パスのRSS増加 | **0.031** | **0.078** | **0.109** |
| 30回パスのRSS観測最大 | 467.94 | 832.88 | 1,708.28 |
| 末尾ソース1件削除後RSS | 467.95 | 832.88 | 1,708.30 |
| 全ソース削除から約2秒後RSS | **467.95** | **832.89** | **1,679.55** |

- 30回の統計再計算では、raw配列のコピーに相当する増加や、繰り返しごとの大きなRSS増加は見られなかった。ただしサンプリング間隔より短いピークや長時間運転のリークはこの測定だけでは否定しない。
- 4ソースの描画準備後→UI付き10回では約67.81 MiB増加した。統計のみの30回では約0.11 MiBなので、前者を統計ジョブの保持リークと判断する根拠はない。描画・UI側の割当ては別途調査対象。
- 各ケースで、実ジョブ開始後にアプリのソース削除処理を実行した。中断通知を受け取り、削除したソースの全time/valueバッファについて`Weak::strong_count()==0`を確認。4→3ソースでは残り27chの統計を再計算し、同期参照と一致した。
- 全ソース削除後はraw論理サイズが0、全rawバッファの強参照も0。それでも100 ms・500 ms・2 s後のRSSは上表の水準に留まった。**参照解放とOSから見た使用量の減少を同一視しない**。allocator／OS側の保持等が候補だが、今回その原因は特定していない。
- 4ソースの読込直後RSSは515.56 MiBでraw論理サイズを下回り、その後の走査・描画で上昇した。常駐状態やシステム状況に依存するため、ロード直後との差だけをコピー量やリーク量として扱わない。

## 数値・回帰検証

- 1・2・4ソースそれぞれの全チャンネルで、非同期結果と同期参照が完全一致。
- NumPy側でも各ファイルのtime列からoffsetとnearest sampleを独立計算し、端点インデックスと9指標（A / B / Delta / Mean / Min / Max / P-P / RMS / slope）を照合。4ソース36chの324項目が`rtol=1e-10, atol=1e-10`内で一致。最大絶対差は約3.37e-10。
- Python最適化モードでも、ソース欠落、ソース取り違え、誤った端点インデックスを拒否することを小規模の独立fixtureで確認。
- 複数ソースの参照解放・残存ソースの再計算・全削除と、時間offsetの丸め誤差で端点がraw範囲外になる入力の回帰テストを追加し、**225 tests passed**。fmt、debug/release build、差分チェック成功。
- 共通測定処理を切り出した既存の`--bench-interval-stats`も再実行し、NumPy照合まで成功。

## 保存した証跡と再現手順

| ケース | 設定 | 測定結果 | NumPy照合 |
|---|---|---|---|
| 1ソース | [config](../study/diary/assets/2026-10-25-multi-source-stats/config-1-sources.json) | [result](../study/diary/assets/2026-10-25-multi-source-stats/result-1-sources.json) | [NumPy](../study/diary/assets/2026-10-25-multi-source-stats/numpy-1-sources.json) |
| 2ソース | [config](../study/diary/assets/2026-10-25-multi-source-stats/config-2-sources.json) | [result](../study/diary/assets/2026-10-25-multi-source-stats/result-2-sources.json) | [NumPy](../study/diary/assets/2026-10-25-multi-source-stats/numpy-2-sources.json) |
| 4ソース | [config](../study/diary/assets/2026-10-25-multi-source-stats/config-4-sources.json) | [result](../study/diary/assets/2026-10-25-multi-source-stats/result-4-sources.json) | [NumPy](../study/diary/assets/2026-10-25-multi-source-stats/numpy-4-sources.json) |

リポジトリルートで実行。Pythonにはnumpyとpyarrowが必要。データ生成は既存ファイルのschema・行数・生成metadataを確認して再利用し、別内容のファイルは上書きせず拒否する。

```bash
python3 code/proto_3_1b/generate_interval_statistics_sources.py \
  study/diary/assets/2026-10-25-multi-source-stats
cargo test --offline --locked --manifest-path code/rust_phase1/Cargo.toml
cargo build --release --offline --locked --manifest-path code/rust_phase1/Cargo.toml
for n in 1 2 4; do
  code/rust_phase1/target/release/rust_phase1 --bench-interval-stats-multi \
    study/diary/assets/2026-10-25-multi-source-stats/config-${n}-sources.json \
    > /tmp/interval-stats-${n}-sources.json
done
for n in 1 2 4; do
  python3 code/proto_3_1b/validate_interval_statistics.py \
    /tmp/interval-stats-${n}-sources.json
done
```

設定内のファイルパスは作業ディレクトリ基準。RSSが取得できない環境では値がnullになる。再計算・中断・参照解放の検査失敗時はCLIが非ゼロで終了する。

## 次の調査対象

描画キャッシュ再利用とmacOSのlarge allocation cacheの調査・改善・再測定を[後続結果](rust_render_memory_results.md)に記録した。以下はこの初回測定時点の調査項目。

1. 36ch描画のp95約57 ms・最大約97 ms、初回約417 msの内訳を計測し、波形生成・描画・tessellationのどこに時間を使うか確認する。
2. ソース削除後もRSSが残る原因と、再読込時にメモリが再利用されるかを調べる。参照解放の成功だけで長時間利用のメモリ問題が解消したとは判断しない。
3. 実計測ログ、4を超えるソース数、長時間の削除・再読込、OS/GPUを含む入力・表示遅延へ検証を広げる。
