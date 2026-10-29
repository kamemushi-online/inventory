# Panel-y converters

Pythonのインストールなしで使える、コマンドライン用の変換ツールです。
ZIPを展開し、そのフォルダでターミナル（WindowsはPowerShell）を開いてください。
Windows x64、macOS Apple Silicon（arm64）、macOS Intel（x64）は別の配布物です。
自分のOS・CPUに合うものを使用してください。

## Windows

```powershell
# 単一ファイル: CSV / MAT / Parquet
.\panel-y-convert.exe input.csv output.parquet --time-unit ms
.\panel-y-convert.exe input.mat output.parquet --time-unit s

# 時間列名の指定
.\panel-y-convert.exe input.csv output.parquet --time-col elapsed --time-unit us

# フォルダ直下のMATを一括変換
.\multiconverter.exe --input .\input --output .\output --time-unit s
```

## macOS

```sh
# 単一ファイル: CSV / MAT / Parquet
./panel-y-convert input.csv output.parquet --time-unit ms
./panel-y-convert input.mat output.parquet --time-unit s

# 時間列名の指定
./panel-y-convert input.csv output.parquet --time-col elapsed --time-unit us

# フォルダ直下のMATを一括変換
./multiconverter --input ./input --output ./output --time-unit s
```

## 変換時の注意点

- 時間単位の未指定時は秒です。値から単位を推定しません。
- `--time-unit ms`は1000で、`us`は100万で割って秒に変換します。
- 不等間隔データは、元と同じ点数の等間隔時刻へ全チャンネルを線形補間します。
  PWMやピーク値が変わる場合があります。
- 単一ファイル変換は出力を上書きします。一括変換は既存出力をスキップし、
  `--force`を付けた場合だけ上書きします。
- 一括変換はフォルダ直下の`.mat`が対象です。CSVやサブフォルダは対象外です。
- 入力は有限・昇順・重複なしの数値時刻と、長さが揃った数値の波形列を想定しています。
  現行版には不正な時刻やMATの配列長不一致を十分に検出しない制約があります。
- MATの構造体・cell等を含む任意のMATLABデータへの対応は保証していません。
- `--help`でオプションを確認できます。空白を含むパスは引用符で囲んでください。

## 配布物の情報

`build-info.json`にビルド環境、ソースと実行ファイルのSHA-256を記録しています。
`tests.txt`は実行ファイルに対して実施した変換テストの結果です。
`LICENSES`にはPythonとビルド環境の依存パッケージが提供するライセンス文書を同梱します。
テストは外部PythonをPATHから外して実行しますが、すべての利用者PCでの動作を
保証するものではありません。

このビルドには配布者のコード署名・Apple公証を付けていません。
ダウンロードしたファイルはOSの保護機能で実行が制限される場合があります。
組織の配布ルールに従って利用してください。
