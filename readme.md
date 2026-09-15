# NodeX — P2P Messenger on Kademlia DHT

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.74%2B-orange?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/status-early%20prototype-yellow" alt="Status" />
  <img src="https://img.shields.io/badge/P2P-E2EE-blue" alt="P2P E2EE" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT" />
</p>

<p align="center">
  <img src="https://media.giphy.com/media/l0MYt5jPR6QX5pnqM/giphy.gif" width="920" alt="P2P network demo" />
</p>

NodeX is a decentralized messaging prototype built on top of a Kademlia DHT. The project is currently in an early stage of development: the core network logic, routing, peer discovery, distributed storage, and a desktop messenger prototype are already in place, but the architecture and feature set are still evolving.

This project is meant as a practical learning platform and a strong foundation for building a real peer-to-peer messaging system in Rust.

> Status: early prototype / pre-alpha
>
> The repo is intentionally open for experimentation, refactoring, improvements, and contributions.

---

## Why this project

This is not a finished commercial messenger. It is a hands-on distributed systems project focused on:

- learning Kademlia DHT mechanics;
- building resilient peer-to-peer networking in Rust;
- experimenting with encrypted messaging flows;
- testing distributed message storage and offline delivery patterns;
- creating a clean foundation for future UI and protocol extensions.

---

## Highlights

- 🌐 Decentralized P2P routing with Kademlia-style node lookup
- 🔐 Encrypted identity and messaging model
- 💬 Local contacts and chat history persistence
- 📦 DHT-backed mailbox flow for offline recipients
- 🧠 Routing table, peer discovery, and distributed storage
- 🖥️ Native desktop UI built with egui
- 🧪 Open for further experimentation and PRs

---

## What is already implemented

### Network layer

- `KademliaNode` with XOR-based routing behavior
- bucketed routing table and peer tracking
- iterative lookup for nodes and values
- UDP networking with Tokio
- retry and timeout handling for RPC calls

### Messenger layer

- persistent contact database
- local message storage
- E2EE envelope creation and decryption
- peer discovery via presence records in DHT
- mailbox-style message relay for offline peers
- GUI with user card, contacts, and chat panel

### Storage and observability

- key-value storage across network nodes
- basic metrics for traffic and RPC flow
- logging of network events and node state
- persistence of identity and local data

---

## Demo

<p align="center">
  <img src="https://media.giphy.com/media/26BRv0ThflsHCqDrG/giphy.gif" width="800" alt="Messaging demo" />
</p>

<p align="center">
  <img src="https://images.unsplash.com/photo-1516321318423-f06f85e504b3?auto=format&fit=crop&w=1200&q=80" width="820" alt="Distributed systems workspace" />
</p>

---

## Quick start

### 1. Build the project

```bash
cargo build
```

### 2. Start first node

```bash
cargo run -- --name "Alice" --port 8000
```

### 3. Start second node with bootstrap

```bash
cargo run -- --name "Bob" --port 8001 --bootstrap 127.0.0.1:8000
```

### 4. Open the desktop app

The app launches a native Rust GUI with:

- user identity card
- contact list
- chat panel
- network state information

---

## Example flow

```text
Alice -> Bob: hello from the P2P network
Bob -> Alice: encrypted reply delivered through DHT mailbox
Alice -> peers: discover route and update network state
```

---

## Architecture

```text
┌──────────────────────────────────────────────┐
│                 GUI / Desktop                 │
│   contacts • chat • node info • status       │
└──────────────────────┬───────────────────────┘
                       │
┌──────────────────────▼───────────────────────┐
│                 KadMessenger                  │
│  identity • presence • encrypted messages    │
│  local DB • contact tracking • inbox         │
└──────────────────────┬───────────────────────┘
                       │
┌──────────────────────▼───────────────────────┐
│                  KademliaNode                 │
│ routing table • lookup • UDP networking      │
│ storage • replication • RPC flow             │
└──────────────────────┬───────────────────────┘
                       │
┌──────────────────────▼───────────────────────┐
│                    C FFI                     │
│ hashing • serialization • verification       │
└──────────────────────────────────────────────┘
```

---

## Roadmap

### Current stage

- working prototype of network routing
- basic messenger flow with persisted contacts and messages
- DHT peer discovery and distributed data exchange
- simple GUI for demo and debugging

### Planned next steps

- [ ] improve message delivery and offline handling
- [ ] refine E2EE and identity verification
- [ ] improve contact management and user profiles
- [ ] improve UI/UX and node status panels
- [ ] add broader test coverage for edge cases and failures
- [ ] clean up API boundaries and project structure

---

## Project structure

```text
.
├── Cargo.toml
├── build.rs
├── Dockerfile
├── docker-compose.yml
├── config.example.json
├── core/
│   ├── hash.c
│   ├── hash.h
│   ├── messenger_core.c
│   ├── messenger_core.h
│   ├── serialize.c
│   └── serialize.h
├── src/
│   ├── config.rs
│   ├── crypto.rs
│   ├── db.rs
│   ├── gui.rs
│   ├── lib.rs
│   ├── lookup.rs
│   ├── main.rs
│   ├── messenger.rs
│   ├── metrics.rs
│   ├── node.rs
│   ├── rpc.rs
│   ├── state.rs
│   ├── storage.rs
│   └── ...
├── tests/
│   ├── integration_test.rs
│   └── messenger_test.rs
├── scripts/
│   ├── demo.ps1
│   └── demo.sh
├── readme.md
├── CONTRIBUTING.md
├── LICENSE
├── .gitignore
└── messenger_db_*.json
```

---

## Important note

This project is an early-stage distributed messaging prototype. It already demonstrates the core idea, but it is still evolving and should be treated as a research/learning project rather than a production-ready solution.

That is exactly why it is a good candidate for:

- code cleanup,
- protocol improvements,
- architecture refactors,
- better security design,
- more polished UI and stronger developer tooling.

---

## Contributing

This project still has plenty of room for improvement, and practical ideas, bug fixes, and architectural proposals are always welcome.

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

<p align="center">
  <img src="https://media.giphy.com/media/3o7aD2saalBwwftBIY/giphy.gif" width="700" alt="Open collaboration" />
</p>

---

## License

MIT