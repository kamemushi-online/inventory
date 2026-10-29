---
project: Panel_y
doc_type: Implementation Results
rfq: "RFQ #1"
created: "2026-10-24"
status: complete
implementation: "code/rust_phase1"
---

# RFQ #1 — Plot config v2 実装・検証結果

2026-10-24、`8854ae1`を基点に実装し、自動テストとmacOS GUI確認を完了した。

## 保存・復元仕様

- top-level `version: 1`を維持し、`data_path`、`rows`、`ch_styles`にはソース順の先頭の互換情報を保存する。選択中ソースによって投影対象を変更しない。
- `rust.version: 2`の`source_key`を使って、ソース順・パス・ラベル・時間offset、全行のチャンネル順、表示/非表示、Line/Step、色、線幅、color index、Y範囲を保存する。空行も保持する。
- `rust.rows`を完全な行構成として扱う。`ymin`/`ymax`は同拡張の各行に保存し、先頭ソースに存在しない行でもY範囲が失われない。
- ファイル内のsource keyと実行時SourceIdを分離する。Loadでは保存順に新しいIDへ対応させ、同名チャンネルを他ソースから補わない。
- Save / Save Asは複数ソースでも利用でき、1ソースでもv2でoffsetを保存する。保存パスは絶対パス、相対パスの読込は既存v1同様に設定ファイル所在を優先し、既存ファイルがなければ作業ディレクトリを参照する。
- Loadは各ソースのスキーマを先に確認する。欠損・読込不能・time列なしのソースと欠損チャンネルは警告付きで部分復元する。全ソースが使用不能なら現在のworkspace・行・波形を保持する。
- 重複source key、空のsources、不正なv2行構造、非対応のsourcesバージョンは変更前に拒否する。無効offsetは0に戻して警告する。
- Apply to Current Dataはパスを正規化して一意に照合する。現在のソース順、offset、raw cacheを保持し、行と表示設定を適用する。未一致・曖昧な対応・複数保存ソースから同じ実行時ソースへの対応は適用前に拒否する。offset差異は警告する。
- v1 Loadは従来どおり単一Source #1へ置換する。v1 Applyは単一ソースのみ許可し、複数ソース時には理由をLoad issueに表示する。
- 非同期チャンネル読込が完了したらLoad後のX範囲を全ソースのunionに合わせる。完了順によって最初のソースだけの範囲が残る問題をGUI確認中に修正した。
- Session presetは従来の単一ソース制限を維持する。任意のX範囲、Cursor/FFT状態、選択状態の永続化は今回のPlot config範囲に含めない。非表示チャンネルは設定のみ復元し、必要になるまで読み込まない。

## 自動検証

`code/rust_phase1`で実行した。

| 検証 | 結果 |
|---|---|
| `cargo test --offline --locked` | 185 passed / 0 failed（10件追加） |
| `cargo fmt --check` | 成功 |
| `cargo build --offline --locked` | 成功 |
| `git diff --check` | 成功 |

追加テストは相対パス・source key検証、無効offset、同名チャンネルの分離、非表示/スタイル、legacy projectionと完全行構成の往復、実ファイルのLoad/Save/Apply、欠損部分復元と全欠損時の保持、曖昧なApplyの拒否、非同期完了後union、v1 Applyの保護を確認する。既存v1読込・適用・visual modeのテストも継続して成功した。

## GUI確認

検証用bundleは `/private/tmp/PanelY-PlotConfigV2-Smoke.app`。debug実行ファイルをコピーし、CUAで操作した。fixtureは `multi_source_validation_1.parquet`（0..2 s、50 kHz）と `_2.parquet`（0.5..1.75 s、64 kHz）。一時設定は `/private/tmp/panely-v2-smoke/` に置いた。

| 確認 | 実測結果 | 判定 |
|---|---|---|
| v2 Load | #1 +0.125 s、#2 −0.250 s、2ソースを復元 | 合格 |
| 初期X範囲 | 非同期読込後に0.125000..2.125000 s | 合格 |
| Save | 保存JSONに2sources、3channels、offset、ラベルと先頭ソースのlegacy projection | 合格 |
| 再Load | 2行、Row1のiu #1/#2、Row2の非表示speed #2を復元。警告なし | 合格 |
| スタイル | #1橙・Line・幅2.5、#2水色・Step・幅1.5、非表示speed紫・Step・幅3.0、Row1 Y=−2..2 | 合格 |
| Apply | #1を一度非表示にしてから表示を復元。#2のoffsetを0へResetした値は保持し、差異を警告 | 合格 |
| Save As | native dialogで別ファイル`view.pyc.json`へ保存。2sourcesと現在のoffsetをJSONでも確認 | 合格 |
| 一部欠損 | #2欠損時、#1波形を復元し、欠損ソース/チャンネルと空になった行の警告を表示 | 合格 |
| 全欠損 | エラーを表示し、直前の#1 +0.125 s・波形・行を保持 | 合格 |
| v1互換 | v1 Applyは2ソース状態を保持して理由を表示。v1 Loadは1ソース・offset 0・0..2 sへ復元 | 合格 |

最終ビルドで保存済みv2の再Loadとv1 Applyのエラー表示を再確認した。検証終了時は保存済みv2をLoadし、2ソース・2行へ戻した。

## 画面証跡

1. [v2 Loadと全体範囲](../study/diary/assets/2026-10-24-plot-config-v2/01-v2-loaded.jpg)
2. [保存後の再Load・行スタイル](../study/diary/assets/2026-10-24-plot-config-v2/02-reloaded-row-styles.jpg)
3. [Apply後のoffset保持](../study/diary/assets/2026-10-24-plot-config-v2/03-apply-keeps-current-offset.jpg)
4. [部分復元の警告](../study/diary/assets/2026-10-24-plot-config-v2/04-partial-restore-warning.jpg)
5. [全欠損時のworkspace保持](../study/diary/assets/2026-10-24-plot-config-v2/05-all-missing-preserves-workspace.jpg)
6. [v1 Applyの保護と理由表示](../study/diary/assets/2026-10-24-plot-config-v2/06-v1-apply-protected.jpg)
7. [v1 Load互換](../study/diary/assets/2026-10-24-plot-config-v2/07-v1-load-compatible.jpg)
