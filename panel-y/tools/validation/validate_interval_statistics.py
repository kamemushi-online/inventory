"""Compare single/multi-source interval statistics with an independent NumPy reference.

Run from the repository root with numpy and pyarrow installed:
  python3 tools/validation/validate_interval_statistics.py <benchmark-report.json>
The Parquet path in the report is resolved relative to the current directory.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq


def validate_source(report: dict, cursors: list[float] | None = None) -> dict:
    parquet = pq.ParquetFile(report["file"])
    if parquet.metadata.num_rows != report["samples_per_channel"]:
        raise ValueError("report sample count does not match the Parquet file")
    reported_channels = [item["channel"] for item in report["full_range_statistics"]]
    if (
        not reported_channels
        or len(set(reported_channels)) != len(reported_channels)
        or sorted(reported_channels) != sorted(report["channels"])
    ):
        raise ValueError("statistics must cover every selected channel exactly once")
    time = parquet.read(columns=[report.get("time_column", "time")]).column(0).to_numpy()
    if len(time) == 0 or not np.all(np.isfinite(time)) or np.any(time[1:] < time[:-1]):
        raise ValueError("reference requires finite, nondecreasing source time")
    targets = [float(time[0]), float(time[-1])] if cursors is None else [
        cursor - report["offset_seconds"] for cursor in cursors
    ]
    indices = []
    for target in targets:
        if not time[0] <= target <= time[-1]:
            raise ValueError("cursor outside source time range")
        insertion = int(np.searchsorted(time, target, side="left"))
        candidates = [i for i in [insertion - 1, insertion] if 0 <= i < len(time)]
        indices.append(min(candidates, key=lambda i: abs(float(time[i]) - target)))
    if "interval_indices" in report and indices != report["interval_indices"]:
        raise ValueError(("nearest-sample indices differ", indices, report["interval_indices"]))
    start, end = min(indices), max(indices) + 1
    del time
    checked = []
    for actual in report["full_range_statistics"]:
        count = 0
        total = squares = 0.0
        minimum, maximum = np.inf, -np.inf
        first = last = None
        row = 0
        for batch in parquet.iter_batches(columns=[actual["channel"]]):
            # Match the viewer's f32 raw storage before doing independent f64 reductions.
            values = batch.column(0).to_numpy(zero_copy_only=False).astype(np.float32).astype(np.float64)
            if row <= indices[0] < row + len(values):
                first = float(values[indices[0] - row])
            if row <= indices[1] < row + len(values):
                last = float(values[indices[1] - row])
            lo, hi = max(0, start - row), min(len(values), end - row)
            row += len(values)
            if lo >= hi:
                continue
            values = values[lo:hi]
            finite = values[np.isfinite(values)]
            count += len(finite)
            total += float(finite.sum(dtype=np.float64))
            squares += float(np.square(finite).sum(dtype=np.float64))
            if finite.size:
                minimum = min(minimum, float(finite.min()))
                maximum = max(maximum, float(finite.max()))
        a = first if np.isfinite(first) else None
        b = last if np.isfinite(last) else None
        delta = b - a if a is not None and b is not None else None
        dt = targets[1] - targets[0]
        expected = {
            "mean": total / count if count else None,
            "min": minimum if count else None,
            "max": maximum if count else None,
            "peak_to_peak": maximum - minimum if count else None,
            "rms": float(np.sqrt(squares / count)) if count else None,
            "a": a, "b": b, "delta": delta,
            "slope_per_second": delta / dt if delta is not None and abs(dt) > np.finfo(float).eps else None,
        }
        differences = {}
        for name, value in expected.items():
            observed = actual[name]
            if value is None:
                if observed is not None:
                    raise ValueError((actual["channel"], name, observed, value))
                continue
            if observed is None or not np.isclose(observed, value, rtol=1e-10, atol=1e-10):
                raise ValueError((actual["channel"], name, observed, value))
            differences[name] = abs(observed - value)
        checked.append({"channel": actual["channel"], "finite_samples": count, "absolute_errors": differences})
    return {"passed": True, "reference": "NumPy float64 streaming value reductions from Parquet with independent nearest-sample selection", "rtol": 1e-10, "atol": 1e-10, "interval_indices": indices, "channels": checked}


def validate(report: dict) -> dict:
    if "sources" not in report:
        return validate_source(report)
    sources = report["sources"]
    ids = [s["source_id"] for s in sources]
    actual = report["interval_statistics"]
    if (not ids or len(set(ids)) != len(ids) or len(ids) != report["source_count"]
            or {s["source_id"] for s in actual} != set(ids)):
        raise ValueError("statistics must cover every source exactly once per channel")
    checked = []
    for source in sources:
        result = validate_source({**source, "full_range_statistics": [
            item for item in actual if item["source_id"] == source["source_id"]
        ]}, report["cursors"])
        checked.append({"source_id": source["source_id"], **result})
    return {"passed": True, "source_count": len(checked), "sources": checked}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    args = parser.parse_args()
    print(json.dumps(validate(json.loads(args.report.read_text())), indent=2))
