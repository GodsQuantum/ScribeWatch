"""First-boot ScribeWatch integration without models or any cloud download."""
import json
import os
import pathlib
import socket
import subprocess
import tempfile
import time
import unittest
import urllib.error
import urllib.request

APP = pathlib.Path(__file__).resolve().parents[2] / "target" / "debug" / (
    "scribewatch.exe" if os.name == "nt" else "scribewatch"
)
APP = pathlib.Path(os.environ.get("SCRIBEWATCH_TEST_BINARY", str(APP)))


def localhost_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class NativeFirstRun(unittest.TestCase):
    def test_fresh_native_providers_and_five_profiles_without_weights(self):
        with tempfile.TemporaryDirectory(prefix="scribe-native-fixture-") as temp:
            root = pathlib.Path(temp)
            for name in ("config", "data", "engines", "frontend"):
                (root / name).mkdir()
            port = localhost_port()
            env = {
                **os.environ,
                "SCRIBEWATCH_NATIVE_AUTOINIT": "1",
                "SCRIBEWATCH_DESKTOP_MODE": "1",
                "SCRIBEWATCH_EMBEDDED_DIR": str(root / "engines"),
                # No external credentials and no actual native engines.
                "SCRIBEWATCH_LLM_CONTEXT_TOKENS": "24576",
                "SCRIBEWATCH_LLM_KV_CACHE": "q4_0",
            }
            args = [
                str(APP), "--host", "127.0.0.1", "--port", str(port),
                "--config-dir", str(root / "config"),
                "--data-dir", str(root / "data"),
                "--dist-dir", str(root / "frontend"),
                "--allowed-roots", str(root / "data"),
            ]
            proc = subprocess.Popen(args, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            base = f"http://127.0.0.1:{port}"
            try:
                for _ in range(85):
                    if proc.poll() is not None:
                        raise AssertionError("Fresh desktop server exited before ready")
                    try:
                        with urllib.request.urlopen(base + "/api/v1/ready", timeout=1.0) as resp:
                            if json.load(resp)["status"] == "ready":
                                break
                    except (OSError, ValueError):
                        time.sleep(0.08)
                else:
                    self.fail("Fresh desktop did not reach /api/v1/ready")

                def get(name):
                    with urllib.request.urlopen(base + name, timeout=3.0) as resp:
                        return json.load(resp)

                stt = get("/api/v1/providers")
                llm = get("/api/v1/llm-providers")
                profiles = get("/api/v1/structure-profiles")
                runtime = get("/api/v1/embedded/status")
                self.assertEqual(len(stt), 1)
                self.assertEqual(stt[0]["id"], "scribewatch-native-parakeet")
                self.assertEqual(stt[0]["model"], "handy-computer/parakeet-ultra-gguf")
                self.assertEqual(len(llm), 1)
                self.assertEqual(llm[0]["id"], "scribewatch-native-qwen4b")
                self.assertEqual(llm[0]["model"], "unsloth/Qwen3.5-4B-GGUF:Q4_K_M")
                self.assertEqual(len(profiles), 5)
                self.assertEqual({item["providerId"] for item in profiles}, {llm[0]["id"]})
                self.assertEqual({item["model"] for item in profiles}, {llm[0]["model"]})
                self.assertEqual({(item["kind"], item["mode"]) for item in runtime["engines"]},
                                 {("stt", "internal"), ("llm", "internal")})
                self.assertEqual(len(get("/api/v1/jobs")), 0)
                # Engine directory intentionally contains no executables.
                # This test MUST NOT attempt a remote Hugging Face download.
                self.assertTrue(all(not engine["running"] for engine in runtime["engines"]))
            finally:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5)
                if proc.stderr:
                    proc.stderr.close()




FAKE_NATIVE_ENGINE = r'''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
from http.server import HTTPServer, BaseHTTPRequestHandler
port = int(sys.argv[sys.argv.index('--port') + 1])
root = Path(os.environ['SCRIBEWATCH_FAKE_FIRST_BOOT_TRACE'])
root.mkdir(parents=True, exist_ok=True)
class Handler(BaseHTTPRequestHandler):
 def do_GET(self):
  if self.path == '/health':
   body = {'status': 'ok'}
  elif self.path == '/v1/models':
   model = ('handy-computer/parakeet-ultra-gguf' if port == 38005 else
             'unsloth/Qwen3.5-4B-GGUF:Q4_K_M')
   body = {'data': [{'id': model}]} if (root / f'installed-{port}').exists() else {'data': []}
  elif self.path == '/models':
   body = {'models': []}
  else:
   self.send_error(404); return
  self.send_response(200)
  self.send_header('Content-Type', 'application/json')
  payload = json.dumps(body).encode()
  self.send_header('Content-Length', str(len(payload)))
  self.end_headers()
  self.wfile.write(payload)
 def do_POST(self):
  n = int(self.headers.get('Content-Length', 0))
  request = json.loads(self.rfile.read(n) or b'{}')
  if self.path == '/models' and port == 38080:
   assert request == {'model': 'unsloth/Qwen3.5-4B-GGUF:Q4_K_M', 'sidecars': []}, request
   body = {'success': True}
   (root / 'installed-38080').write_text(json.dumps(request))
  elif self.path == '/v1/desktop-asr/variants' and port == 38005:
   assert request == {'source': 'handy-computer/parakeet-ultra-gguf'}, request
   body = {'variants': [{'filename': 'parakeet-ultra-Q6_K.gguf', 'label': 'Q6_K'}]}
   (root / 'asr-variants').write_text(json.dumps(request))
  elif self.path == '/v1/desktop-asr/install' and port == 38005:
   assert request == {
     'source': 'handy-computer/parakeet-ultra-gguf',
     'filename': 'parakeet-ultra-Q6_K.gguf'
   }, request
   body = {'status': 'installed'}
   (root / 'installed-38005').write_text(json.dumps(request))
  else:
   self.send_error(404); return
  self.send_response(200)
  self.send_header('Content-Type', 'application/json')
  payload = json.dumps(body).encode()
  self.send_header('Content-Length', str(len(payload)))
  self.end_headers()
  self.wfile.write(payload)
 def log_message(self, *args): pass
HTTPServer(('127.0.0.1', port), Handler).serve_forever()
'''


class FirstBootActualNativeRequests(unittest.TestCase):
    @unittest.skipUnless(os.name == 'posix', 'Fake engines use POSIX executables')
    def test_downloads_exact_qwen4b_and_parakeet_using_native_apis(self):
        import stat
        for port in (38005, 38080):
            with socket.socket() as sock:
                if sock.connect_ex(('127.0.0.1', port)) == 0:
                    self.skipTest(f'Port {port} is occupied; never touch real local engines')
        with tempfile.TemporaryDirectory(prefix='scribe-first-boot-fake-') as temp:
            root = pathlib.Path(temp)
            for name in ('config','data','frontend','engines/speaches','engines/llm','trace'):
                (root/name).mkdir(parents=True)
            for name in ('engines/speaches/Cloud9-Speaches','engines/llm/llama-server'):
                f = root/name
                f.write_text(FAKE_NATIVE_ENGINE)
                f.chmod(f.stat().st_mode | stat.S_IXUSR)
            port = localhost_port()
            env = {
                **os.environ,
                'SCRIBEWATCH_NATIVE_AUTOINIT': '1',
                'SCRIBEWATCH_DESKTOP_MODE': '1',
                'SCRIBEWATCH_EMBEDDED_DIR': str(root/'engines'),
                'SCRIBEWATCH_FAKE_FIRST_BOOT_TRACE': str(root/'trace'),
            }
            args = [str(APP), '--host', '127.0.0.1', '--port', str(port),
                    '--config-dir', str(root/'config'), '--data-dir', str(root/'data'),
                    '--dist-dir', str(root/'frontend'),
                    '--allowed-roots', str(root/'data')]
            with (root/'app.log').open('wb') as log:
                proc = subprocess.Popen(args, env=env, stdout=log, stderr=log)
                try:
                    base = f'http://127.0.0.1:{port}'
                    status = None
                    for _ in range(95):
                        self.assertIsNone(proc.poll(), 'ScribeWatch unexpectedly crashed')
                        try:
                            with urllib.request.urlopen(base+'/api/v1/embedded/status', timeout=1) as response:
                                status=json.load(response)
                            if all(e['running'] for e in status['engines']) and (
                                root/'trace'/'installed-38005'
                            ).exists() and (root/'trace'/'installed-38080').exists():
                                break
                        except (OSError, ValueError):
                            pass
                        time.sleep(.15)
                    else:
                        self.fail('Expected both default native model download requests')
                    self.assertEqual(
                        {e['mode'] for e in status['engines']}, {'internal'}
                    )
                    llm = json.loads((root/'trace'/'installed-38080').read_text())
                    asr = json.loads((root/'trace'/'installed-38005').read_text())
                    self.assertEqual(llm['model'],'unsloth/Qwen3.5-4B-GGUF:Q4_K_M')
                    self.assertEqual(asr['source'],'handy-computer/parakeet-ultra-gguf')
                    self.assertEqual(asr['filename'],'parakeet-ultra-Q6_K.gguf')
                    print('FIRST_RUN_QWEN4B_PARAKEET_NATIVE_DOWNLOAD_REQUESTS_PASS')
                finally:
                    proc.terminate()
                    try:
                        proc.wait(timeout=7)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait(timeout=7)



if __name__ == "__main__":
    unittest.main()
