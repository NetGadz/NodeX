# NodeX Message Lifecycle

```
Sender Node                                          Recipient Node
    │                                                      │
    ├─ 1. User clicks Send                                 │
    ├─ 2. Encrypt with X25519/ChaCha20-Poly1305            │
    ├─ 3. Sign envelope with Ed25519                       │
    ├─ 4. Save locally (Status = Pending / "✓")            │
    │                                                      │
    ├─── DIRECT UDP STORE RPC (if peer online) ───────────►│
    │    (Latency < 5ms)                                   │
    │                                                      ├─ 5. Authenticate signature
    │                                                      ├─ 6. Decrypt payload
    │                                                      ├─ 7. Save message to inbox
    │◄── STORE_ACK ────────────────────────────────────────┤
    │                                                      │
    ├─ 8. Update status to Delivered ("✓✓")                │
    │                                                      │
    └─── DHT MAILBOX STORE (Fallback / Offline Relay) ───► DHT Nodes
```

## Status Indicators
- `✓` (Sent): Message has been signed, encrypted, and dispatched to the network.
- `✓✓` (Delivered): Direct UDP RPC ACK confirmed receipt by recipient node.
- `✓✓` (Read): Read receipt envelope received and verified.
