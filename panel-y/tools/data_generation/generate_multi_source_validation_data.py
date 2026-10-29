"""Generate deterministic Parquet fixtures for multi-source Rust validation.

The two sources intentionally have different time ranges, sample rates, row
counts, and source-specific channels. Shared ``iu`` and ``gate_pwm`` channels
support same-name Line and Step overlay checks.
"""

from __future__ import annotations

import argparse
import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq


from _paths import DEFAULT_DATA_DIR as DATA_DIR


@dataclass(frozen=True)
class SourceSpec:
    filename: str
    start_time: float
    sample_rate: float
    row_count: int
    channels: tuple[str, ...]

    @property
    def end_time(self) -> float:
        return self.start_time + (self.row_count - 1) / self.sample_rate


SOURCE_1 = SourceSpec(
    filename="multi_source_validation_1.parquet",
    start_time=0.0,
    sample_rate=50_000.0,
    row_count=100_001,
    channels=("iu", "torque", "gate_pwm"),
)

SOURCE_2 = SourceSpec(
    filename="multi_source_validation_2.parquet",
    start_time=0.5,
    sample_rate=64_000.0,
    row_count=80_001,
    channels=("iu", "speed", "gate_pwm"),
)


def _time_values(spec: SourceSpec) -> np.ndarray:
    return (
        np.arange(spec.row_count, dtype=np.float64) / spec.sample_rate
        + spec.start_time
    )


def _source_1_values(time: np.ndarray) -> dict[str, np.ndarray]:
    return {
        "iu": (10.0 * np.sin(2.0 * np.pi * 50.0 * time)).astype(np.float32),
        "torque": (
            3.0 + 1.25 * np.sin(2.0 * np.pi * 2.0 * time + np.pi / 8.0)
        ).astype(np.float32),
        "gate_pwm": (
            5.0 * (np.mod(time * 1_000.0, 1.0) < 0.30)
        ).astype(np.float32),
    }


def _source_2_values(time: np.ndarray) -> dict[str, np.ndarray]:
    local_time = time - SOURCE_2.start_time
    return {
        "iu": (
            0.4 + 8.25 * np.sin(2.0 * np.pi * 50.0 * time + np.pi / 5.0)
        ).astype(np.float32),
        "speed": (
            1_200.0
            + 125.0 * np.sin(2.0 * np.pi * 1.5 * local_time + np.pi / 7.0)
        ).astype(np.float32),
        "gate_pwm": (
            5.0 * (np.mod(local_time * 800.0 + 0.17, 1.0) < 0.42)
        ).astype(np.float32),
    }


def _schema(spec: SourceSpec) -> pa.Schema:
    return pa.schema(
        [pa.field("time", pa.float64())]
        + [pa.field(channel, pa.float32()) for channel in spec.channels]
    )


def _validate(
    spec: SourceSpec, time: np.ndarray, values: dict[str, np.ndarray]
) -> None:
    if time.dtype != np.float64:
        raise TypeError(f"{spec.filename}: time must be float64")
    if len(time) != spec.row_count:
        raise ValueError(f"{spec.filename}: unexpected time row count")
    if not np.all(np.isfinite(time)):
        raise ValueError(f"{spec.filename}: time contains NaN or Inf")
    if not np.all(np.diff(time) > 0.0):
        raise ValueError(f"{spec.filename}: time must be strictly increasing")
    if time[0] != spec.start_time or time[-1] != spec.end_time:
        raise ValueError(f"{spec.filename}: unexpected time range")
    if tuple(values) != spec.channels:
        raise ValueError(f"{spec.filename}: unexpected channel list")

    for channel, channel_values in values.items():
        if channel_values.dtype != np.float32:
            raise TypeError(f"{spec.filename}: {channel} must be float32")
        if len(channel_values) != spec.row_count:
            raise ValueError(f"{spec.filename}: {channel} row count mismatch")
        if not np.all(np.isfinite(channel_values)):
            raise ValueError(f"{spec.filename}: {channel} contains NaN or Inf")


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _write_fixture(
    spec: SourceSpec,
    values_factory: Callable[[np.ndarray], dict[str, np.ndarray]],
    output_dir: Path = DATA_DIR,
) -> Path:
    time = _time_values(spec)
    values = values_factory(time)
    _validate(spec, time, values)

    table = pa.table({"time": time, **values}, schema=_schema(spec))
    output = output_dir / spec.filename
    pq.write_table(
        table,
        output,
        compression="zstd",
        use_dictionary=False,
        write_statistics=True,
    )
    return output


def _print_summary(spec: SourceSpec, path: Path) -> None:
    print(f"path: {path}")
    print(f"row count: {spec.row_count:,}")
    print(f"time range: {spec.start_time:.6f} .. {spec.end_time:.6f} s")
    print(f"sample rate: {spec.sample_rate:,.0f} Hz")
    print(f"channels: {', '.join(spec.channels)}")
    print(f"file size: {path.stat().st_size:,} bytes")
    print(f"SHA-256: {_sha256(path)}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate deterministic two-source Parquet validation fixtures."
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="overwrite both output files if either already exists",
    )
    parser.add_argument("--output-dir", type=Path, default=DATA_DIR, help="Output directory (default: repository fixture directory)")
    args = parser.parse_args()

    outputs = [args.output_dir / spec.filename for spec in (SOURCE_1, SOURCE_2)]
    existing = [path for path in outputs if path.exists()]
    if existing and not args.force:
        parser.error(
            "output already exists; pass --force to overwrite: "
            + ", ".join(str(path) for path in existing)
        )

    args.output_dir.mkdir(parents=True, exist_ok=True)
    generated = (
        (SOURCE_1, _write_fixture(SOURCE_1, _source_1_values, args.output_dir)),
        (SOURCE_2, _write_fixture(SOURCE_2, _source_2_values, args.output_dir)),
    )

    for index, (spec, path) in enumerate(generated):
        if index:
            print()
        _print_summary(spec, path)


if __name__ == "__main__":
    main()
