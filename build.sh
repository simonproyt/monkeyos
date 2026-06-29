#!/bin/bash
set -e

echo "Building WASM kernel..."
cargo build --target wasm32-wasip1

echo "Building hello executable..."
cd apps/hello
cargo build --target wasm32-wasip1
cd ../..
echo "Building coreutils executable..."
cd apps/coreutils
cargo build --target wasm32-wasip1 --release
cd ../..
echo "Building sh executable..."
cd apps/sh
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building edit executable..."
cd apps/edit
cargo build --target wasm32-wasip1 --release
cd ../..
mkdir -p public/bin
cp target/wasm32-wasip1/debug/kernel.wasm public/
cp target/wasm32-wasip1/debug/hello.wasm public/bin/
cp target/wasm32-wasip1/release/coreutils.wasm public/bin/
cp target/wasm32-wasip1/release/sh.wasm public/bin/
cp target/wasm32-wasip1/release/edit.wasm public/bin/
echo "Build complete. Run ./serve.sh to start."
