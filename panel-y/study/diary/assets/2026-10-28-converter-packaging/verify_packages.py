"""Verify downloaded distribution ZIPs against the current converter sources."""

import hashlib
import json
from pathlib import Path
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[4]
DIST = ROOT / "tools/preprocess/dist"


def digest(stream):
    result = hashlib.sha256()
    for block in iter(lambda: stream.read(1024 * 1024), b""):
        result.update(block)
    return result.hexdigest()


def check(target):
    name = f"panel-y-converters-{target}"
    archive = DIST / f"{name}.zip"
    with archive.open("rb") as stream:
        archive_hash = digest(stream)
    crlf_sources = []
    assert archive_hash == Path(str(archive) + ".sha256").read_text().split()[0]
    with zipfile.ZipFile(archive) as contents:
        assert contents.testzip() is None
        prefix = name + "/"
        assert all(path.startswith(prefix) and ".." not in Path(path).parts for path in contents.namelist())
        info = json.loads(contents.read(prefix + "build-info.json"))
        assert info["packaged_test_suites_passed"] == 6
        assert "Ran 6 tests" in contents.read(prefix + "tests.txt").decode("utf-8")
        assert contents.read(prefix + "tests.txt").decode("utf-8").strip().endswith("OK")
        assert prefix + "LICENSES/Python-LICENSE.txt" in contents.namelist()
        assert any("LICENSES/numpy/" in name for name in contents.namelist())
        assert any("LICENSES/pyarrow/" in name for name in contents.namelist())
        for path, expected in info["source_sha256"].items():
            data = (ROOT / path.replace("\\", "/")).read_bytes()
            if hashlib.sha256(data).hexdigest() != expected:
                # Git's Windows checkout converted the Python sources to CRLF.
                assert target.startswith("windows"), path
                crlf = data.replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
                assert hashlib.sha256(crlf).hexdigest() == expected, path
                crlf_sources.append(path)
        for binary, expected in info["executables_sha256"].items():
            with contents.open(prefix + binary) as stream:
                assert digest(stream) == expected, binary
            with contents.open(prefix + binary) as stream:
                header = stream.read(4)
                if target.startswith("windows"):
                    assert header[:2] == b"MZ"
                    stream.seek(0x3c)
                    pe_offset = int.from_bytes(stream.read(4), "little")
                    stream.seek(pe_offset)
                    assert stream.read(4) == b"PE\0\0"
                    assert int.from_bytes(stream.read(2), "little") == 0x8664
                else:
                    assert header == b"\xcf\xfa\xed\xfe"
                    expected_cpu = 0x100000c if target.endswith("arm64") else 0x1000007
                    assert int.from_bytes(stream.read(4), "little") == expected_cpu
                    mode = contents.getinfo(prefix + binary).external_attr >> 16
                    assert mode & 0o111, "macOS executable permission missing"
    return {"target": target, "archive": str(archive), "bytes": archive.stat().st_size,
            "sha256": archive_hash, "build_info": info,
            "windows_crlf_sources": crlf_sources, "verified": True}


print(json.dumps([check(target) for target in sys.argv[1:]], ensure_ascii=False, indent=2))
