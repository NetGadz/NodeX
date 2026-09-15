#!/bin/bash
set -e

echo "=========================================="
echo "    Kademlia DHT Automated Demo Cluster    "
echo "=========================================="

echo "[DEMO] Building release binary..."
cargo build --release

EXE="./target/release/kademlia-dht"

echo "[DEMO] Starting Node 1 (Bootstrap) on port 9001..."
$EXE --port 9001 --state-file demo_state1.json &
NODE1_PID=$!

sleep 2

echo "[DEMO] Starting Node 2 on port 9002..."
$EXE --port 9002 --bootstrap 127.0.0.1:9001 --state-file demo_state2.json &
NODE2_PID=$!

sleep 2

echo "[DEMO] Starting Node 3 on port 9003..."
$EXE --port 9003 --bootstrap 127.0.0.1:9002 --state-file demo_state3.json &
NODE3_PID=$!

sleep 2

echo "[DEMO] 3-Node P2P cluster running. Running full test suite..."
cargo test --test integration_test

echo "[DEMO] Cleaning up processes..."
kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null || true

echo "[DEMO] Complete!"
