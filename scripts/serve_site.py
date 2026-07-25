#!/usr/bin/env python3
"""Static file server for the Widget for Jira landing page (site/).

Fronted by a Cloudflare Tunnel at widjira.isemi.io, so this binds to loopback
only and leaves TLS, compression and edge caching to Cloudflare. Stdlib only —
runs under the system /usr/bin/python3 so launchd never depends on nvm/brew.

    PORT=8790 ROOT=./site /usr/bin/python3 scripts/serve_site.py
"""

import os
import posixpath
import re
import sys
from functools import partial
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

def _default_root():
    here = Path(__file__).resolve().parent
    # scripts/serve_site.py in the repo, serve_site.py next to site/ once published.
    for candidate in (here.parent / "site", here / "site"):
        if (candidate / "index.html").is_file():
            return candidate
    return here.parent / "site"


HOST = os.environ.get("HOST", "127.0.0.1")
PORT = int(os.environ.get("PORT", "8790"))
ROOT = Path(os.environ.get("ROOT") or _default_root()).resolve()

# Assets are content-stable, HTML gets a short TTL so copy edits go live fast.
CACHE_ASSET = "public, max-age=604800"
CACHE_PAGE = "public, max-age=300"
CACHE_NONE = "no-store"
ASSET_EXT = {".png", ".jpg", ".jpeg", ".webp", ".svg", ".ico", ".css", ".js", ".woff", ".woff2"}

# The site is pure HTML + CSS with no scripts and no third-party origins, so the
# policy can be this tight. Inline style attributes are why style-src is relaxed.
SECURITY_HEADERS = {
    "Content-Security-Policy": (
        "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; "
        "script-src 'none'; object-src 'none'; frame-ancestors 'none'; "
        "base-uri 'none'; form-action 'none'"
    ),
    "X-Content-Type-Options": "nosniff",
    "Referrer-Policy": "strict-origin-when-cross-origin",
    "Permissions-Policy": "geolocation=(), microphone=(), camera=(), interest-cohort=()",
}

SAFE_HOST = re.compile(r"^[A-Za-z0-9.\-]+(:[0-9]+)?$")


class SiteHandler(SimpleHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "widjira-site"
    sys_version = ""

    cache_control = CACHE_NONE

    def visitor_scheme(self):
        """Scheme the visitor used, which cloudflared forwards; empty when local."""
        headers = getattr(self, "headers", None)
        return (headers.get("X-Forwarded-Proto", "") if headers else "").lower()

    def send_head(self):
        if self.upgrade_to_https():
            return None
        ext = posixpath.splitext(urlsplit(self.path).path)[1].lower()
        self.cache_control = CACHE_ASSET if ext in ASSET_EXT else CACHE_PAGE
        return super().send_head()

    def upgrade_to_https(self):
        """Redirect plain HTTP here rather than relying on a Cloudflare zone rule."""
        if self.visitor_scheme() != "http":
            return False
        host = self.headers.get("Host", "")
        if not SAFE_HOST.match(host):
            return False
        self.cache_control = CACHE_NONE
        self.send_response(HTTPStatus.MOVED_PERMANENTLY)
        self.send_header("Location", f"https://{host}{self.path}")
        self.send_header("Content-Length", "0")
        self.end_headers()
        return True

    def end_headers(self):
        for name, value in SECURITY_HEADERS.items():
            self.send_header(name, value)
        if self.visitor_scheme() == "https":
            self.send_header("Strict-Transport-Security", "max-age=31536000")
        self.send_header("Cache-Control", self.cache_control)
        super().end_headers()

    def list_directory(self, path):
        """Never expose an index listing — a directory without index.html is a 404."""
        self.send_error(HTTPStatus.NOT_FOUND)
        return None

    def send_error(self, code, message=None, explain=None):
        page = ROOT / "404.html"
        if code == HTTPStatus.NOT_FOUND and self.command in ("GET", "HEAD") and page.is_file():
            body = page.read_bytes()
            self.cache_control = CACHE_NONE
            self.send_response(HTTPStatus.NOT_FOUND)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            if self.command == "GET":
                self.wfile.write(body)
            return
        super().send_error(code, message, explain)

    def address_string(self):
        """Every peer is the tunnel, so log the real visitor Cloudflare passes on."""
        return self.headers.get("CF-Connecting-IP") or super().address_string()


def main():
    if not (ROOT / "index.html").is_file():
        sys.exit(f"serve_site: no index.html under {ROOT}")

    handler = partial(SiteHandler, directory=str(ROOT))
    ThreadingHTTPServer.daemon_threads = True
    with ThreadingHTTPServer((HOST, PORT), handler) as httpd:
        print(f"serve_site: {ROOT} on http://{HOST}:{PORT}", flush=True)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
