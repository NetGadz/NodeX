# NodeX network and message protocol (implementation notes)

This document summarizes the current code paths. It is not a stable protocol specification; wire formats and compatibility may change while NodeX is experimental.

## Network discovery and transport

- The networking crate exchanges Kademlia RPC packets over UDP. RPCs include ping, store, and value/node lookup operations; the implementation's request IDs are 64-bit values.
- Clients use configured bootstrap peers to discover the DHT. A DHT is distributed storage and lookup, not a guarantee of anonymity, availability, or direct reachability.
- NAT traversal helpers include STUN, UPnP/NAT-PMP, and hole-punching code. Results depend on the network and peer configuration.

## Message delivery

1. The sender obtains the recipient's X25519 public key from a saved contact or a signed presence card.
2. The sender encrypts a message envelope and attempts to store it under a mailbox key derived from the recipient's public user ID and the current day bucket.
3. The DHT put operation reports the number of replicas. The code treats fewer than two replicas as unconfirmed and queues a retry. This is not an acknowledgement that the recipient received or read the message.
4. The recipient polls current and nearby day-bucket mailbox keys, validates and decrypts envelopes, and keeps duplicate protection in an in-memory replay cache.

Mailbox keys are deterministic from public data; they are routing identifiers, not secrets. Mailbox availability and retention depend on DHT peers and storage TTL. The DHT may observe lookup keys, timing, sizes, and network addresses.

## Envelope versions and client compatibility

- The Rust desktop sender currently emits version 4 envelopes. It derives the content key from ephemeral-sender-to-static-recipient X25519, HKDF-SHA256, and encrypts with ChaCha20-Poly1305. Ed25519 signs the envelope fields.
- The Rust receiver also contains compatibility paths for versions 1–3.
- The Android implementation currently emits version 3 envelopes and derives its content key from both ephemeral and static X25519 terms using the v2 HKDF label. Its signing format uses a `NodeX-envelope-v3` domain separator and includes fields that the Rust v3 verifier does not sign/verify in the same way.
- The Rust v4 and Android v3 encryption and signature formats do not match. Based on the current source paths, cross-client message decryption is incompatible in both directions. Cross-client messaging must not be advertised as working until envelope version negotiation/handling is made consistent and tested.
- None of these paths implements X3DH, a Double Ratchet, or ML-KEM. Do not claim full forward secrecy, post-compromise security, or post-quantum protection.

## Calls

Desktop call setup signals are carried inside messenger payloads. The desktop call media path uses WebRTC ICE and DTLS-SRTP; direct connectivity depends on NAT/firewall conditions, and some networks require a configured TURN server. Legacy UDP audio code is also present. Calls are experimental and not a guarantee of cross-network connectivity.

## Logging and diagnostics

The Rust clients currently use `println!` and `eprintln!` at many call sites. There is no consistent structured logging interface or documented log-level configuration in the reviewed code. GUI diagnostics may not be visible when launched as a windowed application. Logs should not contain message plaintext, recovery phrases, passwords, or private keys.
