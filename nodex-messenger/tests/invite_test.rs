use nodex_messenger::identity::UserIdentity;
use nodex_messenger::invite::InviteManager;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_create_and_parse_valid_invite() {
    let identity = UserIdentity::generate();
    let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
    let endpoints = vec![addr];

    let invite_link = InviteManager::create_invite(&identity, "Alice", endpoints, Some(86400)).unwrap();
    assert!(invite_link.starts_with("nodex://invite/"));

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let parsed = InviteManager::parse_and_verify_invite(&invite_link, now).unwrap();

    assert_eq!(parsed.user_id, identity.user_id_hex());
    assert_eq!(parsed.display_name, "Alice");
    assert_eq!(parsed.endpoints, vec![addr]);
}

#[test]
fn test_expired_invite_rejected() {
    let identity = UserIdentity::generate();
    let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();

    let invite_link = InviteManager::create_invite(&identity, "Alice", vec![addr], Some(10)).unwrap();

    let future = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 100;
    let err = InviteManager::parse_and_verify_invite(&invite_link, future);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("expired"));
}
