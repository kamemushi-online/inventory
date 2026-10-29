#!/usr/bin/env python3
"""Sample RSS for one explicitly selected process; never replace an existing log.

Usage (run where `ps` is permitted):
    python3 sampler.py --pid 12345 --output native-memory.jsonl --events driver.jsonl

The default run lasts 3900 seconds and samples every 2 seconds. Each `ps` query
checks the process start time as well as its PID, so a reused PID is rejected.
Only PID, start time, RSS and CPU percentage are queried; command lines and
environment variables are never recorded. RSS is sampled, not a true peak.

With --events, a matching-PID driver `complete`/`failed` record ends sampling;
the duration becomes a watchdog and reaching it without completion is an error.
Exit status: 0 = requested duration or driver completed; 1 = error/timeout;
130/143 = interrupted by SIGINT/SIGTERM. An `end` record describes the outcome.
"""

import argparse
import datetime as dt
import json
import math
import os
import signal
import subprocess
import sys
import time
from pathlib import Path


def positive_float(value):
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("must be a positive finite number")
    return number


def positive_pid(value):
    number = int(value)
    if number <= 0:
        raise argparse.ArgumentTypeError("PID must be greater than zero")
    return number


def timestamp():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def query_process(pid):
    env = os.environ.copy()
    env["LC_ALL"] = "C"
    before = time.monotonic_ns()
    try:
        result = subprocess.run(
            ["ps", "-p", str(pid), "-o", "lstart=", "-o", "rss=", "-o", "%cpu="],
            capture_output=True,
            text=True,
            check=False,
            timeout=5,
            env=env,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise RuntimeError(f"selected-process query failed: {type(error).__name__}") from error
    after = time.monotonic_ns()
    lines = [line.strip() for line in result.stdout.splitlines() if line.strip()]
    if result.returncode != 0 or len(lines) != 1:
        # Do not treat permission failures as proof of process death.
        raise RuntimeError(
            "selected process unavailable or ps query denied "
            f"(exit={result.returncode}, rows={len(lines)})"
        )
    fields = lines[0].split()
    if len(fields) != 7:
        raise RuntimeError("unexpected selected-process ps output format")
    start_identity = " ".join(fields[:5])
    try:
        dt.datetime.strptime(start_identity, "%a %b %d %H:%M:%S %Y")
        rss_kib = int(fields[5])
        cpu_percent = float(fields[6])
    except ValueError as error:
        raise RuntimeError("invalid selected-process ps values") from error
    if rss_kib < 0 or not math.isfinite(cpu_percent) or cpu_percent < 0:
        raise RuntimeError("invalid selected-process memory/CPU measurement")
    return {
        "process_start_local": start_identity,
        "rss_kib": rss_kib,
        "rss_mib": rss_kib / 1024.0,
        "cpu_percent_ps": cpu_percent,
        "query_started_monotonic_ns": before,
        "query_finished_monotonic_ns": after,
        "query_duration_ms": (after - before) / 1_000_000,
    }


class DriverEvents:
    """Read only newly appended complete JSONL records, keeping a partial line."""

    def __init__(self, path, pid):
        self.path = path
        self.pid = pid
        self.position = 0
        self.partial = b""
        self.identity = None

    def terminal_event(self):
        try:
            with self.path.open("rb") as stream:
                stat = os.fstat(stream.fileno())
                identity = (stat.st_dev, stat.st_ino)
                if self.identity is not None and identity != self.identity:
                    raise RuntimeError("driver event file was replaced")
                if stat.st_size < self.position:
                    raise RuntimeError("driver event file was truncated")
                self.identity = identity
                stream.seek(self.position)
                data = stream.read()
                self.position = stream.tell()
        except FileNotFoundError:
            if self.identity is not None:
                raise RuntimeError("driver event file disappeared")
            return None
        lines = (self.partial + data).split(b"\n")
        self.partial = lines.pop()
        terminal = None
        for line in lines:
            if not line.strip():
                continue
            try:
                event = json.loads(line)
            except (ValueError, UnicodeDecodeError) as error:
                raise RuntimeError("driver event file contains invalid JSONL") from error
            if event.get("pid") != self.pid:
                raise RuntimeError("driver event PID does not match selected process")
            if event.get("event") in ("complete", "failed"):
                terminal = event
        return terminal


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid", required=True, type=positive_pid)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--duration", type=positive_float, default=3900.0)
    parser.add_argument("--interval", type=positive_float, default=2.0)
    parser.add_argument("--events", type=Path, help="Optional driver JSONL completion/error log")
    args = parser.parse_args()
    try:
        stream = args.output.open("x", encoding="utf-8", buffering=1)
    except OSError as error:
        parser.exit(1, f"Cannot create new output log: {error}\n")

    interrupted = []

    def on_signal(signum, _frame):
        interrupted.append(signum)

    old_handlers = {
        sig: signal.signal(sig, on_signal) for sig in (signal.SIGINT, signal.SIGTERM)
    }
    started_ns = time.monotonic_ns()
    deadline_ns = started_ns + round(args.duration * 1_000_000_000)
    interval_ns = max(1, round(args.interval * 1_000_000_000))
    samples = 0
    peak_rss_mib = None
    identity = None
    outcome = "error"
    error_message = None
    exit_code = 1
    driver_events = DriverEvents(args.events, args.pid) if args.events else None

    def write(event, **data):
        now_ns = time.monotonic_ns()
        stream.write(json.dumps({
            "event": event,
            "timestamp_utc": timestamp(),
            "monotonic_ns": now_ns,
            "elapsed_seconds": (now_ns - started_ns) / 1_000_000_000,
            "pid": args.pid,
            **data,
        }, allow_nan=False) + "\n")

    try:
        write(
            "start", schema_version=1, duration_seconds=args.duration,
            interval_seconds=args.interval, sampler_pid=os.getpid(),
            started_monotonic_ns=started_ns,
            driver_events=str(args.events) if args.events else None,
            rss_note="Sampled RSS, not a true maximum or allocation ownership measurement",
            cpu_note="ps %cpu is not an interval CPU measurement",
        )
        next_sample_ns = started_ns
        while not interrupted:
            now_ns = time.monotonic_ns()
            if now_ns < next_sample_ns:
                time.sleep(min((next_sample_ns - now_ns) / 1_000_000_000, 0.2))
                continue
            terminal = driver_events.terminal_event() if driver_events else None
            if terminal is not None:
                write("driver_terminal", driver_event=terminal)
                if terminal["event"] == "failed":
                    raise RuntimeError("driver reported failure; see driver event log")
                # Query once more so a successful terminal record cannot conceal
                # a changed/dead process before the final memory observation.
            measurement = query_process(args.pid)
            measured_identity = measurement["process_start_local"]
            if identity is None:
                identity = measured_identity
                write("identity", process_start_local=identity)
            elif identity != measured_identity:
                raise RuntimeError("selected PID was reused: process start identity changed")
            samples += 1
            peak_rss_mib = max(peak_rss_mib or 0.0, measurement["rss_mib"])
            write("sample", sample_index=samples, **measurement)
            if terminal is not None:
                outcome = "driver_completed"
                exit_code = 0
                break
            if time.monotonic_ns() >= deadline_ns:
                if driver_events:
                    outcome = "timeout"
                    error_message = "driver did not complete before sampler watchdog"
                    break
                outcome = "completed"
                exit_code = 0
                break
            # Preserve the cadence without issuing a burst after a delayed query.
            now_ns = time.monotonic_ns()
            next_slot = max(1, (now_ns - started_ns) // interval_ns + 1)
            next_sample_ns = min(started_ns + next_slot * interval_ns, deadline_ns)
        if interrupted:
            outcome = "interrupted"
            exit_code = 128 + interrupted[0]
    except (RuntimeError, OSError) as error:
        error_message = str(error)
    finally:
        write(
            "end", outcome=outcome, exit_code=exit_code, sample_count=samples,
            process_start_local=identity, sampled_peak_rss_mib=peak_rss_mib,
            error=error_message,
        )
        stream.flush()
        stream.close()
        for sig, old in old_handlers.items():
            signal.signal(sig, old)
    print(json.dumps({
        "output": str(args.output), "outcome": outcome, "sample_count": samples,
        "elapsed_seconds": (time.monotonic_ns() - started_ns) / 1_000_000_000,
        "error": error_message,
    }, allow_nan=False))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
