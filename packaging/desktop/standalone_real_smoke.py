#!/usr/bin/env python3
"""CI-only native E2E: verify both *real* bundled engine servers from installed NSIS tree.

No model weights, clinical examples or microphone are needed. This does not
claim GPU inference / ASR quality certification.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def get_json(base: str, path: str, timeout: int = 5) -> dict:
    with urllib.request.urlopen(base + path, timeout=timeout) as reply:
        if reply.status != 200:
            raise RuntimeError("Unexpected API status " + str(reply.status))
        return json.load(reply)


def post_json(base: str, path: str, payload: dict, timeout: int = 150) -> dict:
    request = urllib.request.Request(
        base + path,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json", "Origin": base},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as reply:
            if reply.status != 200:
                raise RuntimeError("POST failed " + str(reply.status))
            return json.load(reply)
    except urllib.error.HTTPError as error:
        try:
            message = error.read(2048).decode("utf-8", errors="replace")
            raise RuntimeError("POST failed HTTP " + str(error.code) + ": " + message) from error
        finally:
            error.close()


def run(installed: Path) -> None:
    installed = installed.resolve(strict=True)
    server = installed / "bin" / ("scribewatch-server.exe" if os.name == "nt" else "scribewatch-server")
    bundle = installed / "bin" / "engines"
    if not server.is_file() or not bundle.is_dir():
        raise RuntimeError("Full standalone executable/engines missing")
    if not (bundle / "BUNDLED-RUNTIMES-SHA256.json").is_file():
        raise RuntimeError("Bundled native source provenance manifest missing")
    manifest = json.loads((bundle / "BUNDLED-RUNTIMES-SHA256.json").read_text(encoding="utf-8"))
    if len(manifest["files"]) < 3:
        raise RuntimeError("Bundled engines manifest unexpectedly empty")
    for kind in ("speaches", "llm"):
        if not any(name.startswith(kind + "/") for name in manifest["files"]):
            raise RuntimeError("Missing bundled " + kind)
    with tempfile.TemporaryDirectory(prefix="scribewatch-native-two-engine-smoke-") as scratch:
        root = Path(scratch)
        for folder in ("config", "data", "source"):
            (root / folder).mkdir()
        port = free_port()
        base = "http://127.0.0.1:" + str(port)
        env = os.environ | {
            "SCRIBEWATCH_HOST": "127.0.0.1",
            "SCRIBEWATCH_PORT": str(port),
            "SCRIBEWATCH_CONFIG_DIR": str(root / "config"),
            "SCRIBEWATCH_DATA_DIR": str(root / "data"),
            "SCRIBEWATCH_DIST_DIR": str(installed / "frontend"),
            "SCRIBEWATCH_ALLOWED_ROOTS": str(root / "source"),
            "SCRIBEWATCH_DESKTOP_MODE": "1",
            "SCRIBEWATCH_EMBEDDED_DIR": str(bundle),
        }
        if os.name == "nt":
            env["NO_PROXY"] = "localhost,127.0.0.1"
        with (root / "backend.log").open("w", encoding="utf-8") as log:
            creation = getattr(subprocess, "CREATE_NO_WINDOW", 0)
            process = subprocess.Popen(
                [str(server)],
                env=env,
                cwd=installed,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                creationflags=creation,
            )
            started: list[str] = []
            try:
                for i in range(100):
                    if process.poll() is not None:
                        raise RuntimeError("ScribeWatch backend exited before health")
                    try:
                        if get_json(base, "/api/v1/health", 1)["status"] == "ok":
                            break
                    except (OSError, urllib.error.URLError):
                        time.sleep(0.2)
                else:
                    raise RuntimeError("ScribeWatch desktop backend not ready")
                status = get_json(base, "/api/v1/embedded/status")
                if not status.get("desktop") or len(status.get("engines", [])) != 2:
                    raise RuntimeError("Bundled engine list missing")
                for engine in status["engines"]:
                    if not engine["bundled"] or engine["running"] or engine["mode"] != "external":
                        raise RuntimeError("Bundled engine must start disabled: " + str(engine))
                # Real native PyInstaller and llama-server binaries; GPU independent.
                for kind in ("stt", "llm"):
                    enabled = post_json(base, "/api/v1/embedded/mode", {"kind": kind, "mode": "internal"})
                    target = next(entry for entry in enabled["engines"] if entry["kind"] == kind)
                    if not target["running"]:
                        raise RuntimeError("Embedded native " + kind + " did not start")
                    listing = get_json(base, "/api/v1/embedded/models/" + kind)
                    if not isinstance(listing.get("data", listing.get("models")), list):
                        raise RuntimeError("Native " + kind + " model router unavailable")
                    started.append(kind)
                    print("NATIVE_FULL_STANDALONE_" + kind.upper() + "_PASS", flush=True)
                # Existing external provider configuration remains untouched.
                for kind in reversed(started):
                    stopped = post_json(base, "/api/v1/embedded/mode", {"kind": kind, "mode": "external"})
                    target = next(entry for entry in stopped["engines"] if entry["kind"] == kind)
                    if target["running"]:
                        raise RuntimeError("Native engine retained memory after stop")
                print("NATIVE_FULL_STANDALONE_TWO_ENGINES_STOP_PASS", flush=True)
            finally:
                # Best effort release of owned children, never kill unrelated ports.
                for kind in reversed(started):
                    try:
                        post_json(base, "/api/v1/embedded/mode", {"kind": kind, "mode": "external"}, timeout=8)
                    except Exception:
                        pass
                process.terminate()
                try:
                    process.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=8)
        if process.returncode is None:
            raise RuntimeError("Backend process did not stop")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--installed", type=Path, required=True)
    options = parser.parse_args()
    run(options.installed)


if __name__ == "__main__":
    main()
