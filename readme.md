# NodeX

<p align="center">
  <img src="nodex-gui/assets/logo.png" width="144" alt="NodeX logo" />
</p>

<p align="center">
  <strong>Native P2P E2EE Messenger</strong><br />
  Direct UDP delivery, Kademlia DHT discovery, and encrypted offline mailbox relay.
</p>

<p align="center">
  <a href="https://github.com/NetGadz/NodeX/tree/v1.0.0"><img src="https://img.shields.io/badge/release-v1.0.0-06b6d4" alt="Release v1.0.0" /></a>
  <img src="https://img.shields.io/badge/Rust-2021-orange?logo=rust" alt="Rust 2021" />
  <img src="https://img.shields.io/badge/GUI-egui-22c55e" alt="egui" />
  <img src="https://img.shields.io/badge/network-Kademlia-0ea5e9" alt="Kademlia" />
  <img src="https://img.shields.io/badge/security-E2EE-8b5cf6" alt="E2EE" />
  <img src="https://img.shields.io/badge/license-MIT-16a34a" alt="MIT" />
</p>

<p align="center">
  <img src="https://media.giphy.com/media/3oEjI6SIIHBdRxXI40/giphy.gif" width="760" alt="Distributed network animation" />
</p>

> NodeX is an experimental MVP. Local multi-node delivery is tested; public-internet deployment still needs production bootstrap nodes, NAT traversal, and relay infrastructure.

## Product Snapshot

NodeX is a native Rust desktop messenger designed around a distributed network instead of a central message server.
Each running instance owns a transport node, participates in Kademlia routing, and keeps its user identity separate
from its network identity.

### What works today

- Windows desktop GUI built with Rust and egui.
- Account creation and restoration with a 12-word mnemonic flow.
- Persistent user identity with Ed25519 signing and X25519 encryption keys.
- Direct UDP `STORE` delivery with `STORE_ACK` confirmation.
- DHT store-and-forward mailbox for offline recipients.
- Automatic local UDP port fallback when the preferred port is busy.
- Local bootstrap discovery across ports `8000..8010`.
- Contact search by 40-character User ID.
- Encrypted local messenger database and chat history.
- Text and bounded image attachments.
- Contact profiles, unread indicators, status ticks, and diagnostics modules.

<p align="center">
  <img src="https://media.giphy.com/media/26BROrSHl1D5n5gK4/giphy.gif" width="720" alt="P2P message delivery animation" />
</p>

## Architecture

```mermaid
flowchart TB
    GUI[NodeX Desktop GUI\nRust + egui]
    MSG[KAD Messenger\nidentity, contacts, chats]
    NET[Kademlia Node\nUDP RPC, routing, lookup]
    DHT[(Distributed Storage\nTTL + replication)]
    C[C FFI Core\nhashing + wire serialization]

    GUI --> MSG
    MSG --> NET
    NET --> DHT
    NET --> C
```

### Identity separation

```mermaid
flowchart LR
    M[12-word mnemonic] --> K[Deterministic key material]
    K --> U[User E2EE ID]
    K --> S[Ed25519 signing key]
    K --> X[X25519 encryption key]
    N[Transport node state] --> T[Kademlia Transport Node ID]
    U -. persistent account .- MSG[Encrypted messenger identity]
    T -. routing position .- DHT[DHT routing table]
```

| Identity | Used for | Persistence |
| --- | --- | --- |
| User E2EE identity | Signatures, encryption, contacts, account recovery | Persistent |
| Transport node identity | Kademlia XOR distance and routing | Persistent state, can be rotated |

## Message Delivery

```mermaid
sequenceDiagram
    participant A as Alice
    participant B as Bob
    participant D as DHT nodes

    A->>A: Encrypt envelope for Bob
    A->>B: UDP STORE mailbox payload
    alt Bob is online
        B-->>A: STORE_ACK
        B->>B: Inbox worker decrypts envelope
    else Bob is offline
        A->>D: Store encrypted mailbox envelope
        D-->>A: Replication result
        B->>D: FIND_VALUE mailbox key after reconnect
        D-->>B: Encrypted envelope
        B->>B: Verify, decrypt, deduplicate
    end
```

### Message lifecycle

```mermaid
stateDiagram-v2
    [*] --> Sending
    Sending --> Sent: local persistence
    Sent --> Delivered: STORE_ACK or mailbox acceptance
    Delivered --> Read: recipient opens chat
    Sending --> Failed: validation or transport error
    Sent --> Failed: retries exhausted
    Failed --> Sending: retry
```

The current transport uses a shared DHT mailbox key and envelope deduplication. A future protocol revision will use
per-message mailbox keys and signed delivery/read receipts as first-class wire events.

## Security Model

```mermaid
flowchart LR
    P[Mnemonic] --> ID[Persistent identity]
    ID --> SIG[Ed25519 signature]
    ID --> DH[X25519 shared secret]
    DH --> AEAD[ChaCha20-Poly1305 envelope]
    SIG --> VERIFY[Recipient verifies sender]
    AEAD --> STORE[UDP or DHT mailbox]
    STORE --> DECRYPT[Recipient decrypts locally]
```

- Message contents are encrypted before transport.
- DHT storage holds encrypted envelopes, not plaintext chat text.
- Presence cards carry public keys and signatures.
- User IDs and transport IDs are different values.
- The local database is stored in an encrypted format in the current MVP.

Before public production use, the project still needs OS-backed key storage, a reviewed standard mnemonic derivation,
key-change warnings, replay protection, and a complete signed receipt protocol. Do not treat the current build as a
security-audited messenger.

## Download

The current source tag is [v1.0.0](https://github.com/NetGadz/NodeX/tree/v1.0.0).

The Windows package is generated locally as:

```text
dist/NodeX-v1.0.0-windows-x64.zip
```

It contains:

```text
nodex.exe
logo.png
config.json
README.txt
```

Generated binaries and local user databases are intentionally not committed to the repository. Attach the ZIP to a
GitHub Release when distributing the executable to end users.

## Quick Start: Windows Development

Requirements:

- Windows 10 or newer.
- Rust toolchain with Cargo.
- A C compiler supported by the Rust toolchain.

Run tests:

```powershell
cargo test --workspace
```

Start the GUI:

```powershell
cargo run -p nodex-gui
```

Start a second local node by running the same command in another terminal. The second instance automatically tries
the next available UDP port and searches local bootstrap ports. No `--port` or `--bootstrap` argument is required for
the local demo.

Build optimized binaries:

```powershell
.\scripts\build-release.ps1
```

Build the Windows ZIP package:

```powershell
.\scripts\package-windows.ps1 -Version "1.0.0"
```

The output is written to `dist/`.

## First-Run Flow

1. Open NodeX.
2. Create an account and save the 12-word recovery phrase, or restore an existing account.
3. Choose a display name and optional bio.
4. Copy the User ID from the profile panel.
5. Add the second user by User ID.
6. Open the contact and send a text or a small image.
7. Start the recipient later to exercise mailbox store-and-forward.

Never share a recovery phrase. Anyone with the phrase can recreate the account identity.

## Repository Layout

```text
.
|- Cargo.toml
|- core-ffi/                  C hashing, mnemonic, and wire boundary
|- nodex-kademlia/            routing, RPC, DHT storage, bootstrap, peers
|- nodex-messenger/           identity, E2EE, contacts, mailbox, receipts
|- nodex-gui/                 native desktop client and logo asset
|- nodex-cli/                 diagnostics and interactive CLI
|- config/                    development and bootstrap configuration
|- docs/                      architecture, protocol, security, release docs
|- deploy/                    Windows, Linux, and macOS packaging layouts
|- data/                      development-only placeholder
`- scripts/                   build, demo, test, and cleanup scripts
```

### Important modules

| Package | Key modules |
| --- | --- |
| `core-ffi` | `mnemonic.c`, `serialize.c`, `hash.c` |
| `nodex-kademlia` | `node.rs`, `rpc.rs`, `lookup.rs`, `storage.rs`, `bootstrap.rs`, `peer_manager.rs` |
| `nodex-messenger` | `crypto.rs`, `identity.rs`, `messenger.rs`, `mailbox.rs`, `presence.rs`, `receipts.rs` |
| `nodex-gui` | `main.rs`, `gui.rs`, `assets/logo.png` |

## Tests and Verification

The workspace currently contains local unit and integration coverage for:

- C FFI mnemonic roundtrip.
- Node IDs, XOR distance, and k-buckets.
- Port fallback and bootstrap.
- DHT storage, TTL, replication, and node failure.
- E2EE encryption and tamper detection.
- Identity restore and contact verification.
- Mailbox delivery and deduplication.
- Attachments, receipts, replay checks, and reconnect modules.

Run the complete suite:

```powershell
cargo test --workspace
```

The locally verified workspace baseline is 44 discovered tests passing. Always rerun the suite after changing the
wire format, identity derivation, mailbox behavior, or GUI message model.

## Production Gaps

NodeX is not yet a public-internet production messenger. The remaining deployment work includes:

- Public bootstrap nodes with health monitoring.
- NAT traversal or relay support for users behind routers.
- IPv4/IPv6 and network-change recovery.
- A complete per-message queue with signed delivery/read receipts.
- OS-backed local secret storage and security review.
- Signed installers and release assets.
- External-network end-to-end tests.

## Documentation

- [Architecture](docs/architecture.md)
- [Protocol](docs/protocol.md)
- [Networking](docs/networking.md)
- [Security](docs/security.md)
- [Threat model](docs/threat-model.md)
- [Account recovery](docs/account-recovery.md)
- [Message lifecycle](docs/message-lifecycle.md)
- [Bootstrap nodes](docs/bootstrap-nodes.md)
- [Release guide](docs/release.md)
- [Testing guide](docs/testing.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Production MVP specification](docs/production-mvp-tz.txt)

## Roadmap

- [x] Local multi-node bootstrap and automatic port fallback.
- [x] Direct UDP store with acknowledgement.
- [x] DHT mailbox store-and-forward path.
- [x] Mnemonic identity and encrypted local database format.
- [x] Windows release build and `v1.0.0` tag.
- [ ] Public bootstrap and relay infrastructure.
- [ ] NAT traversal across different home networks.
- [ ] Complete signed delivery/read receipt protocol.
- [ ] Security audit and OS-backed secret storage.
- [ ] Signed installer and automatic update channel.

## Contributing

Issues, protocol critiques, tests, and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before
opening a change.

## License

MIT. See [LICENSE](LICENSE).
