---
project: Panel_y
doc_type: Implementation Results
rfq: "RFQ #1"
created: "2026-10-24"
status: implemented_and_validated
implementation: "code/rust_phase1"
---

# RFQ #1 — 3ソース以上のUI 実装・検証結果

2026-10-24、`fcb7043`を基点に実装。自動テスト208件、ビルド、整形チェックが成功した。
GUI全10項目も合格し、11枚の画面証跡を保存した。

## 実装仕様

- `MAX_UI_SOURCES = 2` と追加処理の上限guardを撤去。3番目以降も `Add Source` で登録できる。
- Data Sourcesの先頭にソース数、`Start all at zero`、`Add Source` を置き、カード一覧だけを縦スクロールする。
- 一覧の最大高さは画面高の50%、220〜440 px。外側の設定パネルもスクロールでき、Plot configやChannel browserへ移動できる。
- Path・schema・読込状態・offset・削除はソースごとに操作する。既存のSourceIdによるwidget ID分離を保持する。
- 中間ソースを削除すると表示番号は詰まるが、残るSourceId、ChannelKey、raw/time/cacheは別ソースへ付け替えない。再追加は新しいIDを割り当てる。
- 描画、matching channel、時間合わせ、Plot config v2、Session presetは既存のNソース処理を利用する。
- FFTの対象数は全ソース合計で最大3チャンネルのまま。ソース登録数とは別の制約である。

固定上限の撤廃は無制限のデータ量での性能保証ではない。今回のGUIでは4ソースまで、
追加処理の自動テストでは5ソースを扱った。大規模なソース数での性能測定は今回の範囲外。

## 自動検証

| 検証 | 結果 |
|---|---|
| `cargo test --offline --locked --manifest-path code/rust_phase1/Cargo.toml` | 208 passed / 0 failed |
| `cargo fmt --check --manifest-path code/rust_phase1/Cargo.toml` | 成功 |
| `cargo build --offline --locked --manifest-path code/rust_phase1/Cargo.toml` | 成功 |
| `git diff --check` | 成功 |

旧2ソース制限テストを5ソース追加・中間削除・再追加のテストへ置き換え、次の3件を追加した。

1. 3ソース同名チャンネルのmatchingで重複追加せず、中間削除後も他ソースのraw allocation、描画cache、選択を保持する。
2. 同一パスを含む3ソースのPlot config保存・再読込で、source key、offset、ラベル、行内の独立した参照を保持する。
3. 3ソースSession presetから削除済み中間ソースを復元し、新SourceIdへの対応付けと保存選択を復元する。

保存形式の追加テストは非表示チャンネルを使用し、同期の構成復元を検証した。
表示チャンネルの非同期読込完了と波形表示は下記GUIでも確認した。

## GUI確認

実行アプリは `/private/tmp/PanelY-MultiSource-Smoke.app`（表示名 `PanelY MultiSource Smoke`）。
ビルドとアプリ内実行ファイルのSHA-256:
`2aca0e3873a6b48b98497dba2019f6a5c6370376bdffd2f52f93a1829448efde`。

fixtureは `code/proto_3_1b/data/multi_source_validation_1.parquet`（#1、50 kHz）と
`multi_source_validation_2.parquet`（#2、64 kHz）。#3には#1の複製を
`/private/tmp/panely-multi-source-smoke/source-3.parquet` として用意し、offsetを+0.125 sにした。
#4には#2と同じファイルを別ソースとして登録した。元のfixtureは変更していない。

| 確認 | 実測結果 | 判定 |
|---|---|---|
| 3番目の追加・Path・schema | 2ソース構成からAdd Sourceし、#3のschema 100001行・3チャンネルを読込 | PASS |
| 4番目の追加と独立読込 | #4にもAdd Sourceし、#2と同じファイルから80001行・iuを独立読込 | PASS |
| 一覧と外側パネルのスクロール | 一覧内で#3/#4へ移動でき、上部Addと下部Plot configへ到達。外側ではChannel browser、Rows、Session presetへ移動 | PASS |
| 同名チャンネルの全ソース追加 | 3ソース、続いて4ソースでmatchingを実行。既存行を重複させず、iu #1〜#4の4本を表示 | PASS |
| 中間ソース削除 | 4ソース中の#2を削除し1行参照だけ除去。旧#3の+0.125 sと緑線、旧#4のピンク線を保持して#2/#3へ改番 | PASS |
| 個別offsetとFit Overlap | #3に+0.125 sを設定。3ソース共通区間は0.500000..1.750000 s | PASS |
| Start all at zero | 3ソースに適用しoffset 0 / −0.500 / 0 s、表示開始をすべて0へ変更 | PASS |
| Session preset復元 | 3ソース・1行・3チャンネルを保存。4ソース化と中間削除後に元の3ソース、色・線幅・選択・X範囲を復元 | PASS |
| Plot config v2保存・再Load | saved-three.pyc.jsonに3ソースとoffset 0 / 0 / +0.125 sを保存。Start all後にLoadして3波形・offsetを復元しX範囲0..2.125 sへfit | PASS |
| 3ソースのCursor・FFT | Cursor A 0.751584090 s / B 1.253456220 s、3対象でFFT成功。3ソースのsummary、凡例、100 Hz範囲の曲線と個別binのHoverを確認 | PASS |

解析欄はユーザーが手動で広げた後にFFTを実行した。#1/#3はN 25095、dt 20 μs、
peak 49.811 Hz、振幅4.970769。#2はN 32121、dt 15.625 μs、peak 49.812 Hz、振幅4.101152。
#3は#1の複製をoffset付きで解析しているため、今回の振幅曲線は重なった。
59.207 HzのHoverでは#1/#3は59.773 Hzのbin、#2は59.774 Hzのbinを参照しており、
個別の周波数gridが保持されている。検証用アプリは3ソースのFFT比較表示のまま残した。

最終コードレビューでも修正必須の問題は見つからなかった。

## 画面証跡

1. [3ソースmatchingと波形](../study/diary/assets/2026-10-24-multi-source-ui/01-three-source-matching.jpg)
2. [共通区間と3ソースpreset保存](../study/diary/assets/2026-10-24-multi-source-ui/02-three-source-preset-overlap.jpg)
3. [4番目のカードと一覧スクロール](../study/diary/assets/2026-10-24-multi-source-ui/03-fourth-source-scroll.jpg)
4. [4ソースの波形表示](../study/diary/assets/2026-10-24-multi-source-ui/04-four-source-waveforms.jpg)
5. [中間ソース削除後](../study/diary/assets/2026-10-24-multi-source-ui/05-middle-source-removal.jpg)
6. [3ソースpreset復元](../study/diary/assets/2026-10-24-multi-source-ui/06-preset-restored-three.jpg)
7. [Start all at zero](../study/diary/assets/2026-10-24-multi-source-ui/07-start-all-at-zero.jpg)
8. [3ソースPlot config再読込](../study/diary/assets/2026-10-24-multi-source-ui/08-config-restored-three.jpg)
9. [3ソースCursorとFFT対象](../study/diary/assets/2026-10-24-multi-source-ui/09-three-source-cursors.jpg)
10. [3ソースのFFT結果と凡例](../study/diary/assets/2026-10-24-multi-source-ui/10-three-source-fft-results.jpg)
11. [100 Hz範囲のFFTとソース別Hover](../study/diary/assets/2026-10-24-multi-source-ui/11-three-source-fft-hover.jpg)
