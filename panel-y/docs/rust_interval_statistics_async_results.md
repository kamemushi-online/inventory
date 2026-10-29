---
project: Panel_y
created: "2026-10-25"
status: implemented and validated on a 10M-row / 9-channel synthetic workload
---

# 区間統計の非同期化と大規模データ応答性

Cursor A/B区間の統計をバックグラウンドへ移した。UIスレッドでの全サンプル走査とraw配列のコピーをなくし、同時に実行する統計ジョブを1件に制限する。1,000万行×9チャンネルで数値・応答性を確認した。

## 実装

- `interval_stats.rs`にジョブ管理を分離。`TimeData.time`と`ChannelData.values`を`Arc<Vec<_>>`で共有し、ジョブ準備はチャンネル数に比例するメタデータ処理のみ。
- 16,384サンプルごと、およびチャンネル間で中断を確認する。条件が連続変更された場合、旧ジョブの終了を待って最新条件だけを開始する。UIはjoinせず、要求の待ち行列も作らない。
- 結果の受信前に現在の入力を照合する。カーソル、可視チャンネル、source generation、offset、時刻・値バッファの同一性、ジョブID、キャンセル状態が一致した場合だけ結果を反映する。
- バッファをキー内でも共有するため、同一サイズの再読込やメモリアドレス再利用でも旧結果を誤採用しない。
- 計算中表示と`Cancel statistics`、キャンセル／失敗後の`Retry statistics`を追加。同じ条件で明示キャンセルした後は勝手に再実行しない。条件変更で自動再計算する。
- パネルのHide／Collapse時は中断し、終了通知を受信して参照を解放する。アプリ終了時も待機せずキャンセルを通知する。
- Mean / Min / Max / P-P / RMS / slope、nearest raw sampleによるA/B値、逆順・同一時刻カーソル、非有限値の除外、範囲外ソースの扱いは維持。Pan/Zoomだけでは統計を再計算しない。

## 自動検証

2026-10-25の最終ソースで以下が成功した。

- `cargo test --offline --locked --manifest-path code/rust_phase1/Cargo.toml --quiet`: **223 passed / 0 failed**。
- `cargo fmt --check`、debug build、release build、`git diff --check`。
- 11件の追加テストで、共有バッファ、100回の条件変更、Cancel→Retry、同一キーへ戻る競合、受信待ち結果と入力変更、worker切断、Drop、非有限値、逆順／同一／範囲外カーソル、Cancel/Retryのeguiクリック、非表示パネルでの中断・解放を確認。
- 入力変更のテストはカーソル、source generation、offset、非表示化、ソース削除、同一長の値再読込、同じ両端を持つ時刻配列の置換を含む。
- 既存の単一／複数ソース数値テスト2件も実際の非同期完了を待って期待値と照合する形式へ変更。異なる時刻刻みとoffsetを持つソースを確認する。
- 競合テストは完了通知を明示的に保留するため、マシン速度やsleepの長さに依存しない。
- コミット前レビューで、比較用の統計再計算なしフレームも同じカーソル・パネル高さ・波形描画領域に統一して再測定した。数値検証スクリプトは空／欠落／重複結果を拒否し、Python最適化モードでも検証を省略しない。float64入力からviewerのfloat32保持値への変換と、不正な数値・行数・空結果の拒否も確認した。

## 大規模データ測定

環境はmacOS 26.6.2 / arm64、Rust 1.96.0、releaseビルド。最終測定はビルド・自動テストの終了後に実行した。

| 項目 | 条件 |
|---|---|
| データ | `panely_large_10s_1mhz_9ch.parquet` |
| サイズ | 10,000,000行、9チャンネル、90,000,000チャンネルサンプル |
| 時間 | 1 MHz、0.0–9.999999 s |
| rawキャッシュ | 419.6 MiB |
| 描画 | 1280×900、3行、8 Line + 1 Step、描画キャッシュを準備後に統計を計測 |
| 比較条件 | 統計再計算なし／計算中でカーソルと解析パネル高さ・波形領域を統一。再計算なし時は完了済み統計キャッシュを表示 |
| 統計 | 全区間・全チャンネルを5回再計算 |
| 入力 | Hover、水平ホイールPan、Zoomイベントを実際のアプリUIへ投入 |
| 測定範囲 | egui全画面の入力処理・レイアウト・波形／統計描画・tessellation。OSイベント配送、GPU、画面提示は含まない |

生成は既存の`generate_phase1_datasets.py large`を使用。ファイルサイズ292,253,458 bytes、SHA-256は`0150088509f86a473b34693f424944b263d149a379b84a4c14549099aa2afcd6`。生成ParquetはGit対象外。

| 指標 | 最終実測 |
|---|---:|
| 同期参照計算の所要時間 | 180.35 ms |
| UIでのジョブ開始処理、中央値 / 最大 | **0.073 / 0.089 ms** |
| worker計算時間、中央値 / 最大 | 173.07 / 173.36 ms |
| UIへの完了反映まで、中央値 / 最大 | 178.64 / 189.92 ms |
| 統計待機中の全画面CPU処理、中央値 / p95 / 最大 | **12.09 / 12.95 / 14.44 ms**（90フレーム） |
| 統計再計算なしの全画面CPU処理、中央値 / p95 / 最大 | 12.78 / 14.79 / 15.19 ms（60フレーム） |
| 待機中に反映されたPan / Zoomの変化 | 67 / 11回 |
| 実ジョブのCancel通知から終了受信まで | 0.036 ms（1回の実測） |
| 100回の連続条件変更、UI処理中央値 / 最大 | 0.0024 / 0.0294 ms |
| 描画準備後 → 統計5回後のRSS | 467.95 → 470.80 MiB |
| 初回の全体描画CPU処理（キャッシュ未作成） | 74.82 ms |

約173 msの走査をUIから切り離せた。待機中の画面操作が進み、raw配列を複製する規模のRSS増加も発生していない。CPU処理のみの限られた試行であり、60 fpsや任意データサイズを保証する測定ではない。初回描画キャッシュ作成、多数ソース、実計測ログ、長時間運転は引き続き別の検証対象。

生データ: [最終ベンチマークJSON](../study/diary/assets/2026-10-25-interval-stats/large-9ch-release-final.json)。CLIの同期参照は同じ数式の数値比較用であり、変更前実行ファイルのGUI測定ではない。

## 独立した数値検証

`validate_interval_statistics.py`でParquetをストリーミング読込し、NumPy float64の集計で全9チャンネルの9項目を再計算した。A / B / Delta / Mean / Min / Max / P-P / RMS / slopeを比較し、全項目が`rtol=1e-10, atol=1e-10`を満たした。最大絶対差は約2.08e-10で、RMSの加算順序による丸め差の範囲。非同期結果と同期参照の比較は完全一致。

結果: [NumPy照合JSON](../study/diary/assets/2026-10-25-interval-stats/numpy-validation.json)。

## macOSネイティブ画面

同じreleaseビルドを一時的な`PanelY Statistics Smoke.app`で起動し、一時Plot configから1行・9チャンネルを読み込んだ。

1. 1,000万行×9チャンネルを読込し、約0.165094326–9.716980082 sのA/B区間で統計9件を確認。
2. Aを約2.051886562 sへ変更するとBと旧統計が消え、Aの値表示に戻ることを確認。
3. Bを約5.825471296 sへ設定し、新しい区間のMean / RMS等へ更新されることを確認。
4. X範囲を約3.732750–6.603533 sへズームしてもA/Bと統計値を保持することを確認。

画面証跡:

- [大区間統計](../study/diary/assets/2026-10-25-interval-stats/01-native-large-statistics.png)
- [旧結果の消去](../study/diary/assets/2026-10-25-interval-stats/02-native-cursor-invalidated.png)
- [変更後の再計算](../study/diary/assets/2026-10-25-interval-stats/03-native-recomputed.png)
- [ズームと統計保持](../study/diary/assets/2026-10-25-interval-stats/04-native-zoom-keeps-statistics.png)

ネイティブ確認は表示・操作の確認であり、OS/GPUを含むフレーム時間の計測ではない。短時間で完了するジョブへのCancel/Retryクリックと競合は、完了を保留する決定的なegui自動テストで確認した。パネルの初期高さでは統計表がスクロール表示になる点は従来通り。

## 再現手順

リポジトリルートで実行する。Pythonにはnumpyとpyarrowが必要。

```bash
python3 code/proto_3_1b/generate_phase1_datasets.py large
cargo build --release --offline --locked --manifest-path code/rust_phase1/Cargo.toml
code/rust_phase1/target/release/rust_phase1 --bench-interval-stats \
  code/proto_3_1b/data/panely_large_10s_1mhz_9ch.parquet > /tmp/interval-stats.json
python3 code/proto_3_1b/validate_interval_statistics.py /tmp/interval-stats.json
```

既にデータが存在する場合は生成を省略する。ベンチマークは他のビルド・負荷試験と同時実行しない。RSSが取得できない環境ではJSONがnullになる。
