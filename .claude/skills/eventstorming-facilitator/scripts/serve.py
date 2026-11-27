#!/usr/bin/env python3
"""
EventStorming Facilitator - HTTP server + browser launcher.

Serves the HTML rendered by render.build_page() over localhost and
persists browser edits back to the source MD file via POST /save.
Edits preserve each block's original delimiter style (fenced code or
container directive).

Usage:
  python3 serve.py <md-file>                  # Start server + open browser
  python3 serve.py <md-file> --no-browser     # Start server only (for testing)
  python3 serve.py <md-file> --port 8765      # Custom port (default: 8765)
"""

import sys
import os
import json
import http.server
import threading
import subprocess
from pathlib import Path

from render import build_page

DEFAULT_PORT = 8765
_MAX_CONTENT_BYTES = 10 * 1024 * 1024  # 10MB


class _Handler(http.server.BaseHTTPRequestHandler):
    md_path: str = ''

    def _respond(self, status: int, content_type: str, data: bytes) -> None:
        self.send_response(status)
        self.send_header('Content-Type', content_type)
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path != '/':
            self.send_response(404)
            self.end_headers()
            return
        try:
            page = build_page(self.md_path)
            self._respond(200, 'text/html; charset=utf-8', page.encode('utf-8'))
        except Exception as e:
            self._respond(500, 'text/plain; charset=utf-8', f'Error: {e}'.encode('utf-8'))

    def do_POST(self):
        if self.path != '/save':
            self.send_response(404)
            self.end_headers()
            return
        try:
            length = int(self.headers.get('Content-Length', 0))
            if length > _MAX_CONTENT_BYTES:
                raise ValueError('content too large (max 10MB)')
            body = self.rfile.read(length)
            data = json.loads(body.decode('utf-8'))
            content = data.get('content')
            if not isinstance(content, str):
                raise ValueError('content must be a string')
            Path(self.md_path).write_text(content, encoding='utf-8')
            self._respond(200, 'application/json; charset=utf-8', json.dumps({'ok': True}).encode('utf-8'))
        except Exception as e:
            self._respond(500, 'application/json; charset=utf-8', json.dumps({'ok': False, 'error': str(e)}).encode('utf-8'))

    def log_message(self, fmt, *args):
        pass  # suppress request logs


def main():
    args = sys.argv[1:]
    if not args or args[0].startswith('-'):
        print('Usage: python3 serve.py <md-file> [--no-browser] [--port N]', file=sys.stderr)
        sys.exit(1)

    md_path = os.path.abspath(args[0])
    no_browser = '--no-browser' in args
    port = DEFAULT_PORT
    for i, a in enumerate(args):
        if a == '--port' and i + 1 < len(args):
            port = int(args[i + 1])

    if not Path(md_path).exists():
        print(f'File not found: {md_path}', file=sys.stderr)
        sys.exit(1)

    class Handler(_Handler):
        pass
    Handler.md_path = md_path

    server = http.server.HTTPServer(('localhost', port), Handler)
    url = f'http://localhost:{port}/'
    print(f'EventStorming Viewer: {url}')
    print(f'Serving: {md_path}')
    print('Ctrl-C to stop.')

    if not no_browser:
        threading.Timer(0.5, lambda: subprocess.run(['open', url])).start()

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print('\nStopped.')


if __name__ == '__main__':
    main()
