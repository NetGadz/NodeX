# NodeX Threat Model

## 1. Threat Vectors and Mitigations

### 1.1 Eavesdropping / Man-in-the-Middle (MitM)
- **Threat**: Network snooping on local Wi-Fi or ISP levels.
- **Mitigation**: All payload traffic is encrypted with ChaCha20-Poly1305 using authenticated X25519 public keys. Relay nodes only see ciphertext envelopes. Fingerprints can be verified out-of-band.

### 1.2 Sybil Attacks on DHT
- **Threat**: Malicious nodes attempting to overwhelm routing tables and isolate honest nodes.
- **Mitigation**: Node IDs are derived deterministically or verified with crypto challenge. K-bucket replacement policies require pinging existing contacts before eviction, preventing rapid cache poisoning.

### 1.3 Message Tampering and Replay Attacks
- **Threat**: Malicious actor modifying or re-broadcasting recorded message envelopes.
- **Mitigation**: Every message is signed with sender's Ed25519 private key including a nanosecond timestamp. Recipients reject messages with altered signatures or outdated/duplicate nonces.

### 1.4 Offline Relay Snooping
- **Threat**: DHT relay nodes reading mailbox storage.
- **Mitigation**: Mailbox records only store encrypted envelopes. Relays cannot decrypt message contents or forge valid signatures.
