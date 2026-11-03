"""Validate real desktop staging of optional native engines and file hashes."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
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
            ffmpeg = artifact / "ffmpeg.exe"
            ffmpeg.write_text('#!/usr/bin/env python3\nprint("Test FFmpeg license")\n')
            ffmpeg.chmod(0o755)
            engines = t / "engines"
            (engines / "speaches").mkdir(parents=True)
            (engines / "llm/cpu").mkdir(parents=True)
            (engines / "speaches/Cloud9-Speaches.exe").write_text("asr")
            (engines / "llm/cpu/llama-server.exe").write_text("llm")
            (engines / "BUNDLED-RUNTIMES-SHA256.json").write_text('{"files":{}}')
            command = [
                sys.executable, str(STAGE), "--repo", str(repo),
                "--platform", "windows", "--server", str(artifact / "scribewatch-server.exe"),
                "--launcher", str(artifact / "ScribeWatch.exe"),
                "--ffmpeg", str(artifact), "--engines", str(engines),
                "--output", str(t / "installed"),
            ]
            run = subprocess.run(command, text=True, capture_output=True, check=True, timeout=30)
            self.assertIn("DESKTOP_STAGE_VERIFIED ScribeWatch", run.stdout)
            result = t / "installed"
            manifest = json.loads((result / "manifest-sha256.json").read_text())
            for item in (
                "bin/engines/speaches/Cloud9-Speaches.exe",
                "bin/engines/llm/cpu/llama-server.exe",
                "bin/engines/BUNDLED-RUNTIMES-SHA256.json",
            ):
                self.assertIn(item, manifest["files"])
                digest = hashlib.sha256((result / item).read_bytes()).hexdigest()
                self.assertEqual(manifest["files"][item], digest)


if __name__ == "__main__":
    unittest.main()
