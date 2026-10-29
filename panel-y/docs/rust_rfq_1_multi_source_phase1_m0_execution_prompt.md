---
project: Panel_y
doc_type: Execution Prompt
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M0"
target: Scope lock, validation fixtures, and baseline
created: "2026-07-25"
updated: "2026-07-25"
status: ready to use
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — マルチデータ読み込み対応 Phase 1 / M0 実行指示プロンプト

以下の指示に従い、Panel-y Rust版の「M0 — Scope Lock and Validation Fixtures」だけを実行してください。

---

## 1. 役割

あなたは `/Users/ochiba/newspace/life/factory/panel-y` を対象に作業する実装エージェントです。

今回の目的は、マルチデータ読み込み機能そのものを実装することではありません。Phase 1実装開始前のスコープ固定、現行ベースライン確認、2ソース検証データの再生成手段、後続工程で使う手動確認シナリオを完成させることです。

このターンではM0を完了させてください。調査だけで終了せず、M0の成果物を作成し、生成スクリプトとRust CLIで検証してください。

---

## 2. 最初に読む資料

作業前に次を最後まで読んでください。

1. `AGENTS.md`
2. `docs/rust_rfq_1_multi_source_overlay_design.md`
3. `docs/rust_rfq_1_multi_source_phase1_work_plan.md`
4. `code/rust_phase1/README.md`
5. `code/proto_3_1b/generate_step_validation_data.py`
6. `code/proto_3_1b/generate_ripple_validation_data.py`

`journal/` は参照・編集しないでください。

---

## 3. M0のゴール

次の状態を作ってください。

- Phase 1の採用機能と延期機能が工程表どおりであることを確認できる
- 現行Rust版のテスト結果とGit基準点が記録されている
- 異なる条件を持つ2つのParquet検証データを、決定論的に再生成できる
- 両データのschema、time、同名・異名・StepチャンネルをRust CLIで確認できる
- 後続のM3〜M5で実施するGUI手動確認手順が記録されている
- アプリ本体のマルチソース実装には着手していない

---

## 4. 厳守するスコープ

### 実施すること

- Git状態と基準commitの読取確認
- Rust toolchain情報の記録
- 現行 `cargo test` の実行
- 2ソース検証データ生成スクリプトの追加
- 生成したParquetのschema/channel load確認
- `code/rust_phase1/README.md` への生成・確認手順追加
- M0実行結果文書の作成
- M0成果物に対する整形・差分確認

### 実施しないこと

- `SourceId`、`ChannelKey`、`TimeTransform` の実装
- `DatasetState`、`ViewState`、`RowChannel` の変更
- 2ソースUIの実装
- 描画、Hover、Cursor、FFT、Plot configの変更
- Cargo依存関係の追加
- 既存Parquet生成スクリプトの意味変更
- 既存テストの削除・緩和
- `journal/` の参照・編集
- branch作成、commit、push、PR作成
- 生成した `.parquet` ファイルのforce add

M1以降の変更が必要に見えても、このターンでは実装せず、M0結果文書の「M1への申し送り」へ記録してください。

---

## 5. 作業開始時の確認

リポジトリルートで、少なくとも次を確認してください。

```bash
git status --short --branch
git branch -vv
git rev-parse HEAD
git rev-parse main
git rev-parse origin/main
git log -1 --oneline --decorate
git diff --check
rustc --version
cargo --version
```

ルール:

- 既存の未コミット変更があれば、所有者を推測して上書きしない
- 今回の成果物と重なる変更がある場合は内容を確認し、安全に続行できなければ停止して報告する
- `git fetch`、`git pull`、remote writeは行わない
- `main`、`origin/main`、現在のHEADが異なる場合は、各SHAと関係をM0結果へ記録する

---

## 6. Rustベースライン確認

`code/rust_phase1` で現行テストを実行してください。

```bash
cargo test
```

記録する項目:

- 成功・失敗
- 実行テスト数
- failed / ignored / filtered out
- 実行日
- HEAD commit
- Rust/Cargo version

工程表では既存106テストを期待しています。ただし数を決め打ちせず、実際の出力を正として記録してください。

テストが失敗した場合:

1. M0変更前からのベースライン失敗か確認する
2. M1以降のコード変更で直そうとしない
3. fixture作成と無関係な失敗を隠さない
4. 同じ失敗が再現する場合はM0結果へblockerとして記録する

---

## 7. 2ソース検証データ生成スクリプト

次のファイルを新規作成してください。

```text
code/proto_3_1b/generate_multi_source_validation_data.py
```

### 7.1 実装要件

- Pythonスクリプト単体で2ファイルを生成する
- `numpy` と `pyarrow` を使用する
- `pandas` は必須にしない
- 出力は決定論的にする
- `time` は `float64`
- すべてのチャンネルは `float32`
- null、NaN、Infを生成しない
- timeは単調増加
- Parquet compressionは `zstd`
- `--force` がない場合、既存出力を上書きしない
- 不正な引数は明確なエラーにする
- 生成後に各ファイルについて次を表示する
  - path
  - row count
  - time range
  - sample rate
  - channel list
  - file size
  - SHA-256

### 7.2 出力ファイル

```text
code/proto_3_1b/data/multi_source_validation_1.parquet
code/proto_3_1b/data/multi_source_validation_2.parquet
```

`.gitignore` の `*.parquet` に従い、生成物はGitへ追加しないでください。Git管理対象は再生成スクリプトと文書だけです。

### 7.3 データ仕様

Source #1:

| Item | Value |
|---|---|
| file | `multi_source_validation_1.parquet` |
| time range | `0.0` 秒から `2.0` 秒 |
| sample rate | `50,000 Hz` |
| row count | `100,001` |
| channels | `iu`, `torque`, `gate_pwm` |

Source #2:

| Item | Value |
|---|---|
| file | `multi_source_validation_2.parquet` |
| time range | `0.5` 秒から `1.75` 秒 |
| sample rate | `64,000 Hz` |
| row count | `80,001` |
| channels | `iu`, `speed`, `gate_pwm` |

意図:

- `iu` は同名チャンネル比較用
- `torque` はSource #1だけに存在する異名チャンネル
- `speed` はSource #2だけに存在する異名チャンネル
- `gate_pwm` はStep重ね描き用
- time rangeの和集合は `0.0 .. 2.0`
- time rangeの共通区間は `0.5 .. 1.75`
- sample countとsample rateはソース間で異なる

### 7.4 波形仕様

次の性質を満たす決定論的な式を使用してください。係数の小さな調整は許容しますが、チャンネル名、型、行数、sample rate、time rangeは変更しないでください。

Source #1:

```text
iu
  50 Hzの正弦波
  振幅は約10
  DC offsetは0

torque
  約2 Hzの低周波成分
  正のDC offsetを持つ

gate_pwm
  1 kHz
  duty約30%
  low=0, high=5
```

Source #2:

```text
iu
  50 Hzの正弦波
  Source #1と異なる振幅および位相
  DC offsetは小さな非ゼロ値

speed
  Source #1に存在しない低周波の速度相当信号
  正のDC offsetを持つ

gate_pwm
  Source #1とエッジが完全一致しないPWM
  周波数、duty、または位相の少なくとも1つを変える
  low=0, high=5
```

time値は `np.arange(row_count) / sample_rate + start_time` のように、仕様どおりの開始時刻と行数を明示的に構築してください。

---

## 8. Python環境

次の順で利用可能なPython環境を確認してください。

1. プロジェクト内の既存venv
2. `python3`
3. `python`

必要モジュール確認:

```bash
<python> -c 'import numpy, pyarrow; print(numpy.__version__, pyarrow.__version__)'
```

ルール:

- 新しいvenvや依存関係をリポジトリへコミットしない
- モジュールが不足している場合、ネットワーク取得が必要なら適切な承認を求める
- `requirements` やCargo依存関係をM0の都合だけで変更しない
- 依存不足を理由に未検証の生成スクリプトを完成扱いしない

---

## 9. Fixture生成とRust CLI検証

生成スクリプトを実行してください。

実際に利用可能なPythonコマンドへ置き換えて構いません。

```bash
python3 code/proto_3_1b/generate_multi_source_validation_data.py --force
```

続いて `code/rust_phase1` から両ファイルを検証してください。

```bash
cargo run -- --schema ../proto_3_1b/data/multi_source_validation_1.parquet
cargo run -- --schema ../proto_3_1b/data/multi_source_validation_2.parquet

cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_1.parquet iu
cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_2.parquet iu

cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_1.parquet torque
cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_2.parquet speed

cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_1.parquet gate_pwm
cargo run -- --load-channel ../proto_3_1b/data/multi_source_validation_2.parquet gate_pwm
```

確認すること:

- Source #1は100,001 rows、3 numeric channels
- Source #2は80,001 rows、3 numeric channels
- 両方で `time` が検出される
- 両方で `iu` と `gate_pwm` が検出される
- `torque` はSource #1だけ
- `speed` はSource #2だけ
- `iu`、unique channel、`gate_pwm` をそれぞれロードできる
- time/value length mismatchがない
- 型がtime=`f64`、channel=`f32`としてロードされる

CLI出力と生成スクリプトのmetadataが一致しない場合は、M0完了前に原因を解消してください。

---

## 10. README更新

`code/rust_phase1/README.md` に、既存のvalidation data説明と同じ粒度で次を追加してください。

- generator command
- 出力ファイル2件
- 各ファイルのtime range / sample rate / row count
- チャンネル一覧
- M3〜M5での用途
  - same-name overlay
  - different-name overlay
  - different sample rate/count
  - union/overlap range
  - Step overlay
- Rust CLIによるschema/load確認例
- Parquet生成物はGit管理対象外で、スクリプトから再生成すること

既存READMEの説明を削除・短縮しないでください。

---

## 11. M0実行結果文書

次のファイルを新規作成してください。

```text
docs/rust_rfq_1_multi_source_phase1_m0_results.md
```

front matter:

```yaml
---
project: Panel_y
doc_type: M0 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M0"
created: "<実行日>"
updated: "<実行日>"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
implementation: "code/rust_phase1"
---
```

本文へ最低限記録する内容:

1. M0の目的とスコープ
2. Git baseline
   - current branch
   - HEAD
   - main
   - origin/main
   - worktree状態
3. Toolchain
   - rustc
   - cargo
   - fixture生成に使用したPython
   - numpy / pyarrow version
4. `cargo test` 結果
5. fixture仕様
6. 生成結果
   - row count
   - time range
   - sample rate
   - channel list
   - file size
   - SHA-256
7. Rust `--schema` 結果の要約
8. Rust `--load-channel` 結果の要約
9. 単一ソース回帰の手動確認手順
10. 後続M3〜M5の2ソース手動確認手順
11. 既知の制約
12. M1への申し送り
13. M0完了判定

測定値やSHAを推測で記載しないでください。実際のコマンド結果を使ってください。

---

## 12. 手動確認チェックリスト

M0結果文書に、以下の2種類を分けて記載してください。

### 12.1 現行単一ソース回帰

M0時点のアプリで実施可能な確認手順:

1. Rustアプリを起動する
2. `test_100k.parquet` のschemaをロードする
3. 複数チャンネルを1行へ追加する
4. Line/Stepを切り替える
5. Row追加・削除を行う
6. Pan/Zoom/Box zoom/Reset X Rangeを行う
7. Hover値を確認する
8. Cursor A/Bと区間統計を確認する
9. 同一ソース内のFFTを実行する
10. Plot config version 1のsave/loadを確認する

M0でGUIを実際に操作できない環境の場合は、未実施と明記し、手順だけを成果物として残してください。実施したと偽ってはいけません。

### 12.2 後続2ソース確認

M3〜M5完了後に実施する確認手順:

1. `multi_source_validation_1.parquet` をSource #1としてロードする
2. `multi_source_validation_2.parquet` をSource #2としてロードする
3. `iu #1` と `iu #2` を同一行へLineで追加する
4. `torque #1` と `speed #2` を同一行へ追加する
5. `gate_pwm #1` と `gate_pwm #2` を同一行へStepで追加する
6. Fit Allが `0.0 .. 2.0` を使用することを確認する
7. `0.0 .. 0.5` でSource #2のHover値が表示されないことを確認する
8. `1.75 .. 2.0` でSource #2のHover値が表示されないことを確認する
9. 共通区間で両ソースのHover/Cursor値を確認する
10. Pan/Zoomで両ソースが同じdisplay rangeを使用することを確認する
11. 異なるソースを選んだFFTが拒否されることを確認する
12. Source #1または#2を削除し、他方が保持されることを確認する

---

## 13. 検証と品質確認

編集後に次を実行してください。

```bash
git diff --check
git status --short
```

Python:

```bash
<python> -m py_compile code/proto_3_1b/generate_multi_source_validation_data.py
```

Rust:

```bash
cd code/rust_phase1
cargo test
```

さらに、次を確認してください。

- 変更ファイルがM0成果物だけである
- `.parquet` がGit差分に入っていない
- generatorは `--force` なしで既存出力を保護する
- generatorを2回 `--force` で実行し、SHA-256が一致する
- READMEのコマンドが実際に動く
- M0結果の数値が実出力と一致する
- M1以降のアプリコード変更がない

---

## 14. 想定する変更ファイル

原則として、変更は次の3ファイルに限定してください。

```text
code/proto_3_1b/generate_multi_source_validation_data.py
code/rust_phase1/README.md
docs/rust_rfq_1_multi_source_phase1_m0_results.md
```

追加変更が必要な場合は、M0の目的に不可欠かを説明してください。

次のアプリ実装ファイルは変更しないでください。

```text
code/rust_phase1/src/main.rs
code/rust_phase1/src/fft_analysis.rs
code/rust_phase1/src/parquet_waveform.rs
code/rust_phase1/src/parquet_schema.rs
code/rust_phase1/src/plot_config.rs
code/rust_phase1/Cargo.toml
code/rust_phase1/Cargo.lock
```

---

## 15. 停止条件

次の場合は、勝手にスコープを広げず停止して報告してください。

- M0開始前から `cargo test` が再現性をもって失敗する
- Python依存関係を安全に用意できず、fixtureを実生成・検証できない
- 既存の未コミット変更とM0成果物が競合する
- 指定仕様でParquetを生成すると既存Rust readerが読み込めない
- M0完了にM1以降のアプリコード変更が必要になる
- 生成データ仕様を変更しないと完了できない

停止報告には、実行したコマンド、観測したエラー、変更済みファイル、未完了項目を含めてください。

---

## 16. 完了条件

次をすべて満たした場合だけM0をcompleteとしてください。

- [ ] Phase 1の採用・延期境界を変更していない
- [ ] Git/toolchain baselineを実測して記録した
- [ ] 現行 `cargo test` を実行し結果を記録した
- [ ] 2ソースfixture generatorを追加した
- [ ] 指定どおり異なるtime/sample条件の2ファイルを生成した
- [ ] generatorの決定性をSHA-256で確認した
- [ ] Rust `--schema` で両ファイルを確認した
- [ ] Rust `--load-channel` で同名・異名・Stepチャンネルを確認した
- [ ] READMEへ再生成・検証手順を追加した
- [ ] M0結果文書を実測値で作成した
- [ ] 単一ソース回帰チェックリストを作成した
- [ ] 後続2ソースチェックリストを作成した
- [ ] `.parquet` をGit管理対象にしていない
- [ ] Rustアプリ本体を変更していない
- [ ] `git diff --check` が成功した
- [ ] 最終 `cargo test` が成功した、または開始前からのblockerを明記した

---

## 17. 最終報告フォーマット

最終応答は次の順序で簡潔に報告してください。

1. M0完了 / blocker
2. 作成・変更したファイル
3. Git baseline
4. `cargo test` 結果
5. 生成した2 fixtureのrow count / time range / sample rate / SHA-256
6. Rust schema/load検証結果
7. GUI手動確認の実施・未実施
8. M1への申し送り
9. commit / pushを行っていないこと

成果物へのローカルファイルリンクを付けてください。
