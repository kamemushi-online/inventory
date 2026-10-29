---
project: Panel_y
doc_type: Implementation Results
rfq: "RFQ #1"
created: "2026-10-24"
status: implemented_and_validated
implementation: "code/rust_phase1"
---

# RFQ #1 — ソース別 FFT 結果の重ね表示 実装・検証結果

2026-10-24、`a6459fb`を基点に実装。205件の自動テスト、macOS GUI全10項目、
fmt/build/diffチェックが成功した。

## 実装仕様

- FFT対象は全ソース合計で最大3チャンネル。Runは選択対象をソースごとの独立した計算ジョブへ分割する。
- 計算1ジョブは同一ソースのraw time配列とdtを使用する。異なるソースを1ジョブに渡すmixed-source guardは維持し、時系列の連結やソース間補間は行わない。
- Cursor A/Bはdisplay timeで保持し、各ソースのoffsetを差し引いてraw sample区間へ変換する。一定offsetは元のdtを変更しない。
- 各結果のN・dt・df・native frequency binsを保持し、表示だけを共通Hz軸と振幅/dB軸へ重ねる。異なる周波数binをリサンプリングしない。
- ソース別にN・dt・df・peakのsummaryを表示する。各スペクトルは色凡例とソースラベルで区別する。
- Hoverは各ソースのスペクトルに対して近傍binを検索する。そのソースの周波数範囲外、特にNyquist外では値を返さず、最終binの値で補わない。
- 1ソースの失敗はそのソースのエラーとして残す。他ソースが成功した場合はそのスペクトルを表示する。
- Cursor・offset・FFT window・対象変更は結果全体を無効化する。Source削除/path変更でもCursorが消去されるため、FFT結果全体を無効化する。
- Cancelとrequest identityで古いジョブの結果の再表示を防ぐ。source generation、time、rawデータなど入力変更後にstale結果を適用しない。

Phase 1 M5ではUIで異なるソースを同時選択した時点で実行を拒否していた。
今回の変更はRunで独立ジョブへ分割するものであり、計算1ジョブに異なるtime/dtを混在させない
安全境界は維持する。旧M5の拒否条件は当時の検証記録として残す。

## 自動検証

`code/rust_phase1`で実行した（`git diff --check`はリポジトリ全体）。
FFTモジュールに6件、アプリ統合に5件のテストを追加した。

| 検証 | 結果 |
|---|---|
| `cargo test --offline --locked` | 205 passed / 0 failed（11件追加） |
| `cargo fmt --check` | 成功 |
| `cargo build --offline --locked` | 成功 |
| `git diff --check` | 成功 |

FFTモジュールの追加6件は、ソース別グループ化とoffset付きraw区間、native frequency gridの維持、
ソース別近傍binと周波数範囲外の欠損、片方のCursor区間が無効な場合の部分成功、
全体の対象/Cursor/copy budget検証、独立完了・Cancel、ソース別失敗と古いジョブの拒否を確認した。

アプリ統合の追加5件は、workerの独立完了とnative sample grid、片方のエラーが他方の後着成功で
消えないこと、Cancel時の完了結果保持と旧Run結果拒否、offset変更による全pending結果の拒否、
FFT非対象ソースの削除でもCursor依存ジョブを消去することを確認した。
既存の単一ソースFFTと計算1ジョブのmixed-source guardを含む回帰テストも成功した。

別agentによる最終レビューでも、修正必須の問題は見つからなかった。

## GUI確認

debugビルドを検証用アプリ `PanelY FFT Overlay Smoke` として起動した。
ビルド成果物とアプリ内実行ファイルのSHA-256はともに
`c4651e4ce3896d7e673d8dbf9b4cc155da28f0bf7330ed187397b233fdd32a88`。
解析欄の高さはユーザーが手動で広げ、その後のグラフと操作結果を確認した。

fixtureは `code/proto_3_1b/data/multi_source_validation_1.parquet`（50 kHz、#1）と
`multi_source_validation_2.parquet`（64 kHz、#2）。通常の比較では両ソースのoffsetを0、
Cursor Aを `0.751152098 s`、Bを `1.251152039 s` とし、両方の `iu` をHanning窓で解析した。

| ソース | N | dt | df（画面表示） | Nyquist | peak | 振幅 | dB |
|---|---:|---:|---:|---:|---:|---:|---:|
| #1 | 25001 | 0.000020000 s | 2.000 Hz | 25.000 kHz | 49.998 Hz | 4.999797 | 13.979 |
| #2 | 32001 | 0.000015625 s | 2.000 Hz | 32.000 kHz | 49.998 Hz | 4.124869 | 12.308 |

丸め表示ではdfとpeakが一致するが、各ソースのNとdtから作った周波数binは独立している。
binの差と補間を行わないことは自動テストでも確認した。振幅は既存の窓関数と `2/N` の
正規化を維持しており、窓のcoherent gain補正は追加していない。

| 確認 | 実測結果 | 判定 |
|---|---|---|
| 異なるソースの対象を選びRun | `iu #1` と `iu #2` を同時選択してRunでき、2ソースとも完了 | PASS |
| ソース別native binsの重ね表示 | N 25001 / 32001、dt 20 / 15.625 μsの結果を同一Hz軸へ表示。全範囲は約31.999 kHz | PASS |
| ソース別N・dt・df・peak、色凡例、ラベル | 上表の値と、#1/#2の区別、青/橙の凡例と曲線の一致を確認 | PASS |
| 振幅/dB表示 | Max 100 Hzで2本のピークを表示。50 Hz付近のHoverも上表の振幅/dBに一致 | PASS |
| 正負offsetとCursor A/Bの解析区間 | #1を+0.125 s、#2を−0.250 sにして再Run。display Cursorを保持し、各raw区間で上表と同じN・dt・peakを確認 | PASS |
| Hoverと片方のNyquist外の欠損 | 27.151 kHzでは#2のみ約5.900359e-10を表示し、#1の値は表示されない。50 Hz付近では各結果の近傍binを表示 | PASS |
| 片方失敗時のエラーと成功スペクトル | A 0.101382487 s / B 0.400921673 sで#2に範囲外エラー。#1はN 14978、peak 50.073 Hz、振幅4.998103の結果を表示 | PASS |
| Cursor/offset/window/targets変更による無効化 | それぞれの変更で結果消去を確認。Hammingで再Runすると振幅は#1 5.399800 / #2 4.454883となり、Hanningへ戻すと再び結果消去 | PASS |
| FFT非対象Sourceの削除後の状態 | #1のFFTを表示した状態で#2を削除するとCursor A/BとFFT結果が消去され、Runが無効化。#1の時間波形は残る | PASS |
| 単一ソース回帰と最大3対象 | 4候補で3/3選択時は4個目が無効。2ソース3本を描画し、#1のiu/torqueだけでも2本を表示。torqueはpeak 2.000 Hz、振幅0.624976 | PASS |

Cancel直後の旧結果到着や、新しいRunとの競合はGUIでは実施せず、上記の決定的な自動テストで
確認した。GUIの削除確認は非対象Source削除を対象とし、path変更の操作は今回の10項目に含めない。
検証後は2ソースの `iu`、通常のCursor区間、Hanning、Max 100 HzのdB比較表示に戻した。

## 画面証跡

1. [異なるnative gridとNyquist外のHover](../study/diary/assets/2026-10-24-fft-overlay/01-native-grids-nyquist-hover.jpg)
2. [振幅の重ね表示とHover](../study/diary/assets/2026-10-24-fft-overlay/02-amplitude-overlay-hover.jpg)
3. [dBの重ね表示](../study/diary/assets/2026-10-24-fft-overlay/03-db-overlay.jpg)
4. [正負offset適用後の再解析](../study/diary/assets/2026-10-24-fft-overlay/04-offset-restored-spectrum.jpg)
5. [片方の範囲外エラーと成功結果](../study/diary/assets/2026-10-24-fft-overlay/05-partial-source-error.jpg)
6. [Hamming窓での解析](../study/diary/assets/2026-10-24-fft-overlay/06-hamming-window.jpg)
7. [窓変更による結果消去](../study/diary/assets/2026-10-24-fft-overlay/07-window-change-invalidates.jpg)
8. [最大3対象の制限](../study/diary/assets/2026-10-24-fft-overlay/08-three-target-limit.jpg)
9. [2ソース・3スペクトル](../study/diary/assets/2026-10-24-fft-overlay/09-three-traces.jpg)
10. [単一ソース・2スペクトル](../study/diary/assets/2026-10-24-fft-overlay/10-single-source-two-channels.jpg)
11. [非対象Source削除によるCursorと結果消去](../study/diary/assets/2026-10-24-fft-overlay/11-source-removal-invalidates.jpg)
