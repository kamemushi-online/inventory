@echo off
REM 単一変換と一括変換をWindows用exeにし、検証済みZIPを作る
REM 使い方: build_windows.bat
REM
REM 出力: dist\panel-y-converters-windows-x64.zip (x64 Pythonの場合)
REM macOS 用は build_macos.sh を参照。

setlocal enabledelayedexpansion

cd /d "%~dp0"

python -m pip install -r ../requirements-build.txt -q
if errorlevel 1 goto :error

python package_converters.py
if errorlevel 1 goto :error

goto :eof

:error
echo.
echo [エラー] ビルドに失敗しました
exit /b 1
