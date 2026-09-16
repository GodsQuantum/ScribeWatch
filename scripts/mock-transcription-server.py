#!/usr/bin/env python3
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TEXT = os.environ.get("SCRIBEWATCH_MOCK_TEXT", "Remember to book the train tomorrow.")

class Handler(BaseHTTPRequestHandler):
    def _drain_body(self):
        if self.headers.get("Transfer-Encoding", "").lower() == "chunked":
            while True:
                size_line = self.rfile.readline().strip().split(b";", 1)[0]
                if not size_line:
                    continue
                size = int(size_line, 16)
                if size == 0:
                    self.rfile.readline()
                    break
                self.rfile.read(size)
                self.rfile.read(2)
        else:
            length = int(self.headers.get("Content-Length", "0"))
            if length:
                self.rfile.read(length)

    def do_POST(self):
        if self.path != "/v1/audio/transcriptions":
            self.send_error(404)
            return
        self._drain_body()
        body = json.dumps({"text": TEXT}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        pass

if __name__ == "__main__":
    port = int(os.environ.get("PORT", "39101"))
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
