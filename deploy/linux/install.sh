#!/usr/bin/env bash
set -e

echo "==> Installing NodeX P2P Messenger for Linux..."
cargo build --release -p nodex-gui

BIN_SRC="target/release/nodex"
if [ ! -f "$BIN_SRC" ]; then
    BIN_SRC="$HOME/.cargo_target/kademlia_dht/release/nodex"
fi

sudo install -Dm755 "$BIN_SRC" /usr/local/bin/nodex
sudo install -Dm644 nodex-gui/assets/logo.png /usr/share/pixmaps/nodex.png

mkdir -p ~/.local/share/applications
cat <<EOF > ~/.local/share/applications/nodex.desktop
[Desktop Entry]
Name=NodeX
Comment=Pure Rust Native P2P E2EE Messenger
Exec=/usr/local/bin/nodex
Icon=nodex
Terminal=false
Type=Application
Categories=Network;InstantMessaging;
EOF

echo "[SUCCESS] NodeX installed. Launch it via 'nodex' or from your application menu."
