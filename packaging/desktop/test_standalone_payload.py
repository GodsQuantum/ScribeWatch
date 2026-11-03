"""Standalone Windows installer compressed envelope, preserving every staged file."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from standalone_payload import make_payload


class PayloadTest(unittest.TestCase):
    def test_archive_contains_staged_app_and_both_engines(self) -> None:
        with tempfile.TemporaryDirectory() as base:
            root = Path(base)
            stage = root / "verified"
            files = {
                "ScribeWatch.exe": b"gui",
                "bin/scribewatch-server.exe": b"server",
                "bin/engines/speaches/Cloud9-Speaches.exe": b"speech",
                "bin/engines/llm/cpu/llama-server.exe": b"cpu",
                "bin/engines/llm/cuda/llama-server.exe": b"gpu",
            }
            for filename, data in files.items():
                path = stage / filename
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            manifest = {"files": {
                filename: hashlib.sha256(content).hexdigest()
                for filename, content in files.items()
            }}
            (stage / "manifest-sha256.json").write_text(json.dumps(manifest))
            out = root / "payload.zip"
            count, size = make_payload(stage, out)
            self.assertEqual(count, len(files) + 1)
            self.assertLess(size, 2000_000_000)
            with zipfile.ZipFile(out) as archive:
                self.assertIsNone(archive.testzip())
                self.assertEqual(set(archive.namelist()), set(files) | {"manifest-sha256.json"})
                for filename, expected in files.items():
                    self.assertEqual(archive.read(filename), expected)

    def test_input_manifest_required(self) -> None:
        with tempfile.TemporaryDirectory() as base:
            stage = Path(base) / "empty"
            stage.mkdir()
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                make_payload(stage, Path(base) / "out.zip")

    def test_overflow_removes_disposable_partial_archive(self) -> None:
        with tempfile.TemporaryDirectory() as base:
            stage = Path(base) / "stage"
            stage.mkdir()
            (stage / "ScribeWatch.exe").write_bytes(b"dummy")
            (stage / "manifest-sha256.json").write_text('{"files":{"ScribeWatch.exe":"placeholder"}}')
            output = Path(base) / "payload.zip"
            with patch("standalone_payload.MAX_NSIS_INPUT_BYTES", 1):
                with self.assertRaisesRegex(ValueError, "limit"):
                    make_payload(stage, output)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
