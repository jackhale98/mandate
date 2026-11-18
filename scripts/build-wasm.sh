#!/usr/bin/env bash
# Build the engine to WASM and generate JS bindings for the web app (DEC-066).
# Output: web/src/engine/pkg/ (web target) and target/wasm-node/ (node target, tests).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p mandate-wasm --target wasm32-unknown-unknown --profile release-wasm
WASM=target/wasm32-unknown-unknown/release-wasm/mandate_wasm.wasm
wasm-bindgen "$WASM" --out-dir web/src/engine/pkg --target web --omit-default-module-path
wasm-bindgen "$WASM" --out-dir target/wasm-node --target nodejs
# Keep only what the app imports.
rm -f web/src/engine/pkg/package.json
ls -la web/src/engine/pkg
