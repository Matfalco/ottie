"""Serves the sound picker on 127.0.0.1: its pages (index.html: pairs,
sort.html: sorting) from beside this file, the sounds from <folder>
(sounds.json + sounds/, from library.py), and saves the choices to
<folder>/choices.json (POST /choices: the keys sent replace those saved,
others stay). Only this computer can reach it.

Usage: python -I server.py <folder> [port]
"""
import json
import os
import sys
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

FOLDER = Path(sys.argv[1]).resolve()
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 8766
LIMIT = 1_000_000
PAGES = Path(__file__).resolve().parent


class Handler(SimpleHTTPRequestHandler):
    def translate_path(self, path):
        page = path.split("?", 1)[0].lstrip("/") or "index.html"
        if page in ("index.html", "sort.html"):
            return str(PAGES / page)
        return super().translate_path(path)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def do_POST(self):
        if self.path != "/choices":
            self.send_error(404)
            return
        size = int(self.headers.get("Content-Length", 0))
        if size <= 0 or size > LIMIT:
            self.send_error(413)
            return
        try:
            choices = json.loads(self.rfile.read(size))
        except ValueError:
            self.send_error(400)
            return
        if not isinstance(choices, dict):
            self.send_error(400)
            return
        # Each page sends its own part (the sorting, the pairs): the rest stays.
        saved = FOLDER / "choices.json"
        if saved.exists():
            choices = {**json.loads(saved.read_text(encoding="utf-8")), **choices}
        partial_file = FOLDER / "choices.json.part"
        partial_file.write_text(json.dumps(choices, indent=1, ensure_ascii=False), encoding="utf-8")
        os.replace(partial_file, FOLDER / "choices.json")
        self.send_response(204)
        self.end_headers()

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    server = ThreadingHTTPServer(("127.0.0.1", PORT), partial(Handler, directory=str(FOLDER)))
    print(f"http://127.0.0.1:{PORT}/", flush=True)
    server.serve_forever()
