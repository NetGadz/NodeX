# NodeX

<p align="center">
  <img src="nodex-gui/assets/logo.png" width="140" alt="NodeX logo" />
</p>

<p align="center">
  <strong>Pure Rust Native Decentralized P2P E2EE Messenger</strong><br />
  Direct UDP delivery, STUN RFC 5389 NAT Traversal, Opt-in Encrypted Peer Relay, and Kademlia DHT Mailbox.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-v1.0.0-06b6d4?style=flat-square" alt="Version v1.0.0" />
  <img src="https://img.shields.io/badge/Rust-2021-orange?style=flat-square&logo=rust" alt="Rust 2021" />
  <img src="https://img.shields.io/badge/GUI-egui%20%2F%20eframe-22c55e?style=flat-square" alt="egui" />
  <img src="https://img.shields.io/badge/Network-Kademlia%20DHT-0ea5e9?style=flat-square" alt="Kademlia" />
  <img src="https://img.shields.io/badge/Security-ChaCha20--Poly1305-8b5cf6?style=flat-square" alt="E2EE" />
  <img src="https://img.shields.io/badge/License-MIT-16a34a?style=flat-square" alt="MIT" />
</p>

---

## 🌟 Overview

NodeX is a pure native Rust desktop messaging application built without central message servers. It combines Kademlia DHT routing, STUN NAT traversal, UDP hole punching, signed invites, and opt-in peer relays to provide private, end-to-end encrypted communication across the Internet.

```
┌─────────────────────────────────────────────────────────────┐
│                 NodeX Desktop GUI (egui/eframe)             │
│   Cyberpunk Dark Aesthetic • Lightbox • Emojis • Invites    │
└──────────────────────────────┬──────────────────────────────┘
                               │ UI Events & Commands
┌──────────────────────────────▼──────────────────────────────┐
│                    nodex-messenger Layer                    │
│   User Identity (Ed25519/X25519) • BIP-39 12-Word Mnemonic  │
│   Signed Invites (nodex://invite/...) • Key Rotation Alert  │
│   4-Tier Delivery Cascade • Presence • Encrypted Database   │
└──────────────────────────────┬──────────────────────────────┘
                               │ DHT & Transport
┌──────────────────────────────▼──────────────────────────────┐
│                    nodex-kademlia Layer                     │
│   RFC 5389 STUN Client • UPnP / NAT-PMP • Hole Punching     │
│   UDP Chunking (<=1200 bytes) • Opt-in Encrypted Peer Relay │
│   Multi-Source Bootstrap (Seeds, GitHub Raw, LAN Broadcast) │
└─────────────────────────────────────────────────────────────┘
```

---

## 🚀 Key Features

- **🛡️ 100% Private & Serverless Identity**: 0 phone numbers, 0 emails, 0 central databases. Identity is generated from a 128-bit seed represented as a 12-word BIP-39 mnemonic.
- **⚡ 4-Tier Delivery Cascade**:
  1. **Direct UDP**: Instant delivery when peer endpoint is reachable.
  2. **UDP Hole Punching**: Synchronized burst probes through Cone NAT.
  3. **Opt-in Encrypted Peer Relay**: Transit for peers behind Symmetric NAT / strict CGNAT.
  4. **DHT Mailbox Relay**: Decentralized Store-and-Forward replication when recipient is offline.
- **📦 UDP Chunking ($\le 1200$ bytes)**: Strict packet budget prevents IP MTU fragmentation and packet loss on internet routers.
- **🔗 Signed Invites (`nodex://invite/...`)**: Contains User ID, Ed25519/X25519 keys, endpoint candidates, 7-day TTL, and digital signature.
- **🔒 MitM Key Rotation Alert**: Automatically detects changes in contacts' public keys and blocks message sending until manual user confirmation.
- **⚡ Opt-in Encrypted Peer Relay**: Disabled by default; user can opt-in with strict protections: max 10 sessions, 20 pkts/sec rate limit, 50 MB/day traffic quota, RAM-only processing.

---

## 📁 Project Architecture

```text
NodeX/
├── config/                 # Network configurations (default, dev, production, bootstrap)
├── core-ffi/               # C FFI hashing, mnemonic generation, wire serialization
├── data/                   # Local application data directory (production uses %APPDATA%/NodeX)
├── deploy/                 # Deployment scripts (Windows NSIS installer, Linux systemd, macOS)
├── docs/                   # 11 Technical specifications and architecture guides
├── nodex-cli/              # Headless CLI node runner for servers and bootstrap seeds
├── nodex-gui/              # Modern native desktop GUI built with egui / eframe
├── nodex-kademlia/         # Kademlia DHT, STUN, UPnP, Hole Punching, Relay, Chunking
├── nodex-messenger/        # E2EE Crypto, Invites, Contacts, Mailbox, Presence
└── scripts/                # Utility scripts (build, test, demo, package)
```

---

## 🛠️ Quick Start

### 1. Run GUI Messenger
```powershell
cargo run -p nodex-gui
```

### 2. Run Headless CLI Node / Seed
```powershell
cargo run -p nodex-cli -- --port 8000
```

### 3. Run All Automated Tests
```powershell
cargo test --workspace
```

### 4. Build Release Binaries
```powershell
.\scripts\build-release.ps1
```

---

## 🧪 Test Suite Status

All **49 automated tests** across all crates pass cleanly:
```bash
cargo test --workspace
# 49 passed; 0 failed; 0 warnings
```

---

## 📄 License
MIT License. Open source and decentralized.
