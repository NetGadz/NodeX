# NodeX Test Suite & Verification Guide

## 1. Running Tests
To run all tests across all crates in the workspace:
```bash
cargo test --workspace
```

## 2. Test Suite Overview
- **Kademlia DHT (`nodex-kademlia`)**:
  - `kbucket_respects_capacity`
  - `xor_distance_properties`
  - `storage_put_and_get`
  - `roundtrip_ping_pong`, `roundtrip_store_and_ack`, `roundtrip_find_node_and_resp`, `roundtrip_find_value_and_resp`
  - `test_port_manager_fallback_on_in_use`
  - `test_auto_bootstrap_between_nodes`
  - `test_peer_eviction_on_failure`
  - `test_node_health_state_transitions`
  - Multi-hop iterative DHT lookups and churn recovery.
- **Messenger (`nodex-messenger`)**:
  - `test_identity_creation_and_fingerprint`
  - `test_mnemonic_manager_generate_validate`
  - `test_trust_manager_fingerprint_verification`
  - `test_direct_udp_e2ee_message_delivery`
  - `test_envelope_replay_and_tamper_protection`
  - `test_delivery_receipt_creation`
  - `test_messenger_offline_store_and_forward_e2ee`
