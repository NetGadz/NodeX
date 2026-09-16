# NodeX Account Recovery Guide

## 1. 12-Word Mnemonic Phrase
When a user launches NodeX for the first time, a 128-bit cryptographic seed is generated and represented as a 12-word BIP-39 mnemonic seed phrase (e.g. `abandon amount liar ...`).

## 2. Deterministic Key Derivation
- The 12-word mnemonic is hashed using PBKDF2-HMAC-SHA512 to produce a 64-byte master seed.
- Seed bytes `0..32` derive the Ed25519 signing keypair.
- Seed bytes `32..64` derive the X25519 encryption keypair.
- The User ID is the hexadecimal string of the Ed25519 public key.

## 3. Restoring an Account
1. Open NodeX settings or registration window.
2. Enter your 12-word recovery phrase.
3. NodeX recreates your exact cryptographic identity, keys, and User ID.
4. Contacts and messages can be retrieved or discovered on the DHT without relying on centralized servers.
