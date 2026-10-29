---
project: Panel_y
doc_type: M6 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M6"
created: "2026-07-26"
updated: "2026-08-21"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
source_design: "docs/rust_rfq_1_multi_source_overlay_design.md"
previous_results: "docs/rust_rfq_1_multi_source_phase1_m5_results.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M6 実行結果

## 1. 目的とスコープ

Phase 1の単一source永続化形式が、multi-source workspaceをSource #1だけの設定として
暗黙保存しないようにguardした。Plot config version 1のLoadは互換性を維持し、
multi-source workspace全体を単一のSource #1へ置き換える操作として分離した。

あわせて、source追加・削除・path変更後のselection、X range、Hover、Cursor、FFT、
load statusと、Perf集計・source番号付きprogressの既存実装をM6条件に照らして確認した。

M6専用の実行指示ファイルは開始時点で存在しなかったため、工程表の
`M6 — Persistence Guards and Operational Polish`、overlay設計書、M5結果文書の
申し送りを実装条件として使用した。

## 2. 開始条件

- 実行日: 2026-07-26
- branch: `codex/rfq1-multi-source-phase1-docs`
- HEAD: `60be6a4`
- HEAD subject: `Support source-aware cursor readouts, interval stats, and FFT`
- 開始時worktree: clean
- M5までcommit済み
- ユーザー確認済みM5 GUI smoke: 合格

commit、push、PR作成は行っていない。

## 3. 共通Persistence action policy

Plot configとSession presetの操作可否を
`persistence_action_block_reason()` へ集約した。

| Action | one source | multi-source |
|---|---|---|
| Plot config Load | enabled | enabled; workspace replacement notice |
| Plot config Apply to Current Data | enabled | disabled |
| Plot config Save | enabled | disabled |
| Plot config Save As | enabled | disabled |
| Session preset Save | enabled | disabled |
| Session preset Apply | enabled | disabled |

0 sourceの場合、Load以外は「exactly one data source」を要求する。
multi-sourceの場合、Phase 1形式は1sourceしか保持できず、source欠落を防ぐため
無効化したことを理由として返す。

同じpolicyをUIのbutton enabled判定と、保存・適用method先頭のbackend guardで使用する。
UIを経由しない呼び出しでも部分保存できない。

## 4. Plot config version 1 Load

v1 Loadはmulti-sourceでも利用可能とした。configのread、`data_path` validation、
Parquet schema read、time column確認を完了してからworkspaceを置換する。
validation失敗時は現在のworkspaceを変更しない。

成功時は次を行う。

- `WorkspaceDataState` をSource #1だけの新しい状態へ置換
- configの `data_path` と読込済みschemaをSource #1へ設定
- source ID、selection、generation counterを単一source初期状態へ戻す
- 旧sourceのraw data、channel cache、draw cacheを残さない
- config rowの `ChannelKey` をSource #1で構築
- Hover、Cursor、区間統計、FFT stateをclear
- recent Parquet path履歴を重複排除して維持
- statusへ旧source数とSource #1への置換を表示

Load前のworkspaceが2source以上の場合、Plot config欄へ置換warningを常時表示し、
Load buttonのhoverにも同じ内容を表示する。

## 5. Save / Apply guard

multi-source時は次のbuttonを無効化した。

- Plot config `Apply to Current Data`
- Plot config `Save`
- Plot config `Save As...`
- Session preset `Save`
- Session preset `Apply`

disabled buttonのhoverと各sectionのwarningで、Phase 1 persistenceが1sourceのみを
保持するためであることを説明する。各backend methodも共通policyを再確認し、
statusへaction固有のblock reasonを設定する。

## 6. Operational polish確認

M3–M5で導入済みの次の処理がM6要件を満たすことを、既存Unit testとコード経路で確認した。

- source remove後のselected source fallbackとsource番号rebase
- source remove/path change時のRow channel、selected channel、Hover cleanup
- Cursor、区間統計cache、FFT target/resultのsource別cleanup
- 残存sourceのdisplay range unionへのX range再評価
- source generationによるstale load / FFT result拒否
- load status / progressの `#1` / `#2` 表示とremove後の再番号付け
- `WorkspaceDataState::cache_stats()` による全source Perf集計

## 7. README

`code/rust_phase1/README.md` にPhase 1 Persistence Constraintsを追加した。
multi-source Save / Apply guardと、v1 Loadがworkspaceを単一Source #1へ置換する制約を
明記した。

## 8. 追加テスト

M6で6件を追加した。

1. `persistence_policy_allows_v1_load_but_blocks_partial_multi_source_actions`
2. `zero_source_persistence_policy_requires_one_source_except_for_v1_load`
3. `plot_config_v1_replacement_creates_clean_source_one_and_keeps_recent_paths`
4. `multi_source_backend_guards_do_not_capture_or_apply_partial_state`
5. `plot_config_v1_load_replaces_multi_source_workspace_if_fixture_exists`
6. `failed_plot_config_v1_load_preserves_multi_source_workspace`

5件目はmulti-source validation fixtureが存在する環境で、実際にschemaを読み、
2source workspaceからv1 configをLoadしてSource #1だけになることを確認する。
6件目はconfigの `data_path` validation失敗時に、元の2source workspaceが変更されない
ことを確認する。

既存のplot config parse、capture、apply、round-trip関連testも継続して実行する。

## 9. 自動検証

`code/rust_phase1` で次を実行する。

```text
cargo fmt --check
cargo check
cargo test
cargo build --release
git diff --check
```

最終結果:

```text
test result: ok. 164 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo fmt --check`、`cargo check`、`cargo build --release`、`git diff --check`も成功した。
release executableの起動を確認したが、単体実行ファイルがmacOS GUI自動操作環境の
app targetとして列挙されなかったため、clickを伴うGUI smokeはユーザーが実施した。

## 10. M6 GUI smoke checklist

Load確認には次のportableなv1 configを使用する。

```text
code/proto_3_1b/data/multi_source_validation_1_v1.pyc.json
```

同じdirectoryの `multi_source_validation_1.parquet` を相対参照する。

1. Source #1 / Source #2のschemaをLoadし、Plot config `Save`、`Save As...`、
   `Apply to Current Data`が無効である
2. 無効buttonのhoverとPlot config欄に、1source形式による欠落防止理由が表示される
3. 2source状態でもPlot config `Load`は有効で、workspaceをSource #1へ置換するwarningが
   表示される
4. v1 Plot configをLoadし、Source #2が消えてconfigの `data_path` がSource #1として
   Loadされる
5. 2source状態でSession preset `Save` / `Apply`が無効で、理由が表示される
6. Source #2削除後、単一sourceのPlot config Save / Save As / ApplyとSession presetが
   再度利用できる
7. 単一sourceでPlot config Save / Load round tripを行い、Row、channel style、
   theme、scroll zoomが回帰していない

### 確認結果

2026-08-21にユーザーが主要GUI smokeを実施し、次を確認した。

- 2source時のPlot config / Session preset各buttonの有効・無効状態: 合格
- v1 Plot config Load後にworkspaceがSource #1だけになること: 合格
- 単一source Plot configの既存動作: 自動回帰testで合格

## 11. 変更ファイル

- `code/proto_3_1b/data/multi_source_validation_1_v1.pyc.json`
- `code/rust_phase1/src/main.rs`
- `code/rust_phase1/README.md`
- `docs/rust_rfq_1_multi_source_phase1_m6_results.md`

Cargo依存関係、fixture generator、Parquet生成物は変更していない。

## 12. M6完了判定

実装、164件の自動test、release build、ユーザーによる主要GUI smokeがすべて成功した。
M6を `complete` と判定する。
