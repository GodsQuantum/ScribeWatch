"""End-to-end fake-engine lifecycle test of the authentic ScribeWatch binary.

It never downloads weights or touches Cloud9 production. Required before
shipping a full native installer with actual engine binaries.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import socket
import stat
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.error
import urllib.request


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


FAKE_ENGINE = '''#!/usr/bin/env python3
import json,sys
from http.server import BaseHTTPRequestHandler, HTTPServer
port=int(sys.argv[sys.argv.index('--port')+1])
class Handler(BaseHTTPRequestHandler):
 def do_GET(self):
  if self.path=='/health': data={'status':'ok'}
  elif self.path=='/v1/models': data={'data':[{'id':'handy-computer/parakeet-ultra-gguf'}]}
  elif self.path=='/models' or self.path.startswith('/models?'): data={'models':[{'id':'LiquidAI/LFM3-1.7B-GGUF:Q4_K_M'}]}
  else: self.send_error(404);return
  payload=json.dumps(data).encode()
  self.send_response(200);self.send_header('Content-Length',str(len(payload)))
  self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(payload)
 def do_POST(self):
  if self.path.endswith('/variants'):
   payload=b'{"variants":[{"label":"Q8_0","filename":"mock-Q8_0.gguf"}]}'
  elif self.path.endswith('/install'):
   payload=b'{"status":"installed"}'
  elif self.path=='/models' or self.path.startswith('/v1/models/'):
   payload=b'{"success":true}'
  else:self.send_error(404);return
  self.send_response(200);self.send_header('Content-Length',str(len(payload)))
  self.end_headers();self.wfile.write(payload)
 def log_message(self,*args):pass
HTTPServer(('127.0.0.1',port),Handler).serve_forever()
'''


def request(root: str, path: str, body: dict | None = None, *, origin: str | None = None) -> tuple[int, dict]:
    headers = {"Content-Type": "application/json"}
    if origin:
        headers["Origin"] = origin
    req = urllib.request.Request(
        root + path,
        method="POST" if body is not None else "GET",
        data=json.dumps(body).encode() if body is not None else None,
        headers=headers,
    )
    try:
        with urllib.request.urlopen(req, timeout=125) as response:
            return response.status, json.loads(response.read())
    except urllib.error.HTTPError as error:
        try:
            return error.code, {}
        finally:
            error.close()


class EmbeddedRuntimeTest(unittest.TestCase):
    @unittest.skipUnless(os.name == "posix", "Uses fake executable shell launchers")
    def test_real_backend_dual_engine_modes_and_model_proxy(self) -> None:
        binary = Path(os.environ.get("SCRIBEWATCH_TEST_BINARY", "target/debug/scribewatch")).resolve()
        if not binary.is_file():
            self.skipTest("Run cargo build --bin scribewatch before embedded E2E")
        for port in (38005, 38080):
            with socket.socket() as sock:
                self.assertEqual(sock.connect_ex(("127.0.0.1", port)), 111, "Port busy: skip invasive test")
        with tempfile.TemporaryDirectory(prefix="scribewatch-embedded-e2e-") as tmp:
            root = Path(tmp)
            for folder in ("config", "data", "source", "engines/speaches", "engines/llm"):
                (root / folder).mkdir(parents=True, exist_ok=True)
            for file in (
                root / "engines/speaches/Cloud9-Speaches",
                root / "engines/llm/llama-server",
            ):
                file.write_text(FAKE_ENGINE)
                file.chmod(file.stat().st_mode | stat.S_IXUSR)
            port = free_port()
            origin = "http://127.0.0.1:" + str(port)
            env = os.environ | {
                "SCRIBEWATCH_HOST": "127.0.0.1",
                "SCRIBEWATCH_PORT": str(port),
                "SCRIBEWATCH_CONFIG_DIR": str(root / "config"),
                "SCRIBEWATCH_DATA_DIR": str(root / "data"),
                "SCRIBEWATCH_DIST_DIR": str(root / "source"),
                "SCRIBEWATCH_ALLOWED_ROOTS": str(root / "source"),
                "SCRIBEWATCH_DESKTOP_MODE": "1",
                "SCRIBEWATCH_EMBEDDED_DIR": str(root / "engines"),
            }
            with (root / "stderr.log").open("w") as logs:
                process = subprocess.Popen([str(binary)], env=env, stdout=logs, stderr=logs)
                try:
                    for _ in range(80):
                        try:
                            health, _ = request(origin, "/api/v1/health")
                            if health == 200:
                                break
                        except OSError:
                            time.sleep(0.1)
                    else:
                        self.fail("ScribeWatch backend not ready")
                    _, initial = request(origin, "/api/v1/embedded/status")
                    self.assertTrue(initial["desktop"])
                    self.assertTrue(all(x["bundled"] for x in initial["engines"]))
                    self.assertTrue(all(x["mode"] == "external" for x in initial["engines"]))
                    provider_code, provider_saved = request(origin, "/api/v1/llm-providers", {
                        "name": "External local test LLM",
                        "chatCompletionsUrl": "http://127.0.0.1:11434/v1/chat/completions",
                        "model": "fixture-model",
                        "apiKey": "",
                        "timeoutSeconds": 30,
                        "enabled": True,
                    })
                    self.assertEqual(provider_code, 200)
                    self.assertEqual(provider_saved["name"], "External local test LLM")
                    for kind in ("stt", "llm"):
                        code, status = request(origin, "/api/v1/embedded/mode",
                            {"kind":kind,"mode":"internal"}, origin=origin)
                        self.assertEqual(code, 200)
                        self.assertTrue(next(x for x in status["engines"] if x["kind"] == kind)["running"])
                        code, models = request(origin, "/api/v1/embedded/models/" + kind)
                        self.assertEqual(code, 200)
                        self.assertTrue(models.get("data") or models.get("models"))
                    code, choices = request(origin, "/api/v1/embedded/stt/variants",
                        {"source":"handy-computer/parakeet-ultra-gguf"}, origin=origin)
                    self.assertEqual(code, 200)
                    self.assertEqual(choices["variants"][0]["filename"], "mock-Q8_0.gguf")
                    code, started = request(origin, "/api/v1/embedded/download", {
                        "kind":"stt","model":"handy-computer/parakeet-ultra-gguf"
                    }, origin=origin)
                    self.assertEqual(code, 200)
                    self.assertEqual(started["status"], "downloading")
                    for _ in range(30):
                        _, progress = request(origin, "/api/v1/embedded/status")
                        if progress["download"]["status"] != "downloading":
                            break
                        time.sleep(.08)
                    self.assertEqual(progress["download"]["status"], "installed")
                    code, started = request(origin, "/api/v1/embedded/download", {
                        "kind":"llm","model":"LiquidAI/LFM3-1.7B-GGUF:Q4_K_M"
                    }, origin=origin)
                    self.assertEqual(code, 200)
                    for _ in range(30):
                        _, progress = request(origin, "/api/v1/embedded/status")
                        if progress["download"]["status"] != "downloading":
                            break
                        time.sleep(.08)
                    self.assertEqual(progress["download"]["status"], "download-started")
                    # A bogus/no-ack response must not be misreported as fully installed.
                    invalid, _ = request(origin, "/api/v1/embedded/mode",
                        {"kind":"llm","mode":"disabled"}, origin="https://evil.invalid")
                    self.assertEqual(invalid, 400)
                    code, _ = request(origin, "/api/v1/embedded/mode",
                        {"kind":"llm","mode":"external"}, origin=origin)
                    self.assertEqual(code, 200)
                    code, done = request(origin, "/api/v1/embedded/status")
                    self.assertEqual(code, 200)
                    self.assertEqual(next(x for x in done["engines"] if x["kind"] == "llm")["mode"], "external")
                    _, providers = request(origin, "/api/v1/llm-providers")
                    self.assertEqual(len(providers), 1, "Switching modes must not remove external providers")
                    self.assertEqual(providers[0]["name"], "External local test LLM")
                    print("NATIVE_EMBEDDED_STT_LLM_START_MODELS_STOP_SECURITY_PASS")
                finally:
                    process.terminate()
                    try:
                        process.wait(timeout=8)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=8)


if __name__ == "__main__":
    unittest.main()
