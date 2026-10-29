"""Shared fixture location, independent of the caller's working directory."""

from pathlib import Path

DEFAULT_DATA_DIR = Path(__file__).resolve().parents[2] / "data"
