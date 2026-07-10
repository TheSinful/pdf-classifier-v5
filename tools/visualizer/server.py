#!/usr/bin/env python3
"""
Visualizer server for pdf_classifier_core runs.

Serves the static web UI and (optionally) launches live classifier runs:
it opens the TCP "frontend" socket the classifier requires
(CLASSIFIER_OUTPUT_PORT), spawns the binary, and streams its stderr trace
log, stdout results, and extraction payload frames to the browser over SSE.

Stdlib only — no dependencies.

Usage:
    python server.py [--port 7878] [--exe path/to/pdf_classifier_core.exe]

Then open http://127.0.0.1:7878
"""

import argparse
import json
import os
import socket
import struct
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
STATIC_DIR = HERE / "static"
SAMPLE_DIR = HERE / "sample"

DEFAULT_EXE = REPO_ROOT / "target" / "debug" / "pdf_classifier_core.exe"
DEFAULT_DOC = REPO_ROOT / "data" / "large_test_doc.pdf"

MAX_PAYLOAD_KEEP = 64 * 1024  # keep at most 64 KiB of any single extraction payload

CONTENT_TYPES = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".log": "text/plain; charset=utf-8",
    ".txt": "text/plain; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".svg": "image/svg+xml",
}


class Run:
    """One classifier process + its captured event stream."""

    _next_id = 1
    _lock = threading.Lock()

    def __init__(self, exe, doc, start, end, threads, level):
        with Run._lock:
            self.id = Run._next_id
            Run._next_id += 1
        self.events = []            # [{seq, type, ...}]
        self.cond = threading.Condition()
        self.status = "starting"
        self.proc = None
        self.exe, self.doc = exe, doc
        self.args = (start, end, threads, level)

    def push(self, ev):
        with self.cond:
            ev["seq"] = len(self.events)
            self.events.append(ev)
            self.cond.notify_all()

    def set_status(self, status, code=None):
        self.status = status
        self.push({"type": "status", "status": status, "code": code})

    # ---- lifecycle ----

    def start(self):
        listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        frontend_port = listener.getsockname()[1]

        env = dict(os.environ)
        env["CLASSIFIER_OUTPUT_PORT"] = str(frontend_port)

        start, end, threads, level = self.args
        cmd = [
            str(self.exe),
            "-s", str(start), "-e", str(end),
            "-t", str(threads), "-p", str(self.doc),
            "-l", level,
        ]
        self.proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            cwd=str(REPO_ROOT),
        )
        self.set_status("running")
        threading.Thread(target=self._read_pipe, args=(self.proc.stderr, "stderr"), daemon=True).start()
        threading.Thread(target=self._read_pipe, args=(self.proc.stdout, "stdout"), daemon=True).start()
        threading.Thread(target=self._frontend, args=(listener,), daemon=True).start()
        threading.Thread(target=self._wait, daemon=True).start()

    def _read_pipe(self, pipe, kind):
        for raw in iter(pipe.readline, b""):
            line = raw.decode("utf-8", errors="replace").rstrip("\r\n")
            self.push({"type": kind, "data": line})
        pipe.close()

    def _frontend(self, listener):
        """Act as the classifier's streaming frontend: read extraction frames."""
        listener.settimeout(30)
        try:
            conn, _ = listener.accept()
        except OSError:
            listener.close()
            return
        listener.close()

        def read_exact(n):
            buf = b""
            while len(buf) < n:
                chunk = conn.recv(n - len(buf))
                if not chunk:
                    raise EOFError
                buf += chunk
            return buf

        try:
            while True:
                page = struct.unpack("<I", read_exact(4))[0]
                class_len = struct.unpack("<I", read_exact(4))[0]
                class_name = read_exact(class_len).decode("utf-8", errors="replace")
                payload_len = struct.unpack("<I", read_exact(4))[0]
                payload = read_exact(payload_len)
                self.push({
                    "type": "extract",
                    "page": page,
                    "class": class_name.upper(),
                    "payload": payload[:MAX_PAYLOAD_KEEP].decode("utf-8", errors="replace"),
                })
        except (EOFError, OSError):
            pass
        finally:
            conn.close()

    def _wait(self):
        code = self.proc.wait()
        time.sleep(0.3)  # let pipe readers drain
        self.set_status("done" if code == 0 else "failed", code)

    def stop(self):
        if self.proc and self.proc.poll() is None:
            self.proc.kill()


RUNS = {}


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):  # quieter default log
        print(f"[http] {self.command} {self.path}")

    # ---- helpers ----

    def send_json(self, obj, code=200):
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def send_file(self, path: Path):
        if not path.is_file():
            self.send_json({"error": "not found"}, 404)
            return
        body = path.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", CONTENT_TYPES.get(path.suffix, "application/octet-stream"))
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def read_body_json(self):
        length = int(self.headers.get("Content-Length") or 0)
        return json.loads(self.rfile.read(length) or b"{}")

    # ---- routes ----

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path in ("/", "/index.html"):
            return self.send_file(STATIC_DIR / "index.html")
        if path == "/api/defaults":
            return self.send_json({
                "exe": str(DEFAULT_EXE),
                "exe_exists": DEFAULT_EXE.is_file(),
                "doc": str(DEFAULT_DOC) if DEFAULT_DOC.is_file() else "",
            })
        if path == "/api/runs":
            return self.send_json([
                {"id": r.id, "status": r.status, "events": len(r.events)} for r in RUNS.values()
            ])
        if path.startswith("/api/run/") and path.endswith("/events"):
            return self.sse_events(path)
        if path.startswith("/sample/"):
            name = os.path.basename(path)
            return self.send_file(SAMPLE_DIR / name)
        # static
        name = path.lstrip("/")
        target = (STATIC_DIR / name).resolve()
        if STATIC_DIR.resolve() not in target.parents and target != STATIC_DIR.resolve():
            return self.send_json({"error": "forbidden"}, 403)
        return self.send_file(target)

    def do_POST(self):
        path = self.path.split("?", 1)[0]
        if path == "/api/run":
            return self.api_start_run()
        if path.startswith("/api/run/") and path.endswith("/stop"):
            try:
                run_id = int(path.split("/")[3])
                RUNS[run_id].stop()
                return self.send_json({"ok": True})
            except (KeyError, ValueError, IndexError):
                return self.send_json({"error": "unknown run"}, 404)
        return self.send_json({"error": "not found"}, 404)

    def api_start_run(self):
        try:
            body = self.read_body_json()
        except json.JSONDecodeError:
            return self.send_json({"error": "invalid JSON"}, 400)

        exe = Path(body.get("exe") or DEFAULT_EXE)
        doc = Path(body.get("doc") or DEFAULT_DOC)
        if not exe.is_file():
            return self.send_json({"error": f"classifier binary not found: {exe} — run `cargo build` first"}, 400)
        if not doc.is_file():
            return self.send_json({"error": f"document not found: {doc}"}, 400)
        try:
            start = int(body.get("start", 0))
            end = int(body.get("end", 0))
            threads = max(1, min(16, int(body.get("threads", 4))))
        except (TypeError, ValueError):
            return self.send_json({"error": "start/end/threads must be integers"}, 400)
        if end <= start:
            return self.send_json({"error": "end page must be greater than start page"}, 400)
        level = body.get("level", "trace")
        if level not in ("trace", "debug", "info"):
            return self.send_json({"error": "level must be trace|debug|info"}, 400)

        run = Run(exe, doc, start, end, threads, level)
        RUNS[run.id] = run
        try:
            run.start()
        except OSError as e:
            return self.send_json({"error": f"failed to spawn: {e}"}, 500)
        return self.send_json({"id": run.id})

    def sse_events(self, path):
        try:
            run = RUNS[int(path.split("/")[3])]
        except (KeyError, ValueError, IndexError):
            return self.send_json({"error": "unknown run"}, 404)

        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.end_headers()

        sent = 0
        try:
            while True:
                with run.cond:
                    if sent >= len(run.events) and run.status == "running":
                        run.cond.wait(timeout=15)
                    batch = run.events[sent:]
                for ev in batch:
                    self.wfile.write(b"data: " + json.dumps(ev).encode() + b"\n\n")
                sent += len(batch)
                if not batch:
                    self.wfile.write(b": ping\n\n")
                self.wfile.flush()
                if run.status != "running" and sent >= len(run.events):
                    break
        except (BrokenPipeError, ConnectionError, OSError):
            pass


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, default=7878)
    ap.add_argument("--exe", type=Path, default=None, help="override classifier binary path")
    args = ap.parse_args()

    global DEFAULT_EXE
    if args.exe:
        DEFAULT_EXE = args.exe.resolve()

    srv = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    print(f"classifier visualizer -> http://127.0.0.1:{args.port}")
    print(f"  binary : {DEFAULT_EXE} ({'found' if DEFAULT_EXE.is_file() else 'MISSING — live runs disabled until built'})")
    print(f"  doc    : {DEFAULT_DOC if DEFAULT_DOC.is_file() else '(default test doc missing)'}")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        for r in RUNS.values():
            r.stop()


if __name__ == "__main__":
    main()
