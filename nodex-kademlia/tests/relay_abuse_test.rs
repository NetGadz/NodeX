use nodex_kademlia::relay_client::{
    OptInRelayService, RelayConfig, RelayPacket, RelaySessionRequest, MAX_PACKETS_PER_SEC,
};
use std::net::SocketAddr;

#[tokio::test]
async fn test_opt_in_relay_rate_limiting_and_abuse_protection() {
    let cfg = RelayConfig {
        enabled: true,
        max_sessions: 10,
        daily_traffic_limit_bytes: 50 * 1024 * 1024,
    };
    let relay = OptInRelayService::new(cfg);
    assert!(relay.is_enabled());

    let session_id = [1u8; 16];
    let client_addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();

    let req = RelaySessionRequest {
        session_id,
        sender_user_id: "alice_node".into(),
        recipient_user_id: "bob_node".into(),
        created_at: 1000,
        expires_at: 2000,
        signature_hex: "00".repeat(64),
    };

    // Register session
    let res = relay.handle_session_request(req, client_addr, 1500).await;
    assert_eq!(res, Ok(true));

    // Send packets within rate limit
    let packet = RelayPacket {
        session_id,
        sender_user_id: "alice_node".into(),
        payload: vec![0xEE; 100],
    };

    for _ in 0..MAX_PACKETS_PER_SEC {
        let fwd = relay.forward_packet(packet.clone(), client_addr).await;
        assert!(fwd.is_ok());
    }

    // Next packet in same second exceeds rate limit
    let rate_exceeded = relay.forward_packet(packet.clone(), client_addr).await;
    assert!(rate_exceeded.is_err());
}

#[tokio::test]
async fn test_disabled_relay_rejects_requests() {
    let cfg = RelayConfig {
        enabled: false,
        max_sessions: 10,
        daily_traffic_limit_bytes: 50 * 1024 * 1024,
    };
    let relay = OptInRelayService::new(cfg);
    assert!(!relay.is_enabled());

    let req = RelaySessionRequest {
        session_id: [2u8; 16],
        sender_user_id: "alice_node".into(),
        recipient_user_id: "bob_node".into(),
        created_at: 1000,
        expires_at: 2000,
        signature_hex: "00".repeat(64),
    };

    let client_addr: SocketAddr = "127.0.0.1:9002".parse().unwrap();
    let res = relay.handle_session_request(req, client_addr, 1500).await;
    assert!(res.is_err());
}
