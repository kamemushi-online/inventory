"""Independent NumPy FFT reference for the post-cycle native GUI observation.

This keeps the earlier interval-statistics observation untouched. It compares
only the GUI's reported sample count and rounded peak frequency/amplitude.
"""

from __future__ import annotations

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
CURSORS = (0.040485555, 0.089966673)
OBSERVED = {
    "input": {"samples": 49_482, "peak_hz": "40.419", "peak_amplitude": "4.849870"},
    "reversal": {"samples": 49_482, "peak_hz": "20.209", "peak_amplitude": "4.625901"},
    "speed": {"samples": 49_482, "peak_hz": "60.628", "peak_amplitude": "5.327021"},
}


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
    require(time[0] <= target <= time[-1], "cursor outside source time range")
    i = int(np.searchsorted(time, target, side="left"))
    candidates = [j for j in (i - 1, i) if 0 <= j < len(time)]
    return min(candidates, key=lambda j: abs(float(time[j]) - target))


def reference(run: dict) -> dict:
    path = Path(run["parquet"])
    actual_hash = sha256(path)
    require(actual_hash == run["parquet_sha256"], "Parquet differs from generation manifest")
    table = pq.read_table(path, columns=["time", "Ia_phys"])
    time = table["time"].to_numpy()
    values = table["Ia_phys"].to_numpy()
    first, last = (nearest_index(time, value) for value in CURSORS)
    segment = values[first : last + 1].astype(np.float64)
    sample_time = time[first : last + 1]
    count = len(segment)
    require(bool(np.all(np.isfinite(segment))), "non-finite input values")
    dt = float(sample_time[1] - sample_time[0])
    dt_error = float(np.max(np.abs(np.diff(sample_time) - dt)))
    require(dt > 0 and dt_error <= max(abs(dt) * 1e-6, 1e-12), "non-uniform sample time")

    # Current viewer contract: remove the unweighted mean, symmetric Hanning
    # window (denominator N-1), N-point FFT, one-sided bins with 2/N everywhere.
    # There is no coherent-gain correction, resampling or zero padding.
    removed_mean = float(np.mean(segment))
    window = np.hanning(count)
    spectrum = np.fft.rfft((segment - removed_mean) * window)
    amplitude = np.abs(spectrum) * (2.0 / count)
    frequency = np.fft.rfftfreq(count, dt)
    peak_bin = int(np.argmax(amplitude))
    peak_hz = float(frequency[peak_bin])
    peak_amplitude = float(amplitude[peak_bin])
    observed = OBSERVED[run["name"]]
    comparisons = {
        "sample_count": count == observed["samples"],
        "peak_frequency_three_decimals": f"{peak_hz:.3f}" == observed["peak_hz"],
        "peak_amplitude_six_decimals": f"{peak_amplitude:.6f}" == observed["peak_amplitude"],
    }
    return {
        "name": run["name"],
        "parquet": str(path),
        "parquet_sha256": actual_hash,
        "channel": "Ia_phys",
        "offset_seconds": 0.0,
        "indices_zero_based_inclusive": [first, last],
        "sample_times_seconds": [float(sample_time[0]), float(sample_time[-1])],
        "sample_count": count,
        "dt_seconds": dt,
        "max_dt_deviation_seconds": dt_error,
        "frequency_spacing_hz": 1.0 / (count * dt),
        "removed_unweighted_mean": removed_mean,
        "peak_bin": peak_bin,
        "peak_frequency_hz": peak_hz,
        "peak_amplitude": peak_amplitude,
        "peak_db": float(20.0 * np.log10(max(peak_amplitude, 1e-12))),
        "expected_formatted_peak": {"frequency_hz": f"{peak_hz:.3f}", "amplitude": f"{peak_amplitude:.6f}"},
        "gui_observed": observed,
        "comparisons": comparisons,
        "passed": all(comparisons.values()),
        "first_ten_bins": [{"index": i, "frequency_hz": float(frequency[i]), "amplitude": float(amplitude[i])} for i in range(min(10, len(amplitude)))],
    }


def main() -> int:
    manifest = json.loads((HERE / "generation.json").read_text())
    report = {
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "case": "post-five-cycle-native-gui-fft",
        "provenance": manifest["provenance"],
        "cursors_seconds_as_displayed": list(CURSORS),
        "window": "Hanning / symmetric Hann: 0.5 - 0.5*cos(2*pi*n/(N-1))",
        "processing": "Parquet float32 values promoted to float64; subtract unweighted mean; apply NumPy hanning; NumPy rfft without padding; amplitudes abs(rfft)*2/N, including DC/Nyquist; no coherent-gain correction or resampling.",
        "sampling_interval": "First consecutive time difference in the inclusive nearest-sample interval, matching the current viewer contract.",
        "comparison_scope": "Sample counts plus GUI-transcribed peak frequencies at 3 decimals and amplitudes at 6 decimals. Full native spectra and physical motor accuracy are not compared.",
        "viewer_contract": "code/rust_phase1/src/fft_analysis.rs: prepare_fft_input, compute_fft, fft_window_value; inspected without changes.",
        "viewer_fft_source_sha256": sha256(ROOT / "code/rust_phase1/src/fft_analysis.rs"),
        "script_sha256": sha256(Path(__file__).resolve()),
        "environment": {"python": sys.version, "platform": platform.platform(), "numpy": np.__version__, "pyarrow": pa.__version__},
        "runs": [],
        "passed": False,
    }
    try:
        report["runs"] = [reference(run) for run in manifest["runs"]]
        report["passed"] = all(run["passed"] for run in report["runs"])
    except Exception as error:
        report["error"] = str(error)
    (HERE / "fft-reference.json").write_text(json.dumps(report, indent=2, allow_nan=False) + "\n")
    print(json.dumps({"passed": report["passed"], "peaks": [{"name": run["name"], **run["expected_formatted_peak"], "samples": run["sample_count"]} for run in report["runs"]], "error": report.get("error")}))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
