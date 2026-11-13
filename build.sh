#!/usr/bin/env bash
# Baut Zento: Rust → WebAssembly → web/pkg
# Voraussetzungen: rustup target add wasm32-unknown-unknown
#                  cargo install wasm-bindgen-cli --version 0.2.130
set -euo pipefail
cd "$(dirname "$0")"

# Versionsstempel (wird in den Einstellungen angezeigt)
WWP_BUILD="$(git rev-parse --short HEAD 2>/dev/null || echo dev) · $(date -u +%d.%m.%Y)"
export WWP_BUILD
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/wwp.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O3 --enable-bulk-memory --enable-nontrapping-float-to-int \
    -o web/pkg/wwp_bg.wasm web/pkg/wwp_bg.wasm
fi

# Cache-Version des Service Workers erneuern, damit Updates auf dem Tablet ankommen
VERSION="$(date +%Y%m%d%H%M%S)"
sed -i.bak "s/const VERSION = '[^']*'/const VERSION = '${VERSION}'/" web/sw.js && rm -f web/sw.js.bak

echo "Fertig. Lokal testen: python3 -m http.server -d web 8080  →  http://localhost:8080"
