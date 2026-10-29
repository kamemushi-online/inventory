"""Generate viewer inputs with the existing No8/Prius simulator; no plant changes.

Usage: python generate_motor_logs.py BASE_ROOT BINARY_DIR OUTPUT_DIR REPORT
Build motor_sim_runner and sim-export from BASE_ROOT/rust/Cargo.toml first.
The output directory must not exist. These are simulated, not measured, signals.
"""

import hashlib
import json
import pathlib
import subprocess
import sys
import time


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    base, binaries, output, report = map(pathlib.Path, sys.argv[1:])
    base, binaries, output = base.resolve(), binaries.resolve(), output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / ".gitignore").write_text("*\n!.gitignore\n")
    runner, exporter = binaries / "motor_sim_rust", binaries / "sim-export"
    evidence = {
        "provenance": "No8/Prius closed-loop simulation, not physical measurement",
        "base_root": str(base),
        "base_revision": subprocess.check_output(
            ["git", "-C", str(base), "rev-parse", "HEAD"], text=True
        ).strip(),
        "plant_status": "generating; no physical-model qualification claimed",
        "runner_sha256": sha256(runner),
        "exporter_sha256": sha256(exporter),
        "runs": [],
    }
    report.parent.mkdir(parents=True, exist_ok=True)
    for name, rows in [("input", 100_000), ("reversal", 320_000), ("speed", 320_000)]:
        scenario = base / "matlab/task_shell" / f"rust_comparison_{name}.csv"
        csv_path = output / f"motor_{name}.csv"
        parquet_path = output / f"motor_{name}.parquet"
        command = [str(runner), "run", "--mode", "ipm-current", "--controller", "no8",
                   "--plant", "prius", "--scenario", str(scenario),
                   "--output-dir", str(output), "--scenario-out", str(csv_path),
                   "--output-dt", "0.000001", "--duration", "auto", "--vdc", "300",
                   "--dead-time", "0", "--no-debug"]
        export_command = [str(exporter), "csv-to-parquet", str(csv_path), str(parquet_path)]
        started = time.monotonic()
        with (output / f"{name}.log").open("w") as log:
            subprocess.run(command, cwd=base, stdout=log, stderr=subprocess.STDOUT, check=True)
            simulation_seconds = time.monotonic() - started
            subprocess.run(export_command, cwd=base, stdout=log, stderr=subprocess.STDOUT, check=True)
        with csv_path.open() as data:
            header = next(data).strip().split(",")
            actual_rows = sum(1 for _ in data)
        if actual_rows != rows:
            raise ValueError(f"{name}: expected {rows} rows, got {actual_rows}")
        run = {"name": name, "scenario": str(scenario), "scenario_sha256": sha256(scenario),
               "command": command, "export_command": export_command, "csv_columns": header,
               "rows": actual_rows, "simulation_seconds": simulation_seconds,
               "simulation_and_export_seconds": time.monotonic() - started,
               "csv": str(csv_path), "csv_sha256": sha256(csv_path),
               "parquet": str(parquet_path), "parquet_sha256": sha256(parquet_path),
               "parquet_bytes": parquet_path.stat().st_size}
        evidence["runs"].append(run)
        report.write_text(json.dumps(evidence, indent=2) + "\n")
        print(f"{name}: {actual_rows:,} rows, {run['parquet_bytes']:,} bytes, "
              f"{run['simulation_and_export_seconds']:.2f} seconds", flush=True)


if __name__ == "__main__":
    main()
