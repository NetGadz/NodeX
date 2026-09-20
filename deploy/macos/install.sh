#!/usr/bin/env bash
set -e

echo "==> Building and installing NodeX on macOS..."
cargo build --release -p nodex-gui

BIN_SRC="target/release/nodex"
if [ ! -f "$BIN_SRC" ]; then
    BIN_SRC="$HOME/.cargo_target/kademlia_dht/release/nodex"
fi

mkdir -p /Applications/NodeX.app/Contents/MacOS
mkdir -p /Applications/NodeX.app/Contents/Resources

cp "$BIN_SRC" /Applications/NodeX.app/Contents/MacOS/nodex
cp nodex-gui/assets/logo.png /Applications/NodeX.app/Contents/Resources/AppIcon.png

echo "[SUCCESS] NodeX installed to /Applications/NodeX.app"
