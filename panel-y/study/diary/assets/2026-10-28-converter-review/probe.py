"""Small, synthetic probes of the existing converter; does not change its code."""

import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import warnings

import numpy as np
import pandas as pd
import pyarrow

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT))

from tools.preprocess.utils.converter import convert_to_parquet
from tools.preprocess.utils.reader_mat import _mat_dict_to_df
from tools.preprocess.utils.resampler import check_uniform


def convert_case(directory, name, time, values=None, unit="s"):
    source = directory / f"{name}.parquet"
    output = directory / f"{name}-out.parquet"
    frame = pd.DataFrame({"time": time, "value": values if values is not None else np.arange(len(time), dtype=float)})
    frame.to_parquet(source, index=False)
    record = {"name": name, "input": frame.to_dict(orient="list"), "unit_argument": unit}
    log = io.StringIO()
    with warnings.catch_warnings(record=True) as caught, contextlib.redirect_stdout(log):
        warnings.simplefilter("always")
        record["input_check_uniform"] = bool(check_uniform(frame["time"]))
        try:
            convert_to_parquet(source, output, time_unit=unit)
            record["status"] = "success"
        except Exception as error:
            record["status"] = "error"
            record["error"] = f"{type(error).__name__}: {error}"
        record["output_exists"] = output.exists()
        if output.exists():
            result = pd.read_parquet(output)
            record["output"] = result.to_dict(orient="list")
            record["output_dtypes"] = {key: str(value) for key, value in result.dtypes.items()}
            record["output_check_uniform"] = bool(check_uniform(result["time"]))
        record["warnings"] = [str(item.message) for item in caught]
    record["log"] = log.getvalue()
    return record


with tempfile.TemporaryDirectory(prefix="panel-y-converter-review-") as temp:
    directory = Path(temp)
    cases = [
        ("seconds_1200_autodetect", [0, 600, 1200], None, None),
        ("milliseconds_500_autodetect", [0, 250, 500], None, None),
        ("milliseconds_explicit", [1000, 1001, 1003, 1006], [0, 2, 6, 12], "ms"),
        ("digital", [0., 1., 3., 6.], [0, 0, 1, 1], "s"),
        ("peak", [0., 1., 3., 6.], [0, 10, 0, 0], "s"),
        ("gap", [0., 1., 2., 100.], [0, 1, 2, 100], "s"),
        ("descending_irregular", [3., 2., 0.], None, "s"),
        ("unsorted", [0., 2., 1., 3.], [0, 20, 10, 30], "s"),
        ("duplicate", [0., 1., 1., 2.], [0, 10, 20, 30], "s"),
        ("constant_time", [1., 1., 1.], [10, 20, 30], "s"),
        ("missing_time", [0., 1., float("nan"), 3., 4.], None, "s"),
        ("infinite_endpoint", [0., 1., float("inf")], None, "s"),
        ("one_row", [0.], [7.], "s"),
        ("empty", [], [], "s"),
        ("epoch_microsecond_grid", (1_700_000_000. + np.arange(10) * 1e-6).tolist(), None, "s"),
        ("boolean_channel", [0., 1., 2.], [False, True, False], "s"),
    ]
    records = [convert_case(directory, *case) for case in cases]

mat_frame = _mat_dict_to_df({"time": np.arange(4.), "signal": np.arange(4.), "scalar": np.array([[100]])})
report = {
    "python": sys.version,
    "versions": {module.__name__: module.__version__ for module in (np, pd, pyarrow)},
    "cases": records,
    "mat_scalar_truncation": mat_frame.to_dict(orient="list"),
}


def clean(value):
    if isinstance(value, float) and not np.isfinite(value):
        return str(value)
    if isinstance(value, dict):
        return {key: clean(item) for key, item in value.items()}
    if isinstance(value, list):
        return [clean(item) for item in value]
    return value


print(json.dumps(clean(report), ensure_ascii=False, indent=2, allow_nan=False))
