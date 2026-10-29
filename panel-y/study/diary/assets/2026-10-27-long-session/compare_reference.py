#!/usr/bin/env python3
"""Compare every native soak numeric event with the independent NumPy reference.

python3 compare_reference.py --events native-events.jsonl --output comparison.json
Use --allow-incomplete only for a running-log snapshot; it never claims completion.
The output file is created exclusively. No NumPy installation is required here.
"""

import argparse
import hashlib
import json
import math
import sys
from collections import Counter
from pathlib import Path


HERE = Path(__file__).resolve().parent
METRICS = ("value_a", "value_b", "delta", "mean", "min", "max", "peak_to_peak", "rms", "slope_per_second")
ATOL = 1e-9
RTOL = 1e-10


def finite(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


class Comparison:
    def __init__(self):
        self.comparisons = 0
        self.exact_comparisons = 0
        self.failures = 0
        self.examples = []
        self.errors = {}

    def fail(self, context, message):
        self.failures += 1
        if len(self.examples) < 50:
            self.examples.append({"context": context, "message": message})

    def exact(self, actual, expected, context):
        self.exact_comparisons += 1
        if isinstance(actual, bool) or not isinstance(actual, int) or actual != expected:
            self.fail(context, f"expected integer {expected}, got {actual!r}")

    def number(self, actual, expected, metric, context):
        self.comparisons += 1
        if not finite(actual) or not finite(expected):
            self.fail(context, f"non-finite or missing numeric value: {actual!r} vs {expected!r}")
            return
        absolute = abs(actual - expected)
        relative = absolute / abs(expected) if expected != 0 else None
        allowed = ATOL + RTOL * abs(expected)
        state = self.errors.setdefault(metric, {"count": 0, "max_absolute": 0.0, "max_relative_nonzero_reference": 0.0, "max_tolerance_fraction": 0.0})
        state["count"] += 1
        state["max_absolute"] = max(state["max_absolute"], absolute)
        if relative is not None:
            state["max_relative_nonzero_reference"] = max(state["max_relative_nonzero_reference"], relative)
        state["max_tolerance_fraction"] = max(state["max_tolerance_fraction"], absolute / allowed)
        if absolute > allowed:
            self.fail(context, f"actual {actual!r}, reference {expected!r}, absolute error {absolute:.12g} > {allowed:.12g}")


def read_events(raw, allow_incomplete):
    records = []
    trailing_partial = False
    lines = raw.splitlines(keepends=True)
    for index, line in enumerate(lines):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except ValueError:
            if allow_incomplete and index == len(lines) - 1 and not line.endswith(b"\n"):
                trailing_partial = True
                break
            raise ValueError(f"invalid JSON at event line {index + 1}")
        if not isinstance(record, dict):
            raise ValueError(f"event line {index + 1} is not an object")
        records.append(record)
    return records, trailing_partial


def normalize_path(value):
    return str(Path(value).resolve()) if isinstance(value, str) else "<missing>"


def compare_statistics(record, sources, comparison):
    context = f"cycle {record.get('cycle')}, statistics, after_restore={record.get('detail', {}).get('after_restore')}"
    values = record.get("detail", {}).get("values")
    if not isinstance(values, list):
        comparison.fail(context, "missing values array")
        return
    expected_keys = {(path, channel) for path, source in sources.items() for channel in source["statistics"]}
    actual_keys = [(normalize_path(row.get("source_path")), row.get("channel")) for row in values]
    if len(actual_keys) != len(expected_keys) or set(actual_keys) != expected_keys:
        comparison.fail(context, "source/channel coverage differs from the twelve unique reference channels")
    for row, key in zip(values, actual_keys):
        if key not in expected_keys:
            continue
        source = sources[key[0]]
        expected = source["statistics"][key[1]]
        location = f"{context}, {Path(key[0]).name}/{key[1]}"
        for metric in METRICS:
            comparison.number(row.get(metric), expected[metric], f"statistics.{metric}", f"{location}.{metric}")
        for field, expected_value in (("sample_count", expected["sample_count"]), ("index_start", source["indices_zero_based_inclusive"][0]), ("index_end_exclusive", source["indices_zero_based_inclusive"][1] + 1)):
            if field in row:
                comparison.exact(row[field], expected_value, f"{location}.{field}")


def compare_fft(record, sources, cursors, comparison):
    context = f"cycle {record.get('cycle')}, fft, after_restore={record.get('detail', {}).get('after_restore')}"
    values = record.get("detail", {}).get("values")
    if not isinstance(values, list):
        comparison.fail(context, "missing values array")
        return
    actual_paths = [normalize_path(row.get("source_path")) for row in values]
    if len(actual_paths) != len(sources) or set(actual_paths) != set(sources):
        comparison.fail(context, "source coverage differs from the three unique reference sources")
    for row, path in zip(values, actual_paths):
        if path not in sources:
            continue
        source = sources[path]
        expected = source["fft"]
        location = f"{context}, {Path(path).name}"
        for field, expected_value in (("index_start", source["indices_zero_based_inclusive"][0]), ("index_end_exclusive", source["indices_zero_based_inclusive"][1] + 1), ("sample_count", expected["sample_count"]), ("frequencies_count", expected["frequency_bin_count"])):
            comparison.exact(row.get(field), expected_value, f"{location}.{field}")
        for field, expected_value in (("cursor_a", cursors[0]), ("cursor_b", cursors[1]), ("dt", expected["dt_seconds"])):
            comparison.number(row.get(field), expected_value, f"fft.{field}", f"{location}.{field}")
        spectra = row.get("spectra")
        if not isinstance(spectra, list) or len(spectra) != 1 or spectra[0].get("channel") != expected["channel"]:
            comparison.fail(location, "expected exactly one Ia_phys spectrum")
            continue
        spectrum = spectra[0]
        comparison.exact(spectrum.get("amplitude_count"), expected["frequency_bin_count"], f"{location}.amplitude_count")
        for metric in ("peak_frequency_hz", "peak_amplitude", "peak_db"):
            comparison.number(spectrum.get(metric), expected[metric], f"fft.{metric}", f"{location}.{metric}")


def run(reference, records, comparison, allow_incomplete):
    sources = {normalize_path(source["path"]): source for source in reference["sources"]}
    if len(sources) != 3:
        raise ValueError("reference must contain three unique source paths")
    counts = Counter(record.get("event") for record in records)
    complete = counts["complete"] == 1 and bool(records) and records[-1].get("event") == "complete"
    if not complete and not allow_incomplete:
        comparison.fail("completion", "one final complete event is required; use --allow-incomplete only for a running snapshot")
    for record in records:
        if record.get("event") in ("failed", "aborted", "error"):
            comparison.fail("completion", f"driver reports {record['event']}: {record.get('detail')}")
        elif record.get("event") == "statistics":
            compare_statistics(record, sources, comparison)
        elif record.get("event") == "fft":
            compare_fft(record, sources, reference["cursors_seconds"], comparison)
    if not counts["statistics"] or not counts["fft"]:
        comparison.fail("coverage", "at least one statistics and FFT event is required")
    if complete and counts["statistics"] != counts["fft"]:
        comparison.fail("coverage", "completed run has unequal statistics and FFT event counts")
    numeric = [record for record in records if record.get("event") in ("statistics", "fft")]
    checkpoints = [{key: record.get(key) for key in ("event", "cycle", "session_elapsed_seconds", "completed_cycles")} for record in numeric]
    return {
        "driver_completed": complete,
        "scope": "all observed statistics and FFT events; full spectrum equality is not independently compared",
        "event_counts": dict(counts),
        "numeric_events_after_restore": sum(bool(record.get("detail", {}).get("after_restore")) for record in numeric),
        "first_numeric_checkpoint": checkpoints[0] if checkpoints else None,
        "last_numeric_checkpoint": checkpoints[-1] if checkpoints else None,
        "numeric_cycles": sorted({record.get("cycle") for record in numeric}),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, default=HERE / "numeric-reference.json")
    parser.add_argument("--events", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--allow-incomplete", action="store_true")
    args = parser.parse_args()
    comparison = Comparison()
    raw = args.events.read_bytes()
    reference_raw = args.reference.read_bytes()
    report = {"events_path": str(args.events.resolve()), "events_sha256": hashlib.sha256(raw).hexdigest(), "reference_sha256": hashlib.sha256(reference_raw).hexdigest(), "allow_incomplete": args.allow_incomplete, "tolerances": {"absolute": ATOL, "relative": RTOL}, "counts_and_indices": "exact; statistics indices/counts only when emitted; FFT indices/counts required"}
    try:
        records, trailing_partial = read_events(raw, args.allow_incomplete)
        report.update(run(json.loads(reference_raw), records, comparison, args.allow_incomplete))
        report["ignored_trailing_partial_record"] = trailing_partial
    except (ValueError, KeyError, TypeError, AttributeError) as error:
        comparison.fail("input", str(error))
    report.update({"passed_observed_comparisons": comparison.failures == 0, "numeric_comparisons": comparison.comparisons, "exact_comparisons": comparison.exact_comparisons, "max_errors": comparison.errors, "failure_count": comparison.failures, "failure_examples": comparison.examples})
    with args.output.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(report, indent=2, allow_nan=False) + "\n")
    print(json.dumps({key: report.get(key) for key in ("passed_observed_comparisons", "driver_completed", "numeric_comparisons", "exact_comparisons", "failure_count", "first_numeric_checkpoint", "last_numeric_checkpoint")}))
    return 0 if comparison.failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
