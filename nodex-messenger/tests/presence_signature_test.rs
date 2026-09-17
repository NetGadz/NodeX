use nodex_kademlia::nat_type::NatCategory;
use nodex_messenger::identity::UserIdentity;
use nodex_messenger::presence::UserPresenceCard;
use std::net::SocketAddr;

#[test]
fn test_presence_card_creation_and_signature_verification() {
    let identity = UserIdentity::generate();
    let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
    let endpoints = vec![addr, "192.168.1.100:8000".parse().unwrap()];

    let card = UserPresenceCard::create_with_endpoints(
        &identity,
        "Alice".into(),
        "Hello P2P".into(),
        addr,
        endpoints,
        NatCategory::DirectPossible,
        1700000000,
    );

    assert!(card.verify_signature().is_ok());
}

#[test]
fn test_tampered_presence_card_fails_verification() {
    let identity = UserIdentity::generate();
    let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();

    let mut card = UserPresenceCard::create_with_endpoints(
        &identity,
        "Alice".into(),
        "Hello P2P".into(),
        addr,
        vec![addr],
        NatCategory::DirectPossible,
        1700000000,
    );

    // Tamper with display name or address
    card.socket_addr = "127.0.0.1:9999".parse().unwrap();
    assert!(card.verify_signature().is_err());
}
