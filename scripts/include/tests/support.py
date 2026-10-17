"""What the tests of the scripts share: a local HTTP server and fake command-line tools."""

import contextlib
import http.server
import os
import stat
import sys
import threading
from pathlib import Path
from unittest import mock


@contextlib.contextmanager
def serve(handler):
    """A server on a free local port running `handler`; yields its base address."""
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.01}, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_address[1]}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def file_handler(body: bytes, ranges: bool, seen: list):
    """Serves `body` at any path; with `ranges` it honours `Range: bytes=N-` (206, or 416 beyond the end), without it
    always sends the whole file. Each request's Range header goes to `seen`."""

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            wanted = self.headers.get("Range")
            seen.append(wanted)
            start = int(wanted.removeprefix("bytes=").removesuffix("-")) if wanted and ranges else 0
            if start >= len(body) and start:
                self.send_response(416)
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            self.send_response(206 if start else 200)
            self.send_header("Content-Length", str(len(body) - start))
            self.end_headers()
            self.wfile.write(body[start:])

    return Handler


@contextlib.contextmanager
def fake_tools(directory: Path, **scripts: str):
    """Puts a program called each name in `directory` ahead of PATH; its body is Python run with the program's
    arguments in `args`."""
    directory.mkdir(parents=True, exist_ok=True)
    for name, body in scripts.items():
        program = directory / name
        program.write_text(f"#!{sys.executable}\nimport sys\nargs = sys.argv[1:]\n{body}\n")
        program.chmod(program.stat().st_mode | stat.S_IXUSR)
    with mock.patch.dict(os.environ, {"PATH": f"{directory}{os.pathsep}{os.environ['PATH']}"}):
        yield
