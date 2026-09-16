# NodeX Production MVP Architecture

## 1. High-Level Overview

NodeX is a pure native Rust decentralized, peer-to-peer, end-to-end encrypted messaging application.

```
┌─────────────────────────────────────────────────────────────┐
│                 NodeX Desktop GUI (egui/eframe)             │
│  - Contacts, Active Chat, Lightbox, User Profile, Settings  │
└──────────────────────────────┬──────────────────────────────┘
                               │ UI Events & Commands
┌──────────────────────────────▼──────────────────────────────┐
│                    nodex-messenger Layer                    │
│  - User Identity (Ed25519 / X25519)                         │
│  - Mailbox Relay & Direct UDP RPC Store Engine              │
│  - BIP-39 12-Word Mnemonic Identity Recovery                │
│  - Contacts, Presence, Delivery Receipts & Status           │
└──────────────────────────────┬──────────────────────────────┘
                               │ DHT & Transport
┌──────────────────────────────▼──────────────────────────────┐
│                    nodex-kademlia Layer                     │
│  - XOR Distance Metric & Routing Table (k=20, α=3)          │
│  - UDP RPC Engine (PING, STORE, FIND_NODE, FIND_VALUE)      │
│  - PortManager (Auto Port Binding 8000..8050)               │
│  - BootstrapEngine, ReconnectManager, HealthTracker         │
└──────────────────────────────┬──────────────────────────────┘
                               │ Cryptographic Support
┌──────────────────────────────▼──────────────────────────────┐
│                      core-ffi Layer                         │
│  - Blake3 Hashing, C-compatible FFI, Safe Memory Wrappers   │
└─────────────────────────────────────────────────────────────┘
```

## 2. Key Modules
- **`nodex-kademlia`**: Distributed hash table implementation following Maymounkov & Mazières Kademlia specification with production extensions for node auto-binding, reconnect supervision, and peer tracking.
- **`nodex-messenger`**: Identity management, ChaCha20-Poly1305 + X25519 E2EE messaging, dual-path delivery (Direct UDP RPC for instant online delivery + DHT Mailbox Relay for offline store-and-forward), and local encrypted JSON storage.
- **`nodex-gui`**: Clean, modern Telegram-style user interface built with native pure Rust `eframe`/`egui`.
