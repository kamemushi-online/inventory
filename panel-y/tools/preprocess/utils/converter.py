"""Panel_y 前処理ユーティリティ — 公開 API + CLI エントリポイント。

使い方（CLI）:
    python -m utils.converter input.csv output.parquet
    python -m utils.converter input.csv output.parquet --time-col elapsed --time-unit ms
    python -m utils.converter data.mat output.parquet
"""

from pathlib import Path
import pandas as pd

from .reader_csv import read_csv
from .reader_mat import read_mat
from .time_normalizer import detect_time_col, normalize_time
from .column_filter import filter_numeric_columns
from .resampler import check_uniform, resample_uniform


def convert_to_parquet(
    input_path: str | Path,
    output_path: str | Path,
    time_col: str | None = None,
    time_unit: str | None = None,
) -> Path:
    """任意の入力ファイルを Panel_y 標準 Parquet に変換して保存する。

    Args:
        input_path:  入力ファイル（.csv / .mat / .parquet）
        output_path: 出力 Parquet ファイルパス
        time_col:    時間カラム名を明示指定（省略時は自動検出）
        time_unit:   時間単位: "s" | "ms" | "us"（省略時・Noneは秒。自動推定なし）

    Returns:
        出力した Parquet ファイルの Path
    """
    input_path = Path(input_path)
    output_path = Path(output_path)

    # 1. 読み込み
    suffix = input_path.suffix.lower()
    readers = {
        ".csv": read_csv,
        ".mat": read_mat,
        ".parquet": pd.read_parquet,
    }
    reader = readers.get(suffix)
    if reader is None:
        raise ValueError(f"非対応の拡張子: {suffix}")
    df = reader(input_path)
    print(f"[読み込み完了] {input_path.name}  shape={df.shape}")

    # 2. 時間カラム検出
    detected_col = detect_time_col(df, hint=time_col)

    # 3. 指定単位から秒へ正規化（未指定時は秒）
    unit = "s" if time_unit is None else time_unit
    if unit != "s":
        print(f"[時間変換] '{detected_col}' を {unit} → s に変換します")
    df = normalize_time(df, detected_col, unit)

    # 4. 非数値カラム除外
    df = filter_numeric_columns(df, time_col="time")

    # 5. 等間隔チェック・補間
    if not check_uniform(df["time"]):
        print("[警告] 非等時間間隔データを検出。線形補間でリサンプリングします")
        df = resample_uniform(df)

    # 6. 出力
    output_path.parent.mkdir(parents=True, exist_ok=True)
    df.to_parquet(output_path, index=False)
    channels = [c for c in df.columns if c != "time"]
    ts = df["time"].iloc[1] - df["time"].iloc[0]
    fs = 1 / ts if ts > 0 else 0
    print(
        f"[出力完了] {output_path}\n"
        f"  チャンネル: {channels}\n"
        f"  サンプル数: {len(df):,} 点\n"
        f"  サンプリング: {fs:,.1f} Hz (Ts={ts*1000:.4f} ms)"
    )
    return output_path


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description="Panel_y 前処理ユーティリティ")
    parser.add_argument("input", help="入力ファイル (.csv / .mat / .parquet)")
    parser.add_argument("output", help="出力 Parquet ファイルパス")
    parser.add_argument(
        "--time-col", default=None, help="時間カラム名を明示指定"
    )
    parser.add_argument(
        "--time-unit",
        choices=["s", "ms", "us"],
        default="s",
        help="時間単位（既定: s）。自動判定は行わない。",
    )
    args = parser.parse_args()

    convert_to_parquet(
        input_path=args.input,
        output_path=args.output,
        time_col=args.time_col,
        time_unit=args.time_unit,
    )


if __name__ == "__main__":
    main()
