#!/usr/bin/env python3
"""Summarize native-driver events and selected-PID RSS samples using stdlib only.

Usage:
    python3 analyzer.py --driver driver.jsonl --samples native-memory.jsonl \
        --output analysis.json

Phase RSS values are the driver's checkpoint observations. Periodic RSS samples
are summarized separately; neither series establishes an allocation leak or a
true peak. No universal memory pass threshold or frame-latency p95 is invented.
"""

import argparse
import datetime as dt
import json
import math
import statistics
import sys
from pathlib import Path


PHASES = ("loaded", "statistics", "fft", "before_remove", "empty", "restored", "cycle_complete", "final_empty", "complete")


def read_jsonl(path):
    records = []
    with path.open(encoding="utf-8") as stream:
        for number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except ValueError as error:
                raise ValueError(f"{path.name}:{number}: invalid JSON record") from error
            if not isinstance(record, dict):
                raise ValueError(f"{path.name}:{number}: expected JSON object")
            records.append(record)
    return records


def finite_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def distribution(values):
    numbers = [float(value) for value in values if finite_number(value)]
    if not numbers:
        return {"count": 0, "min": None, "median": None, "max": None}
    return {
        "count": len(numbers), "min": min(numbers),
        "median": statistics.median(numbers), "max": max(numbers),
    }


def slope(points):
    if len(points) < 2:
        return None
    mean_x = statistics.mean(point[0] for point in points)
    mean_y = statistics.mean(point[1] for point in points)
    divisor = sum((x - mean_x) ** 2 for x, _ in points)
    if not divisor:
        return None
    return sum((x - mean_x) * (y - mean_y) for x, y in points) / divisor


def phase_cycle(record):
    cycle = record.get("cycle")
    if isinstance(cycle, int) and not isinstance(cycle, bool) and cycle > 0:
        return cycle
    completed = record.get("completed_cycles")
    if not isinstance(completed, int) or isinstance(completed, bool) or completed < 0:
        return None
    if record.get("event") in ("cycle_complete", "final_empty", "complete"):
        return completed
    return completed + 1


def phase_summary(records, total_cycles):
    by_cycle = {}
    for record in records:
        cycle = phase_cycle(record)
        rss = record.get("rss_mib")
        if cycle is not None and cycle > 0 and finite_number(rss):
            by_cycle.setdefault(cycle, []).append(float(rss))
    points = sorted((cycle, statistics.median(values)) for cycle, values in by_cycle.items())
    late_start = max(6, total_cycles - 9)
    groups = {
        "warmup_cycles_1_to_5": (1, 5),
        "middle_cycles_6_to_15": (6, 15),
        "last_10_post_warmup_cycles": (late_start, total_cycles),
    }
    grouped = {}
    for name, (start, end) in groups.items():
        selected = [(cycle, rss) for cycle, rss in points if start <= cycle <= end]
        grouped[name] = {
            "requested_cycle_range": [start, end],
            "cycles": [cycle for cycle, _ in selected],
            "rss_mib": distribution([rss for _, rss in selected]),
        }
    middle = grouped["middle_cycles_6_to_15"]["rss_mib"]["median"]
    late = grouped["last_10_post_warmup_cycles"]["rss_mib"]["median"]
    return {
        "event_count": len(records),
        "checkpoint_rss_mib": distribution([record.get("rss_mib") for record in records]),
        "per_cycle_median_rss_mib": [{"cycle": cycle, "rss_mib": rss} for cycle, rss in points],
        "groups": grouped,
        "late_minus_middle_median_mib": late - middle if late is not None and middle is not None else None,
        "post_warmup_ols_mib_per_cycle": slope([(cycle, rss) for cycle, rss in points if cycle >= 6]),
    }


def timestamp_seconds(record):
    unix = record.get("timestamp_unix_seconds")
    if finite_number(unix):
        return float(unix)
    text = record.get("timestamp_utc")
    if not isinstance(text, str):
        return None
    try:
        value = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError:
        return None
    return value.timestamp() if value.tzinfo is not None else None


def clock_drift(records):
    pairs = [(timestamp_seconds(record), record.get("elapsed_seconds")) for record in records]
    pairs = [(wall, elapsed) for wall, elapsed in pairs if finite_number(wall) and finite_number(elapsed)]
    if not pairs:
        return distribution([])
    wall0, elapsed0 = pairs[0]
    return distribution([(wall - wall0) - (elapsed - elapsed0) for wall, elapsed in pairs])


def continuity_audit(driver, config):
    """Detect stalled/sleep-and-catch-up evidence without claiming GPU frame timing."""
    problems = []
    elapsed = [record.get("elapsed_seconds") for record in driver]
    frames = [record.get("native_frames") for record in driver]
    navigation = [record.get("navigation_steps") for record in driver]
    valid = all(finite_number(value) for value in elapsed + frames + navigation)
    if not valid:
        return {"errors": ["driver continuity counters/timestamps are absent or invalid"]}
    if any(right < left for left, right in zip(elapsed, elapsed[1:])):
        problems.append("driver elapsed times decrease")
    if any(right < left for left, right in zip(frames, frames[1:])):
        problems.append("native frame counter decreases")
    if any(right < left for left, right in zip(navigation, navigation[1:])):
        problems.append("navigation counter decreases")
    gaps = [right - left for left, right in zip(elapsed, elapsed[1:])]
    # The frozen driver emits a heartbeat at least every 30 seconds while running.
    # Two missed heartbeat periods indicate a continuity gap needing investigation.
    maximum_event_gap = 60.0
    long_gaps = [{"after_event_index": index + 1, "from_event": driver[index].get("event"),
                  "to_event": driver[index + 1].get("event"), "seconds": gap}
                 for index, gap in enumerate(gaps) if gap > maximum_event_gap]
    if long_gaps:
        problems.append("driver has an event gap exceeding two 30-second heartbeat periods")
    beats = [record for record in driver if record.get("event") in ("initialization", "heartbeat", "complete", "failed")]
    beat_intervals = []
    stagnant_frames = []
    stagnant_navigation = []
    for left, right in zip(beats, beats[1:]):
        seconds = right["elapsed_seconds"] - left["elapsed_seconds"]
        frame_count = right["native_frames"] - left["native_frames"]
        steps = right["navigation_steps"] - left["navigation_steps"]
        beat_intervals.append({"from_elapsed_seconds": left["elapsed_seconds"],
            "to_elapsed_seconds": right["elapsed_seconds"], "seconds": seconds,
            "native_frame_delta": frame_count, "navigation_step_delta": steps,
            "from_stage": left.get("detail", {}).get("stage"),
            "to_stage": right.get("detail", {}).get("stage")})
        if seconds > 1.0 and frame_count <= 0:
            stagnant_frames.append(len(beat_intervals))
        active_stages = ("Active", "RestoredHold")
        if (seconds > 1.0 and steps <= 0 and left.get("detail", {}).get("stage") in active_stages
                and right.get("detail", {}).get("stage") in active_stages):
            stagnant_navigation.append(len(beat_intervals))
    if stagnant_frames:
        problems.append("native frames did not advance across a heartbeat interval")
    if stagnant_navigation:
        problems.append("navigation did not advance across an active heartbeat interval")
    cycles = [record for record in driver if record.get("event") == "cycle_complete"]
    cycle_times = [record.get("session_elapsed_seconds") for record in cycles]
    cycle_gaps = ([cycle_times[0]] + [right - left for left, right in zip(cycle_times, cycle_times[1:])]
                  if cycle_times and all(finite_number(value) for value in cycle_times) else [])
    previous_navigation = 0
    previous_frames = 0
    cycle_details = []
    for index, (record, seconds) in enumerate(zip(cycles, cycle_gaps)):
        frame_delta = record["native_frames"] - previous_frames
        navigation_delta = record["navigation_steps"] - previous_navigation
        cycle_details.append({"cycle": index + 1, "seconds": seconds,
                              "native_frame_delta": frame_delta, "navigation_step_delta": navigation_delta})
        previous_frames = record["native_frames"]
        previous_navigation = record["navigation_steps"]
        if frame_delta <= 0 or navigation_delta <= 0:
            problems.append(f"native rendering/navigation did not advance in cycle {index + 1}")
    configured_duration = config.get("duration_seconds")
    configured_cycles = config.get("cycles")
    period = (configured_duration / configured_cycles
              if finite_number(configured_duration) and finite_number(configured_cycles) and configured_cycles > 0 else None)
    compressed = [{"cycle": index + 1, "seconds": gap} for index, gap in enumerate(cycle_gaps)
                  if period is not None and gap < period / 2]
    if compressed:
        problems.append("cycle completion intervals suggest compressed catch-up after a delay")
    return {
        "errors": problems, "driver_event_gap_seconds": distribution(gaps),
        "maximum_allowed_event_gap_seconds": maximum_event_gap, "long_event_gaps": long_gaps,
        "heartbeat_count": sum(record.get("event") == "heartbeat" for record in driver),
        "heartbeat_intervals": beat_intervals,
        "heartbeat_native_frame_deltas": distribution([item["native_frame_delta"] for item in beat_intervals]),
        "heartbeat_navigation_step_deltas": distribution([item["navigation_step_delta"] for item in beat_intervals]),
        "stagnant_frame_intervals": stagnant_frames, "stagnant_active_navigation_intervals": stagnant_navigation,
        "cycle_interval_seconds": distribution(cycle_gaps), "configured_cycle_period_seconds": period,
        "cycle_intervals": cycle_details,
        "compressed_cycle_intervals": compressed,
        "compressed_cycle_guard_fraction_of_period": 0.5,
        "driver_wall_minus_elapsed_drift_seconds": clock_drift(driver),
        "scope": "Checks logged native rendering/navigation continuity; neither frame presentation timing nor a universal FPS requirement.",
    }
def analyze(driver, sampler, min_duration, min_cycles):
    errors = []
    if not driver:
        errors.append("driver log is empty")
    if not sampler:
        errors.append("sampler log is empty")
    pids = {record.get("pid") for record in driver + sampler if record.get("pid") is not None}
    if len(pids) != 1:
        errors.append("driver and sampler do not identify one shared PID")
    terminal = [record for record in driver if record.get("event") in ("complete", "failed")]
    complete = terminal[-1] if terminal else {}
    if len(terminal) != 1 or complete.get("event") != "complete":
        errors.append("driver does not have exactly one successful terminal record")
    if driver and driver[-1].get("event") != "complete":
        errors.append("driver final record is not complete")
    cycle_events = [record for record in driver if record.get("event") == "cycle_complete"]
    cycle_numbers = [record.get("completed_cycles") for record in cycle_events]
    total_cycles = complete.get("completed_cycles", 0)
    if not isinstance(total_cycles, int) or isinstance(total_cycles, bool):
        total_cycles = 0
        errors.append("invalid terminal completed_cycles")
    if cycle_numbers != list(range(1, total_cycles + 1)):
        errors.append("cycle_complete counters are not sequential from 1 through final count")
    if total_cycles < min_cycles:
        errors.append(f"fewer than {min_cycles} completed cycles")
    session_duration = complete.get("session_elapsed_seconds")
    active_duration = cycle_events[-1].get("session_elapsed_seconds") if cycle_events else None
    if not finite_number(active_duration) or active_duration < min_duration:
        errors.append(f"driver active cycle duration is below {min_duration:g} seconds or absent")
    initializations = [record for record in driver if record.get("event") == "initialization"]
    config = initializations[0].get("detail", {}) if len(initializations) == 1 else {}
    if len(initializations) != 1:
        errors.append("driver must have exactly one initialization record")
    configured_quiet = config.get("final_quiet_seconds")
    observed_quiet = complete.get("detail", {}).get("quiet_seconds")
    if not finite_number(configured_quiet) or not finite_number(observed_quiet) or observed_quiet < configured_quiet:
        errors.append("configured final quiet period was not completed or is absent")
    for flag in ("all_numeric_comparisons_passed", "empty_cache_and_raw_lifetime_passed"):
        if complete.get("detail", {}).get(flag) is not True:
            errors.append(f"driver terminal flag {flag} is not true")
    for phase in ("empty", "restored"):
        phase_records = [record for record in driver if record.get("event") == phase]
        if [phase_cycle(record) for record in phase_records] != list(range(1, total_cycles + 1)):
            errors.append(f"{phase} checkpoints do not cover each completed cycle exactly once")
    for phase in ("statistics", "fft"):
        records = [record for record in driver if record.get("event") == phase]
        initial = [record for record in records if record.get("detail", {}).get("after_restore") is False]
        restored = [record for record in records if record.get("detail", {}).get("after_restore") is True]
        if len(initial) != 1 or len(records) != total_cycles + 1:
            errors.append(f"{phase} must have one initial result plus one result per restoration")
        if [phase_cycle(record) for record in restored] != list(range(1, total_cycles + 1)):
            errors.append(f"{phase} after-restore results do not cover every completed cycle exactly once")
    before_remove = [record for record in driver if record.get("event") == "before_remove"]
    if [phase_cycle(record) for record in before_remove] != list(range(1, total_cycles + 1)):
        errors.append("before_remove checkpoints do not cover each cycle exactly once")
    if any(record.get("detail", {}).get("weak_time_buffers") != 3
           or record.get("detail", {}).get("weak_channel_buffers") != 12 for record in before_remove):
        errors.append("before_remove did not track exactly 3 time and 12 channel buffers")
    loaded_records = [record for record in driver if record.get("event") in ("loaded", "restored")]
    if sum(record.get("event") == "loaded" for record in loaded_records) != 1:
        errors.append("expected exactly one initial loaded checkpoint")
    for record in loaded_records:
        cache = record.get("cache", {})
        if cache.get("sources") != 3 or cache.get("loaded_channels") != 12:
            errors.append(f"{record.get('event')} checkpoint does not have 3 sources and 12 loaded channels")
        if record.get("event") == "loaded":
            layout = record.get("detail", {}).get("layout", {})
            rows = layout.get("rows", [])
            if len(rows) != 3 or sum(len(row.get("channels", [])) for row in rows) != 12:
                errors.append("initial loaded layout is not 3 rows and 12 channels")
        if record.get("event") == "restored":
            detail = record.get("detail", {})
            if detail.get("layout_matches") is not True or detail.get("analysis_reset") is not True:
                errors.append("restored checkpoint lacks successful layout/analysis-reset assertions")
    if any(not finite_number(record.get("rss_mib")) or record["rss_mib"] < 0 for record in driver if record.get("event") in PHASES):
        errors.append("driver phase RSS checkpoints contain missing/invalid values")
    empty_events = [record for record in driver if record.get("event") in ("empty", "final_empty")]
    zero_fields = ("sources", "loaded_channels", "raw_bytes", "envelopes", "line_tiles", "line_tile_bytes", "step_edges", "step_edge_bytes", "step_fallback_hints")
    empty_violations = []
    for record in empty_events:
        cache = record.get("cache", {})
        bad = {field: cache.get(field) for field in zero_fields if cache.get(field) != 0}
        if bad:
            empty_violations.append({"event": record.get("event"), "completed_cycles": record.get("completed_cycles"), "nonzero_or_missing": bad})
        detail = record.get("detail", {})
        if detail.get("weak_time_alive") != 0 or detail.get("weak_channel_alive") != 0:
            errors.append("empty checkpoint does not confirm zero raw buffer strong references")
    if empty_violations:
        errors.append("empty checkpoint cache fields are nonzero or absent")
    if sum(record.get("event") == "final_empty" for record in driver) != 1:
        errors.append("expected exactly one final_empty checkpoint")
    samples = [record for record in sampler if record.get("event") == "sample"]
    starts = [record for record in sampler if record.get("event") == "start"]
    ends = [record for record in sampler if record.get("event") == "end"]
    if len(starts) != 1 or len(ends) != 1 or sampler[-1].get("event") != "end":
        errors.append("sampler start/end records are incomplete")
    sampler_end = ends[-1] if ends else {}
    if sampler_end.get("outcome") not in ("completed", "driver_completed") or sampler_end.get("exit_code") != 0:
        errors.append("sampler did not complete successfully")
    if sampler_end.get("sample_count") != len(samples):
        errors.append("sampler terminal sample count does not match records")
    if any(not finite_number(record.get("rss_mib")) or record["rss_mib"] < 0 for record in samples):
        errors.append("RSS samples contain missing/invalid values")
    identities = {record.get("process_start_local") for record in samples}
    if not samples or len(identities) != 1 or None in identities:
        errors.append("sample process start identities are absent or inconsistent")
    times = [record.get("monotonic_ns") for record in samples]
    if any(not finite_number(value) for value in times):
        errors.append("sample monotonic timestamps are invalid")
        gaps = []
    else:
        gaps = [(right - left) / 1_000_000_000 for left, right in zip(times, times[1:])]
        if any(gap <= 0 for gap in gaps):
            errors.append("sample monotonic timestamps are not strictly increasing")
    interval = starts[0].get("interval_seconds") if starts else None
    terminals = [record for record in sampler if record.get("event") == "driver_terminal"]
    approximate_sampler_offset = None
    if len(terminals) == 1:
        t = terminals[0]
        driver_elapsed = t.get("driver_event", {}).get("elapsed_seconds")
        sampler_elapsed = t.get("elapsed_seconds")
        if finite_number(driver_elapsed) and finite_number(sampler_elapsed):
            approximate_sampler_offset = driver_elapsed - sampler_elapsed
    restoration_kinds = {}
    for record in driver:
        if record.get("event") == "restored":
            kind = record.get("detail", {}).get("kind", "unknown")
            restoration_kinds.setdefault(kind, []).append(record)
    kinds = [record.get("detail", {}).get("kind") for record in driver if record.get("event") == "restored"]
    expected_kinds = ["session_preset" if cycle % 2 else "plot_config" for cycle in range(1, total_cycles + 1)]
    if kinds != expected_kinds:
        errors.append("restoration kinds do not alternate session preset and plot config")
    sample_span_seconds = sum(gaps) if gaps else None
    approximate_active_coverage = None
    first_loaded = next((record for record in driver if record.get("event") == "loaded"), {})
    active_start = first_loaded.get("elapsed_seconds")
    active_end = cycle_events[-1].get("elapsed_seconds") if cycle_events else None
    if (approximate_sampler_offset is not None and samples
            and all(finite_number(value) for value in (active_start, active_end,
                    samples[0].get("elapsed_seconds"), samples[-1].get("elapsed_seconds")))):
        sample_start = approximate_sampler_offset + samples[0]["elapsed_seconds"]
        sample_end = approximate_sampler_offset + samples[-1]["elapsed_seconds"]
        overlap = max(0.0, min(active_end, sample_end) - max(active_start, sample_start))
        active_span = active_end - active_start
        approximate_active_coverage = {
            "sample_first_driver_relative_seconds": sample_start,
            "sample_last_driver_relative_seconds": sample_end,
            "active_window_driver_relative_seconds": [active_start, active_end],
            "overlap_seconds": overlap,
            "overlap_fraction": overlap / active_span if active_span > 0 else None,
            "missing_start_seconds": max(0.0, sample_start - active_start),
            "missing_end_seconds": max(0.0, active_end - sample_end),
        }
    utc_active_coverage = None
    if samples:
        utc_first = timestamp_seconds(samples[0])
        utc_last = timestamp_seconds(samples[-1])
        utc_start = timestamp_seconds(first_loaded)
        utc_end = timestamp_seconds(cycle_events[-1]) if cycle_events else None
        if all(finite_number(value) for value in (utc_first, utc_last, utc_start, utc_end)):
            overlap = max(0.0, min(utc_end, utc_last) - max(utc_start, utc_first))
            active_span = utc_end - utc_start
            utc_active_coverage = {
                "method": "driver Unix timestamps aligned with sampler UTC timestamps",
                "sample_unix_seconds": [utc_first, utc_last],
                "active_window_unix_seconds": [utc_start, utc_end],
                "overlap_seconds": overlap,
                "overlap_fraction": overlap / active_span if active_span > 0 else None,
                "missing_start_seconds": max(0.0, utc_first - utc_start),
                "missing_end_seconds": max(0.0, utc_end - utc_last),
                "clock_note": "Wall-clock alignment; inspect driver/sampler wall-minus-elapsed drift. Sampler UTC is rounded to milliseconds.",
            }
    continuity = continuity_audit(driver, config)
    errors.extend(continuity["errors"])
    return {
        "schema_version": 1,
        "requirements": {"minimum_active_session_seconds": min_duration, "minimum_completed_cycles": min_cycles},
        "record_consistency_errors": errors,
        "record_consistency_ok": not errors,
        "pid": next(iter(pids)) if len(pids) == 1 else None,
        "driver": {
            "event_count": len(driver), "completed_cycles": total_cycles,
            "session_elapsed_seconds": session_duration,
            "active_cycles_elapsed_seconds": active_duration,
            "configured_final_quiet_seconds": configured_quiet,
            "observed_final_quiet_seconds": observed_quiet,
            "total_elapsed_seconds": complete.get("elapsed_seconds"),
            "terminal_event": complete.get("event"),
            "native_frames": complete.get("native_frames"),
            "empty_checkpoint_count": len(empty_events),
            "empty_cache_violations": empty_violations,
            "phase_cycle_rule": "explicit driver cycle when present; otherwise completed_cycles + 1 for in-cycle phases and completed_cycles for terminal phases",
            "phases": {phase: phase_summary([record for record in driver if record.get("event") == phase], total_cycles) for phase in PHASES},
            "restored_by_kind": {kind: phase_summary(records, total_cycles) for kind, records in restoration_kinds.items()},
            "continuity_audit": continuity,
        },
        "periodic_sampler": {
            "sample_count": len(samples), "outcome": sampler_end.get("outcome"),
            "elapsed_seconds": sampler_end.get("elapsed_seconds"),
            "requested_interval_seconds": interval,
            "sample_gap_seconds": distribution(gaps),
            "sample_span_seconds": sample_span_seconds,
            "gaps_over_three_intervals": sum(gap > 3 * interval for gap in gaps) if finite_number(interval) else None,
            "rss_mib": distribution([record.get("rss_mib") for record in samples]),
            "process_start_local": next(iter(identities)) if len(identities) == 1 else None,
            "approximate_start_offset_after_driver_start_seconds": approximate_sampler_offset,
            "offset_note": "driver terminal elapsed minus sampler terminal-observation elapsed; undershoots actual start delay by terminal polling latency; not clock synchronization",
            "approximate_active_coverage": approximate_active_coverage,
            "active_coverage": utc_active_coverage if utc_active_coverage is not None else approximate_active_coverage,
            "sampler_wall_minus_elapsed_drift_seconds": clock_drift(samples),
        },
        "interpretation_limits": [
            "Record consistency is not a universal memory/stability pass threshold.",
            "Memory trends compare equivalent phase checkpoints; positive slope alone does not prove a leak.",
            "Periodic maximum is sampled RSS, not the true process peak or GPU allocation ownership.",
            "UI pointer validation and numeric-reference comparisons require their separate evidence.",
            "No native presentation-latency p95 is inferred from driver wall time or RSS samples.",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--minimum-duration", type=float, default=3600.0)
    parser.add_argument("--minimum-cycles", type=int, default=30)
    args = parser.parse_args()
    if not math.isfinite(args.minimum_duration) or args.minimum_duration < 0 or args.minimum_cycles < 1:
        parser.error("minimum duration must be finite/nonnegative and cycles must be positive")
    try:
        result = analyze(read_jsonl(args.driver), read_jsonl(args.samples), args.minimum_duration, args.minimum_cycles)
        with args.output.open("x", encoding="utf-8") as stream:
            json.dump(result, stream, indent=2, allow_nan=False)
            stream.write("\n")
    except (OSError, ValueError) as error:
        parser.exit(1, f"Analysis failed: {error}\n")
    print(json.dumps({"output": str(args.output), "record_consistency_ok": result["record_consistency_ok"], "errors": result["record_consistency_errors"]}))
    return 0 if result["record_consistency_ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
