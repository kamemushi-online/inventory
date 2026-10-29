---
project: Panel_y
created: "2026-10-27"
status: validated on motor-control simulation logs; physical measurement logs remain unvalidated
---

# モータ制御ログによる連続操作検証

ユーザー指定のモータ制御スペースからParquetを3本生成し、2026-10-27にmacOSの
ネイティブビューアで連続操作を確認した。中間ソース削除→Session preset復元を5回、
Plot config Loadを5回実施し、反復後にも統計・FFTを実行できた。
今回の操作ではクラッシュ、操作不能、波形の取り違えを観測しなかった。

入力は**C/Rustモータ制御シミュレーションの出力**であり、実機実測ログではない。
実機データ固有の欠損・不等間隔時刻・雑音等の検証は未完了のままとする。

## 入力と生成条件

生成元は `/Users/ochiba/newspace/yamatoshi-ops/no9-tm_261025/motor_simulation`、
revision `27c5c36b42c18d17aebb99d4e0131e39c8f28448`。
既存のNo8/Prius閉ループrunnerとRust exporterを使用し、制御・モデル・scenarioを変更していない。
Priusの管理状態は `generating`。この検証はビューアの確認であり、モデルの実機精度の評価ではない。

| Parquet | 運転条件 | 行数 | 時刻範囲 [s] | ファイルサイズ [bytes] |
|---|---|---:|---|---:|
| `motor_input.parquet` | 5 N·m、200 rad/s、起動から一定運転 | 100,000 | 0.000001–0.100000 | 7,507,409 |
| `motor_reversal.parquet` | 0→5→−5→0 N·m、200 rad/s | 320,000 | 0.000001–0.320000 | 22,961,335 |
| `motor_speed.parquet` | 5 N·m、200→400→100→200 rad/s | 320,000 | 0.000001–0.320000 | 23,404,618 |

共通条件は300 V、理想インバータ、内部刻み・出力間隔1 µs。各ファイルはtime＋94信号。
時刻をFloat64、信号をFloat32またはUInt8として出力した。
文字列列 `pwm_dir` と `controller_mode` はexporterの仕様に従い除外した。
保存先は `code/proto_3_1b/data/motor_control_validation_20261026/`。
CSV・Parquet・実行ログは同ディレクトリでGit対象外とし、元のモータ制御スペースはcleanを保持した。

既存の `duty_voltage_reconstruction` のtrace CSVは、複数scenarioの代表区間抜粋だった。
連続時刻を必要とする今回のFFT入力には採用せず、既存の比較用scenarioから完全な時系列を生成した。

生成コマンド・入力／出力hash・所要時間は[生成記録](../study/diary/assets/2026-10-26-motor-session/generation.json)、
再生成処理は[generate_motor_logs.py](../study/diary/assets/2026-10-26-motor-session/generate_motor_logs.py)に保存した。
生成は10月26日、GUI操作は10月27日。証跡ディレクトリの日付は着手日を使用している。

## 入力の独立照合

[検証スクリプト](../study/diary/assets/2026-10-26-motor-session/validate_inputs.py)で、
CSVとParquetの全740,000行×95列、**70,300,000セル**を比較した。
timeと、意図した型変換後の信号値はbit単位で一致した。Float64→Float32の丸めは
変換前との誤差も保存しており、元のFloat64と無丸めで同一という意味ではない。

- 欠損・非有限値なし。時刻は単調増加し、全件が連続する1 µs格子に一致。
- 元CSV、生成Parquet、scenarioのhashを照合。
- GUI区間A=0.070313604 s、B=0.090501695 sでは各20,189点。
  NumPyで12チャンネルの参照統計を求め、画面から読み取った10指標は表示6桁で全件一致。
  `Ia_phys #1`のMeanは2.849023、RMSは7.881946。

詳細は[入力検証JSON](../study/diary/assets/2026-10-26-motor-session/input-validation.json)。
画面未確認の参照値までGUI合格件数へ含めていない。

反復後のFFTも[独立NumPy照合](../study/diary/assets/2026-10-26-motor-session/fft-reference.json)で確認した。
平均除去→対称Hanning窓→`rfft`→`2/N`の振幅係数を適用し、標本数と画面のpeak周波数3桁・
振幅6桁が3対象すべて一致した。inputは40.419 Hz / 4.849870、reversalは20.209 Hz / 4.625901、
speedは60.628 Hz / 5.327021。全スペクトルの点ごとのGUI比較までは行っていない。

## ネイティブGUIの連続操作

対象はmain `65f68696d8c0c938eddc85b9dc8f019dd0c208ee`のreleaseビルド、
macOS 27.0.1 / arm64 / Rust 1.96.0。
検証用 `.app` の起動環境に `MallocLargeCache=0` を設定した。
同一プロセスPID 58599の最終チェック時の経過時間は**12分38秒**。
起動待ち時間を連続操作時間へ算入していない。
[環境・実行ファイルhash](../study/diary/assets/2026-10-26-motor-session/environment.json)を参照。

3ソースを3行へ配置し、`Ia_phys`、`Te`をLine、`Trq_cmd`、`Hi_a`をStepで表示した。
合計12チャンネル、画面のrawキャッシュ表示は16.9 MiB。
各ファイルの94信号すべてを同時表示した検証ではない。

| 操作 | 観測結果 |
|---|---|
| 3ソース読込・Line/Step・Hover | 3行12チャンネルを表示。短いsource #1は0.1 sで終了し、残る2ソースは0.32 sまで表示 |
| ホイールZoom・drag Pan | 全行のX軸が同期。電流・トルク・PWMとソース別Hoverを確認 |
| Cursor A/B・統計 | 異なる2区間で再計算。最初の区間の観測済み10指標をNumPyと照合 |
| 3対象FFT | `Ia_phys`をソースごとに解析。反復の前後で完了し、結果・凡例・スペクトルを表示 |
| Fit Overlap | 共通区間0.000001–0.100000 sへfit。A/Bと解析結果は保持 |
| offset変更・中間削除 | #3を+0.010 sにして#2を削除。緑色のspeedが#2へ改番され、offsetと波形を保持 |
| 削除→Session preset Apply ×5 | 毎回3ソース・3行・12チャンネルと保存X範囲0.059443–0.125727 sを復元。初回にoffsetも0へ復元。Cursor/FFTは仕様どおり消去 |
| Plot config Save | v2を別ファイルへ保存。source path、offset、行内参照、色、線幅、Line/Step、表示状態、Y設定を元構成と照合して一致 |
| 全ソース削除 | 0ソース・0チャンネル、raw・描画cacheとも0。波形が消去された |
| Plot config Load ×5 | 1回目は全削除状態から復元。2〜5回目はZoom/Pan後に再Load。毎回3ソース・12チャンネルとunion 0.000001–0.320000 sへ復元 |
| 反復後の再解析 | A=0.040485555 s、B=0.089966673 s、49,482点/ソースで統計・FFTを再計算。FFTの表示上限200 Hzでも3曲線を確認 |

設定の比較結果は[config-roundtrip.json](../study/diary/assets/2026-10-26-motor-session/config-roundtrip.json)、
再Loadの時刻は[plot-reload-events.json](../study/diary/assets/2026-10-26-motor-session/plot-reload-events.json)。
Session presetは任意X範囲を復元する一方、Plot config Loadはunionへfitするという違いを確認した。

解析欄は初期の1280×832画面では表とFFT操作の同時表示が狭く、ウィンドウを縦に拡大し、
FFT欄をスクロールして確認した。小さいウィンドウでの解析操作性は改善候補として残る。

## メモリ観測と補助stress

RSSは同一nativeプロセスを`ps`で読んだチェックポイント値であり、連続サンプリングの最大値ではない。

| チェックポイント | RSS [MiB] |
|---|---:|
| 初回3ソース読込 | 203.22 |
| 削除→preset復元1 / 2 / 3 / 4 / 5回目 | 167.33 / 169.42 / 171.58 / 171.95 / 172.17 |
| 全ソース削除 | 162.92 |
| Plot config再Load 5回後 | 176.19 |
| 最終FFT後 | 179.06 |

rawキャッシュは削除で0になったが、プロセスRSS全体は0にはならない。
preset復元1→5回目には4.84 MiBの増加があり、この短い観測だけでメモリリークなしと断定しない。
最後の2回の増分は0.38 / 0.22 MiBだった。GPU・UI・allocatorを含む残存RSSの内訳は測っていない。
生記録は[native-memory.jsonl](../study/diary/assets/2026-10-26-motor-session/native-memory.jsonl)。

別プロセスで既存 `--stress-channel <Parquet> Ia_phys 2000` を各ファイルへ実行し、
合計6,000回の可視範囲抽出が正常終了した。
[stress記録](../study/diary/assets/2026-10-26-motor-session/stress-runs.json)のRSSは取得不可のため、
メモリ判断には使わない。この補助CLIはネイティブ描画・長時間運転の代替ではない。

## 確認できた範囲と残件

モータ制御由来の連続時系列でも、3ソース12チャンネルの読込・操作・解析・復元の経路は成立した。
製品コードは変更していない。今回release buildと入力・保存設定・補助CLI・GUIを確認した。
既存235自動テストの再実行や新しいCPUフレームベンチは、この作業では行っていない。

実機実測ログ、4を超える大規模ソース、数時間以上の反復、OS/GPUを含む入力から画面提示までの遅延、
他OS・配布版は未確認。時間計測のない体感操作をp95やfpsの保証へ読み替えない。

## 再現方法

既に生成済みのファイルを使う場合はビューアを起動し、Plot configのPathへ
[saved-motor.pyc.json](../study/diary/assets/2026-10-26-motor-session/saved-motor.pyc.json)の絶対パスを指定してLoadする。
3ソース・3行・12チャンネルを読み込む。Cursor/FFTは手動で再配置・再実行する。

波形を再生成する場合は、生成元revisionを確認してrunnerとexporterをbuildし、
`generate_motor_logs.py BASE_ROOT BINARY_DIR OUTPUT_DIR REPORT`を実行する。
`BASE_ROOT`は上記motor_simulation、`BINARY_DIR`はbuildしたrelease実行ファイルのdirectory。
`OUTPUT_DIR`は未使用の保存先を指定する。既存directoryへの上書きは拒否する。
今回のbuildコマンドは次のとおり（panel-y rootから実行）。

```sh
CARGO_TARGET_DIR=/private/tmp/panely-motor-validation-target \
SIM_CONTROLLER_NO8_CONST_PLANT=prius \
cargo build --release --offline --locked \
  --manifest-path /Users/ochiba/newspace/yamatoshi-ops/no9-tm_261025/motor_simulation/rust/Cargo.toml \
  -p motor_sim_runner -p sim-export
```

生成後のCSV/Parquet照合は`validate_inputs.py`、FFT照合は`fft-reference.py`を、
NumPy・pandas・PyArrowを備えたPythonで実行する。両scriptは隣接する`generation.json`の入力を使う。
今回の検証用Pythonは一時環境 `/private/tmp/panely-session-python/bin/python`。
再生成で保存先を変えた場合は生成manifestとPlot configのsource pathも対応させる。

## 主な画面証跡

- [初回3ソース](../study/diary/assets/2026-10-26-motor-session/01-three-source-loaded.png)
- [Zoom・Pan・Hover](../study/diary/assets/2026-10-26-motor-session/02-zoom-pan-hover.png)
- [統計とFFT](../study/diary/assets/2026-10-26-motor-session/03-statistics-and-fft.png)
- [Fit Overlapと解析保持](../study/diary/assets/2026-10-26-motor-session/04-fit-overlap-keeps-analysis.png)
- [offset付き中間ソース削除](../study/diary/assets/2026-10-26-motor-session/06-middle-source-removed-offset-kept.png)
- [5回目preset復元](../study/diary/assets/2026-10-26-motor-session/08-fifth-preset-cycle-restored.png)
- [全ソース削除](../study/diary/assets/2026-10-26-motor-session/09-all-sources-removed.png)
- [Plot configによるunion復元](../study/diary/assets/2026-10-26-motor-session/10-plot-config-restored-union.png)
- [反復後の統計・200 Hz範囲のFFT](../study/diary/assets/2026-10-26-motor-session/11-final-recomputed-fft-200hz.png)
