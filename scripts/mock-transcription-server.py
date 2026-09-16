#!/usr/bin/env python3
import json
import os
import re
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TEXT = os.environ.get("SCRIBEWATCH_MOCK_TEXT", "Remember to book the train tomorrow.")
MODELS = ["test-model", "fail-model", "slow-model", "fallback-model"]

class Handler(BaseHTTPRequestHandler):
    def _read_body(self):
        if self.headers.get("Transfer-Encoding", "").lower() == "chunked":
            chunks = []
            while True:
                size_line = self.rfile.readline().strip().split(b";", 1)[0]
                if not size_line:
                    continue
                size = int(size_line, 16)
                if size == 0:
                    self.rfile.readline()
                    break
                chunks.append(self.rfile.read(size))
                self.rfile.read(2)
            return b"".join(chunks)
        length = int(self.headers.get("Content-Length", "0"))
        return self.rfile.read(length) if length else b""

    def _multipart_field(self, body, name):
        content_type = self.headers.get("Content-Type", "")
        match = re.search(r"boundary=([^;]+)", content_type)
        if not match:
            return ""
        boundary = match.group(1).strip().strip('"').encode()
        marker = b'--' + boundary
        needle = f'name="{name}"'.encode()
        for part in body.split(marker):
            headers, separator, payload = part.partition(b"\r\n\r\n")
            if separator and needle in headers:
                return payload.rstrip(b"\r\n-").decode("utf-8", "replace").strip()
        return ""

    def _json(self, status, payload):
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/v1/models":
            self._json(200, {"data": [{"id": model} for model in MODELS]})
            return
        self.send_error(404)

    def do_POST(self):
        if self.path != "/v1/audio/transcriptions":
            self.send_error(404)
            return
        body = self._read_body()
        model = self._multipart_field(body, "model")
        if model == "fail-model":
            self._json(503, {"error": "intentional model failure"})
            return
        if model == "slow-model":
            time.sleep(float(os.environ.get("SCRIBEWATCH_MOCK_SLOW_SECONDS", "2")))
        text = "Fallback route worked." if model == "fallback-model" else TEXT
        self._json(200, {"text": text})

    def log_message(self, fmt, *args):
        pass

if __name__ == "__main__":
    port = int(os.environ.get("PORT", "39101"))
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
