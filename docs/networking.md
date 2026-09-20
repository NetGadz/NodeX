# NodeX Networking & Port Management

## 1. Zero-Configuration UDP Auto-Binding
- In consumer environments, static port binding often conflicts with running applications.
- NodeX's `PortManager` attempts to bind to default port `8000`.
- If the socket returns `WSAEADDRINUSE` or `EADDRINUSE`, it automatically scans ports `8001..8050` until an open port is bound.
- The user is never blocked by socket conflicts.

## 2. Automatic Local & Remote Bootstrap
- When running locally, nodes automatically discover and link to peer ports `8000..8010`.
- In WAN deployments, nodes query bootstrap addresses defined in `config.json` with exponential backoff retries.

## 3. Reconnection & Health Supervision
- The `HealthTracker` continuously tracks network states (`Starting`, `Binding`, `Connecting`, `SearchingPeers`, `Online`, `Degraded`, `Offline`).
- Dead nodes are evicted from active routing buckets, and periodic bucket refreshes maintain routing table freshness.
