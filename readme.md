<div align="center">

# ⚡ NodeX

### Next-Generation Sovereign P2P Communications Engine
**Zero Central Servers • End-to-End Cryptographic Sovereignty • Decentralized Kademlia Mesh • Stego-Carriers**

[![Release](https://img.shields.io/github/v/release/NetGadz/NodeX?color=3b82f6&style=for-the-badge&logo=github)](https://github.com/NetGadz/NodeX/releases/latest)
[![Build Status](https://img.shields.io/github/actions/workflow/status/NetGadz/NodeX/release.yml?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/NetGadz/NodeX/actions)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL_3.0-10b981.svg?style=for-the-badge)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/Language-Rust_1.75+-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet?style=for-the-badge)](https://github.com/NetGadz/NodeX/releases)

<br/>

[📥 **Download NodeX for Windows (.exe)**](https://github.com/NetGadz/NodeX/releases/latest/download/NodeX.exe) • [📦 **Download ZIP Bundle (.zip)**](https://github.com/NetGadz/NodeX/releases/latest/download/NodeX-windows-x64.zip) • [📖 **Documentation**](#-architecture--5-tier-delivery-cascade) • [🛡️ **Security Model**](#-cryptographic--security-architecture)

</div>

---

## 📌 Executive Summary

**NodeX** is a sovereign, serverless, peer-to-peer (P2P) desktop communication platform written from the ground up in modern **Rust**. 

Unlike conventional "secure" messengers that rely on centralized routing servers, phone number registrations, or proprietary cloud infrastructure, NodeX creates an autonomous distributed overlay network. Every client is an independent cryptographic node that participates directly in routing, distributed storage, and zero-knowledge message relaying.

---

## 🚀 Quick Download & Installation

### Pre-Built Binaries (Windows x64)
| Asset | Description | Direct Download Link |
| :--- | :--- | :--- |
| 🚀 **NodeX.exe** | Standalone production executable (Portable) | [**Download NodeX.exe**](https://github.com/NetGadz/NodeX/releases/latest/download/NodeX.exe) |
| 📦 **NodeX-windows-x64.zip** | Full archive (Binary + Docs + License) | [**Download Archive (.zip)**](https://github.com/NetGadz/NodeX/releases/latest/download/NodeX-windows-x64.zip) |
| 🌐 **GitHub Releases Page** | All tags, release notes, and checksums | [**View Releases**](https://github.com/NetGadz/NodeX/releases) |

---

## ✨ Key Architectural Highlights

```mermaid
graph TD
    A[NodeX Client A] -->|1. Direct UDP Fast Path / 0-RTT| B[NodeX Client B]
    A -.->|2. NAT Hole Punching STUN/ICE| B
    A ==>|3. Blind 2-Hop Onion Circuit| R1[Relay Node 1] ==> R2[Relay Node 2] ==> B
    A -->|4. Store & Forward Encrypted Mailbox| DHT[(Kademlia DHT Mesh)]
    DHT -.->|Retrieve on Node Online| B
    A -->|5. LSB Steganographic Image Carrier| B
```

### 1. 🛡️ Cryptographic Zero-Trust Core
- **Asymmetric Identity**: Native **Ed25519** signing keys and **X25519** key-agreement identities.
- **Double Ratchet Engine**: Forward Secrecy (PFS) and Break-in Recovery for every session using **HKDF-SHA256** and **ChaCha20-Poly1305** AEAD.
- **Zero-Knowledge Addressing**: User identities are 256-bit cryptographic public hashes (`NodeId`). No phone numbers, emails, or KYC required.

### 2. ⚡ 5-Tier Resilient Delivery Cascade
1. **Direct Fast-Path UDP (0-RTT)**: Low-latency direct socket transport for active peers.
2. **Autonomous NAT Traversal (ICE/STUN)**: Hole-punching mechanism resolving Symmetric and Cone NATs across diverse WAN topologies.
3. **Blind 2-Hop Onion Routing**: Multi-layered payload encapsulation across random intermediary nodes, hiding IP origins from recipients and ISPs.
4. **Decentralized DHT Mailbox (Store-and-Forward)**: When a recipient is offline, encrypted blobs are sharded and deposited into Kademlia DHT neighborhood storage with time-to-live (TTL) expiration.
5. **Steganographic Crypto-Carriers**: Out-of-band identity and credential transmission through covert LSB embedding inside standard images.

### 3. 🎙️ Real-Time P2P Voice & Media Subsystem
- **Direct Low-Latency Voice Calling**: Low-overhead UDP fast-path with sub-50ms latency.
- **Encrypted Media Buffering**: Dedicated ring-buffer audio pipeline powered by `cpal` and `rodio` with non-blocking audio capture and playback.
- **Dynamic Connection Recovery**: Seamless fallback if UDP ports or network interfaces switch mid-call.

### 4. 🖼️ Steganographic Crypto-Avatars (Stego-Carrier)
- **Deep LSB Encoding**: Embeds complete cryptographic contact descriptors (public keys, DHT rendezvous tokens, endpoints) invisibly inside 24-bit PNG/JPEG pixel matrices.
- **Air-Gapped Contact Onboarding**: Share your contact avatar across regular public image hosting platforms or social channels without triggering DPI surveillance or metadata scrapers.

### 5. 🎨 High-Performance Native UI
- **Hardware-Accelerated Rendering**: Pure Rust interface rendered via `egui` and `eframe` (Glow/OpenGL backend).
- **Movable & Resizable Workspaces**: Flexible floating modals for Network Statistics, Contact Discovery, Calls, and Group Management.
- **Dynamic Themes**: Curated `Dark Space`, `Midnight OLED (True Black)`, and `Swiss Day Light` palettes with zero runtime performance cost.

---

## 🏗️ Monorepo Architecture

The workspace is organized into clean, modular Rust crates:

```
NodeX/
├── nodex-gui/          # Native GPU-accelerated desktop interface (eframe/egui)
├── nodex-messenger/    # Double Ratchet, E2EE Session Engine, Audio Pipeline, Stego
├── nodex-kademlia/     # Distributed Hash Table, Routing Table (k-buckets), RPC
├── nodex-cli/          # Headless daemon, management CLI and automation harness
├── core-ffi/           # C-compatible FFI bindings for mobile/cross-platform embedding
├── .github/workflows/  # Automated CI/CD release build pipeline
├── LICENSE             # GNU General Public License v3.0
└── Cargo.toml          # Workspace root manifest
```

---

## 🛠️ Building From Source

### Prerequisites
- **Rust Toolchain**: `1.75.0` or later ([rustup.rs](https://rustup.rs/))
- **Build Tools**: CMake, MSVC (Windows) or `build-essential` / `libasound2-dev` (Linux)

### Build Steps

1. **Clone the repository:**
   ```bash
   git clone https://github.com/NetGadz/NodeX.git
   cd NodeX
   ```

2. **Compile the Desktop Application in Release Mode:**
   ```bash
   cargo build --release -p nodex-gui
   ```

3. **Run NodeX:**
   ```bash
   # Windows
   .\target\release\nodex-gui.exe

   # Linux / macOS
   ./target/release/nodex-gui
   ```

4. **Run Headless CLI Node:**
   ```bash
   cargo run --release -p nodex-cli -- --help
   ```

---

## 🔐 Cryptographic & Security Specification

| Layer | Primitive / Algorithm | Specification & Purpose |
| :--- | :--- | :--- |
| **Node Identity** | `Ed25519` (RFC 8032) | Digital signatures, authentication & Node ID hashing |
| **Key Agreement** | `X25519` (RFC 7748) | Diffie-Hellman ephemeral key exchanges |
| **Symmetric Cipher** | `ChaCha20-Poly1305` | Authenticated Encryption with Associated Data (AEAD) |
| **Key Derivation** | `HKDF-SHA256` & `Argon2id` | KDF for double ratchet states and local vault master keys |
| **Routing Protocol**| `Kademlia DHT` ($k=20, \alpha=3$) | $\mathcal{O}(\log N)$ distributed peer and message discovery |
| **Steganography** | Custom 2-bit LSB Carrier | Encrypted payload embedding with CRC32 integrity validation |

---

## 🤝 Contributing

Contributions from the security, cryptography, and decentralized networking communities are welcomed.

1. **Fork** the repository.
2. Create your feature branch (`git checkout -b feature/quantum-hardening`).
3. Ensure the workspace compiles cleanly (`cargo check --workspace`).
4. Commit your changes (`git commit -m 'feat: implement hybrid post-quantum ratchet'`).
5. Push to the branch (`git push origin feature/quantum-hardening`).
6. Open a **Pull Request**.

---

## 📄 License

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**.  
See the [LICENSE](LICENSE) file for full license text and permissions.

---

<div align="center">
  <sub>Built with ❤️ and Rust for privacy, human sovereignty, and true decentralization.</sub>
  <br/>
  <sub>© 2026 NodeX Project Contributors</sub>
</div>
