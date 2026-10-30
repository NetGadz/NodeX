# NodeX threat model (experimental)

This is a high-level description of intended protections and known gaps in the current repository. It is not a formal protocol proof or an independent audit. See [security model and limitations](security.md).

## Assets

- Message and attachment contents while stored or transported as encrypted envelopes.
- Account identity and signing/encryption secrets stored on the user's device.
- Contact identity keys and local conversation data.

## Adversaries considered

- A passive network observer or DHT participant that records traffic and mailbox entries.
- A malicious or compromised bootstrap/DHT node that can observe, withhold, replay, or misroute data.
- An active network attacker that can modify, inject, replay, delay, or drop packets.
- An attacker who obtains a user's long-term account keys or compromises their device.

## Current protections

- Message envelopes use ChaCha20-Poly1305 with keys derived via HKDF-SHA256 from X25519. Current v4 envelopes use a fresh sender ephemeral key and the recipient's static X25519 key. Ed25519 signatures cover envelope fields, and the receiver checks the sender ID against the signing key.
- Invite and presence metadata can be signed. This protects integrity relative to the included identity key; it does not establish that the key belongs to a real-world person unless the contact is authenticated separately.
- A replay cache rejects duplicate signatures and envelopes outside its configured time window while the process cache retains the entry. This is bounded replay mitigation, not general MitM or denial-of-service protection.
- DHT nodes store ciphertext envelopes rather than plaintext message bodies.

## Important gaps and limits

- There is no X3DH/prekey handshake, no Double Ratchet, and no post-quantum ML-KEM handshake in the current message-encryption path. Recorded v4 envelopes may be decrypted if the recipient's long-term X25519 secret is later compromised. Do not claim full forward secrecy or post-compromise security.
- Self-signed identity data does not by itself authenticate a person. A secure out-of-band fingerprint comparison is needed to confirm that a key belongs to the intended contact. The current GUI does not expose a complete key-verification flow.
- Transport metadata, including IP addresses, traffic timing, sizes, and DHT lookups, may be visible to peers and infrastructure. The onion packet module is not connected to normal message delivery.
- The DHT depends on configured bootstrap/discovery endpoints and participating peers. Nodes may censor, observe, drop, or fail to retain traffic. Delivery and availability are best-effort.
- STUN does not relay traffic. Calls may require TURN in restrictive NAT/firewall environments and have not been independently audited or comprehensively tested across deployments.
- Compromise of a device or account secrets can expose local data, permit identity impersonation, or compromise communications. Password protection of the local database does not protect a device while it is unlocked or malware-infected.

## Timestamp and replay details

Envelope timestamps are Unix seconds. Replay protection is an in-memory cache with a seven-day acceptance window and a fixed entry cap; it is not durable across restarts. These checks should not be described as preventing all replay, MitM, or DoS attacks.

## Security reporting

Report suspected vulnerabilities privately to the maintainers before publishing exploit details. NodeX is experimental and should not be used for high-risk communications pending independent review.
