#!/bin/bash
set -e

echo "Building WASM kernel..."
cargo build --manifest-path kernel/Cargo.toml --target wasm32-unknown-unknown --release

echo "Building hello executable..."
cd apps/hello
cargo build --target wasm32-wasip1 --release
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
cp target/wasm32-unknown-unknown/release/kernel.wasm public/
cp target/wasm32-wasip1/release/hello.wasm public/bin/
cp target/wasm32-wasip1/release/coreutils.wasm public/bin/
cp target/wasm32-wasip1/release/sh.wasm public/bin/
cp target/wasm32-wasip1/release/edit.wasm public/bin/
echo "Build complete. Run ./serve.sh to start."
