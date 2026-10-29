"""Generate four independent large sources and 1/2/4-source benchmark configs.

Requires numpy and pyarrow. Generated configs contain absolute data paths. Existing files are
reused only if their schema, row count and generator metadata match; never overwritten.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from generate_phase1_datasets import DATA_DIR, LARGE_SPEC, DatasetSpec, _inspect, _large_chunk, _schema


def generate(report_dir: Path, output_dir: Path = DATA_DIR) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    report_dir.mkdir(parents=True, exist_ok=True)
    sources = []
    specs = [(10_000_000, 1_000_000.0, 0.0), (8_000_000, 800_000.0, 100.0),
             (12_000_000, 1_200_000.0, -20.0), (6_000_000, 600_000.0, 1000.0)]
    for index, (rows, fs, start) in enumerate(specs):
        filename = f"panely_interval_source_{index + 1}_{rows // 1_000_000}m.parquet"
        spec = DatasetSpec("large", filename, rows, fs, LARGE_SPEC.channels, 250_000)
        provenance = json.dumps({"generator": 1, "index": index, "rows": rows, "fs": fs,
                                 "raw_start": start, "scale": 1 + 0.15 * index,
                                 "bias": 0.25 * index}, sort_keys=True).encode()
        schema = _schema(spec).with_metadata({b"interval_statistics_fixture": provenance})
        path = output_dir / filename
        if path.exists():
            existing = pq.ParquetFile(path)
            if existing.metadata.num_rows != rows or not existing.schema_arrow.equals(schema, check_metadata=True):
                raise ValueError(f"Refusing to reuse unrelated/incomplete file: {path}")
            print(f"reusing {path}", flush=True)
        else:
            with pq.ParquetWriter(path, schema, compression="zstd", compression_level=3,
                                 use_dictionary=False) as writer:
                for first in range(0, rows, spec.row_group_size):
                    count = min(spec.row_group_size, rows - first)
                    arrays = _large_chunk(first, count, spec)
                    arrays["time"] += start
                    for name in spec.channels:
                        arrays[name] = (arrays[name] * np.float32(1 + 0.15 * index)
                                        + np.float32(0.25 * index)).astype(np.float32)
                    writer.write_table(pa.table(arrays, schema=schema), row_group_size=count)
            print(f"generated {path}: {rows:,} rows", flush=True)
        info = _inspect(path)
        # Absolute paths allow the generated config to run from any working directory.
        info["path"] = str(path.resolve())
        info.update({"source_id": index + 1, "sample_rate_hz": fs, "raw_start": start,
                     "offset_seconds": -start, "scale": 1 + 0.15 * index, "bias": 0.25 * index})
        sources.append(info)
    (report_dir / "datasets.json").write_text(json.dumps(sources, indent=2) + "\n")
    for count in [1, 2, 4]:
        config = {"sources": [{"path": s["path"], "offset_seconds": s["offset_seconds"]}
                              for s in sources[:count]],
                  "cursors": [0.25, 9.5], "trials": 10, "memory_repeats": 30}
        (report_dir / f"config-{count}-sources.json").write_text(json.dumps(config, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report_dir", type=Path)
    parser.add_argument("--output-dir", type=Path, default=DATA_DIR, help="Output directory for Parquet sources")
    args = parser.parse_args()
    generate(args.report_dir, args.output_dir)
