"""Independently validate motor-simulation CSV-to-Parquet conversion and time grids.

Run with NumPy, pandas and PyArrow installed. Inputs are never modified. The
generation manifest alongside this script identifies all input files and hashes.
This is data validation, not a viewer performance benchmark or model validation.
"""

from __future__ import annotations

import csv
import hashlib
import json
import platform
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import pandas as pd
import pyarrow as pa
import pyarrow.parquet as pq


HERE = Path(__file__).resolve().parent
CURSORS = (0.03, 0.09)
GUI_CURSORS = (0.070313604, 0.090501695)
# Values transcribed from the native viewer during this validation session.
# Only the entries below are observed; the other channels are reference values.
GUI_OBSERVED = {
    "input": {
        "Ia_phys": {"value_a": "-10.782298", "value_b": "4.918699", "delta": "15.700997", "mean": "2.849023", "max": "10.957077", "min": "-11.145513", "peak_to_peak": "22.102590", "rms": "7.881946"},
    },
    "reversal": {"Ia_phys": {"mean": "2.861646", "rms": "7.926446"}},
}
NOMINAL_DT = 1e-6
CHUNK_ROWS = 16_384


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def nearest_index(time: np.ndarray, value: float) -> int:
    require(time[0] <= value <= time[-1], f"cursor {value} outside time range")
    insertion = int(np.searchsorted(time, value, side="left"))
    candidates = [i for i in (insertion - 1, insertion) if 0 <= i < len(time)]
    return min(candidates, key=lambda i: abs(float(time[i]) - value))


def interval_statistics(time: np.ndarray, values: np.ndarray, cursors=CURSORS) -> dict:
    first, last = (nearest_index(time, value) for value in cursors)
    selected = values[first : last + 1].astype(np.float64)
    a, b = float(values[first]), float(values[last])
    return {
        "indices_zero_based": [first, last],
        "samples_inclusive": len(selected),
        "sample_times_seconds": [float(time[first]), float(time[last])],
        "value_a": a,
        "value_b": b,
        "delta": b - a,
        "mean": float(np.mean(selected)),
        "min": float(np.min(selected)),
        "max": float(np.max(selected)),
        "peak_to_peak": float(np.ptp(selected)),
        "rms": float(np.sqrt(np.mean(selected * selected))),
        "slope_per_second": (b - a) / (cursors[1] - cursors[0]),
    }


def validate_run(run: dict) -> dict:
    csv_path, parquet_path = Path(run["csv"]), Path(run["parquet"])
    hashes = {"csv_sha256": sha256(csv_path), "parquet_sha256": sha256(parquet_path)}
    for key, value in hashes.items():
        require(value == run[key], f"{run['name']}: {key} differs from generation manifest")

    with csv_path.open(newline="") as stream:
        headers = next(csv.reader(stream))
    require(headers == run["csv_columns"], "CSV columns differ from generation manifest")
    require(len(headers) == len(set(headers)), "CSV contains duplicate column names")
    table = pq.read_table(parquet_path).combine_chunks()
    require(table.num_rows == run["rows"], "Parquet row count differs from generation manifest")
    require(table.column_names[0] == "time", "Parquet time must be its first column")
    omitted = [name for name in headers if name != "t" and name not in table.column_names]
    require(omitted == ["pwm_dir", "controller_mode"], f"unexpected omitted columns: {omitted}")
    require(
        table.column_names == ["time" if name == "t" else name for name in headers if name not in omitted],
        "Parquet column order or coverage differs from CSV",
    )

    columns = {}
    values = {}
    for field in table.schema:
        require(table[field.name].null_count == 0, f"{field.name}: null Parquet values")
        expected_type = pa.float64() if field.name == "time" else field.type
        require(
            expected_type in (pa.float64(), pa.float32(), pa.uint8())
            and (field.name == "time" or field.type in (pa.float32(), pa.uint8()))
            and field.type == expected_type,
            f"{field.name}: unsupported type {field.type}",
        )
        column = table[field.name].to_numpy()
        require(bool(np.all(np.isfinite(column))), f"{field.name}: non-finite Parquet values")
        values[field.name] = column
        columns[field.name] = {
            "type": str(field.type),
            "rows_checked": 0,
            "bitwise_conversion_mismatches": 0,
            "max_abs_cast_error_from_csv_float64": 0.0,
            "min": float(np.min(column)),
            "max": float(np.max(column)),
            "mean": float(np.mean(column, dtype=np.float64)),
            "all_finite": True,
        }

    omitted_values = {name: set() for name in omitted}
    offset = 0
    numeric_types = {name: np.float64 for name in headers if name not in omitted}
    # round_trip avoids pandas' default fast float parser changing decimal->f64
    # rounding. NumPy casting then independently checks every exported value.
    for chunk in pd.read_csv(
        csv_path,
        chunksize=CHUNK_ROWS,
        dtype={**numeric_types, **{name: str for name in omitted}},
        float_precision="round_trip",
        keep_default_na=False,
    ):
        count = len(chunk)
        require(offset + count <= table.num_rows, "CSV has more rows than Parquet")
        for name in omitted:
            omitted_values[name].update(chunk[name].unique().tolist())
        for name, column in values.items():
            csv_name = "t" if name == "time" else name
            original = chunk[csv_name].to_numpy(dtype=np.float64)
            require(bool(np.all(np.isfinite(original))), f"{csv_name}: non-finite CSV values")
            if column.dtype == np.uint8:
                require(bool(np.all((original == 0) | (original == 1))), f"{name}: invalid bool")
            expected = original.astype(column.dtype)
            actual = column[offset : offset + count]
            # Compare bits, including signed zero, for float columns.
            bits = np.dtype(f"u{column.dtype.itemsize}")
            mismatch_count = int(np.count_nonzero(expected.view(bits) != actual.view(bits)))
            columns[name]["bitwise_conversion_mismatches"] += mismatch_count
            columns[name]["rows_checked"] += count
            columns[name]["max_abs_cast_error_from_csv_float64"] = max(
                columns[name]["max_abs_cast_error_from_csv_float64"],
                float(np.max(np.abs(original - actual.astype(np.float64)))),
            )
            require(mismatch_count == 0, f"{run['name']} {name}: {mismatch_count} mismatches at row {offset}")
        offset += count
    require(offset == table.num_rows, "CSV has fewer rows than Parquet")
    require(omitted_values["pwm_dir"] == {"up", "down"}, "unexpected omitted pwm_dir values")
    require(omitted_values["controller_mode"] == {"ipm-current"}, "unexpected controller mode")

    time = values["time"]
    dt = np.diff(time)
    expected_grid = np.arange(1, len(time) + 1, dtype=np.float64) * NOMINAL_DT
    # A conservative floating-point roundoff bound, not a sampling-gap tolerance.
    grid_tolerance = 32 * np.finfo(np.float64).eps * max(1.0, float(time[-1]))
    grid_error = float(np.max(np.abs(time - expected_grid)))
    dt_error = float(np.max(np.abs(dt - NOMINAL_DT)))
    require(bool(np.all(dt > 0)), "time is not strictly increasing")
    require(grid_error <= grid_tolerance, "time differs from the complete 1 us sample grid")
    require(dt_error <= grid_tolerance, "time interval is not uniformly 1 us")
    gui_statistics = {
        name: interval_statistics(time, values[name], GUI_CURSORS)
        for name in ("Ia_phys", "Te", "Trq_cmd", "Hi_a")
    }
    observed_comparisons = []
    for channel, metrics in GUI_OBSERVED.get(run["name"], {}).items():
        for metric, observed in metrics.items():
            expected = f"{gui_statistics[channel][metric]:.6f}"
            observed_comparisons.append({
                "channel": channel, "metric": metric,
                "expected_six_decimals": expected,
                "observed_six_decimals": observed, "passed": observed == expected,
            })
            require(observed == expected, f"GUI {run['name']} {channel} {metric}: {observed} != {expected}")
    return {
        "name": run["name"],
        "passed": True,
        "csv": str(csv_path),
        "parquet": str(parquet_path),
        **hashes,
        "hashes_match_generation_manifest": True,
        "rows": len(time),
        "csv_columns": len(headers),
        "parquet_columns": len(table.column_names),
        "channel_count": len(table.column_names) - 1,
        "numeric_cells_compared": len(time) * len(table.column_names),
        "conversion_mismatches": 0,
        "omitted_columns": {name: sorted(omitted_values[name]) for name in omitted},
        "time": {
            "range_seconds": [float(time[0]), float(time[-1])],
            "strictly_increasing": True,
            "complete_uniform_grid": True,
            "nominal_dt_seconds": NOMINAL_DT,
            "min_dt_seconds": float(np.min(dt)),
            "max_dt_seconds": float(np.max(dt)),
            "max_abs_dt_error_seconds": dt_error,
            "max_abs_grid_error_seconds": grid_error,
            "float_roundoff_tolerance_seconds": grid_tolerance,
        },
        "column_summaries": columns,
        "representative_interval": {
            "cursors_seconds": list(CURSORS),
            "definition": "Inclusive nearest samples; float64 reductions over exported values; slope uses cursor delta time.",
            "channels": {name: interval_statistics(time, values[name]) for name in ("Ia_phys", "Te", "Hi_a")},
        },
        "gui_interval": {
            "cursors_seconds_as_displayed": list(GUI_CURSORS),
            "definition": "Inclusive nearest samples from displayed cursor times. Slope uses displayed cursor difference and may differ slightly from full internal cursor precision.",
            "channels": gui_statistics,
            "observations_source": "Native viewer values transcribed by parent agent; only listed metrics were visually compared.",
            "observed_metric_comparisons": observed_comparisons,
            "all_observed_metrics_match_six_decimals": all(item["passed"] for item in observed_comparisons) if observed_comparisons else None,
        },
    }


def main() -> int:
    manifest_path = HERE / "generation.json"
    manifest = json.loads(manifest_path.read_text())
    report = {
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "provenance": manifest["provenance"],
        "base_revision": manifest["base_revision"],
        "scope": "All CSV rows and exported numeric columns, exact time and intended float32/uint8 conversion, finite values and full uniform grid. No product performance or physical-model qualification.",
        "method": "pandas round-trip decimal to float64 parsing, independent NumPy casting and bitwise comparison to PyArrow Parquet values; float64 statistics.",
        "environment": {"python": sys.version, "platform": platform.platform(), "numpy": np.__version__, "pandas": pd.__version__, "pyarrow": pa.__version__},
        "generation_manifest_sha256": sha256(manifest_path),
        "validation_script_sha256": sha256(Path(__file__).resolve()),
        "runs": [],
        "passed": False,
    }
    try:
        for run in manifest["runs"]:
            print(f"Validating {run['name']} ({run['rows']} rows)", file=sys.stderr, flush=True)
            report["runs"].append(validate_run(run))
        report["passed"] = True
        report["numeric_cells_compared"] = sum(run["numeric_cells_compared"] for run in report["runs"])
    except Exception as error:
        report["error"] = str(error)
    (HERE / "input-validation.json").write_text(json.dumps(report, indent=2, allow_nan=False) + "\n")
    print(json.dumps({"passed": report["passed"], "runs": len(report["runs"]), "numeric_cells_compared": report.get("numeric_cells_compared"), "error": report.get("error")}), flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
