# Security model and limitations

NodeX is an experimental messenger. This document describes the code and configuration in this repository; it is not an independent security audit or a guarantee that the implementation is free of vulnerabilities. Do not use NodeX for high-risk communications.

## What the current code does

- User identities use Ed25519 signing keys and separate X25519 encryption keys derived from the account seed.
- Current message envelopes (version 4) use one ephemeral sender X25519 key and the recipient's long-term X25519 public key. HKDF-SHA256 derives a ChaCha20-Poly1305 key. The envelope is signed with the sender's Ed25519 key, and the sender ID is checked against that signing key.
- The receiver accepts legacy envelope versions 1–3 as well. Those versions use older key derivation schemes; they do not provide forward secrecy.
- The Android client currently emits version 3 with a different signature format and static-plus-ephemeral X25519 derivation. These do not match desktop v4; source inspection indicates Android and desktop cannot currently decrypt each other's messages.
- Invite and presence cards are signed by the identity key they advertise. This proves that the card was signed by the corresponding key; it does not independently prove a real-world identity.
- Desktop local database encryption uses Argon2id and ChaCha20-Poly1305 in the password-protected `NODEXENC3` path. A legacy local-key path remains for CLI/backend callers. See the database implementation and migration behavior before making claims about other clients.
- Desktop call signaling is carried in encrypted messenger messages; the desktop media path uses WebRTC ICE and DTLS-SRTP. Calls remain experimental and network-dependent.
- Mailbox entries are encrypted envelopes. DHT availability, retention, and delivery are not guaranteed.

## Properties not provided or not established

- **No Double Ratchet / no full forward secrecy:** Version 4 uses ephemeral-to-static DH. A recorded envelope can be decrypted later if the recipient's long-term X25519 secret is compromised. The sender's Ed25519 signing-key compromise alone does not derive that envelope's content key, but it can enable forged messages under that identity. Legacy versions have older constructions. Do not claim full forward secrecy or post-compromise security.
- **No X3DH or post-quantum handshake:** The current envelope code is not X3DH, does not use signed or one-time prekeys, and does not implement a Double Ratchet. It has no ML-KEM/Kyber hybrid key exchange.
- **Identity authentication depends on how a contact is obtained:** Self-signatures bind advertised data to a key. To confirm that key belongs to the person intended, users need an authenticated channel or an independently compared fingerprint. A valid signature alone does not establish a person's real-world identity. The GUI does not currently provide a complete key-verification workflow.
- **No anonymity or metadata protection:** Direct peers, bootstrap nodes, DHT nodes, STUN services, and network observers may see IP addresses, timing, sizes, lookup activity, and other transport metadata. The onion packet module is not used by the normal message-delivery path; the GUI toggle does not activate onion routing.
- **Not serverless in the absolute sense:** The client uses configured bootstrap/discovery infrastructure. DHT storage is distributed, but bootstrap availability and participating peers affect discovery and delivery.
- **Delivery is best-effort:** NAT traversal, direct connections, DHT storage, offline mailbox retrieval, and calls depend on peer availability, NAT/firewall behavior, configuration, and network conditions. A local send or DHT storage result is not proof that the recipient read a message.
- **Steganography is not guaranteed concealment:** Image resizing, recompression, or format conversion can destroy embedded data, and embedding data does not hide network traffic.
- **No published independent audit:** The protocol and implementation have not received a published independent cryptographic/security audit or a comprehensive deployment test across supported network conditions.

## Local data hygiene

Runtime databases, key sidecars, backups, state files, screenshots, and scratch scripts are intended to be excluded by `.gitignore`. Ignoring a file does not remove it from Git history if it was committed earlier. Check repository history before publishing and rotate secrets that may have been exposed.

## Reporting security issues

Please contact the maintainers privately before publishing exploit details. Treat NodeX as experimental until the protocol and implementation have received independent review.
