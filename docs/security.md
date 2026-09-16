# NodeX Security Specification

## 1. Cryptographic Primitives
- **Identity & Signatures**: Ed25519 (Edwards-curve Digital Signature Algorithm).
- **Key Exchange**: X25519 (ECDH key agreement).
- **Authenticated Encryption**: ChaCha20-Poly1305 AEAD.
- **Hashing & Fingerprints**: BLAKE3 / SHA-256 (64 hex characters formatted as `XXXX-XXXX-...-XXXX`).
- **Mnemonic Key Derivation**: BIP-39 (12-word English wordlist, 128-bit entropy, PBKDF2-HMAC-SHA512).

## 2. End-to-End Encryption Workflow
1. Ephemeral or sender identity X25519 private key derives a shared secret via Diffie-Hellman with recipient's public key.
2. A 256-bit symmetric key is derived using BLAKE3 KDF.
3. Plaintext payload (text + optional image attachment) is encrypted with ChaCha20-Poly1305 with a 96-bit random nonce.
4. An Ed25519 signature is computed across `(sender_pubkey || recipient_pubkey || ciphertext || timestamp)` to ensure non-repudiation and replay protection.
5. Encrypted envelopes cannot be decrypted by intermediate DHT nodes or relay mailboxes.
