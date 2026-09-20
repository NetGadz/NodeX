# NodeX Production Specification: WAN, Performance, and Media

## 1. Purpose and scope

NodeX must support at least 100 concurrently active users distributed across different cities and networks. Users must be able to exchange text and photos without a central message server. The application must remain responsive while networking, encryption, disk I/O, database maintenance, and image processing are in progress.

The phrase "never lags" is defined here as measurable behavior:

- no blocking network, filesystem, cryptographic, or image work on the GUI thread;
- no unbounded queues, allocations, retries, or mailbox growth;
- every long operation has progress, timeout, cancellation, retry, and failure feedback;
- degraded network conditions must reduce delivery speed, not freeze or crash the application.

This specification covers the desktop client, the peer-to-peer transport, relay and mailbox behavior, media transfer, observability, packaging, and release acceptance.

## 2. Non-goals and constraints

- There is no central message server or central plaintext message store.
- Relays and DHT nodes must see only routing metadata and encrypted envelopes.
- A STUN result is not proof of inbound reachability.
- A successful UDP send is not proof of delivery.
- No requirement may promise zero latency under an unavailable peer, broken ISP, full disk, or power loss. The required behavior in those cases is bounded waiting followed by an explicit state.

## 3. Current baseline

### 3.1 Present capabilities

- UDP Kademlia transport and DHT mailbox fallback.
- STUN reflexive endpoint discovery.
- Direct UDP delivery when a known endpoint is reachable.
- Relay service primitives with session and traffic limits.
- E2EE envelopes with authenticated encryption.
- Signed presence identity, endpoints, and public keys.
- Encrypted local database and encrypted password backup.
- GUI image texture caching.

### 3.2 Release blockers

The following are not considered production-complete until implemented and tested end-to-end:

- real UPnP IGD SOAP `AddPortMapping` and NAT-PMP mapping, with confirmation and cleanup;
- live integration of bidirectional authenticated hole punching into peer negotiation;
- a transport manager that executes the direct, mapped, hole-punch, relay, and mailbox cascade;
- resumable chunked media transfer; the current single-RPC payload limit is not a large-photo protocol;
- media delivery acknowledgements, retry state, and durable transfer recovery;
- bounded GUI worker queues and cancellation for all expensive operations;
- structured metrics, crash reporting, and operational diagnostics;
- automated Windows packaging, clean install, upgrade, uninstall, and data migration tests.

## 4. Production targets for 100 users

The capacity target is 100 registered users, 100 simultaneously connected users, at least 50 concurrent conversations, and 20 concurrent media transfers. The system must tolerate 2x these values in load tests without correctness failures.

### 4.1 Service-level objectives

- GUI input-to-render p95: <= 50 ms during normal operation.
- GUI frame time p99: <= 32 ms; no frame may exceed 250 ms in the standard workload.
- Chat open p95: <= 200 ms for locally stored messages.
- Text send acknowledgement p95: <= 2 s when the peer is reachable.
- Direct transport failover decision: <= 5 s per candidate path.
- Offline mailbox enqueue acknowledgement: <= 3 s when at least one DHT route is available.
- Photo manifest acknowledgement p95: <= 3 s when the recipient is reachable.
- Media progress update interval: 250-1000 ms, coalesced and bounded.
- No retry loop may run without a deadline; total automatic retry window is <= 60 s before entering a visible failed state.
- Memory ceiling for the GUI process in the standard workload: <= 512 MB, excluding the operating-system file cache.
- A 1000-message conversation with cached thumbnails must remain scrollable without blocking the GUI thread.

### 4.2 Capacity limits

All limits must be explicit configuration values and must have visible failure behavior:

- max active media transfers per user: 3 uploads and 3 downloads;
- max queued outgoing operations: 100;
- max queued bytes per peer: 64 MB;
- max decoded image dimension: 4096 x 4096;
- max cached thumbnail memory: 128 MB with LRU eviction;
- max incomplete media age: 24 hours;
- max mailbox envelope age: 24 hours;
- max relay sessions per node: 10 by default, configurable with hard upper bound;
- max relay packet size: below the serialized RPC transport limit;
- max log rate per subsystem: bounded and sampled.

## 5. Transport architecture

The connection manager must own one state machine per peer. It must expose the current transport, attempt number, deadline, RTT, retry count, and failure reason to the GUI and diagnostics.

The required cascade is:

1. authenticated direct endpoint;
2. confirmed UPnP IGD or NAT-PMP endpoint;
3. STUN-discovered endpoint, subject to reachability validation;
4. coordinated bidirectional UDP hole punching;
5. opt-in encrypted relay;
6. DHT mailbox for offline delivery.

A failed path must not block the GUI or prevent the next path from starting. Transport attempts must be cancellable when a conversation is closed or the application exits.

### 5.1 Peer authentication

- Presence cards are signed over every identity, key, endpoint, NAT, display, and timestamp field.
- The user ID must be derived from and match the Ed25519 public key.
- X25519 recipient keys must be verified before encryption; no key may be synthesized from a node ID.
- Endpoint changes must trigger key and trust validation before use.
- All transport control packets must bind peer ID, session ID, nonce, and expiration.

### 5.2 STUN

- Query at least three configured STUN servers with transaction ID, source, magic-cookie, and response validation.
- Record the reflexive endpoint and expiry time.
- Never advertise a STUN endpoint as reachable until a peer-side probe succeeds.
- Refresh in the background with jitter and a bounded timeout.

### 5.3 UPnP IGD and NAT-PMP

- Discover and validate a real gateway response.
- Parse the IGD control URL and invoke SOAP `AddPortMapping` with a bounded lease, protocol, internal port, and description.
- Implement NAT-PMP mapping and response validation where supported.
- Confirm the external port before advertising it.
- Report `Unsupported`, `Denied`, `TimedOut`, `Discovered`, `Mapped`, and `Removed` as separate states.
- Never report `enabled` or `mapped_port` based only on an SSDP probe.
- Remove mappings on clean shutdown and expire them automatically on crash.

### 5.4 Hole punching

- Exchange signed endpoint candidates through the authenticated presence path.
- Generate a fresh 128-bit nonce and session ID for each attempt.
- Both peers send probes during a bounded synchronized window.
- A peer responds only to a valid probe and returns a nonce-bound acknowledgement.
- Reject stale, replayed, malformed, or wrong-peer acknowledgements.
- Confirm the selected path with an authenticated ping/pong before sending application data.
- Record RTT, endpoint, NAT category, and failure cause.

### 5.5 Relay and mailbox

- Relay nodes forward encrypted payloads only and enforce session, packet-rate, byte-rate, and lifetime limits.
- Mailbox records have a maximum TTL of 24 hours and are encrypted for the recipient.
- Mailbox polling uses exponential backoff with jitter and resets on new data.
- Consumption must acknowledge individual envelope IDs and must not delete unrelated envelopes.
- Duplicate envelopes are deduplicated by message ID.
- Relay and mailbox failures must leave the message in a durable local outbox.

## 6. Reliable text protocol

Every application message contains:

- protocol version;
- stable globally unique message ID;
- sender and recipient IDs;
- creation timestamp and expiration policy;
- content type;
- payload or media manifest reference;
- authenticated encryption and replay protection.

The local outbox stores `Queued`, `Sending`, `Sent`, `Delivered`, `Read`, `Retrying`, `Failed`, and `Cancelled` states. A successful UDP write may transition only to `Sent`; `Delivered` requires a recipient acknowledgement.

The receiver must process a message ID at most once, persist the result before acknowledging it, and safely handle reordered, duplicated, delayed, or expired envelopes.

## 7. Reliable photo protocol

The existing single-message RPC limit must not be used for large images. A photo is a resumable transfer with a small chat message referencing a media object.

### 7.1 Sender flow

1. Read the file on a worker thread.
2. Validate MIME type, dimensions, and size before encoding.
3. Resize or compress only on a worker thread, preserving the original only when the user requests it.
4. Generate a media ID and SHA-256 of the final bytes.
5. Send an encrypted manifest containing media ID, MIME type, byte length, hash, dimensions, chunk size, and expiry.
6. Split the bytes into chunks below the transport maximum.
7. Encrypt and authenticate each chunk with media ID, index, total count, and recipient binding as associated data.
8. Send chunks through the currently selected transport.
9. Retry only missing chunks using a bounded exponential backoff.
10. Mark the transfer complete only after the receiver confirms the final hash.

### 7.2 Receiver flow

1. Validate and persist the manifest before accepting chunks.
2. Store chunks in a temporary transfer directory, never in the visible media store.
3. Authenticate and range-check every chunk.
4. Return a received-range bitmap or compact missing-range list.
5. Reassemble only after all chunks are present.
6. Verify the final SHA-256 and dimensions.
7. Atomically rename the verified file into the media store.
8. Emit one UI event containing the media path/hash; do not send raw megabytes through the GUI event channel.
9. Delete incomplete transfers after 24 hours or cancellation.

A photo must never silently disappear because it exceeds an RPC limit. The sender must receive a visible progress state, a successful completion, or a specific failure with retry/cancel actions. The receiver must never display a partially assembled or hash-invalid image.

## 8. GUI responsiveness and resource safety

- The egui thread must perform only layout, input handling, and presentation.
- Network I/O, filesystem I/O, KDF, encryption of large data, image decoding, resizing, and thumbnail generation run in bounded worker pools.
- Worker queues are bounded; queue overflow produces a visible status instead of unbounded allocation.
- Every task has a deadline and cancellation token.
- UI events are coalesced; progress events are emitted at most four times per second per transfer.
- Message lists use pagination or virtualization for large histories.
- Images are decoded once, cached by content hash, dimension-capped, and evicted with LRU policy.
- Failed sends immediately remove the pending spinner and enter `Failed` with a retry action.
- Shutdown waits for cancellation and closes sockets, files, and worker channels cleanly.

## 9. Data protection and recovery

- The database remains encrypted with a per-installation master key.
- Instance key files use restrictive ACLs and are never included in backups.
- Backups use Argon2id with documented parameters and authenticated encryption; old backup formats remain importable during the migration window.
- Media files are encrypted or stored only as encrypted application payloads where required by the threat model.
- Wipe removes the database, instance key, media store, temporary transfers, outbox, caches, and in-memory routing state.
- Loss of the instance key is documented as intentional, unrecoverable local-data loss.

## 10. Observability and supportability

Use structured, privacy-preserving logs and metrics. Never log message text, plaintext media, passwords, seeds, or private keys.

Required metrics:

- GUI frame time p50/p95/p99 and frames above 250 ms;
- worker queue depth and rejected tasks;
- active transfers, bytes, throughput, RTT, retries, and failures;
- transport selection and failover counts;
- STUN, UPnP, NAT-PMP, hole-punch, relay, and mailbox outcomes;
- database read/write latency and errors;
- memory usage, thumbnail-cache size, and temporary-transfer size;
- dropped/coalesced UI events and crash/forced-exit count.

Provide a diagnostic export containing versions, configuration with secrets redacted, transport states, recent error codes, and metric summaries. It must be safe to attach to a bug report.

## 11. Security requirements

- Run `cargo check --workspace`, strict Clippy, all tests, and dependency audit in CI.
- Reject unsigned or malformed presence cards and tombstones.
- Reject missing or unverified recipient X25519 keys.
- Apply replay windows and expiration to messages, control packets, manifests, and chunks.
- Enforce maximum lengths before allocation or deserialization.
- Rate-limit mailbox, relay, discovery, and media operations per peer.
- Fuzz parsers for RPC, presence, manifests, chunks, and backup headers.
- Run dependency updates and RustSec review before each release.

## 12. Test and acceptance matrix

### 12.1 WAN acceptance

- Two nodes on different home networks establish direct, mapped, hole-punched, relay, or mailbox delivery.
- At least one failed transport falls through to the next path within its deadline.
- NAT types include public, full-cone, restricted-cone, port-restricted, and symmetric NAT where test infrastructure permits.
- Endpoint/key tampering, stale probes, replayed acknowledgements, and invalid signatures are rejected.
- 100 concurrent users and 50 simultaneous conversations meet the stated SLOs.

### 12.2 Media acceptance

- 100 KB, 1 MB, 5 MB, and 20 MB images arrive byte-for-byte verified or produce an explicit failure.
- Transfers survive disconnect and resume from missing chunks.
- Delivery works over direct UDP, relay, and mailbox fallback.
- Duplicate chunks and manifests do not create duplicate chat messages.
- Corrupted chunks and final hashes are rejected and never displayed.
- Sending media does not freeze typing, scrolling, chat switching, or window resizing.

### 12.3 Performance acceptance

- A 1000-message conversation remains responsive with visible thumbnails.
- KDF, image processing, and database operations do not block the GUI thread.
- Memory remains below the production ceiling during a 20 MB transfer and thumbnail-cache churn.
- Queue saturation, disk-full, timeout, cancellation, and process restart have visible and recoverable outcomes.
- Shutdown completes without leaked sockets, orphaned temporary files, or test-process crashes.

## 13. Release and operations

Before public release:

1. CI passes build, lint, unit, integration, security, fuzz-smoke, and packaging jobs.
2. A clean Windows install, upgrade, rollback, uninstall, and reinstall are tested.
3. Configuration defaults are reviewed for safe relay, logging, ports, and storage behavior.
4. A staged pilot with at least 10 users runs before the 100-user release.
5. The release has a versioned protocol, migration plan, rollback plan, and signed artifacts.
6. A support runbook covers NAT failures, mailbox backlog, corrupted media, key loss, disk-full, and upgrade recovery.

## 14. Definition of Done

NodeX is ready for general use by 100 users only when every release blocker is implemented, every SLO has an automated or repeatable test, WAN and media acceptance pass on real heterogeneous networks, no GUI operation blocks the event thread, and failures are visible, bounded, recoverable, and diagnosable.
# NodeX: WAN, Performance, and Media Delivery Specification

## 1. Goal

Two users in different cities or countries must be able to exchange text and photos without a central message server, while the UI remains responsive during network, encryption, disk, and image operations.

"Never lags" is translated into measurable requirements: no blocking work on the egui thread, bounded memory, bounded queues, cancellation, and visible progress or failure for every long operation.

## 2. Current implementation status

Implemented today:

- UDP Kademlia transport and DHT mailbox fallback.
- STUN endpoint discovery.
- Direct UDP delivery when a known peer endpoint is reachable.
- Relay service primitives with rate and session limits.
- Encrypted E2EE envelopes.
- Presence cards with signed identity, endpoint, and key fields.
- Encrypted local database and encrypted backups.
- Evidence-based UPnP IGD discovery status.
- Verified hole-punch probe/ack API.
- GUI texture caching for received images.

Not yet sufficient for the final WAN acceptance criteria:

- UPnP IGD SOAP `AddPortMapping` and real NAT-PMP mapping are not complete.
- Hole-punch probe/ack is not wired into the live peer negotiation path.
- Large media is not chunked and resumable; the current single-RPC payload has a bounded size.
- End-to-end delivery acknowledgements for media are not yet a complete protocol.

## 3. WAN connection cascade

The connection manager must attempt transports in this order:

1. Existing authenticated direct endpoint.
2. Public endpoint discovered by STUN.
3. Confirmed UPnP IGD or NAT-PMP mapped endpoint.
4. Coordinated bidirectional UDP hole punching with a nonce and acknowledgement.
5. Opt-in encrypted relay.
6. DHT mailbox for offline delivery.

Every candidate must have a state and timeout. A failed path must not block the UI or prevent the next path from starting.

### 3.1 STUN

- Query at least three configured STUN endpoints.
- Validate transaction ID, source endpoint, magic cookie, and response type.
- Record reflexive address lifetime and refresh it periodically.
- Never treat a STUN result as proof that inbound traffic is reachable.

### 3.2 UPnP and NAT-PMP

- Discover a real gateway response.
- Parse the gateway control URL or NAT-PMP response.
- Request a mapping with a bounded lease and the NodeX protocol/port.
- Confirm the mapping by querying or validating the gateway response.
- Set `mapped_port` only after confirmation.
- Report unsupported, denied, timeout, and confirmed states separately.
- Remove the mapping on clean shutdown when possible.

### 3.3 UDP hole punching

- Exchange signed endpoint candidates through the authenticated presence path.
- Generate a fresh 128-bit nonce per attempt.
- Send probes from both peers during a bounded window.
- Accept a connection only after a matching authenticated acknowledgement.
- Reject stale, replayed, malformed, or wrong-peer acknowledgements.
- Record RTT, selected endpoint, and failure reason.

### 3.4 Relay and mailbox

- Relay forwards only encrypted envelopes and enforces quotas.
- Mailbox records have a maximum TTL of 24 hours.
- Mailbox polling uses backoff and jitter.
- Successful consumption is acknowledged and removes only the consumed envelope.
- Offline delivery must preserve message IDs and deduplicate envelopes.

## 4. Text and photo protocol

### 4.1 Message envelope

Every message has:

- protocol version;
- stable message ID;
- sender and recipient IDs;
- creation timestamp;
- content type;
- payload or media manifest;
- authenticated encryption and replay protection.

### 4.2 Photo transfer

The current single-message RPC limit is not a suitable large-photo protocol. Implement media as a resumable transfer:

1. Send a small encrypted media manifest containing media ID, MIME type, byte length, SHA-256, dimensions, and chunk size.
2. Split the compressed photo into fixed-size encrypted chunks below the transport maximum.
3. Give every chunk a media ID, index, total count, byte range, and authentication tag.
4. Receiver acknowledges each chunk or a compact received-range bitmap.
5. Sender retries only missing chunks with exponential backoff and a hard retry limit.
6. Receiver verifies every chunk and the final SHA-256 before exposing the photo in the chat.
7. Store incomplete transfers separately and expire them after 24 hours.
8. After verification, atomically move the assembled file into the media store and emit one UI event.
9. Support cancellation, resume after reconnect, and relay/mailbox transport for chunks.

A photo must never silently disappear because it exceeds an RPC limit. The sender must receive a visible error, progress state, retry action, or successful completion.

## 5. Performance and no-freeze requirements

- No network I/O, filesystem I/O, password KDF, image decode, image resize, or encryption of large media on the egui thread.
- GUI event handling must complete within 8 ms in normal operation and never wait on a mutex held by network or disk work.
- Use bounded worker queues and reject new work with a visible status when full.
- Coalesce periodic status updates; do not redraw for every transport packet.
- Decode each image once and cache the texture by content hash.
- Cap decoded image dimensions and memory before creating a texture.
- Limit concurrent media transfers and use cancellation tokens when a chat is closed.
- Use structured tracing with operation ID, peer ID, transport, bytes, retries, and duration.
- Add a watchdog metric for UI frame time, queue depth, pending transfers, and dropped events.

## 6. Acceptance tests

### WAN

- Two nodes on different networks establish direct, hole-punched, relay, or mailbox delivery.
- At least one path failure falls through to the next path within the configured timeout.
- Presence and endpoint tampering is rejected.
- Stale and replayed hole-punch acknowledgements are rejected.

### Photos

- 100 KB, 1 MB, 5 MB, and 20 MB photos either arrive byte-for-byte verified or show a clear failure.
- Transfer survives temporary disconnect and resumes from missing chunks.
- Receiver never shows a partially assembled or corrupted image.
- Photo delivery works over direct UDP, relay, and mailbox fallback.
- Duplicate chunks and duplicate manifests do not create duplicate chat messages.

### Performance

- Sending a photo does not freeze typing, scrolling, or chat switching.
- A 1000-message chat remains responsive while thumbnails are visible.
- UI frame time, queue depth, memory, and transfer progress are recorded in a diagnostic mode.
- Shutdown cancels workers cleanly without test-process crashes or leaked sockets.
