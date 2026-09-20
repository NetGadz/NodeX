# Operating NodeX Bootstrap Seed Nodes

## 1. Role of Bootstrap Nodes
Bootstrap nodes act as stable rendezvous points for new nodes joining the DHT network. They run headlessly in CLI daemon mode.

## 2. Running a Seed Node
```bash
nodex --cli --port 8000 --ip 0.0.0.0 --state-file /var/lib/nodex/seed_state.json
```

## 3. High-Availability Setup
- Run at least 3 seed nodes across different data centers / cloud providers.
- Configure public static IPv4 / IPv6 addresses or stable DNS hostnames (e.g. `seed1.nodex.network:8000`).
- Configure systemd unit file (provided in `deploy/linux/nodex.service`).
