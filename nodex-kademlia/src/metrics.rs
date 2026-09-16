use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct MetricsTracker {
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    rpcs_sent: AtomicU64,
    rpcs_received: AtomicU64,
    rpcs_failed: AtomicU64,

    // Index 1..=8 for message types (MSG_PING to MSG_FIND_VALUE_RESP)
    rpcs_sent_by_type: [AtomicU64; 9],
    rpcs_recv_by_type: [AtomicU64; 9],

    lookups_total: AtomicU64,
    lookup_hops_total: AtomicU64,
    lookup_latency_ms_total: AtomicU64,
}

impl MetricsTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn inc_bytes_sent(&self, bytes: usize) {
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn inc_bytes_received(&self, bytes: usize) {
        self.bytes_received
            .fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn inc_rpc_sent(&self, msg_type: u8) {
        self.rpcs_sent.fetch_add(1, Ordering::Relaxed);
        if (msg_type as usize) < 9 {
            self.rpcs_sent_by_type[msg_type as usize].fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_rpc_received(&self, msg_type: u8) {
        self.rpcs_received.fetch_add(1, Ordering::Relaxed);
        if (msg_type as usize) < 9 {
            self.rpcs_recv_by_type[msg_type as usize].fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_rpc_failed(&self) {
        self.rpcs_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_lookup(&self, hops: usize, latency_ms: u64) {
        self.lookups_total.fetch_add(1, Ordering::Relaxed);
        self.lookup_hops_total
            .fetch_add(hops as u64, Ordering::Relaxed);
        self.lookup_latency_ms_total
            .fetch_add(latency_ms, Ordering::Relaxed);
    }

    pub fn format_report(&self) -> String {
        let sent_bytes = self.bytes_sent.load(Ordering::Relaxed);
        let recv_bytes = self.bytes_received.load(Ordering::Relaxed);
        let sent_rpcs = self.rpcs_sent.load(Ordering::Relaxed);
        let recv_rpcs = self.rpcs_received.load(Ordering::Relaxed);
        let failed_rpcs = self.rpcs_failed.load(Ordering::Relaxed);

        let lookups = self.lookups_total.load(Ordering::Relaxed);
        let hops = self.lookup_hops_total.load(Ordering::Relaxed);
        let latency_ms = self.lookup_latency_ms_total.load(Ordering::Relaxed);

        let avg_hops = if lookups > 0 {
            hops as f64 / lookups as f64
        } else {
            0.0
        };
        let avg_latency = if lookups > 0 {
            latency_ms as f64 / lookups as f64
        } else {
            0.0
        };

        format!(
            "\n📊 === Kademlia DHT Node Metrics ===\n\
             Traffic:       {sent_bytes} bytes sent | {recv_bytes} bytes received\n\
             RPCs Total:    {sent_rpcs} sent | {recv_rpcs} received | {failed_rpcs} failed\n\
             Lookups:       {lookups} total | {avg_hops:.2} avg hops | {avg_latency:.1} ms avg latency\n\
             ====================================\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_increment_and_report() {
        let metrics = MetricsTracker::new();
        metrics.inc_bytes_sent(500);
        metrics.inc_bytes_received(1200);
        metrics.inc_rpc_sent(1);
        metrics.inc_rpc_received(2);
        metrics.inc_rpc_failed();
        metrics.record_lookup(2, 45);

        let report = metrics.format_report();
        assert!(report.contains("500 bytes sent"));
        assert!(report.contains("1200 bytes received"));
        assert!(report.contains("1 total"));
    }
}
