---
project: Panel_y
doc_type: Implementation Results
rfq: "RFQ #1"
created: "2026-10-24"
status: implemented_and_validated
implementation: "code/rust_phase1"
---

# RFQ #1 — 複数ソース Session preset 実装・検証結果

2026-10-24、`ed2889b`を基点に実装。194件の自動テスト、ビルド、macOS GUI確認10項目が成功した。

## 保存・復元仕様

- Session presetは実行中のメモリ内snapshotであり、アプリ終了で消失する。ファイルへの永続化にはPlot config v2を使用する。
- Saveは全ソースの順序・パス・ラベル・時間offset、選択中ソース/チャンネル/行、全行のチャンネル順・表示状態・色・線幅・Line/Step・Y範囲、X範囲、大規模データpreview設定を保存する。空行も保持する。
- Apply時、保存したIDとパスが現在のソースに一致し、schemaが存在すればrawデータ、time配列、generationを再利用する。時間offsetと表示設定は保存値へ戻す。
- 削除済みソースまたはパス変更済みソースは、保存パスのschemaを事前確認して新しいSourceIdを割り当てる。行のChannelKeyと選択状態は復元後のIDへ対応させ、別ソースの同名チャンネルから補わない。
- 新規割当した実行時SourceIdとの対応を保持する。繰り返しApplyしても復元済みソースを再生成せず、そのrawデータ・time配列・generationを再利用する。
- 保存順にworkspaceを復元し、snapshotに含まれない現在のソースは除外する。現在のソースが0件でもApplyできる。
- 復元できないソース・チャンネルは警告して部分復元する。保存した全ソースが復元不能な場合は、現在のworkspace・行・波形を保持する。
- 未読込の表示チャンネルは非同期で読み込む。読み込み完了後に保存したX範囲を復元する。非表示チャンネルは設定のみ復元し、非同期ロードを開始しない。全チャンネルが非表示の場合も保存したX範囲を保持する。
- Applyでは描画cacheと解析状態を消去する。Cursor/FFT結果のsnapshotではない。
- ロード中はSave / Applyを無効化する。Saveにはschemaを持つソースが1つ以上必要。Clearは保存snapshotだけを消去し、現在のworkspaceを変更しない。

## 自動検証

`code/rust_phase1`で実行した（`git diff --check`はリポジトリ全体）。

| 検証 | 結果 |
|---|---|
| `cargo test --offline --locked` | 194 passed / 0 failed（9件追加） |
| `cargo fmt --check` | 成功 |
| `cargo build --offline --locked` | 成功 |
| `git diff --check` | 成功 |

追加テストで複数ソースsnapshotの往復、offsetと選択状態、raw/time/generationの再利用、
削除・パス変更後のSourceId再割当、欠損の部分復元と全欠損時の状態保持、非同期ロード完了後の
保存X範囲復元、非表示チャンネルの遅延読込、空workspaceからの復元、Save / Applyのguardを確認した。
繰り返しApply時に実行時IDの対応を失う問題と、全チャンネル非表示時に保存X範囲を失う問題も
修正し、回帰テストを追加した。

## GUI確認

`PanelY SessionPreset Smoke`として修正版debugバイナリを起動し、10項目を確認した。
基準fixtureは `multi_source_validation_1.parquet`（0..2 s、50 kHz、100001 samples）と
`multi_source_validation_2.parquet`（0.5..1.75 s、64 kHz、80001 samples）。
欠損の検証には `/private/tmp/panely-session-smoke/` に複製した `source1.parquet` と
`source2.parquet` を使い、一時的に改名した。検証後は元の名前へ復帰済み。

検証用appとビルド出力のSHA-256はともに
`d56f2fd47c72b5464a7e1c185e816b386852edcfbbe603fd091cab0b83643367`。

| 確認 | 実測結果 | 判定 |
|---|---|---|
| 2ソース状態のSave | `2 source(s), 2 row(s), 3 channel(s)` を保存 | PASS |
| offset・行スタイル・表示状態のApply | offsetを0へ変更後、+0.125 s / -0.250 sへ復元。Row 1の2波形、Line/Step、橙/水色、幅2.5/1.5、Y [-2, 2]を保持。previewはoffからonへ復元 | PASS |
| 選択ソース/チャンネル/行の復元 | #2、`iu #2`、Row 2を復元。非同期読込後も選択を維持 | PASS |
| 保存X範囲の復元 | Fit Allやoffset変更後、保存時の0.250000..1.500000 sへ復元。削除・再読込後も同じ範囲 | PASS |
| ソース削除後のApplyと非同期読込 | #2削除後に再生成し、表示中のiuを自動読込。さらにApplyを2回行っても表示と選択を保持 | PASS |
| 0ソースからのApply | 全ソース削除後もApplyが有効。2ソース・2行・2表示波形とoffset・X範囲を復元 | PASS |
| 一部欠損時の部分復元・警告 | #2削除とファイル退避後、#1のみ復元。欠損ソースと`Source 2:iu, Source 2:speed`を警告し、Row 2は空行として保持 | PASS |
| 全欠損時のworkspace保持 | 別パスの単一ソースworkspaceを読込後、両保存ファイルを欠損させてApply。失敗と保持を通知し、現在の1行・1波形、X 0..2 sを維持 | PASS |
| 非表示チャンネルの設定保持 | Row 2の`speed #2`を非表示、紫、Step、幅3.0で保持。ソース再生成後もunloadedのままで、表示チャンネルのみ読込 | PASS |
| Clear後のsnapshot消去とworkspace保持 | ファイル復帰後に全体を復元してClear。`No preset saved`となりApply/Clearは無効化。2ソース・2行・波形・X範囲は維持 | PASS |

raw/timeの同一allocationとgenerationの再利用、全チャンネル非表示のX範囲保持、
busy/schema未読込guardは自動テストで検証した。Session presetはメモリ内のみで、
アプリを終了してからの復元には対応しない。

## 画面証跡

1. [2ソースsnapshotのSave](../study/diary/assets/2026-10-24-session-preset/01-saved.jpg)
2. [表示・選択・X範囲の復元](../study/diary/assets/2026-10-24-session-preset/02-view-restored.jpg)
3. [正負offsetの復元](../study/diary/assets/2026-10-24-session-preset/03-offsets-restored.jpg)
4. [削除後の復元と繰り返しApply](../study/diary/assets/2026-10-24-session-preset/04-repeat-after-source-restore.jpg)
5. [全ソース削除後もApplyが有効](../study/diary/assets/2026-10-24-session-preset/05-empty-workspace.jpg)
6. [空workspaceからの復元](../study/diary/assets/2026-10-24-session-preset/06-restored-from-empty.jpg)
7. [一部欠損時の部分復元と警告](../study/diary/assets/2026-10-24-session-preset/07-partial-restore.jpg)
8. [全欠損時の現在workspace保持](../study/diary/assets/2026-10-24-session-preset/08-all-missing-preserved.jpg)
9. [Clear後のworkspace保持](../study/diary/assets/2026-10-24-session-preset/09-clear-preserves-workspace.jpg)
