"""True ScribeWatch HTTP/SQLite workspace lifecycle, no user recordings or private data."""
from __future__ import annotations

import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import tempfile
import time
import unittest
import urllib.error
import urllib.request


def reserve_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def api(base: str, path: str, method: str = "GET", data: dict | None = None) -> tuple[int, object]:
    payload = json.dumps(data).encode() if data is not None else None
    request = urllib.request.Request(
        base + path,
        data=payload,
        method=method,
        headers={"Content-Type": "application/json"} if data is not None else {},
    )
    try:
        with urllib.request.urlopen(request, timeout=3) as response:
            if response.status == 204:
                return response.status, None
            return response.status, json.loads(response.read())
    except urllib.error.HTTPError as error:
        try:
            return error.code, json.loads(error.read())
        finally:
            error.close()


class SpaceE2E(unittest.TestCase):
    def test_server_keeps_space_memberships_without_touching_recordings(self) -> None:
        binary = Path(os.environ.get("SCRIBEWATCH_TEST_BINARY", "target/debug/scribewatch")).resolve()
        if not binary.is_file():
            self.skipTest("Compile the real backend executable first")
        with tempfile.TemporaryDirectory(prefix="scribewatch-spaces-e2e-") as temp:
            root = Path(temp)
            for name in ("config", "data", "files"):
                (root / name).mkdir()
            port = reserve_port()
            base = "http://127.0.0.1:" + str(port)
            env = os.environ | {
                "SCRIBEWATCH_HOST": "127.0.0.1",
                "SCRIBEWATCH_PORT": str(port),
                "SCRIBEWATCH_DESKTOP_MODE": "0",
                "SCRIBEWATCH_CONFIG_DIR": str(root / "config"),
                "SCRIBEWATCH_DATA_DIR": str(root / "data"),
                "SCRIBEWATCH_ALLOWED_ROOTS": str(root / "files"),
            }
            def with_server(action):
                with (root / "server.log").open("a") as log:
                    process = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
                    try:
                        for _ in range(100):
                            if process.poll() is not None:
                                raise AssertionError("ScribeWatch exited before being ready")
                            try:
                                if api(base, "/api/v1/health")[0] == 200:
                                    break
                            except (OSError, urllib.error.URLError):
                                time.sleep(.08)
                        else:
                            self.fail("Backend did not open its loopback listener")
                        return action()
                    finally:
                        process.terminate()
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait(timeout=5)

            def first_pass():
                status, empty = api(base, "/api/v1/spaces")
                self.assertEqual((status, empty), (200, []))
                status, _ = api(base, "/api/v1/spaces", "POST", {"name": "  ", "kind": "meeting"})
                self.assertEqual(status, 400)
                status, _ = api(base, "/api/v1/spaces", "POST", {"name": "A\nB", "kind": "meeting"})
                self.assertEqual(status, 400)
                status, created = api(base, "/api/v1/spaces", "POST", {"name": "Project sample", "kind": "project"})
                self.assertEqual(status, 200)
                self.assertEqual(created["name"], "Project sample")
                status, _ = api(base, "/api/v1/jobs/unknown/space", "PUT", {"spaceId": created["id"]})
                self.assertEqual(status, 404)
                return created["id"]

            space_id = with_server(first_pass)

            # Synthetic fixture written only into this disposable private test database.
            record = {
                "id": "synthetic-recording-001", "kind": "quick",
                "providerId": "test-provider", "originalName": "neutral-audio.wav",
                "sourcePath": str(root / "files" / "neutral-audio.wav"),
                "sourceSize": 100, "sourceMtimeNs": 1, "model": "mock",
                "status": "done", "createdAtMs": 100, "updatedAtMs": 100,
            }
            with sqlite3.connect(root / "config" / "scribewatch.sqlite3") as conn:
                conn.execute(
                    "INSERT INTO sw_objects(kind,id,json,updated_at) VALUES(?,?,?,unixepoch())",
                    ("job", record["id"], json.dumps(record)),
                )
                conn.commit()

            def second_pass():
                status, spaces = api(base, "/api/v1/spaces")
                self.assertEqual(status, 200)
                self.assertEqual(spaces[0]["id"], space_id)
                status, jobs = api(base, "/api/v1/jobs")
                self.assertEqual(status, 200)
                self.assertEqual(len(jobs), 1)
                status, _ = api(base, "/api/v1/jobs/" + record["id"] + "/space",
                                "PUT", {"spaceId": "invalid-space"})
                self.assertEqual(status, 404)
                status, _ = api(base, "/api/v1/jobs/" + record["id"] + "/space",
                                "PUT", {"spaceId": space_id})
                self.assertEqual(status, 204)
                self.assertEqual(api(base, "/api/v1/spaces/assignments")[1],
                                 [{"jobId": record["id"], "spaceId": space_id}])
            with_server(second_pass)

            def third_pass():
                self.assertEqual(api(base, "/api/v1/spaces/assignments")[1],
                                 [{"jobId": record["id"], "spaceId": space_id}])
                status, _ = api(base, "/api/v1/spaces/" + space_id, "DELETE")
                self.assertEqual(status, 204)
                self.assertEqual(api(base, "/api/v1/spaces")[1], [])
                self.assertEqual(api(base, "/api/v1/spaces/assignments")[1], [])
                status, job = api(base, "/api/v1/jobs/" + record["id"])
                self.assertEqual(status, 200)
                self.assertEqual(job["originalName"], record["originalName"])
            with_server(third_pass)
            print("REAL_SQLITE_SPACES_CREATE_ASSIGN_RESTART_DELETE_WITH_RECORDING_PRESERVED_PASS")


if __name__ == "__main__":
    unittest.main()
