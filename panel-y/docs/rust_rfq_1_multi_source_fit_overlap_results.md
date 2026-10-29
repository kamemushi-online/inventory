---
doc_type: Fit Overlap Execution Results
status: complete
date: 2026-10-24
---

# RFQ #1 — Fit Overlap 実行結果

## 1. 実装範囲

Phase 2 Time Alignment完了後の追加機能として、View欄の `Fit All` の隣に
`Fit Overlap` を追加した。offset適用後の共通区間へX表示範囲を合わせる。

```text
start = max(loaded source display starts)
end   = min(loaded source display ends)
```

- `shared_time` をロード済みの全Sourceが対象。チャンネルの表示・非表示や選択に依存しない。
- time未ロードのSourceは対象外。1ソースだけなら、そのdisplay time全範囲となる。
- `start < end` の共通区間がない場合は無効。端点だけの接触も無効とする。
- ロード済みtimeの範囲が空・ゼロ幅・非有限の場合も無効とする。
- ボタン操作時だけ範囲を合わせる。offset変更に追従し続けるモードではない。
- X範囲とEnvelope cacheだけを更新し、raw time、チャンネル、Row構成、offset、Cursor A/Bを変更しない。
- 狭い共通区間が描画・Pan/Zoomの最小幅制限で広がらないよう、共通区間が存在する間は
  ワークスペースの最小表示幅を「従来の最小幅」と「共通区間幅」の小さい方にする。
- `Fit All` は引き続き全ロード済みSourceのdisplay rangeの和集合を表示する。

実装は `13b9ef7` に対する追加変更。debugビルドを一時アプリ
`/private/tmp/PanelY-FitOverlap-Smoke.app` から起動して確認した。

## 2. 自動テスト

5テストを追加した。

1. 正負のoffsetを持つ3ソースの共通区間と、Source順序に依存しないこと。
2. 非重複・端点接触では共通区間がなく、第3ソースを含めても誤って復活しないこと。
3. time未ロードのSourceを除外し、0ソース・1ソース・無効なロード済みtimeを扱えること。
4. offset変更、共通区間なし時の操作、Source削除、`Fit All` 復帰、Cursor・Row・ロード状態の保持。
5. サンプル間隔より狭い共通区間が、実際の描画データ生成と範囲clampを経ても維持されること。

結果:

- `cargo fmt --check`: success
- `cargo test --offline --locked`: **175 passed / 0 failed**
- `cargo build --offline --locked`: success
- `git diff --check`: success

## 3. GUI確認

Source #1は `multi_source_validation_1.parquet`（raw `0.0 .. 2.0 s`）、
Source #2は `multi_source_validation_2.parquet`（raw `0.5 .. 1.75 s`）。
両Sourceの `iu` をRow 1へ追加した。

| 操作・条件 | 観察結果 | 判定 |
|---|---|---|
| 両offset 0で `Fit All` | `0.0 .. 2.0 s` | 合格 |
| 両offset 0で `Fit Overlap` | `0.5 .. 1.75 s` | 合格 |
| #2 offset `+0.25 s` → `Fit Overlap` | `0.75 .. 2.0 s` | 合格 |
| #2 offset `+1.49999 s` → `Fit Overlap` | `1.999990 .. 2.000000 s`。10μs幅を保持 | 合格 |
| #2 offset `+1.5 s`（端点接触） | `Fit Overlap` が無効 | 合格 |
| #2 offset `+2.0 s`（非重複） | `Fit Overlap` が無効。`Fit All` は `0.0 .. 3.75 s` を表示 | 合格 |
| #2 offset `-0.75 s`へ戻す | 再び有効となり、操作後は `0.0 .. 1.0 s` | 合格 |
| #1 offset `+0.125 s` にして#2を除外 | `Fit Overlap` で残った#1の `0.125 .. 2.125 s` を表示 | 合格 |
| 表示範囲の切替 | Source除外前まで1 Row・2 loadedチャンネルとCursor A `0.799528301 s`を保持 | 合格 |

Cursor A/B両方の保持は自動テスト、実GUIではCursor Aの保持を確認した。
Source除外時のCursorクリアは既存仕様どおり。time未ロード・無効値・3ソースの境界は
自動テストで確認し、GUIは通常fixtureの2ソースと除外後の1ソースで確認した。

画面証跡:

- [共通区間](../study/diary/assets/2026-10-24-fit-overlap/01-common-range.jpg)
- [正のoffset適用後](../study/diary/assets/2026-10-24-fit-overlap/02-offset-common-range.jpg)
- [10μsの共通区間](../study/diary/assets/2026-10-24-fit-overlap/03-narrow-common-range.jpg)
- [端点接触で無効](../study/diary/assets/2026-10-24-fit-overlap/04-touching-disabled.jpg)
- [非重複とFit All](../study/diary/assets/2026-10-24-fit-overlap/05-disjoint-fit-all.jpg)
- [負のoffsetで再有効化](../study/diary/assets/2026-10-24-fit-overlap/06-negative-offset-reenabled.jpg)
- [Source除外後](../study/diary/assets/2026-10-24-fit-overlap/07-single-source-after-remove.jpg)
