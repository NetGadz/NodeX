# NodeX Troubleshooting Guide

## 1. Network Issues
- **Problem**: 0 Peers visible in status bar.
  - **Solution**: Ensure your firewall permits outbound/inbound UDP on ports `8000..8050`. Check bootstrap node connectivity in `config.json`.

- **Problem**: Port Conflict on Startup.
  - **Solution**: NodeX automatically shifts to the next open port (up to 8050). If all ports are occupied, specify `--port <PORT>` manually.

## 2. Message Delivery Issues
- **Problem**: Message stays pending (`✓`).
  - **Solution**: Peer is currently offline or unreachable behind strict symmetric NAT. The message is stored in the DHT mailbox relay and will be retrieved as soon as the recipient comes online.
