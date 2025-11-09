#!/bin/bash
set -e

echo "Building WASM..."
cargo build --target wasm32-unknown-unknown --profile wasm-release --package wasm-responses

echo "Generating bindings..."
~/.cargo/bin/wasm-bindgen \
    target/wasm32-unknown-unknown/wasm-release/wasm_responses.wasm \
    --out-dir ./wasm-responses/responses \
    --target web \
    --no-typescript \
    # -- -C target-feature=+reference-types \
    # --browser \

echo "Optimizing wasm with wasm-opt..."
wasm-opt -Oz ./wasm-responses/responses/wasm_responses_bg.wasm \
    -o wasm-responses/responses/wasm_responses_bg.wasm

echo "WASM build complete!"