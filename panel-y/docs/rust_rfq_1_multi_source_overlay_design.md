---
project: Panel_y
doc_type: RFQ Design Spec
rfq: "RFQ #1"
target: Rust multi-source waveform overlay
created: "2026-07-25"
updated: "2026-10-24"
status: approved and closed
implementation: "code/rust_phase1"
---

# RFQ #1 — 複数時系列データ重ね描き設計書

---

## 1. 文書の目的

本書は、Panel-y Rust 版に複数の Parquet 時系列データをロードし、データソースをまたいだチャンネルを同一プロット行へ重ね描きする機能の設計を定義する。

RFQ #1 では、次の要求と設計判断を合意済みとする。

- 2つの時系列データを同時にロードできる
- 同名チャンネルだけに限定せず、任意のソースの任意チャンネルを同一プロット行へ追加できる
- 同名チャンネル比較は、汎用的な重ね描き機能の上に便利操作として提供する
- すべてのチャンネル表示名に、重複の有無にかかわらずソース番号 `#1` / `#2` を付ける
- 内部のチャンネル名や Parquet のチャンネルパスは変更しない
- 時間オフセット操作UIはベース機能完成後に追加する
- 時間オフセットを後付けできるよう、データモデルと時刻変換境界は初期実装から用意する

本書の承認をもって RFQ #1の設計合意はクローズとする。`approved and closed` は設計の
承認状態を示し、全機能の実装完了を意味しない。

本書の設計範囲・初期実装手順は2026-07-25の合意内容である。2026-10-24時点ではPhase 1
M0–M7、Phase 2の時間合わせ、Fit Overlap、Plot config v2、複数ソースSession presetの
実装・検証を完了している。ソース別FFT結果の重ね表示も実装・自動テスト・GUI確認を完了した。
3ソース以上のUIも実装・自動テスト208件・fmt/build/diff・GUI全10項目の確認を完了した。現在の進捗は
[Phase 1工程表](./rust_rfq_1_multi_source_phase1_work_plan.md)、時間合わせの検証状況は
[Phase 2実行結果](./rust_rfq_1_multi_source_phase2_time_alignment_results.md)を参照する。

---

## 2. 対象範囲

### 2.1 In scope

- 2つの Parquet データソースの登録、スキーマ読込、チャンネル読込
- データソースごとの独立した時刻配列
- データソースごとの独立したチャンネルキャッシュ
- ソースをまたぐチャンネルの同一プロット行への追加
- 同一 `channel_path` を持つチャンネルの共存
- 異なるチャンネル名を持つチャンネルの共存
- 全チャンネル表示での `#1` / `#2` サフィックス
- ソース別の非同期ロードと stale result の排除
- 複数ソースを考慮したXレンジ、Envelope、Line、Step、Hover、Cursor、区間統計
- 複数ソース状態のセッションプリセットおよび Plot config 保存方針
- 将来の時間オフセットに備えた内部モデルと変換API
- 既存単一ソース Plot config の読込互換

### 2.2 Initial UI scope

Phase 1の初期UIでは同時に扱うデータソースを2つまでとした。これは当時の範囲であり、
現在はソース数上限のないUIへ拡張済みである（7節参照）。

ただし、内部モデルには `source_1` / `source_2` の固定フィールドを設けず、Nソースへ拡張可能なコレクションとして実装する。将来ソース数を増やす際に、チャンネル識別子、ロード処理、描画処理を再設計しないことを目的とする。

### 2.3 Out of scope

- 時間オフセットを編集するUI
- トリガーエッジの自動検出による位置合わせ
- 相互相関による自動時間合わせ
- 表示用の暗黙的なリサンプリング
- 異なるソースのチャンネルを1回のFFT入力へ混在させる処理
- 異なるサンプルレート間のクロススペクトル、コヒーレンス解析
- 3ソース以上を登録する初期UI
- ソース間での生データキャッシュ共有

---

## 3. 設計時点の実装（2026-07-25）

対象実装は `code/rust_phase1` とする。

設計時点のコードは、単一データソース内の複数チャンネル表示について、次の機能を持つ。

- `PlotRow` が複数の `RowChannel` を保持する
- 同一行の複数トレースを重ね描きする
- 表示中トレースを統合してAuto Yレンジを求める
- Line / Step をチャンネル単位で選択する
- チャンネル単位の色、線幅、表示・非表示を管理する
- 同一チャンネルを複数行で参照しても、生データを1コピーだけ保持する
- 表示範囲に応じたEnvelopeとStep edge cacheを使用する
- Hover、Cursor、区間統計、FFTが生データへアクセスする

複数ソース化の主な制約は、次の単一データソース前提にある。

```text
PanelYApp
  DatasetState
    parquet_path
    schema
    shared_time
    loaded_channels: ChannelStore

  ViewState
    selected_channel: String
    rows: Vec<PlotRow>

  RowChannel
    channel_path: String
```

現行 `ChannelStore` は `channel_path: String` をキーにするため、異なるファイルに同じ `channel_path` が存在すると衝突する。また、描画・Hover・Cursor・FFTは1つの `shared_time` を全チャンネルで共有する前提になっている。

---

## 4. 設計原則

### 4.1 チャンネルの同一性と表示名を分離する

チャンネルの内部識別子は次の組とする。

```text
(source_id, channel_path)
```

`#1` / `#2` は表示上のソース番号であり、内部識別子、Parquet スキーマ、チャンネルパス、キャッシュキーへ混入させない。

### 4.2 PlotRow はデータを所有しない

`PlotRow` / `RowChannel` は表示構成だけを保持する。生の時刻配列とチャンネル値は各データソースが所有する。

### 4.3 時刻配列はソース単位で共有する

同一ソース内では従来どおり、複数チャンネルが1つの時刻配列を共有する。異なるソース間では、サンプル数、サンプルレート、開始時刻が同一であることを仮定しない。

### 4.4 表示のためにリサンプリングしない

各ソースは自身の時刻配列を使って、現在の表示範囲に対応するEnvelopeまたはStep traceを生成する。描画時に共通の画面X座標へ写像する。

### 4.5 時間オフセットは変換として扱う

時間オフセットは時刻配列そのものを書き換える処理ではなく、raw time と display time の変換として扱う。

```text
display_time = raw_time + offset_seconds
raw_time     = display_time - offset_seconds
```

初期値は全ソース `0.0` 秒とし、初期UIからは編集できなくてよい。

---

## 5. 状態モデル

### 5.1 識別子

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SourceId(u64);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ChannelKey {
    source_id: SourceId,
    channel_path: String,
}
```

`SourceId` は実行中の内部識別子である。画面上の `#1` / `#2` やファイルパスを内部IDとして使用しない。

理由:

- ソースの並び替えや削除で画面番号が変わっても、ロードジョブとキャッシュを安全に識別できる
- 同じファイルを複数回登録する場合にも区別できる
- ファイルパスの編集途中に識別子が変わらない

### 5.2 TimeTransform

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
struct TimeTransform {
    offset_seconds: f64,
}

impl TimeTransform {
    fn to_display(self, raw_time: f64) -> f64;
    fn to_raw(self, display_time: f64) -> f64;
    fn raw_range(self, display_range: (f64, f64)) -> (f64, f64);
    fn display_range(self, raw_range: (f64, f64)) -> (f64, f64);
}
```

初期実装でも、時刻変換を直接加減算せず、このAPIを通す。

### 5.3 DataSourceState

```rust
struct DataSourceState {
    id: SourceId,
    label: String,
    parquet_path: String,
    schema: Option<SchemaSummary>,
    shared_time: Option<TimeData>,
    loaded_channels: ChannelStore,
    time_transform: TimeTransform,
    generation: u64,
}
```

各フィールドの責務:

| Field | Responsibility |
|---|---|
| `id` | 実行中の安定したソース識別 |
| `label` | ファイル名などの補助表示 |
| `parquet_path` | 対象Parquetファイル |
| `schema` | ソース固有のスキーマ |
| `shared_time` | ソース内で共有するraw time |
| `loaded_channels` | ソース内チャンネルと描画キャッシュ |
| `time_transform` | raw time と display time の変換 |
| `generation` | パス変更・再読込後のstale result排除 |

### 5.4 WorkspaceDataState

現行 `DatasetState` は、複数ソースを所有するワークスペース状態へ置き換える。

```rust
struct WorkspaceDataState {
    sources: BTreeMap<SourceId, DataSourceState>,
    source_order: Vec<SourceId>,
    selected_source_id: Option<SourceId>,
    next_source_id: u64,
    recent_parquet_paths: Vec<String>,
}
```

`source_order` の並びを画面上の `#1`, `#2`, ... に対応させる。

画面番号は表示上の番号であり、`SourceId` とは別物である。初期UIでは並び替え操作を提供しない。ソースを削除した場合は残ったソースを先頭から再採番し、画面内のすべてのラベルを同時に更新する。

### 5.5 ViewState

```rust
struct ViewState {
    selected_channel: Option<ChannelKey>,
    selected_row_id: Option<u64>,
    x_range: Option<(f64, f64)>, // display time
    hover_x: Option<f64>,        // display time
    rows: Vec<PlotRow>,
    // existing fields...
}
```

`ViewState` が保持する時刻はすべて display time とする。

### 5.6 RowChannel

```rust
struct RowChannel {
    key: ChannelKey,
    color_index: usize,
    draw_mode: DrawMode,
    visible: bool,
    color_override: Option<egui::Color32>,
    line_width: f32,
}
```

同一行に同じ `ChannelKey` を2回追加することは禁止する。

次の2つは別のチャンネルとして同一行へ追加できる。

```text
(source #1, "iu")
(source #2, "iu")
```

---

## 6. チャンネル表示名

### 6.1 基本ルール

重複の有無や、現在表示されているソース数にかかわらず、チャンネル表示名には常にソース番号を付ける。

```text
iu #1
iu #2
torque #1
speed #2
```

単一ソースだけがロードされている場合も `iu #1` と表示する。

### 6.2 内部名を変更しない

次の値は元の値を維持する。

```text
channel_name = "iu"
channel_path = "iu"
```

`iu #1` は表示時に生成する。

### 6.3 統一表示API

行設定、軸タイトル、Hover、Cursor統計、FFT、キャッシュ一覧、ロード状態で異なる命名処理を持たないよう、表示情報を一箇所で生成する。

```rust
struct ChannelPresentation {
    short_label: String, // "iu #1"
    source_label: String,
    tooltip: String,     // "run_A.parquet / iu"
}

fn channel_presentation(
    workspace: &WorkspaceDataState,
    key: &ChannelKey,
) -> Option<ChannelPresentation>;
```

### 6.4 同名チャンネル検索

「同名チャンネルを全ソースから追加」操作では次の順序で照合する。

1. 完全な `channel_path` 一致
2. `channel_path` が一致しない場合、各ソース内で一意な `display_name` 一致
3. 1ソース内に複数候補がある場合は自動選択しない

比較はソース番号サフィックスを除いた元の名前・パスで行う。

---

## 7. データソースUI

左パネルに `Data Sources` セクションを追加する。

Phase 1当初のUI案:

```text
Data Sources
  #1  run_A.parquet  [Browse] [Load Schema] [Remove]
      schema: ready, time: ready, channels: 3 loaded

  #2  run_B.parquet  [Browse] [Load Schema] [Remove]
      schema: ready, time: ready, channels: 2 loaded

  [Add Source]
```

Phase 1当初のルール（ソース数上限は後続実装で解除）:

- 初期UIでは最大2ソース
- ソースごとにPath、Browse、Load Schema、状態を表示する
- 一方のスキーマ読込失敗で、もう一方のソースをクリアしない
- ソースのPath変更は対象ソースだけをリセットする
- ソース削除時は、そのソースを参照する `RowChannel` をすべて削除する
- ソース削除時はCursor統計、FFT結果、関連キャッシュを無効化する
- アクティブなロードジョブがあるソースの削除・パス変更は一時的に無効化する

### 現在の3ソース以上のUI

2026-10-24に実装し、208件の自動テストとfmt/build/diffチェックが成功した。
GUI全10項目が合格し、画面証跡11枚を保存した。3ソースFFTでは各ソースのnative binsとdtを保持した重ね表示・Hoverを確認した。
固定のソース数上限を撤廃し、`Add Source` を3ソース以上でも使用できる。
カード一覧は高さを制限した縦スクロール領域とし、`Add Source` と
`Start all at zero` はその上に置く。ソースが増えてもチャンネルブラウザへアクセスできる。

カードと各コントロールのwidget IDは表示順ではなく安定した `SourceId` を使う。
途中のソースを削除すると表示番号は詰めるが、残ったソースのID・チャンネル参照は保持する。
登録ソース数とFFT対象数の制限は別であり、FFTは全ソース合計で最大3対象を維持する。
検証記録は[3ソース以上のUI実行結果](./rust_rfq_1_multi_source_ui_results.md)を参照する。

### 7.1 Channel browser

チャンネルブラウザはソース別にグループ表示する。

```text
#1 run_A.parquet
  iu #1
  iv #1
  torque #1

#2 run_B.parquet
  iu #2
  speed #2
```

検索フィルタは次を対象にする。

- 元の `channel_name`
- 元の `channel_path`
- 表示名 `iu #1`
- ソースラベル、ファイル名

既存の `Loaded / Loading / Unloaded` フィルタは `ChannelKey` 単位で判定する。

### 7.2 Rowへの追加

通常操作:

- 任意の `ChannelKey` を選択
- 選択中の行へ `Load / Add`

便利操作:

- `Add matching channel from all sources`

便利操作は、選択チャンネルと一致するチャンネルを各ソースから検索し、同一行へ追加する。存在しないソースはスキップし、結果をステータスへ表示する。

---

## 8. ロード処理

### 8.1 ソース単位のロード

現行の `read_time_column()` と `read_channel_values()` は再利用する。

各ソースは次を独立して保持する。

- `SchemaSummary`
- `TimeData`
- `ChannelStore`

同一ソースの2本目以降のチャンネル読込では、読み込み済み `shared_time` を再利用する。

### 8.2 ロードジョブ識別子

```rust
struct ActiveLoadJob {
    source_id: SourceId,
    source_generation: u64,
    channel_key: ChannelKey,
    target_row_id: u64,
    // existing fields...
}

struct LoadJobResult {
    job_id: u64,
    source_id: SourceId,
    source_generation: u64,
    channel_path: String,
    result: Result<LoadedChannelData, String>,
}
```

結果適用時に次を確認する。

1. `job_id` がactiveである
2. `source_id` が現在も存在する
3. `source_generation` が現在のソースと一致する
4. `channel_path` が要求と一致する

一致しない結果はstaleとして破棄する。

### 8.3 LoadState

ロード中判定は `channel_path` ではなく `ChannelKey` で行う。

```rust
fn is_channel_loading(&self, key: &ChannelKey) -> bool;
fn loading_channel_keys(&self) -> BTreeSet<ChannelKey>;
```

異なるソースに同じパスがあっても、別ジョブとして同時にロードできる。

---

## 9. キャッシュ設計

`ChannelStore` は各 `DataSourceState` の内部へ配置する。

これにより、既存キャッシュ内部のキーは引き続き `channel_path: String` を使用できる。

```text
Source #1
  shared_time #1
  ChannelStore
    raw_by_channel["iu"]
    envelope_cache["iu", range, buckets]

Source #2
  shared_time #2
  ChannelStore
    raw_by_channel["iu"]
    envelope_cache["iu", range, buckets]
```

利点:

- 既存 `ChannelStore` の変更範囲を抑えられる
- 同名チャンネルのキャッシュが衝突しない
- 一方のソースを再ロードしても、他方のキャッシュを保持できる
- ソース固有の時刻配列とEnvelope planを自然に対応させられる

メモリ方針:

- 時刻配列はソースごとに1コピー
- チャンネル値は `(source_id, channel_path)` ごとに1コピー
- 同じチャンネルを複数行で使用しても追加コピーしない
- ソース間のデータ重複排除は行わない

---

## 10. Xレンジと時刻変換

### 10.1 座標系

次の2つを明確に区別する。

| Coordinate | Usage |
|---|---|
| raw time | Parquet値、sample lookup、Envelope生成、FFT入力 |
| display time | 画面X軸、Pan/Zoom、Hover、Cursor、保存対象の表示範囲 |

### 10.2 Envelope生成

ソースごとの処理:

1. 共通の `view.x_range` をdisplay timeで受け取る
2. `TimeTransform::raw_range()` でソース固有のraw rangeへ変換する
3. ソース固有の `shared_time` と `ChannelStore` からEnvelopeを生成する
4. 描画時にEnvelope bucketのraw timeをdisplay timeへ変換する

生の時刻配列とEnvelope cacheへオフセットを焼き込まない。

### 10.3 描画API

`draw_waveform_traces()` は最初のトレースからX範囲を推測せず、共通のdisplay time rangeを明示的に受け取る。

```rust
fn draw_waveform_traces(
    painter: &egui::Painter,
    rect: egui::Rect,
    display_time_range: (f64, f64),
    visible_traces: &[VisibleTrace],
    y_range: RowYRange,
    // ...
);
```

`VisibleTrace` は少なくとも次を持つ。

```rust
struct VisibleTrace {
    key: ChannelKey,
    display_label: String,
    time_transform: TimeTransform,
    // existing trace/style fields...
}
```

### 10.4 Full range

初期の `Reset X Range` / `Fit All` は、ロード済み全ソースのdisplay time rangeの和集合を使用する。

```text
start = min(all source display starts)
end   = max(all source display ends)
```

`Fit Overlap`（2026-10-24実装）は、time読込済み全ソースの共通区間を使用する。

```text
start = max(all source display starts)
end   = min(all source display ends)
```

共通区間が存在しない場合、端点のみで接する場合、ロード済みtimeの範囲が空・ゼロ幅・
非有限の場合は `Fit Overlap` を無効化する。time未ロードのソースは対象外とし、
1ソースの場合はそのdisplay time全範囲を使う。操作はX範囲と描画cacheだけを更新する。
検証結果は[Fit Overlap実行結果](./rust_rfq_1_multi_source_fit_overlap_results.md)を参照。

### 10.5 最小ズーム幅

ワークスペース全体の最小ズーム幅は、ロード済みソースごとに算出した最小ズーム幅の最小値を採用する。高サンプルレート側のデータを、低サンプルレート側の制約で拡大できなくなることを避ける。

正の幅を持つ共通区間がある場合は、上記の最小ズーム幅と共通区間幅の小さい方を使用する。
これにより `Fit Overlap` がサンプル間隔より狭い共通区間を表示しても、描画・Pan/Zoom処理で
その区間外へ自動的に広がらない。

---

## 11. Line / Step描画

### 11.1 Line

各ソースのraw time上で既存のrange-aware min/max envelopeを生成する。

異なるサンプル数、異なるサンプルレートでも同一行へ描画できる。表示のための補間・リサンプリングは行わない。

### 11.2 Step

Raw Step、change-point cache、dense Step envelopeは各ソース固有の時刻配列を使用する。

`StepEdgeCache` はソース内 `ChannelStore` に保持されるため、同名チャンネル間で衝突しない。

### 11.3 Yレンジ

既存どおり、同一行で表示中の全ソース・全チャンネルの有限値範囲を統合する。

```text
Row Auto Y = union(value ranges of visible traces in the row)
```

Manual Yの仕様は変更しない。

---

## 12. Hover、Cursor、区間統計

### 12.1 Hover

Hover位置はdisplay timeで保持する。

チャンネルごとに次を行う。

1. `raw_hover_time = transform.to_raw(display_hover_time)`
2. 所属ソースの `shared_time` からsampleを検索する
3. Lineはnearest sample、Stepはheld sampleを返す
4. ラベルを `iu #1` の形式で表示する

ソースの表示範囲外では値を表示しない。

### 12.2 Cursor

Cursor A/Bはdisplay timeで保持する。

区間統計では、チャンネルごとにA/Bを所属ソースのraw timeへ変換してsample範囲を求める。

同一のdisplay intervalでも、ソースごとに対象sample数が異なってよい。

### 12.3 統計キャッシュ

統計キャッシュキーへ次を含める。

- `ChannelKey`
- source generationまたはtime fingerprint
- Cursor A/B display time
- 必要な解析条件

`channel_path` だけをキーにしない。

---

## 13. FFT

### 13.1 計算ジョブの境界

FFT計算1ジョブのチャンネルは同一ソースに限定し、1つのraw time配列と `dt` を共有する。
異なるソースを1ジョブへ渡す入力は引き続き拒否する。時系列の連結やソース間補間は行わない。

Phase 1 M5ではUIで異なるソースを同時選択した時点で実行を拒否した。これは初期実装時の
安全境界であり、後続のソース別FFT重ね表示ではRun側でソースごとに独立ジョブへ分割する。
1ジョブ1ソースの計算guardは維持する。

### 13.2 ソース別結果の重ね表示

2026-10-24に実装し、205件の自動テスト、fmt/build/diffチェック、GUI全10項目が成功した。結果は
[FFT overlay実行結果](./rust_rfq_1_multi_source_fft_overlay_results.md)を参照する。

- FFT対象は全ソース合計で最大3チャンネル。対象一覧・結果・凡例にはソース番号を付ける。
- Runは対象をソース別にまとめ、各ソースのraw timeで独立したFFTジョブを開始する。
- display timeのCursor A/Bから対象ソースのoffsetを引いてraw sample区間を決める。
- 一定offsetは元の `dt` を変更しない。選ばれるsample区間はoffsetに応じて変わる。
- 各ソースのN、`dt`、周波数間隔 `df`、native frequency binsを保持し、表示だけを共通Hz軸と振幅/dB軸に重ねる。周波数binをリサンプリングしない。
- ソース別にN・`dt`・`df`・peakを表示し、各トレースを色付き凡例とソースラベルで識別する。
- Hoverはソースごとの近傍binを参照する。そのソースの周波数範囲外では欠損とし、Nyquist外の値を端点で補わない。
- 1ソースの失敗はソース別エラーとして残し、他ソースの成功結果は表示する。
- Cursor、offset、FFT window、対象変更では結果全体を無効化する。Source削除・path変更もCursorを消去するため結果全体を無効化する。
- Cancelとrequest identityで古いジョブ結果の再表示を防ぐ。rawデータ、time、generationなどソース入力の変更後も古い結果を適用しない。

---

## 14. Plot configとSession preset

### 14.1 後方互換

既存 `.pyc.json` version 1 は単一 `data_path` と文字列チャンネルパスを前提とする。

読込時:

- `rust.sources` がなければ、既存 `data_path` をソース `#1` としてロードする
- 既存の行チャンネルはすべてソース `#1` の `ChannelKey` へ変換する
- 既存ファイルを変更なしでロードできること

### 14.2 Rust拡張

2026-10-24に実装・検証済み。詳細は[Plot config v2実行結果](./rust_rfq_1_multi_source_plot_config_v2_results.md)を参照。完全な行構成は各`rust.rows`の`ymin`/`ymax`も含む。

複数ソース状態はtop-level version 1だけでは完全に表現できないため、`rust` namespaceを拡張する。

概念例:

```json
{
  "version": 1,
  "data_path": "/data/run_A.parquet",
  "rows": [
    {
      "channels": ["iu"],
      "step_channels": []
    }
  ],
  "ch_styles": {
    "iu": {
      "width": 1.25
    }
  },
  "theme": "dark",
  "scroll_zoom": false,
  "rust": {
    "version": 2,
    "sources": [
      {
        "key": "source-1",
        "data_path": "/data/run_A.parquet",
        "label": "run_A.parquet",
        "time_offset_seconds": 0.0
      },
      {
        "key": "source-2",
        "data_path": "/data/run_B.parquet",
        "label": "run_B.parquet",
        "time_offset_seconds": 0.0
      }
    ],
    "rows": [
      {
        "channels": [
          {
            "source_key": "source-1",
            "path": "iu",
            "visible": true,
            "draw_mode": "line",
            "line_width": 1.25,
            "color_index": 0
          },
          {
            "source_key": "source-2",
            "path": "iu",
            "visible": true,
            "draw_mode": "line",
            "line_width": 1.25,
            "color_index": 1
          }
        ]
      }
    ]
  }
}
```

ファイル内の `source_key` は永続化用の論理キーであり、実行中の `SourceId(u64)` とは分離する。

### 14.3 Legacy projection

複数ソース設定を保存するときも、top-level `version: 1`、`data_path`、`rows`、`ch_styles` はソース `#1` を表す互換情報として残す。top-level `rows` からはソース `#2` 以降のチャンネルを除外する。

完全な行構成は `rust.version: 2` を正とする。旧実装で開いた場合はソース `#1` の情報だけが利用可能であり、ソース `#2` の完全再現は保証しない。

### 14.4 Time offset

`time_offset_seconds` は省略可能フィールド。時間合わせUIとv2保存・復元は実装済み。

- 省略時: `0.0`
- NaN / Inf: 無効値として `0.0` にフォールバックし警告
- Save/Load: 現在のUIのoffsetを保存・復元
- Apply to Current Data: 現在のoffsetを保持し、保存値との差異を警告

### 14.5 Session preset

2026-10-24に複数ソース対応を実装し、194件の自動テスト・ビルド・GUI全10項目が成功した。
詳細は[Session preset実行結果](./rust_rfq_1_multi_source_session_preset_results.md)を参照。
実行中のメモリ内snapshotとして保持し、アプリ終了で破棄する。ファイルへの永続化は
Plot config v2を使用する。

Session presetは次を保存する。

- ソース順
- ソースごとのパス、表示ラベル、時間オフセット
- 選択中のソース、`selected_channel: Option<ChannelKey>` 相当、選択行
- 行ごとの `ChannelKey` とチャンネル順、空行
- Xレンジ、色、線幅、Line/Step、表示状態、Yレンジ
- 大規模データのpreview設定

Applyは保存時のソース順と構成へ戻す。保存したID・パスが現在のソースと一致しschemaが
存在する場合はrawデータ、time配列、generationを再利用する。削除済みソースやパス変更済みの
ソースはschemaを事前確認して新しいIDを割り当て、行と選択状態の参照を対応させる。
この実行時IDの対応を保持し、繰り返しApplyしても復元済みソースのraw/timeを再利用する。
保存対象以外の現在のソースは除外する。復元不能なソース・チャンネルは警告して部分復元し、
全ソースが復元不能なら現在のworkspaceを保持する。

未読込の表示チャンネルは非同期で読み込み、完了後に保存したXレンジを復元する。
非表示チャンネルは設定だけを戻す。全チャンネルが非表示の場合も保存Xレンジを保持する。
Applyでは描画cacheと解析状態を消去する。
Save / Applyはロード中に無効化する。Saveにはschemaを持つソースが1つ以上必要だが、
Applyは現在のソースが0件でも利用できる。Clearはsnapshotだけを消去する。

---

## 15. エラー処理

エラーは可能な限りソース単位で隔離する。

| Condition | Behavior |
|---|---|
| #2のスキーマ読込失敗 | #1と現在の表示を保持し、#2だけエラー表示 |
| #2のチャンネル読込失敗 | #1の同名チャンネルを保持し、#2だけunloaded/error |
| ソースにtime列がない | そのソースの波形ロードを禁止 |
| sample count mismatch | 対象ソース・対象チャンネルだけ適用しない |
| stale background result | source id/generation確認後に破棄 |
| 同名チャンネルがない | matching addでそのソースをスキップ |
| display time range外 | Hover/Cursor値を欠損表示 |
| ソース間に時間重なりなし | Fit Allは使用可、Fit Overlapは使用不可 |
| Plot config / Session presetのsource欠損 | 復元可能なソース・チャンネルだけ部分復元し警告。全欠損では現在のworkspaceを保持 |

---

## 16. パフォーマンス方針

### 16.1 維持する特性

- Parquetは必要なtime列と選択チャンネルだけをprojection readする
- UIスレッドで大規模ファイル読込を行わない
- 表示幅に応じたEnvelopeを使用する
- 生データは表示行数に比例して複製しない
- Step edge cacheとLine tile cacheを再利用する

### 16.2 複数ソースでの追加コスト

メモリ使用量は、各ソースの時刻配列とロード済みチャンネル数に比例する。2ソース化により同じ構成を2つロードすれば、raw data memoryは概ね2倍になる。

Envelope生成は表示中の `(source_id, channel_path)` ごとに必要となる。ただし同じチャンネルを複数行に置いた場合は、ソース内キャッシュを再利用する。

### 16.3 Perf表示

既存Perf統計は全ソースの値を集計する。

最低限追加する項目:

- source count
- loaded channel count across sources
- raw time memory across sources
- raw channel memory across sources
- per-sourceまたはaggregate envelope cache hit/miss

---

## 17. 実装手順

### Step 1: 識別子と状態モデル

- `SourceId`、`ChannelKey`、`TimeTransform` を追加
- `DataSourceState`、`WorkspaceDataState` を追加
- `RowChannel.channel_path` を `RowChannel.key` へ置換
- `selected_channel` を `Option<ChannelKey>` へ置換
- 単一ソース動作を `source_order[0]` 上で再現

完了条件:

- UIはまだ1ソースでも既存106テスト相当が通る
- すべてのチャンネル参照が `ChannelKey` を使用する

### Step 2: ソース別ロードとキャッシュ

- `ChannelStore` を各 `DataSourceState` 内へ移動
- Load job/resultへsource idとgenerationを追加
- スキーマ、time、channelのロードをソース単位にする
- ソース別エラー隔離を追加

完了条件:

- 同じ `channel_path` を2ソースから同時ロードできる
- 片方の再ロードで他方のraw/cacheが消えない

### Step 3: 2ソースUI

- `Data Sources` UIを追加
- 2つ目のPath/Browse/Load Schemaを追加
- Channel browserをソース別グループ表示
- 全表示名に `#1` / `#2` を付ける
- 任意チャンネルを任意行へ追加できるようにする

完了条件:

- 同名・異名の両方を同一行へ追加できる
- 軸、行設定、Hover、状態表示でソースを判別できる

### Step 4: 複数時刻軸描画

- `view.x_range` をdisplay timeとして扱う
- ソースごとにdisplay rangeからraw rangeへ変換
- `VisibleTrace`へ `ChannelKey` と `TimeTransform` を追加
- 描画関数へ共通display rangeを明示的に渡す
- Line/Step/Auto Y/Reset X Rangeを複数ソース化

完了条件:

- 異なるサンプル数・サンプルレートのデータを重ね描きできる
- Pan/Zoomで全ソースが同期して移動する

### Step 5: Hover、Cursor、統計、FFT

以下は初期Phase 1 M5の記録。後続のFFT重ね表示ではUI選択をソース別ジョブへ分割するが、
1ジョブ内の異なるソース混在は引き続き拒否する（13節）。

- Hover/Cursorをソース固有raw timeへ変換
- 統計キャッシュを `ChannelKey` 対応
- FFT対象を同一ソースに制限
- すべての解析ラベルへソース番号を表示

完了条件:

- 同名チャンネルの値と統計を区別できる
- 異なるソースを混在したFFTを誤って実行しない

### Step 6: 保存・復元

- Session presetを複数ソース対応
- `rust.version: 2` のPlot config拡張を追加
- version 1単一ソース設定の読込互換を維持
- `time_offset_seconds: 0.0` を保存・復元可能にする

完了条件:

- 複数ソース、行構成、同名チャンネルをround tripできる
- 既存version 1設定をソース `#1` としてロードできる

---

## 18. テスト計画

### 18.1 Unit tests

- `ChannelKey` は同じpathでも異なるsourceなら別キーになる
- 同一行で同じ `ChannelKey` の重複追加を拒否する
- 同一行で異なるsourceの同じpathを許可する
- 表示名は単一ソース時も必ず `#1` を含む
- 表示名は2ソース時に `#1` / `#2` を正しく付ける
- ソース削除後の再採番が全表示で一致する
- matching addがpath一致を優先する
- `TimeTransform` のraw/display往復
- オフセット付きdisplay rangeとraw rangeの変換
- 複数ソースrangeの和集合
- 重なりがある場合の共通区間
- 重なりがない場合の共通区間なし判定
- 同名チャンネルのロード状態が衝突しない
- source generation不一致のロード結果を破棄する
- ソース別キャッシュの独立性
- ソース削除時に該当RowChannelだけを削除する
- FFTが異なるソースの混在入力を拒否する
- version 1設定をsource #1へ変換する
- rust version 2設定をround tripする
- offset欠損時に `0.0` を使用する

### 18.2 Integration fixtures

次の2ファイルをテスト用に用意する。

```text
source_1.parquet
  time: 0.0 .. 10.0
  iu
  torque

source_2.parquet
  time: 1.0 .. 8.0
  iu
  speed
  source_1とは異なるsample count / sample rate
```

確認項目:

- `iu #1` と `iu #2` の同時表示
- `torque #1` と `speed #2` の同時表示
- 同期Pan/Zoom
- 片方の範囲外で不正なHover値を表示しない
- LineとStepをソースごとに独立設定
- ソース別Unload/Reload
- 複数ソースPlot configの保存・復元

### 18.3 Regression

現行ベースラインのRustテスト106件を維持する。

単一ソース利用時にも、次を回帰させない。

- Schema read
- on-demand channel load
- multi-row/multi-channel display
- Envelope/Line tile/Step edge cache
- Hover/Cursor/interval statistics
- FFT
- Plot config version 1
- Rock/Plain表示

---

## 19. 受入条件

RFQ #1の実装完了条件は次のとおり。

1. 2つのParquetファイルを独立して登録し、スキーマをロードできる
2. 同名チャンネルを同一行へ追加して同時表示できる
3. 異なる名前のチャンネルもソースをまたいで同一行へ追加できる
4. チャンネル表示名は重複に関係なく常に `#1` / `#2` を持つ
5. 内部のチャンネル名・パスへ `#1` / `#2` を付加しない
6. 同じpathを持つ別ソースのraw dataと描画キャッシュが衝突しない
7. 異なるsample count / sample rateでも表示できる
8. Pan/ZoomのXレンジが全ソースで同期する
9. Hover/Cursor/区間統計が所属ソースの時刻配列を使用する
10. FFTが異なるソースの混在入力を暗黙に処理しない
11. 既存version 1 Plot configを単一ソース設定としてロードできる
12. 複数ソース設定をRust拡張で保存・復元できる
13. `TimeTransform` と `time_offset_seconds` が初期値 `0.0` で組み込まれている
14. 時間オフセットUIを後から追加する際、生時刻データとチャンネル識別モデルの再設計を必要としない
15. 既存テストと追加テストがすべて成功する

---

## 20. 後続RFQ候補

以下は設計合意時に挙げた後続候補の記録。時間offset編集、先頭時刻の0秒合わせ、offset微調整は
Phase 2で実装済み。ソース別FFT重ね表示も実装・自動テスト・GUI確認完了。3ソース以上のUIは
実装・自動テスト208件・GUI全10項目確認完了。現状は工程表と7・13節を参照する。
それ以外の候補は未実装である。

- 時間オフセット編集UI
- ソース先頭時刻を0秒に合わせる操作
- Cursorまたはトリガー位置を基準にした整列
- オフセットの微調整操作
- 3ソース以上のUI
- ソース順の並び替え
- ソース別FFT結果の重ね表示

これらはRFQ #1のベース実装完了後、独立した要求として扱う。

---

## 21. RFQ #1 決定事項

| Item | Decision |
|---|---|
| 内部ソース数 | Nソース対応可能なモデル |
| 初期UIソース数 | Phase 1は最大2。現在は上限なし（7節） |
| 行への追加 | 任意ソース・任意チャンネル |
| 同名比較 | 汎用追加機能上の便利操作 |
| チャンネル内部ID | `(SourceId, channel_path)` |
| 表示名 | 常時 `channel_name #n` |
| 時刻配列 | ソースごとに共有 |
| 表示時リサンプリング | 行わない |
| Xレンジ | 共通display time |
| 初期リセット範囲 | 全ソース範囲の和集合 |
| 時間オフセット値 | 初期値 `0.0` でモデルへ含める |
| 時間オフセットUI | 後続 |
| FFT | 1ジョブ1ソース。複数ソース選択は独立ジョブへ分割し結果表示を重ねる |
| Plot config | version 1互換 + Rust extension version 2 |
| RFQ status | Approved / Closed |

---

## 22. 関連資料

- [RFQ #1 — マルチデータ読み込み対応 Phase 1 実装工程表](./rust_rfq_1_multi_source_phase1_work_plan.md)
- [Rust Migration Phase 2 Design](./rust_migration_phase2_design.md)
- [Rust Migration Phase 3 — Plot Config Detailed Design](./rust_migration_phase3_plotconfig_design.md)
- [Native viewer README](../apps/panel-y/README.md)
- [Parquet Data Specification](./parquet_data_spec.md)
