#!/usr/bin/env bash
# Build the engine to WASM and generate JS bindings for the web app (DEC-066).
# Output: web/src/engine/pkg/ (web target) and target/wasm-node/ (node target, tests).
# Honours CARGO_TARGET_DIR; MANDATE_WASM_NODE overrides the node output directory
# (side-by-side checkouts sharing a machine keep their own builds).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p mandate-wasm --target wasm32-unknown-unknown --profile release-wasm
TARGET="${CARGO_TARGET_DIR:-target}"
NODE_OUT="${MANDATE_WASM_NODE:-target/wasm-node}"
WASM="$TARGET/wasm32-unknown-unknown/release-wasm/mandate_wasm.wasm"
# The web build drops the function-name section (~3.8 MB, about 45% of the file);
# the node build used by tests keeps it for readable traces.
wasm-bindgen "$WASM" --out-dir web/src/engine/pkg --target web --omit-default-module-path --remove-name-section
wasm-bindgen "$WASM" --out-dir "$NODE_OUT" --target nodejs
# Keep only what the app imports.
rm -f web/src/engine/pkg/package.json
ls -la web/src/engine/pkg
