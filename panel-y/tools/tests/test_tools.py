"""CLI integration checks. Run: python -m unittest discover -s tools/tests -v."""

from pathlib import Path
import contextlib
import io
import os
import subprocess
import sys
import tempfile
import unittest

import h5py
import numpy as np
import pandas as pd
import pyarrow.parquet as pq
from scipy.io import savemat

from tools.preprocess.utils import convert_to_parquet


ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / "tools"


class ToolIntegrationTests(unittest.TestCase):
    def setUp(self):
        prefix = "panel-y tools 日本語-" if os.environ.get("PANEL_Y_CONVERTER_BIN_DIR") else "panel-y-tools-test-"
        self.temporary = tempfile.TemporaryDirectory(prefix=prefix)
        self.addCleanup(self.temporary.cleanup)
        self.work = Path(self.temporary.name)

    def run_tool(self, script, *arguments, succeeds=True):
        binary_dir = os.environ.get("PANEL_Y_CONVERTER_BIN_DIR")
        converter_names = {"preprocess/convert.py": "panel-y-convert",
                           "preprocess/multiconverter.py": "multiconverter"}
        environment = None
        if binary_dir and script in converter_names:
            suffix = ".exe" if sys.platform == "win32" else ""
            command = [str(Path(binary_dir) / (converter_names[script] + suffix))]
            environment = {key: value for key, value in os.environ.items()
                           if not key.upper().startswith("PYTHON") and key.upper() != "VIRTUAL_ENV"}
            if sys.platform == "win32":
                system_root = Path(os.environ["SystemRoot"])
                environment["PATH"] = os.pathsep.join(map(str, [system_root / "System32", system_root]))
            else:
                environment["PATH"] = "/usr/bin:/bin:/usr/sbin:/sbin"
        else:
            command = [sys.executable, str(TOOLS / script)]
        result = subprocess.run(
            [*command, *map(str, arguments)],
            cwd=self.work,
            env=environment,
            text=True,
            encoding="utf-8" if binary_dir and script in converter_names else None,
            capture_output=True,
            timeout=60,
        )
        if succeeds:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def test_csv_time_normalization_filter_and_interpolation(self):
        pd.DataFrame({"tick": [0, 1, 3, 6], "value": [0, 2, 6, 12],
                      "label": ["a", "b", "c", "d"]}).to_csv(self.work / "input.csv", index=False)
        self.run_tool("preprocess/convert.py", "input.csv", "nested/output.parquet",
                      "--time-col", "tick", "--time-unit", "ms")
        actual = pd.read_parquet(self.work / "nested/output.parquet")
        self.assertEqual(list(actual), ["time", "value"])
        np.testing.assert_allclose(actual["time"], [0, .002, .004, .006])
        np.testing.assert_allclose(actual["value"], [0, 4, 8, 12])

    def test_mat_classic_and_hdf5_conversion(self):
        expected = {"time": np.array([0., 1., 2.]), "value": np.array([3., -2., 5.])}
        savemat(self.work / "classic.mat", expected)
        with h5py.File(self.work / "hdf5.mat", "w") as file:
            for name, values in expected.items():
                file.create_dataset(name, data=values.reshape(-1, 1))
        for name in ["classic", "hdf5"]:
            with self.subTest(name=name):
                self.run_tool("preprocess/convert.py", f"{name}.mat", f"{name}.parquet",
                              "--time-unit", "s")
                actual = pd.read_parquet(self.work / f"{name}.parquet")
                self.assertEqual(list(actual), ["time", "value"])
                for column in expected:
                    np.testing.assert_array_equal(actual[column], expected[column])

    def test_parquet_input(self):
        expected = pd.DataFrame({"time": [0., .1, .2], "current": [3., -1., 2.]})
        expected.to_parquet(self.work / "input.parquet", index=False)
        self.run_tool("preprocess/convert.py", "input.parquet", "output.parquet")
        pd.testing.assert_frame_equal(pd.read_parquet(self.work / "output.parquet"), expected)

    def test_single_file_default_unit_is_seconds(self):
        for maximum in [500., 1000., 1_000_000.]:
            expected = pd.DataFrame({"time": [0., maximum / 2, maximum],
                                     "value": [3., -1., 2.]})
            expected.to_csv(self.work / "input.csv", index=False)
            expected.to_parquet(self.work / "input.parquet", index=False)
            savemat(self.work / "input.mat", {name: expected[name].to_numpy() for name in expected})
            for suffix in ["csv", "mat", "parquet"]:
                with self.subTest(maximum=maximum, suffix=suffix):
                    self.run_tool("preprocess/convert.py", f"input.{suffix}", "output.parquet")
                    actual = pd.read_parquet(self.work / "output.parquet")
                    pd.testing.assert_frame_equal(actual, expected)

    def test_api_default_unit_is_seconds(self):
        expected = pd.DataFrame({"time": [0., 500_000., 1_000_000.], "value": [3., -1., 2.]})
        source = self.work / "input.parquet"
        output = self.work / "output.parquet"
        expected.to_parquet(source, index=False)
        for options in [{}, {"time_unit": None}]:
            with self.subTest(options=options), contextlib.redirect_stdout(io.StringIO()):
                convert_to_parquet(source, output, **options)
                pd.testing.assert_frame_equal(pd.read_parquet(output), expected)

    def test_explicit_time_units_single_and_batch(self):
        inputs = self.work / "inputs"
        inputs.mkdir()
        expected = pd.DataFrame({"time": [1., 1.25, 1.5], "value": [3., -1., 2.]})
        for unit, scale in [("s", 1), ("ms", 1000), ("us", 1_000_000)]:
            with self.subTest(unit=unit):
                source = {name: expected[name].to_numpy() for name in expected}
                source["time"] = source["time"] * scale
                savemat(inputs / "input.mat", source)
                self.run_tool("preprocess/convert.py", inputs / "input.mat", "single.parquet",
                              "--time-unit", unit)
                self.run_tool("preprocess/multiconverter.py", "--input", inputs, "--output", "batch",
                              "--time-unit", unit, "--force")
                for output in ["single.parquet", "batch/input.parquet"]:
                    pd.testing.assert_frame_equal(pd.read_parquet(self.work / output), expected)

    def test_batch_skip_force_and_continue_after_failure(self):
        inputs = self.work / "inputs"
        inputs.mkdir()
        source = {"time": np.array([0., 1., 2.]), "value": np.array([1., 2., 3.])}
        savemat(inputs / "valid.mat", source)
        self.run_tool("preprocess/multiconverter.py", "--input", inputs, "--output", "outputs")
        output = self.work / "outputs/valid.parquet"
        original = output.read_bytes()
        source["value"] *= 2
        savemat(inputs / "valid.mat", source)
        self.run_tool("preprocess/multiconverter.py", "--input", inputs, "--output", "outputs")
        self.assertEqual(output.read_bytes(), original)
        (inputs / "broken.mat").write_text("invalid MAT input")
        self.run_tool("preprocess/multiconverter.py", "--input", inputs, "--output", "outputs",
                      "--force", succeeds=False)
        np.testing.assert_array_equal(pd.read_parquet(output)["value"], [2., 4., 6.])
        self.assertFalse((self.work / "outputs/broken.parquet").exists())

    def test_sample_generation_and_overwrite_protection(self):
        script = "data_generation/generate_test_data.py"
        self.run_tool(script, "--output-dir", "samples")
        output = self.work / "samples/test_100k.parquet"
        original = output.read_bytes()
        actual = pd.read_parquet(output)
        self.assertEqual(actual.shape, (100_000, 5))
        np.testing.assert_allclose(actual["time"].iloc[[0, -1]], [0., .99999])
        self.run_tool(script, "--output-dir", "samples", succeeds=False)
        self.assertEqual(output.read_bytes(), original)
        self.run_tool(script, "--output-dir", "samples", "--force")
        pd.testing.assert_frame_equal(pd.read_parquet(output), actual)

    def test_step_generation_keeps_one_sample_offsets(self):
        script = "data_generation/generate_step_validation_data.py"
        self.run_tool(script, "--output-dir", "step")
        actual = pd.read_parquet(self.work / "step/step_validation_100k.parquet")
        self.assertEqual(actual.shape, (100_000, 4))
        np.testing.assert_array_equal(actual["pwm_1kHz_delay_1sample"].iloc[1:],
                                      actual["pwm_1kHz"].iloc[:-1])
        np.testing.assert_array_equal(actual["pwm_1kHz_advance_1sample"].iloc[:-1],
                                      actual["pwm_1kHz"].iloc[1:])
        original = (self.work / "step/step_validation_100k.parquet").read_bytes()
        self.run_tool(script, "--output-dir", "step", succeeds=False)
        self.assertEqual((self.work / "step/step_validation_100k.parquet").read_bytes(), original)

    def test_multi_source_outputs_and_pair_overwrite_protection(self):
        script = "data_generation/generate_multi_source_validation_data.py"
        self.run_tool(script, "--output-dir", "multi")
        for index, rows, first, last in [(1, 100_001, 0., 2.), (2, 80_001, .5, 1.75)]:
            actual = pq.read_table(self.work / f"multi/multi_source_validation_{index}.parquet")
            self.assertEqual(actual.num_rows, rows)
            np.testing.assert_array_equal(actual["time"].to_numpy()[[0, -1]], [first, last])
        first = self.work / "multi/multi_source_validation_1.parquet"
        second = self.work / "multi/multi_source_validation_2.parquet"
        original = second.read_bytes()
        first.unlink()
        self.run_tool(script, "--output-dir", "multi", succeeds=False)
        self.assertFalse(first.exists())
        self.assertEqual(second.read_bytes(), original)
        self.run_tool(script, "--output-dir", "multi", "--force")
        self.assertTrue(first.exists())

    def test_ripple_chunked_output(self):
        script = "data_generation/generate_ripple_validation_data.py"
        arguments = ["--output-dir", "ripple", "--rows", "10001", "--chunk-size", "2000"]
        self.run_tool(script, *arguments)
        path = self.work / "ripple/ripple_validation_2_4m.parquet"
        file = pq.ParquetFile(path)
        self.assertEqual(file.metadata.num_rows, 10_001)
        self.assertEqual(file.num_row_groups, 6)
        self.assertEqual(file.read().num_columns, 6)
        self.run_tool(script, *arguments, succeeds=False)

    def test_importing_generators_has_no_output_side_effect(self):
        code = (
            f"import sys; sys.path.insert(0, {str(TOOLS / 'data_generation')!r}); "
            "import generate_test_data, generate_step_validation_data, "
            "generate_phase1_datasets, generate_multi_source_validation_data, "
            "generate_ripple_validation_data, generate_interval_statistics_sources"
        )
        result = subprocess.run([sys.executable, "-c", code], cwd=self.work,
                                text=True, capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(list(self.work.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
