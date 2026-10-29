---
project: Panel_y
doc_type: M0 Execution Results
rfq: "RFQ #1"
phase: "Multi-source Phase 1 / M0"
created: "2026-07-25"
updated: "2026-07-25"
status: complete
source_plan: "docs/rust_rfq_1_multi_source_phase1_work_plan.md"
implementation: "code/rust_phase1"
---

# RFQ #1 — Multi-source Phase 1 / M0 実行結果

## 1. 目的とスコープ

M0では、Phase 1の採用・延期境界を工程表どおりに固定し、Rust版の
実装開始前ベースラインと、後続のM3〜M5で使用する2ソース検証fixtureを
確立した。

実施範囲はGit/toolchain確認、既存テスト、fixture generator、Rust CLI検証、
README、手動確認シナリオである。`SourceId`、`ChannelKey`、
`TimeTransform`、データ所有構造、UI、描画、Hover、Cursor、FFT、
Plot configを含むM1以降のアプリ実装には着手していない。

Phase 1で採用する2ソース独立ロード、任意チャンネルoverlay、共通display
time、Line/Step、Hover/Cursor統計、同一ソースFFT制限、および未対応保存の
guardは工程表から変更していない。時間オフセットUI、自動整列、
`Fit Overlap`、matching-channel便利操作、複数ソース永続化、3ソース以上の
UI、ソース別FFT結果overlay、ソース間resamplingは引き続き延期対象である。

## 2. Git baseline

実行日: 2026-07-25

| Item | Measured value |
|---|---|
| Current branch | `codex/rfq1-multi-source-phase1-docs` |
| HEAD | `6a003e35d2133db219caf64dabd2323414f43bd8` |
| `main` | `2ba81459519735003838d61d24d4efa2f8359b38` |
| `origin/main` | `2ba81459519735003838d61d24d4efa2f8359b38` |
| HEAD relation | `main` / `origin/main`を直接の共通基点とする1 commit ahead |
| HEAD subject | `6a003e3 Document RFQ #1 multi-source Phase 1 plan` |
| Initial `git diff --check` | 成功 |

M0開始時のworktreeには、既存の未コミット変更として次が存在した。

```text
 M docs/rust_rfq_1_multi_source_phase1_work_plan.md
?? docs/rust_rfq_1_multi_source_phase1_m0_execution_prompt.md
```

前者はM0 promptへのリンク追加、後者は今回の実行指示である。いずれも
既存変更として保持し、内容を変更していない。

## 3. Toolchain

| Tool | Measured value |
|---|---|
| Rust | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| Cargo | `cargo 1.96.0 (30a34c682 2026-05-25)` |
| Fixture Python | Python `3.14.6` via isolated `uv run` environment |
| NumPy | `2.5.1` |
| PyArrow | `25.0.0` |

リポジトリ内に既存venvはなく、`/usr/bin/python3` とpyenvの `python` は
どちらもNumPyを持っていなかった。このため、repositoryのrequirementsを
変更せず、`/tmp/panel-y-uv-cache` を使用する一時的なisolated環境で
generatorを実行した。

## 4. `cargo test` baseline

`code/rust_phase1` でM0変更前に実行した結果:

```text
test result: ok. 106 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- 実行日: 2026-07-25
- HEAD: `6a003e35d2133db219caf64dabd2323414f43bd8`
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`

開始時blockerはなかった。

M0成果物の編集後にも再実行し、同じく106 passed、failed / ignored /
filtered outはすべて0であることを確認した。

## 5. Fixture仕様

| Item | Source #1 | Source #2 |
|---|---:|---:|
| File | `multi_source_validation_1.parquet` | `multi_source_validation_2.parquet` |
| Time range | `0.0 .. 2.0 s` | `0.5 .. 1.75 s` |
| Sample rate | `50,000 Hz` | `64,000 Hz` |
| Row count | `100,001` | `80,001` |
| Channels | `iu`, `torque`, `gate_pwm` | `iu`, `speed`, `gate_pwm` |
| Time type | `float64` | `float64` |
| Channel type | `float32` | `float32` |

`iu` は50 Hz正弦波で、Source #2は振幅、位相、DC offsetが異なる。
`torque` と `speed` は正のDC offsetを持つ低周波信号である。
`gate_pwm` はSource #1が1 kHz / duty 30%、Source #2が800 Hz /
duty 42%かつ位相差を持ち、どちらもlow=0、high=5である。

timeは各sourceで
`np.arange(row_count, dtype=np.float64) / sample_rate + start_time`
として構築した。generatorは全配列の型、長さ、有限値、timeの厳密な
単調増加、開始・終了時刻をwrite前に検証する。

## 6. 生成結果

実行コマンド:

```bash
UV_CACHE_DIR=/tmp/panel-y-uv-cache uv run --isolated --no-project \
  --with numpy --with pyarrow \
  python code/proto_3_1b/generate_multi_source_validation_data.py --force
```

| Item | Source #1 | Source #2 |
|---|---:|---:|
| Row count | `100,001` | `80,001` |
| Time range | `0.000000 .. 2.000000 s` | `0.500000 .. 1.750000 s` |
| Sample rate | `50,000 Hz` | `64,000 Hz` |
| Channels | `iu`, `torque`, `gate_pwm` | `iu`, `speed`, `gate_pwm` |
| File size | `600,180 bytes` | `367,803 bytes` |
| SHA-256 | `98f0c24c35a96d25283034a08180d810802f13e0e5dca02bcb34b8ad4f449079` | `3b342b589d31ca2b6417f7efbd9515c0fe4f3419acff829acfa8fc694fa82393` |

同じ環境で `--force` 生成を2回実行し、両回でfile sizeとSHA-256が一致した。
`--force` なしの再実行はexit code 2と対象pathを含む明確なエラーで停止し、
既存出力を上書きしなかった。compressionは `zstd` である。
生成後のParquetを再読込し、両方でnull count 0、全列有限値、time厳密単調増加、
compression `ZSTD`を確認した。

## 7. Rust `--schema` 結果

`code/rust_phase1` から両fixtureを確認した。

| Check | Source #1 | Source #2 |
|---|---|---|
| Rows | `100001` | `80001` |
| Row groups | `1` | `1` |
| Columns | `4` | `4` |
| Time detection | `time` | `time` |
| Time physical type | `DOUBLE` | `DOUBLE` |
| Numeric channels | `iu`, `torque`, `gate_pwm` | `iu`, `speed`, `gate_pwm` |
| Channel physical type | all `FLOAT` | all `FLOAT` |

Source #1にだけ `torque`、Source #2にだけ `speed` があり、両方に `iu` と
`gate_pwm` があることを確認した。Rust CLI表示の`DOUBLE`は `f64`、
`FLOAT`は `f32` に対応する。

## 8. Rust `--load-channel` 結果

| Source | Channel | Samples | Time range | Value range | Result |
|---|---|---:|---|---|---|
| #1 | `iu` | `100001` | `0.000000 .. 2.000000` | `-10.000000 .. 10.000000` | success |
| #2 | `iu` | `80001` | `0.500000 .. 1.750000` | `-7.850000 .. 8.650000` | success |
| #1 | `torque` | `100001` | `0.000000 .. 2.000000` | `1.750000 .. 4.250000` | success |
| #2 | `speed` | `80001` | `0.500000 .. 1.750000` | `1075.000000 .. 1325.000000` | success |
| #1 | `gate_pwm` | `100001` | `0.000000 .. 2.000000` | `0.000000 .. 5.000000` | success |
| #2 | `gate_pwm` | `80001` | `0.500000 .. 1.750000` | `0.000000 .. 5.000000` | success |

全6 loadでtime/value length mismatchはなく、要求したprojectionから
Envelopeを生成できた。

## 9. 現行単一ソース回帰の手動確認手順

M0は非GUIのCLI実行として行ったため、次のGUI手動確認は未実施である。
後続のGUI実行環境で次を順に確認する。

- [ ] Rustアプリを起動する。
- [ ] `test_100k.parquet` のschemaをロードする。
- [ ] 複数チャンネルを1行へ追加する。
- [ ] Line / Stepを切り替える。
- [ ] Rowを追加・削除する。
- [ ] Pan / Zoom / Box zoom / Reset X Rangeを実行する。
- [ ] Hover値を確認する。
- [ ] Cursor A/Bと区間統計を確認する。
- [ ] 同一ソース内のFFTを実行する。
- [ ] Plot config version 1のsave/loadを確認する。

## 10. M3〜M5の2ソース手動確認手順

次の手順はM3〜M5実装完了後に実施する。

- [ ] `multi_source_validation_1.parquet` をSource #1としてロードする。
- [ ] `multi_source_validation_2.parquet` をSource #2としてロードする。
- [ ] `iu #1` と `iu #2` を同一行へLineで追加する。
- [ ] `torque #1` と `speed #2` を同一行へ追加する。
- [ ] `gate_pwm #1` と `gate_pwm #2` を同一行へStepで追加する。
- [ ] Fit Allが `0.0 .. 2.0` を使用することを確認する。
- [ ] `0.0 .. 0.5` でSource #2のHover値が表示されないことを確認する。
- [ ] `1.75 .. 2.0` でSource #2のHover値が表示されないことを確認する。
- [ ] 共通区間で両ソースのHover/Cursor値を確認する。
- [ ] Pan/Zoomで両ソースが同じdisplay rangeを使用することを確認する。
- [ ] 異なるソースを選んだFFTが拒否されることを確認する。
- [ ] Source #1または#2を削除し、他方が保持されることを確認する。

## 11. 既知の制約

- M0ではマルチソース対応アプリを実装していないため、2ソースGUI操作は
  現時点では実行できない。
- GUI単一ソース回帰は未実施であり、M1開始時またはGUIを利用できる
  release gateで実施する。
- generatorの実行にはNumPyとPyArrowが必要である。
- 複数ソースPlot config、Session preset、時間オフセット操作など、
  工程表の延期機能は未対応のままである。
- 生成した `.parquet` は `.gitignore` の `*.parquet` によりGit管理外である。

## 12. M1への申し送り

- M1は工程表どおりidentity/state modelに限定し、2ソースUIや描画分岐へ
  先行しない。
- 単一ソース動作を維持しながら `SourceId`、`ChannelKey`、
  `TimeTransform(offset_seconds = 0.0)` とworkspace accessorを導入する。
- fixtureはM4の異なるtime/sample条件とLine/Step、M5の範囲外Hover、
  Cursor、FFT拒否確認に使用する。
- 開始時から存在したwork planとexecution promptの未コミット変更を
  引き続き他者所有として扱う。

## 13. M0完了判定

M0はcompleteと判定する。採用・延期境界、Git/toolchain/test baseline、
決定論的fixture generator、2回のSHA一致、overwrite protection、Rust
schema/load検証、README手順、単一ソースおよび後続2ソースの手動確認
checklistを完了した。アプリ本体とCargo依存関係は変更していない。
