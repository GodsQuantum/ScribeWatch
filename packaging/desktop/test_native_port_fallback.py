"""Native launcher collision regression on a real bundled Rust backend (Linux CI)."""
import json
import os
import pathlib
import shutil
import signal
import socket
import subprocess
import tempfile
import time
import unittest
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
LAUNCHER = pathlib.Path(os.environ.get("SCRIBEWATCH_LAUNCHER_TEST_BINARY", ""))
SERVER = pathlib.Path(os.environ.get("SCRIBEWATCH_TEST_BINARY", str(ROOT / "target" / "debug" / "scribewatch")))


def find_free_socket():
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    sock.listen(8)
    return sock


@unittest.skipUnless(os.name == "posix" and LAUNCHER.is_file() and SERVER.is_file(),
                     "Requires compiled native launcher and backend on POSIX")
class NativePortFallback(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="scribe-native-ports-")
        self.addCleanup(self.temp.cleanup)
        self.home = pathlib.Path(self.temp.name) / "home"
        self.home.mkdir()
        root = pathlib.Path(self.temp.name) / "bundle"
        (root / "bin").mkdir(parents=True)
        # Mimic the full installer location. No engine executables: no remote
        # model download, but native provider URLs are still initialized.
        (root / "bin" / "engines").mkdir()
        (root / "frontend").mkdir()
        (root / "frontend" / "index.html").write_text("<!doctype html><title>ScribeWatch</title>")
        shutil.copyfile(LAUNCHER, root / "ScribeWatch")
        (root / "ScribeWatch").chmod(0o755)
        (root / "bin" / "scribewatch-server").symlink_to(SERVER.resolve())
        for cmd in ("ffmpeg", "ffprobe"):
            (root / "bin" / cmd).symlink_to(shutil.which(cmd))
        self.launcher = root / "ScribeWatch"
        self.env = {**os.environ, "HOME": str(self.home)}
        self.base = self.home / ".local/share/ScribeWatch"

    def wait_backend(self, proc):
        port_file = self.base / "server.port"
        for _ in range(160):
            if proc.poll() is not None:
                raise AssertionError(f"launcher died with {proc.returncode}; "
                                     f"log: {(self.base/'desktop.log').read_text()[-2000:] if (self.base/'desktop.log').exists() else 'none'}")
            if port_file.is_file():
                port = int(port_file.read_text().strip())
                try:
                    with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/v1/ready", timeout=.6) as res:
                        if res.status == 200:
                            return port
                except OSError:
                    pass
            time.sleep(.12)
        self.fail("ScribeWatch never became healthy on the OS-selected port")

    def start(self):
        proc = subprocess.Popen([str(self.launcher), "--no-browser", "--no-tray"],
                                env=self.env, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, start_new_session=True)
        self.addCleanup(self.stop, proc)
        return proc

    @staticmethod
    def stop(proc):
        if proc.poll() is None:
            try:
                os.killpg(proc.pid, signal.SIGTERM)
                proc.wait(timeout=5)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait(timeout=5)
        if proc.stdout:
            proc.stdout.close()
        if proc.stderr:
            proc.stderr.close()

    def test_busy_application_and_engine_ports_recovered_and_persisted(self):
        with find_free_socket() as busy_main, find_free_socket() as busy_stt, find_free_socket() as busy_llm:
            # Bind all default engine ports if possible, simulating third-party software.
            busy_default_stt = socket.socket()
            busy_default_llm = socket.socket()
            try:
                busy_default_stt.bind(("127.0.0.1", 38005))
                busy_default_stt.listen(8)
                busy_default_llm.bind(("127.0.0.1", 38080))
                busy_default_llm.listen(8)
            except OSError:
                busy_default_stt.close()
                busy_default_llm.close()
                self.skipTest("A real service owns an engine port; never interfere")
            try:
                preferred = busy_main.getsockname()[1]
                self.env["SCRIBEWATCH_PORT"] = str(preferred)
                proc = self.start()
                assigned = self.wait_backend(proc)
                self.assertNotEqual(assigned, preferred, "Other application's socket is preserved")
                def get(path):
                    with urllib.request.urlopen(f"http://127.0.0.1:{assigned}{path}", timeout=4) as res:
                        return json.load(res)
                engines = {e["kind"]: e for e in get("/api/v1/embedded/status")["engines"]}
                stt = get("/api/v1/providers")[0]
                llm = get("/api/v1/llm-providers")[0]
                self.assertNotIn(":38005/", engines["stt"]["endpoint"])
                self.assertNotIn(":38080/", engines["llm"]["endpoint"])
                self.assertEqual(engines["stt"]["endpoint"], stt["transcriptionUrl"])
                self.assertEqual(engines["llm"]["endpoint"], llm["chatCompletionsUrl"])
                # A second launch must connect to the persisted assigned port,
                # not start a duplicate, and must not touch any busy sockets.
                second = subprocess.run([str(self.launcher), "--no-browser", "--no-tray"],
                                        env=self.env, timeout=15, capture_output=True)
                self.assertEqual(second.returncode, 0, second.stderr.decode(errors="replace"))
                self.assertEqual(int((self.base / "server.port").read_text()), assigned)
                self.assertIsNone(proc.poll())
                self.assertEqual(busy_main.getsockname()[1], preferred)
                self.assertEqual(busy_default_stt.getsockname()[1], 38005)
                self.assertEqual(busy_default_llm.getsockname()[1], 38080)
            finally:
                busy_default_stt.close()
                busy_default_llm.close()


if __name__ == "__main__":
    unittest.main()
