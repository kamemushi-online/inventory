---
project: Panel_y
created: "2026-10-27"
status: Scripted native 60-minute / 30-cycle run and 120-second final quiet period passed within documented scope
---

# 長時間の連続操作とメモリ解放の検証

**判定:** モータシミュレーション3ソース・12チャンネルの条件で、60分・30サイクルの
スクリプトによるネイティブ連続処理と、全削除後120秒の静置が完了した。
毎回のrawバッファ解放、設定復元、解析値の再現性は合格。
この実行では終盤まで持続するRSS増加は観測されなかった。
後半はMacのロックを検出したため、全時間を通した目視・マウス操作や画面提示の合格とはしない。

## 検証計画

対象は現行Rustビューアと、既存のモータ制御シミュレーション由来Parquet 3本。
実機実測ログではない。3ソース・3行・12チャンネルの構成を使う。

同じネイティブプロセスで60分以上、30サイクル以上を時間全体へ分散して実施する。
通常ビルドでは無効な検証モードから、製品の読込・解析・削除・復元処理を呼び出し、
通常のeframe描画と非同期処理を動かす。全反復をマウスで操作した検証とは区別する。

各サイクルでは次を確認する。

1. 3ソース・3行・12チャンネルの読込と表示設定を照合する。
2. X範囲を変更してPan/Zoom相当の描画を行い、固定区間の統計と3対象のFFTを完了させる。
3. 全ソースを削除し、処理が落ち着いてからraw・描画キャッシュが0であることを確認する。
4. Session preset ApplyとPlot config Loadを交互に使って復元する。
   前者は保存X範囲、後者は全体範囲へ戻る仕様に合わせて照合する。
5. 同じ標本範囲・設定での統計とFFTを基準結果と比較する。

RSSは対象PIDを指定して2秒間隔で記録し、同一段階の値を比較する。
最初の5サイクル、6〜15サイクル、最後の10サイクルを分け、中央値・範囲・増加傾向を示す。
観測最大値はサンプリングによる最大値であり、瞬間的なピークを保証しない。
最後は全ソースを削除して2分間観測する。起動時には既存macOSランチャーと同じ
`MallocLargeCache=0`を設定し、途中の強制メモリ解放は行わない。

判定には正常完了したサイクル数、経過時間、状態照合、解析値照合、キャッシュ解放と
同じ状態でのメモリ推移を使用する。RSSが0や起動直後の値へ戻ることは要求しない。
持続的な増加があれば追加調査し、安定していても任意データ・無期限のリーク不存在とは断定しない。

## 結果

本測定は2026-10-27 20:46:48〜21:48:50 JST、macOS 27.0.1 / arm64、同一PID `79377`で実施。
基準revisionは`519bda495dbe2a48dec6b152e4842deaf42597c8`。
通常buildでは除外される`soak-validation` featureを追加した検証用releaseを使用した。
実行ファイルSHA-256は`ae256db035865d0bf8d3a0b99fa7739a459f5d7b2b1456552efe33caf84505cf`。
入力・設定・関連ソース・実行ファイル17件のhash一致を確認した。
[環境記録](../study/diary/assets/2026-10-27-long-session/environment.json)、
[整合性確認](../study/diary/assets/2026-10-27-long-session/integrity-check.json)に詳細を残した。

| 項目 | 結果 |
|---|---|
| 反復時間・回数 | 3,600.007秒、30サイクル。周期119.971〜120.030秒 |
| 終了後の観測 | 全削除・参照解放の確認後120.015秒。読込完了から合計3,722.126秒 |
| 復元 | Session preset 15回、Plot config Load 15回。3ソース・3行・12ch、表示設定、期待するX範囲、解析状態リセットを照合 |
| 解放 | 反復30回＋最後の全削除1回でraw・描画キャッシュ0。元の時刻3本・波形12本の強参照数0 |
| 解析 | 初回＋30復元後の統計・FFT各31回。統計は12ch、FFTは各ソースのIa_phys、計3対象 |
| 連続動作 | 13,209回の範囲変更、111,556 native egui UIフレーム。最大イベント間隔30.041秒。停止・まとめ実行の監査違反なし |
| RSS定期取得 | 2秒間隔1,855件。実測間隔1.988〜2.013秒、欠落判定なし |
| 回帰確認 | feature付き243テスト、通常buildのcargo check、release build、fmt確認が成功 |

UIフレーム数はOS/GPUによる画面提示完了数やfpsではない。
RSS定期取得は読込完了の15.919秒後から開始したため、最初の約16秒には周期サンプルがない。
その間も初期・読込・統計・FFTのdriver checkpointは記録した。
事前の24秒・2サイクル試行も通過したが、本測定の時間・回数には加算していない。

### 数値照合

固定Cursor A/Bは0.04 / 0.09秒。全ソースで添字39,999〜89,999の50,001標本を使用した。
統計9指標とFFTのカーソル・dt・ピーク周波数／振幅／dBについて、独立NumPy参照との
数値比較3,906件が`atol=1e-9, rtol=1e-10`内で一致した。
標本数・区間添字等の厳密一致確認1,581件も通過した。
最大絶対誤差は統計RMSで`1.79e-13`、FFTピーク振幅で`8.88e-16`。

| FFT対象 | ピーク周波数 [Hz] | ピーク振幅 |
|---|---:|---:|
| input / Ia_phys | 39.9992000162 | 4.88747936442 |
| reversal / Ia_phys | 39.9992000162 | 4.60272159666 |
| speed / Ia_phys | 59.9988000244 | 5.30041576818 |

既定Hanning窓、平均除去、片側`abs(FFT)*2/N`、窓利得補正なしの既存仕様を照合した。
反復間では統計値を直接比較し、FFT全周波数・振幅配列のFNV-1a hashが同じことも確認した。
NumPyとの独立照合は上記の指標であり、全スペクトル配列の独立比較とは区別する。
[参照値](../study/diary/assets/2026-10-27-long-session/numeric-reference.json)、
[最終照合](../study/diary/assets/2026-10-27-long-session/numeric-comparison.json)を参照。

### メモリ推移

同じ段階のRSS中央値を比較する。単位はMiB。

| 段階 | 1〜5回 | 6〜15回 | 21〜30回 | 終盤−中盤 |
|---|---:|---:|---:|---:|
| 全削除後 | 195.30 | 196.59 | 186.45 | −10.14 |
| 復元直後 | 206.88 | 210.08 | 199.05 | −11.03 |
| サイクル完了時 | 204.72 | 207.33 | 197.74 | −9.59 |

全削除後は途中で200.125 MiBまで増えた後、終盤に低下した。
周期サンプルの観測最大RSSは235.125 MiBで、瞬間的な真の最大値ではない。
最後の全削除後120秒の静置中60サンプルと、完了検知後の追加1サンプルは
すべて185.0625 MiBだった。
元のrawデータ17,760,000 bytes（16.94 MiB）は毎回所有終了を確認している。
RSSにはUI・描画資源・allocator・OSの常駐状態が含まれ、0にはならない。

RSS低下をアプリの改善効果や特定の解放機構の効果とは解釈しない。
途中のロックやOS側の常駐状態も含め、RSS変動の原因を完全には分解していない。
読み取り専用の[途中vmmap](../study/diary/assets/2026-10-27-long-session/vmmap-midrun.txt)と
[終了後vmmap](../study/diary/assets/2026-10-27-long-session/vmmap-final-empty.txt)も保存したが、
異なる操作段階の診断であり、領域別のリーク判定には使わない。強制trimは実施していない。

[全イベント](../study/diary/assets/2026-10-27-long-session/longsession-events.jsonl)、
[RSS時系列](../study/diary/assets/2026-10-27-long-session/native-memory.jsonl)、
[集計・連続性監査](../study/diary/assets/2026-10-27-long-session/analysis.json)を保存した。

## 確認範囲と残件

- 操作は既存アプリ関数を呼ぶスクリプトで、通常のnative eframe描画と非同期処理を使用した。
  全30回をOSのマウス・キー入力で操作した検証ではない。
- 初期は1280×832、最初の約1分でnativeのウィンドウ拡大操作により1920×960へ変更した。
  [開始時](../study/diary/assets/2026-10-27-long-session/native-start.png)、
  [約10分](../study/diary/assets/2026-10-27-long-session/native-10min.png)、
  [約30分](../study/diary/assets/2026-10-27-long-session/native-midpoint.png)の画面を保存した。
  約45分でMacのロックを検出し、以降の測定中の画面証跡は取得できなかった。ロック開始時刻は不明。
  UIフレームと操作回数は最後まで増加しており、後半の合格範囲は処理・描画コール・状態ログまで。
  ユーザーのロック解除後、同日22:52:25〜22:52:39 JSTに同一PID `79377`の
  [終了後の全体画面](../study/diary/assets/2026-10-27-long-session/post-unlock-empty-overview.png)と
  [キャッシュ欄](../study/diary/assets/2026-10-27-long-session/post-unlock-empty-cache.png)を追加取得した。
  0ソース・0チャンネル、rawと全描画キャッシュ0、Cursor A/Bなし、FFT対象0を画面でも確認した。
  [撮影記録](../study/diary/assets/2026-10-27-long-session/post-unlock-capture.json)に時刻を保存した。
  これは測定終了約64分後の追加確認であり、測定中のロック区間の画面証跡を補うものではない。
- 3本合計740k行、12ロード済みチャンネルの条件である。Line tile cacheはこの規模では0。
  大規模ソース5本以上、実機実測ログ、数時間以上、他OS・配布版には結果を外挿しない。
- 解析を完了させてから削除した。計算中のCancel／削除を高速反復する別の負荷試験は含まない。
- 常時表示・ロックなしの長時間GUI確認、OS/GPU提示遅延、より大きい負荷は引き続き残件。
  この1時間で問題を観測しなかったことを、無期限のリーク不存在とは表現しない。

## 再現方法

リポジトリrootで実行する。既存モータParquetの生成と保存設定については
[前回の記録](rust_motor_log_continuous_validation_results.md)を参照する。

```sh
cargo build --release --offline --locked \
  --manifest-path code/rust_phase1/Cargo.toml --features soak-validation
./code/rust_phase1/run_macos.sh --native-soak /absolute/path/to/run.json
```

`run.json`の例。出力先は毎回未使用のパスを指定する。

```json
{
  "plot_config_path": "/absolute/path/to/saved-motor.pyc.json",
  "output_jsonl": "/absolute/path/to/events.jsonl",
  "duration_seconds": 3600,
  "cycles": 30,
  "final_quiet_seconds": 120,
  "close_when_done": false
}
```

feature付きbuildでは`PANEL_Y_NATIVE_SOAK_CONFIG`でも設定パスを渡せる。
`--native-soak`の明示指定を優先する。今回の開発用macOS app bundleはこの環境変数と
`MallocLargeCache=0`を起動時に渡した。配布パッケージの検証ではない。

別の端末で、初期イベントに記録された対象PIDだけを計測する。

```sh
python3 study/diary/assets/2026-10-27-long-session/sampler.py \
  --pid TARGET_PID --events /absolute/path/to/events.jsonl \
  --output /absolute/path/to/memory.jsonl --duration 3900 --interval 2
python3 study/diary/assets/2026-10-27-long-session/analyzer.py \
  --driver /absolute/path/to/events.jsonl --samples /absolute/path/to/memory.jsonl \
  --output /absolute/path/to/analysis.json
python3 study/diary/assets/2026-10-27-long-session/compare_reference.py \
  --events /absolute/path/to/events.jsonl --output /absolute/path/to/comparison.json
```

入力・設定を変えた場合はNumPy参照も再生成する。現在の参照scriptは今回の3入力と
固定カーソル区間0.04〜0.09秒を対象とし、任意データ用の汎用検証器ではない。
