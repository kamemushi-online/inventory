"""Independent NumPy reference for the fixed interval in the native soak run.

Run with the validation Python environment (NumPy and PyArrow required):
  python numeric_reference.py --output numeric-reference.json

Output creation is exclusive. This computes reference values, not a GUI pass.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
DEFAULT_CONFIG = HERE.parent / "2026-10-26-motor-session/saved-motor.pyc.json"
GENERATION = HERE.parent / "2026-10-26-motor-session/generation.json"
CURSORS = (0.04, 0.09)
CHANNELS = ("Ia_phys", "Te", "Trq_cmd", "Hi_a")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def nearest_index(time: np.ndarray, target: float) -> int:
    require(float(time[0]) <= target <= float(time[-1]), "cursor outside time range")
    right = int(np.searchsorted(time, target, side="left"))
    candidates = [index for index in (right - 1, right) if 0 <= index < len(time)]
    # The lower index wins exact ties, as required by the viewer contract.
    return min(candidates, key=lambda index: abs(float(time[index]) - target))


def statistics(values: np.ndarray, first: int, last: int) -> dict:
    selected = values[first : last + 1].astype(np.float64)
    require(bool(np.all(np.isfinite(selected))), "non-finite statistics input")
    a, b = float(selected[0]), float(selected[-1])
    return {
        "sample_count": len(selected),
        "value_a": a,
        "value_b": b,
        "delta": b - a,
        "mean": float(np.mean(selected)),
        "min": float(np.min(selected)),
        "max": float(np.max(selected)),
        "peak_to_peak": float(np.ptp(selected)),
        "rms": float(np.sqrt(np.mean(np.square(selected)))),
        "slope_per_second": (b - a) / (CURSORS[1] - CURSORS[0]),
    }


def fft_reference(time: np.ndarray, values: np.ndarray, first: int, last: int) -> dict:
    selected = values[first : last + 1].astype(np.float64)
    sample_time = time[first : last + 1]
    count = len(selected)
    require(count >= 4, "FFT needs at least four samples")
    require(bool(np.all(np.isfinite(selected))), "non-finite FFT input")
    dt = float(sample_time[1] - sample_time[0])
    dt_error = float(np.max(np.abs(np.diff(sample_time) - dt)))
    require(dt > 0 and dt_error <= max(abs(dt) * 1e-6, 1e-12), "non-uniform time")

    removed_mean = float(np.mean(selected))
    amplitude = np.abs(np.fft.rfft((selected - removed_mean) * np.hanning(count))) * (2.0 / count)
    frequency = np.fft.rfftfreq(count, dt)
    peak_bin = int(np.argmax(amplitude))
    peak_amp = float(amplitude[peak_bin])
    return {
        "channel": "Ia_phys",
        "sample_count": count,
        "dt_seconds": dt,
        "max_dt_deviation_seconds": dt_error,
        "frequency_bin_count": len(frequency),
        "frequency_spacing_hz": float(frequency[1]),
        "removed_mean": removed_mean,
        "peak_bin": peak_bin,
        "peak_frequency_hz": float(frequency[peak_bin]),
        "peak_amplitude": peak_amp,
        "peak_db": float(20.0 * np.log10(max(peak_amp, 1e-12))),
        "first_ten_bins": [
            {"index": index, "frequency_hz": float(frequency[index]), "amplitude": float(amplitude[index])}
            for index in range(min(10, len(frequency)))
        ],
    }


def source_reference(source: dict, generation: dict[str, dict]) -> dict:
    path = Path(source["data_path"]).resolve()
    expected = generation[str(path)]
    actual_hash = sha256(path)
    require(actual_hash == expected["parquet_sha256"], f"input hash differs: {path.name}")
    require(source["time_offset_seconds"] == 0.0, "reference expects zero source offsets")
    table = pq.read_table(path, columns=["time", *CHANNELS])
    time = table["time"].to_numpy().astype(np.float64)
    require(bool(np.all(np.isfinite(time))), "non-finite time")
    require(bool(np.all(np.diff(time) > 0)), "time is not strictly increasing")
    first, last = [nearest_index(time, cursor) for cursor in CURSORS]
    # The application stores channel arrays as float32, including u8 input.
    values = {channel: table[channel].to_numpy().astype(np.float32) for channel in CHANNELS}
    return {
        "name": expected["name"],
        "path": str(path),
        "sha256": actual_hash,
        "source_rows": len(time),
        "source_offset_seconds": 0.0,
        "indices_zero_based_inclusive": [first, last],
        "sample_times_seconds": [float(time[first]), float(time[last])],
        "statistics": {channel: statistics(values[channel], first, last) for channel in CHANNELS},
        "fft": fft_reference(time, values["Ia_phys"], first, last),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
    parser.add_argument("--output", type=Path, default=HERE / "numeric-reference.json")
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    manifest = json.loads(GENERATION.read_text())
    generation = {str(Path(run["parquet"]).resolve()): run for run in manifest["runs"]}
    configured = config["rust"]["sources"]
    require(len(configured) == 3, "reference expects three sources")
    configured_channels = {(item["source_key"], item["path"]) for row in config["rust"]["rows"] for item in row["channels"] if item["visible"]}
    require(configured_channels == {(source["key"], channel) for source in configured for channel in CHANNELS}, "configured channels differ from the twelve-channel workload")
    report = {
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "case": "native-soak-fixed-interval-independent-reference",
        "provenance": manifest["provenance"],
        "config_path": str(args.config.resolve()),
        "config_sha256": sha256(args.config),
        "cursors_seconds": list(CURSORS),
        "statistics_contract": "Inclusive nearest samples (earlier sample wins ties); float32 channel storage promoted to float64 for NumPy reductions; slope uses exact cursor delta, not selected sample delta.",
        "fft_contract": "Default Hanning = symmetric Hann; subtract unweighted mean; NumPy hanning and rfft without padding; one-sided abs(rfft)*2/N including DC/Nyquist; no coherent-gain correction or resampling.",
        "sample_interval_contract": "First consecutive time difference of the inclusive selected interval.",
        "comparison_scope": "Reference only; computing these values does not establish native agreement. Compare statistics and FFT count/peak values with native driver output separately.",
        "suggested_tolerances": {"absolute": 1e-9, "relative": 1e-10, "sample_counts_and_indices": "exact"},
        "script_sha256": sha256(Path(__file__)),
        "viewer_contract_source_sha256": {name: sha256(ROOT / "code/rust_phase1/src" / name) for name in ("main.rs", "fft_analysis.rs", "interval_stats.rs")},
        "environment": {"python": sys.version, "platform": platform.platform(), "numpy": np.__version__, "pyarrow": pa.__version__},
        "sources": [source_reference(source, generation) for source in configured],
    }
    with args.output.open("x") as stream:
        stream.write(json.dumps(report, indent=2, allow_nan=False) + "\n")
    print(json.dumps({"reference_created": str(args.output), "sources": [{"name": source["name"], "samples": source["fft"]["sample_count"], "peak_hz": source["fft"]["peak_frequency_hz"], "peak_amplitude": source["fft"]["peak_amplitude"]} for source in report["sources"]]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
