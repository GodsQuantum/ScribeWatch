"""Validate real desktop staging of optional native engines and file hashes."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
from unittest.mock import patch

import stage
from tempfile import TemporaryDirectory
import unittest

STAGE = Path(__file__).with_name("stage.py")


class StageTest(unittest.TestCase):
    def test_full_installer_has_stt_llm_and_immutable_file_manifest(self) -> None:
        with TemporaryDirectory(prefix="scribewatch-standalone-stage-") as temp:
            t = Path(temp)
            repo = t / "repo"
            (repo / "frontend/build").mkdir(parents=True)
            (repo / "frontend/build/index.html").write_text("<!doctype html>test")
            (repo / "Cargo.toml").write_text('[package]\nname="scribewatch"\nversion="0.5.3"\n')
            artifact = t / "resources"
            artifact.mkdir()
            for file in ("scribewatch-server.exe", "ScribeWatch.exe", "ffmpeg.exe", "ffprobe.exe"):
                (artifact / file).write_text("fake")
            # No fake .exe is invoked: Windows cannot launch a Python shebang
            # pretending to be a PE binary. The real FFmpeg --license probe
            # is exercised by native_pipeline.py in the subsequent CI stage.
            engines = t / "engines"
            (engines / "speaches").mkdir(parents=True)
            (engines / "llm/cpu").mkdir(parents=True)
            (engines / "speaches/Cloud9-Speaches.exe").write_text("asr")
            (engines / "llm/cpu/llama-server.exe").write_text("llm")
            (engines / "BUNDLED-RUNTIMES-SHA256.json").write_text('{"files":{}}')
            exporters = t / "exporters"
            (exporters / "fonts").mkdir(parents=True)
            (exporters / "licenses").mkdir(parents=True)
            (exporters / "pandoc.exe").write_bytes(b"portable-pandoc")
            (exporters / "typst.exe").write_bytes(b"portable-typst")
            (exporters / "fonts/NotoSansCJK-Regular.ttc").write_bytes(b"portable-font")
            (exporters / "licenses/pandoc-COPYING.md").write_text("GPL")
            (exporters / "EXPORT-TOOLS-SHA256.json").write_text('{"files_sha256":{}}')
            command = [
                sys.executable, str(STAGE), "--repo", str(repo),
                "--platform", "windows", "--server", str(artifact / "scribewatch-server.exe"),
                "--launcher", str(artifact / "ScribeWatch.exe"),
                "--ffmpeg", str(artifact), "--engines", str(engines),
                "--export-tools", str(exporters),
                "--output", str(t / "installed"),
            ]
            with patch.object(sys, "argv", command[1:]), patch(
                "subprocess.check_output", return_value="Test FFmpeg license"
            ):
                stage.main()
            result = t / "installed"
            manifest = json.loads((result / "manifest-sha256.json").read_text())
            for item in (
                "bin/engines/speaches/Cloud9-Speaches.exe",
                "bin/engines/llm/cpu/llama-server.exe",
                "bin/engines/BUNDLED-RUNTIMES-SHA256.json",
                "bin/pandoc.exe",
                "bin/typst.exe",
                "bin/fonts/NotoSansCJK-Regular.ttc",
                "bin/EXPORT-TOOLS-SHA256.json",
            ):
                self.assertIn(item, manifest["files"])
                digest = hashlib.sha256((result / item).read_bytes()).hexdigest()
                self.assertEqual(manifest["files"][item], digest)


if __name__ == "__main__":
    unittest.main()
