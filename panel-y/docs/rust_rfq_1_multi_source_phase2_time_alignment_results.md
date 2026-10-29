---
doc_type: Phase 2 Execution Results
status: complete
phase: "Multi-source Phase 2 / Time Alignment"
date: 2026-08-21
updated: 2026-10-24
---

# RFQ #1 — Multi-source Phase 2 / Time Alignment 実行結果

## 1. Status

時間offset編集、Source別 `Start at zero`、ロード済みSource一括の
`Start all at zero` を実装した。自動テストとGUI smoke 10項目を完了した。

2026-09-08に実装コミット `563da91` を再確認し、170テストが成功した。
この時点ではGUI smokeが未確認だった。

2026-10-23に `23be017` のdebugビルドをGUI操作し、全10項目が合格した。
Phase 2 Time Alignmentを `complete` とする。実測値と画面証跡は7.1節に記録する。

## 2. Time model

Phase 1で全time boundaryへ接続済みの `TimeTransform` をそのまま使用する。

```text
display_time = raw_time + offset_seconds
raw_time     = display_time - offset_seconds
```

offset変更時も次を変更しない。

- Parquetからロードしたraw time配列
- channel value配列
- `ChannelKey`
- `ChannelStore`の所有構造
- source generation

Source別 `Start at zero` は、先頭raw timestampを `raw_start` として
`offset_seconds = -raw_start` を設定する。`Start all at zero` はtime dataを
ロード済みの各Sourceへ同じ計算を独立に適用し、未ロードSourceはskipする。

## 3. UI

各Data Sourceカードへ次を追加した。

- `Offset`: 秒単位の数値編集
- `Reset`: offsetを `0.0` 秒へ戻す
- `Start at zero`: そのSourceの先頭時刻をdisplay time 0へ合わせる
- `display time range`: offset適用後のSource範囲

Data Sources欄の末尾へ `Start all at zero` を追加した。

## 4. Dependent state

offset変更後はworkspaceのdisplay rangeを再評価する。現在のX rangeが新しい
workspace範囲外になった場合だけclampし、それ以外のPan/Zoom位置は保持する。

Cursor A/Bはdisplay time座標のまま保持する。offset変更でSourceとの対応raw timeが
変わるため、区間統計cacheを無効化して次回表示時に再計算する。

FFT request keyは従来からoffset bitsを含む。UI操作時には表示済み結果とrunning jobを
明示的に無効化し、変更前offsetのstale job resultが再表示されないようにする。

## 5. Persistence boundary

Phase 2実装時点では、時間offsetは実行中workspaceにだけ保持し、Plot config v1と
Phase 1 Session presetには保存しなかった。2026-10-24にPlot config v2のファイル保存・復元を
実装・検証した。同日に複数ソースSession presetのメモリ内保存・復元も実装した。
194件の自動テスト・ビルド・GUI全10項目が成功した。
現在の仕様は[Plot config v2実行結果](./rust_rfq_1_multi_source_plot_config_v2_results.md)と
[Session preset実行結果](./rust_rfq_1_multi_source_session_preset_results.md)を参照。

## 6. Automated validation

追加テスト:

1. manual offsetがdisplay transformだけを変更する
2. non-finite offsetを拒否し、`-0.0` を `0.0` へ正規化する
3. Source別および全Sourceの `Start at zero` が先頭display timeを0へ揃える
4. raw timeとsource generationが不変である
5. Cursor A/Bを保持し、統計cacheとFFT結果を無効化する

実装時の結果（2026-08-21）:

- `cargo fmt --check`: success
- `cargo check`: success
- `cargo test`: 170 passed / 0 failed
- `cargo build --release`: success
- `cargo clippy --all-targets`: success（既存の非blocker warningのみ）
- Debug GUI startup: success

再確認結果（2026-09-08、`563da91`）:

- `cargo fmt --check`: success
- `cargo check --offline --locked`: success
- `cargo test --offline --locked`: 170 passed / 0 failed
- `git diff --check`: success

2026-09-08の再確認ではRelease build、Clippy、GUI起動・操作は実施していない。
2026-10-23は `cargo build --offline --locked` と7節のGUI操作を実施し、コード変更はない。

## 7. GUI smoke checklist

状態: 2026-10-23に10/10項目が合格。
自動テスト成功およびGUI起動確認だけでは、本節を完了扱いにしない。

検証fixture:

- Source #1: `multi_source_validation_1.parquet`、raw `0.0 .. 2.0 s`
- Source #2: `multi_source_validation_2.parquet`、raw `0.5 .. 1.75 s`

確認項目:

1. Source #2のOffsetを `0.25 s` にして、display rangeが
   `0.75 .. 2.0 s` となりSource #1が移動しない
2. Source #2の `Reset` で `0.5 .. 1.75 s` に戻る
3. Source #2の `Start at zero` でoffsetが `-0.5 s`、display rangeが
   `0.0 .. 1.25 s` になる
4. `Start all at zero` で両Sourceのdisplay startが `0.0 s` になる
5. offset変更後もロード済みchannelがUnloadedにならず、Row構成が維持される
6. offset適用後のoverlap/source-only区間でHover値が正しい
7. Cursor A/Bが画面上の同じdisplay timeへ残り、両Sourceの値と統計が再計算される
8. FFT結果表示後に対象Sourceのoffsetを変え、古い結果が消えて再表示されない
9. `Fit All` がoffset適用後の全Source unionを表示する
10. Source #2削除後もSource #1のoffset、Hover、Cursor、FFTが動作する

### 7.1 GUI確認記録（2026-10-23）

対象はHEAD `23be017`（時間合わせ実装 `563da91`）を `cargo build --offline --locked` で
ビルドしたdebugバイナリ。製品コードを変更せず、一時macOSアプリから起動してCodexが
GUIを操作した。fixture 2件のSHA-256は[M0結果](./rust_rfq_1_multi_source_phase1_m0_results.md)
と一致した。両Sourceの `iu` をRow 1へ追加して確認した。

| 項目 | 判定 | 実測・観察 |
|---|---|---|
| 1. Offset | 合格 | #2を `+0.25 s` にすると `0.75 .. 2.0 s`。#1は `0.0 .. 2.0 s` を保持 |
| 2. Reset | 合格 | #2がoffset `0.0 s`、範囲 `0.5 .. 1.75 s` へ復帰 |
| 3. Start at zero | 合格 | #2がoffset `-0.5 s`、範囲 `0.0 .. 1.25 s` へ変化 |
| 4. Start all at zero | 合格 | #1を `+0.125 s`、#2を `0.0 s` に設定後、一括操作でoffsetがそれぞれ `0.0` / `-0.5 s`、両開始時刻が0秒になった |
| 5. ロード・Row維持 | 合格 | 上記操作とその後のoffset変更を通じ、1 Row・2チャンネル、各Source 1チャンネルloadedを保持。raw cacheは100001 / 80001 samples |
| 6. Hover | 合格 | #2 offset `+0.25 s`、display `0.502304 s` では #1 `6.613119` のみ。display `1.0 s` では #1 `1.964387e-14`、#2 `-4.449228`。#2 offset `+0.375 s`、display `2.048506 s` では #2のみを表示 |
| 7. Cursor / 統計 | 合格 | #2 offsetを `+0.25` → `+0.375 s` へ変更してもA/B時刻と#1の数値は不変。#2のA/B値・Mean・RMSが下表のとおり更新 |
| 8. FFT結果無効化 | 合格 | #2 offset `+0.375 s` でFFT結果（12830 samples、peak `49.883 Hz`）を表示後、`+0.25 s` へ変更。結果表とスペクトルが消え、その後のHover操作・再描画でも復活しなかった |
| 9. Fit All | 合格 | #2 offset `+0.25 s` で `0.0 .. 2.0 s`、`+0.375 s` で `0.0 .. 2.125 s` をボタン操作により表示 |
| 10. Source #2削除後 | 合格 | #1を `+0.125 s` に設定し#2を除外。#1のoffsetと範囲 `0.125 .. 2.125 s`、1 Row・1 loadedチャンネルを保持。Cursor再配置後の値・統計・Hoverが独立計算と一致し、FFTは10008 samples、peak `49.960 Hz` を表示 |

Cursor A/Bのdisplay timeは `0.900921643 s` / `1.101382494 s`。
数値はfixture生成式、float32丸め、最近傍サンプルから独立計算し、表示精度で一致した。

| Channel / offset | A値 | B値 | Mean | RMS |
|---|---:|---:|---:|---:|
| #1 / 0.0 s（変更前後とも） | 2.850193 | 4.200857 | 0.008455 | 7.064739 |
| #2 / +0.25 s | -6.153374 | -6.798092 | 0.383896 | 5.848913 |
| #2 / +0.375 s | 5.411566 | 4.431125 | 0.410589 | 5.845366 |

Source #2除外時には既存仕様によりCursor A/Bもクリアされた。その後、#1のdisplay time
`0.900429308 s` / `1.100561440 s` に再配置し、A値 `-9.913076`、B値 `-9.845643`、
Mean `-0.007898`、RMS `7.073759` を確認した。B位置のHoverも `-9.845643` と一致した。
FFTはHanning / Amplitude、#1のみを対象として実行し、peak amp `4.999291` を表示した。
FFT無効化は表示済み結果で確認しており、running jobとの競合をGUIで追加検証したものではない。

証跡:

- [Cursor変更前](../study/diary/assets/2026-10-23-phase2-smoke/01-cursors-before-offset.jpg)
- [Cursor変更後](../study/diary/assets/2026-10-23-phase2-smoke/02-cursors-after-offset.jpg)
- [Fit AllとSource #2単独区間のHover](../study/diary/assets/2026-10-23-phase2-smoke/03-fit-all-source2-only-hover.jpg)
- [offset変更前のFFT結果](../study/diary/assets/2026-10-23-phase2-smoke/04-fft-before-offset.jpg)
- [offset変更・再描画後のFFT無効化](../study/diary/assets/2026-10-23-phase2-smoke/05-fft-invalidated.jpg)
- [Source #2除外後のoffset・Hover・Cursor・FFT](../study/diary/assets/2026-10-23-phase2-smoke/06-source1-after-remove.jpg)
- [独立計算値とfixture SHA-256](../study/diary/assets/2026-10-23-phase2-smoke/expected-values.json)

FFT操作欄の高さが不足し、自動ドラッグでは拡張できなかったため、ユーザーが
Cursor information上端の境界を手動で広げた。その後、CodexのGUI操作で項目8と10を完了した。
付随観察として、2ソースの `Raw columns` にwidget ID重複のdebug警告が表示された。
時間合わせの上記判定とは分けて記録し、2026-10-23の検証時点では製品コードを修正していない。

2026-10-24にこの警告を修正した。`Raw columns` のwidget IDへ `SourceId` を含め、
2ソースの開閉状態を独立させた。debug GUIで警告消失と片方ずつの開閉・両方展開を確認した。
ビルドと既存170テストも成功。詳細と証跡は[修正記録](../study/diary/2026-10-24.md)を参照。

## 8. Completion

自動テストの成功記録とGUI smoke全10項目の合格により、Phase 2 Time Alignmentは完了。
2026-10-23の完了記録時点では製品コードの変更はない。翌日の付随警告修正は7.1節末尾に記録した。
別工程の `Fit Overlap` は2026-10-24に実装・検証を完了した（[実行結果](./rust_rfq_1_multi_source_fit_overlap_results.md)）。
複数SourceとoffsetのPlot config v2永続化も同日に実装・検証を完了した（[実行結果](./rust_rfq_1_multi_source_plot_config_v2_results.md)）。複数ソースSession presetも同日に実装し、194件の自動テスト・ビルド・GUI全10項目が成功した（[実行結果](./rust_rfq_1_multi_source_session_preset_results.md)）。
