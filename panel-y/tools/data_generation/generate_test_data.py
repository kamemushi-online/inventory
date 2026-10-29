"""データ間引きテスト用の大規模波形データを生成する。

100kHz サンプリング × 1秒 = 100,000点
チャンネル:
  - sine_50Hz: 50Hz正弦波（アナログ信号の代表）
  - pwm_1kHz: 1kHz PWM波（デジタル信号、step表示向き）
  - noise: ホワイトノイズ（ランダム、ピーク保持のテスト）
  - chirp: チャープ信号（1Hz→500Hz掃引、ズーム時の解像度テスト）
"""

import argparse
from pathlib import Path

import numpy as np
import pandas as pd
from scipy import signal

from _paths import DEFAULT_DATA_DIR as DATA_DIR

N = 100_000
FS = 100_000  # 100kHz
T = N / FS    # 1秒


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=DATA_DIR)
    parser.add_argument("--force", action="store_true", help="Overwrite existing output")
    args = parser.parse_args()
    out = args.output_dir / "test_100k.parquet"
    if out.exists() and not args.force:
        parser.error(f"{out} already exists; pass --force to overwrite")

    t = np.linspace(0, T, N, endpoint=False)

    # 50Hz 正弦波 (振幅 ±10)
    sine_50 = 10 * np.sin(2 * np.pi * 50 * t)

    # 1kHz PWM (duty 30%)
    pwm_1k = (signal.square(2 * np.pi * 1000 * t, duty=0.3) + 1) / 2 * 5  # 0-5V

    # ホワイトノイズ (±1)
    noise = np.random.default_rng(42).normal(0, 1, N)

    # チャープ信号 (1Hz→500Hz, 振幅 ±5)
    chirp = 5 * signal.chirp(t, f0=1, f1=500, t1=T, method="linear")

    df = pd.DataFrame({
        "time": t,
        "sine_50Hz": sine_50,
        "pwm_1kHz": pwm_1k,
        "noise": noise,
        "chirp_1_500Hz": chirp,
    })

    out.parent.mkdir(parents=True, exist_ok=True)
    df.to_parquet(out, index=False)
    print(f"生成完了: {out} ({len(df):,}点, {len(df.columns)-1}ch)")


if __name__ == "__main__":
    main()
