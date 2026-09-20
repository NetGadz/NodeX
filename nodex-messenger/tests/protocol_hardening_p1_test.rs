use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use nodex_kademlia::node::{Contact, NodeId, RoutingTable, UpdateResult};
use nodex_kademlia::rpc::{RpcMessage, RpcPayload};
use nodex_messenger::crypto::*;
use nodex_messenger::db::*;

#[test]
fn test_redacted_debug_does_not_leak_secrets() {
    let mut db = MessengerDb::default();
    db.mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string();
    db.user_seed = [0x5A; 32];
    db.display_name = "Alice".to_string();

    let debug_output = format!("{:?}", db);
    assert!(!debug_output.contains("abandon"), "Mnemonic must not appear in Debug output");
    assert!(!debug_output.contains("5a, 5a"), "User seed must not appear in Debug output");
    assert!(debug_output.contains("[REDACTED]"), "Debug output must contain [REDACTED]");
}

#[test]
fn test_group_messages_isolated_from_direct_contact_queries() {
    let mut db = MessengerDb::default();
    let contact_id = "887fafbc12a0f10522ba4e19a82b7b768ca0e662";
    let my_id = "1111111111111111111111111111111111111111";

    // 1. Direct 1-on-1 message
    db.add_message(SavedChatMessage {
        id: "msg_direct_1".into(),
        sender_id_hex: contact_id.into(),
        recipient_id_hex: my_id.into(),
        text: "Direct message".into(),
        group_id: None,
        ..Default::default()
    });

    // 2. Group message sent by the same contact
    db.add_message(SavedChatMessage {
        id: "msg_group_1".into(),
        sender_id_hex: contact_id.into(),
        recipient_id_hex: my_id.into(),
        text: "Group message".into(),
        group_id: Some("group_alpha_123".into()),
        ..Default::default()
    });

    // Direct query should only return direct messages
    let direct_msgs = db.get_messages_for_contact(contact_id);
    assert_eq!(direct_msgs.len(), 1);
    assert_eq!(direct_msgs[0].id, "msg_direct_1");

    // Clearing direct chat should not wipe group messages
    db.clear_messages_for_contact(contact_id);
    assert_eq!(db.get_messages_for_contact(contact_id).len(), 0);
    assert!(db.messages.iter().any(|m| m.id == "msg_group_1"), "Group messages must be preserved after clear_messages_for_contact");
}

#[test]
fn test_routing_table_rejects_bogon_ips() {
    let local_id = NodeId::generate_random();
    let mut rt = RoutingTable::new(local_id);

    // 1. Unspecified IP 0.0.0.0
    let c_unspecified = Contact::new(NodeId::generate_random(), SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8444));
    assert_eq!(rt.update(c_unspecified), UpdateResult::SelfIgnored);

    // 2. Port 0
    let c_port0 = Contact::new(NodeId::generate_random(), SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0));
    assert_eq!(rt.update(c_port0), UpdateResult::SelfIgnored);

    // 3. Multicast IP 224.0.0.1
    let c_multicast = Contact::new(NodeId::generate_random(), SocketAddr::new(IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1)), 8444));
    assert_eq!(rt.update(c_multicast), UpdateResult::SelfIgnored);

    // 4. Valid routable contact
    let c_valid = Contact::new(NodeId::generate_random(), SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)), 8444));
    assert!(matches!(rt.update(c_valid), UpdateResult::Inserted));
}

#[test]
fn test_safe_wire_codec_no_unsafe_or_union() {
    let sender = NodeId::generate_random();
    let target = NodeId::generate_random();

    // Test FindNode roundtrip
    let msg = RpcMessage::new(999, sender, RpcPayload::FindNode { target_id: target });
    let encoded = msg.encode().unwrap();
    let decoded = RpcMessage::decode(&encoded).unwrap();
    assert_eq!(msg, decoded);

    // Test Store roundtrip with arbitrary binary payload
    let store_msg = RpcMessage::new(1000, sender, RpcPayload::Store {
        key: target,
        value: vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03],
    });
    let enc_store = store_msg.encode().unwrap();
    let dec_store = RpcMessage::decode(&enc_store).unwrap();
    assert_eq!(store_msg, dec_store);
}

#[test]
fn test_midnight_3_window_mailbox_keys() {
    let user_id_hex = "887fafbc12a0f10522ba4e19a82b7b768ca0e662";
    let now = 1700000000; // arbitrary timestamp

    let key_prev = derive_secure_mailbox_key(user_id_hex, now - 86400);
    let key_curr = derive_secure_mailbox_key(user_id_hex, now);
    let key_next = derive_secure_mailbox_key(user_id_hex, now + 86400);

    assert_ne!(key_prev, key_curr);
    assert_ne!(key_curr, key_next);
    assert_ne!(key_prev, key_next);

    assert!(key_prev.starts_with("mbx_"));
    assert!(key_curr.starts_with("mbx_"));
    assert!(key_next.starts_with("mbx_"));
}
