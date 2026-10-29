"""Build and verify native converter executables, then assemble a distribution ZIP."""

import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import sysconfig
import zipfile


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
DIST = HERE / "dist"
PROGRAMS = {"panel-y-convert": "convert.py", "multiconverter": "multiconverter.py"}
PACKAGED_TESTS = [
    "test_csv_time_normalization_filter_and_interpolation",
    "test_mat_classic_and_hdf5_conversion",
    "test_parquet_input",
    "test_single_file_default_unit_is_seconds",
    "test_explicit_time_units_single_and_batch",
    "test_batch_skip_force_and_continue_after_failure",
]


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def copy_licenses(staging):
    destination = staging / "LICENSES"
    destination.mkdir()
    for distribution in importlib.metadata.distributions():
        for entry in distribution.files or []:
            parts = entry.parts
            if not parts or not parts[0].endswith(".dist-info"):
                continue
            if not any(word in str(entry).lower() for word in ["license", "copying", "notice"]):
                continue
            source = Path(distribution.locate_file(entry))
            if source.is_file():
                target = destination / distribution.metadata["Name"] / Path(*parts[1:])
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, target)
    for source in [Path(sys.base_prefix) / "LICENSE.txt",
                   Path(sysconfig.get_path("stdlib")) / "LICENSE.txt"]:
        if source.is_file():
            shutil.copy2(source, destination / "Python-LICENSE.txt")
            return
    raise RuntimeError("Python LICENSE.txt was not found in the build environment")


def main():
    if sys.platform not in {"darwin", "win32"}:
        raise SystemExit("Build this package on macOS or Windows.")
    system = "macos" if sys.platform == "darwin" else "windows"
    machine = platform.machine().lower()
    architecture = {"amd64": "x64", "x86_64": "x64", "aarch64": "arm64"}.get(machine, machine)
    if architecture not in {"x64", "arm64"}:
        raise SystemExit(f"Unsupported CPU architecture: {machine}")
    suffix = ".exe" if system == "windows" else ""
    name = f"panel-y-converters-{system}-{architecture}"
    staging = DIST / name
    archive = DIST / f"{name}.zip"
    # An unsuccessful rebuild must not leave an old ZIP looking like its result.
    archive.unlink(missing_ok=True)
    (DIST / f"{name}.zip.sha256").unlink(missing_ok=True)

    for program, script in PROGRAMS.items():
        subprocess.run([
            sys.executable, "-m", "PyInstaller", "--noconfirm", "--onefile",
            "--python-option", "X utf8",
            "--name", program, "--clean", "--distpath", str(DIST),
            "--workpath", str(HERE / "build"), "--specpath", str(HERE), script,
        ], cwd=HERE, check=True)

    environment = dict(os.environ, PANEL_Y_CONVERTER_BIN_DIR=str(DIST),
                       PYTHONDONTWRITEBYTECODE="1", PYTHONUTF8="1")
    tests = subprocess.run([
        sys.executable, "-m", "unittest", "-v",
        *[f"tools.tests.test_tools.ToolIntegrationTests.{test}" for test in PACKAGED_TESTS],
    ], cwd=ROOT, env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        text=True, encoding="utf-8", errors="replace", timeout=1800)
    DIST.mkdir(parents=True, exist_ok=True)
    (DIST / f"{name}-tests.txt").write_text(tests.stdout, encoding="utf-8")
    print(tests.stdout, flush=True)
    tests.check_returncode()

    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir()
    checksums = {}
    for program in PROGRAMS:
        binary = DIST / (program + suffix)
        shutil.copy2(binary, staging / binary.name)
        checksums[binary.name] = sha256(binary)
    shutil.copy2(HERE / "DISTRIBUTION.md", staging / "README.md")
    copy_licenses(staging)
    (staging / "tests.txt").write_text(tests.stdout, encoding="utf-8")
    (staging / "SHA256SUMS.txt").write_text(
        "".join(f"{digest}  {filename}\n" for filename, digest in checksums.items()), encoding="utf-8")

    revision = subprocess.run(["git", "describe", "--always", "--dirty"], cwd=ROOT,
                              capture_output=True, text=True, check=True).stdout.strip()
    sources = sorted({HERE / script for script in PROGRAMS.values()} | set((HERE / "utils").glob("*.py")))
    metadata = {
        "revision": revision, "platform": platform.platform(), "architecture": architecture,
        "python": sys.version, "signed_for_distribution": False,
        "versions": {package: importlib.metadata.version(package)
                     for package in ["pyinstaller", "numpy", "pandas", "scipy", "pyarrow", "h5py"]},
        "source_sha256": {str(path.relative_to(ROOT)): sha256(path) for path in sources},
        "executables_sha256": checksums, "packaged_test_suites_passed": len(PACKAGED_TESTS),
    }
    (staging / "build-info.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
        for path in sorted(staging.rglob("*")):
            if path.is_file():
                output.write(path, arcname=f"{name}/{path.relative_to(staging).as_posix()}")
    (DIST / f"{name}.zip.sha256").write_text(f"{sha256(archive)}  {archive.name}\n", encoding="utf-8")
    print(f"Package: {archive}\nSize: {archive.stat().st_size:,} bytes", flush=True)


if __name__ == "__main__":
    main()
