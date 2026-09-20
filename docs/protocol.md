# NodeX Network and RPC Protocol

## 1. Transport Layer
NodeX operates directly over UDP. Messages are framed as compact binary packets or JSON envelopes for maximum compatibility and resilience across network firewalls.

## 2. Kademlia RPC Messages
All Kademlia DHT RPCs include a 16-byte random `RpcId` for request-response correlation:
- **`PING` / `PONG`**: Liveness probe.
- **`STORE` / `STORE_ACK`**: Stores a key-value pair with a 24-hour TTL.
- **`FIND_NODE` / `FIND_NODE_RESP`**: Queries the $k$ closest contacts to a 160-bit target ID.
- **`FIND_VALUE` / `FIND_VALUE_RESP`**: Queries the value associated with a key, or returns the $k$ closest nodes if the value is unknown.

## 3. Direct Message Delivery RPC
When peer $A$ sends a message to online peer $B$:
1. $A$ resolves $B$'s socket address from DHT presence or cached contact card.
2. $A$ directly executes a `STORE` RPC with key `mailbox:<recipient_pubkey>` to $B$'s socket.
3. $B$'s Kademlia transport layer receives the payload, verifies envelope authenticity, and delivers the message directly to $B$'s inbox within <5ms.
4. $B$ responds with `STORE_ACK`, causing $A$'s UI to immediately display `✓✓` (Delivered).
