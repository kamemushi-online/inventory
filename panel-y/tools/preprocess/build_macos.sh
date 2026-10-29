#!/bin/bash
# 単一変換と一括変換をmacOS用実行ファイルにし、検証済みZIPを作る
# 使い方: bash build_macos.sh
#
# 出力: dist/panel-y-converters-macos-<CPU>.zip
# Windows .exe は別途 Windows-PC 上でビルドする想定。

set -euo pipefail

cd "$(dirname "$0")"

python -m pip install -r ../requirements-build.txt -q

python package_converters.py
