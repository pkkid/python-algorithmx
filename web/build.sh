#!/usr/bin/env bash
# Build the WASM solver and the web demo.
#   web/index.html + web/puzzle_a_day.wasm  -> deploy both files to your website
#   web/standalone.html                     -> single file with the WASM inlined (works from file://)
# Requires the wasm32 target: `rustup target add wasm32-unknown-unknown`
# (Fedora: `sudo dnf install rust-std-static-wasm32-unknown-unknown`).
set -euo pipefail
WEB="$(cd "$(dirname "$0")" && pwd)"
cd "$WEB/../rust"
cargo build --release --target wasm32-unknown-unknown --lib
cp target/wasm32-unknown-unknown/release/puzzle_a_day.wasm "$WEB/"
python3 - "$WEB" <<'EOF'
import base64, sys, pathlib
web = pathlib.Path(sys.argv[1])
html = (web / 'index.html').read_text()
b64 = base64.b64encode((web / 'puzzle_a_day.wasm').read_bytes()).decode()
tag = '<script id="wasm-b64" type="application/octet-stream"></script>'
assert tag in html, 'wasm placeholder not found in index.html'
(web / 'standalone.html').write_text(html.replace(tag, tag.replace('></', f'>{b64}</')))
EOF
ls -lh "$WEB/puzzle_a_day.wasm" "$WEB/standalone.html"
