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

echo "Building beep executable..."
cd apps/beep
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building fetch executable..."
cd apps/fetch
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building imgview executable..."
cd apps/imgview
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building GUI Apps..."
cd apps/calc
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building File Manager..."
cd apps/fileman
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building Notepad..."
cd apps/notepad
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building Piano..."
cd apps/piano
cargo build --target wasm32-wasip1 --release
cd ../..

echo "Building MIDI Player..."
cd apps/midiplayer
cargo build --target wasm32-wasip1 --release
cd ../..


mkdir -p public/bin
cp target/wasm32-unknown-unknown/release/kernel.wasm public/
cp target/wasm32-wasip1/release/hello.wasm public/bin/
cp target/wasm32-wasip1/release/coreutils.wasm public/bin/
cp target/wasm32-wasip1/release/sh.wasm public/bin/
cp target/wasm32-wasip1/release/edit.wasm public/bin/
cp target/wasm32-wasip1/release/beep.wasm public/bin/
cp target/wasm32-wasip1/release/fetch.wasm public/bin/
cp target/wasm32-wasip1/release/imgview.wasm public/bin/
cp target/wasm32-wasip1/release/calc.wasm public/bin/
cp target/wasm32-wasip1/release/fileman.wasm public/bin/
cp target/wasm32-wasip1/release/notepad.wasm public/bin/
cp target/wasm32-wasip1/release/piano.wasm public/bin/
cp target/wasm32-wasip1/release/midiplayer.wasm public/bin/
echo "Build complete. Run ./serve.sh to start."
