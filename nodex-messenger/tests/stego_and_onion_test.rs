use image::{DynamicImage, RgbaImage};
use nodex_kademlia::node::NodeId;
use nodex_kademlia::onion::OnionRouter;
use nodex_messenger::mesh::*;
use nodex_messenger::stego::*;
use std::net::SocketAddr;
use x25519_dalek::{PublicKey, StaticSecret};

#[test]
fn test_stego_carrier_embed_and_extract_roundtrip() {
    // Create a 128x128 synthetic test image
    let mut img = RgbaImage::new(128, 128);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([120, 80, 200, 255]);
    }
    let dyn_img = DynamicImage::ImageRgba8(img);

    let contact = StegoContactCard {
        user_id_hex: "3c98f98a72bca120938f712903841a029384bc12".to_string(),
        display_name: "CyberAlice".to_string(),
        bio: "Decentralized P2P node".to_string(),
        ed25519_pub_hex: "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20".to_string(),
        x25519_pub_hex: "a1b2c3d4e5f60718293a4b5c6d7e8f901234567890abcdef1234567890abcdef".to_string(),
        endpoints: vec!["95.172.50.12:49152".parse().unwrap()],
        timestamp: 1726860000,
        signature_hex: "aabbccdd".to_string(),
    };

    // 1. Embed without passphrase
    let encoded_png = StegoCarrier::embed_contact_into_image(&dyn_img, &contact, None).unwrap();
    assert!(!encoded_png.is_empty());

    // Extract without passphrase
    let extracted = StegoCarrier::extract_contact_from_image(&encoded_png, None).unwrap();
    assert_eq!(extracted.user_id_hex, contact.user_id_hex);
    assert_eq!(extracted.display_name, contact.display_name);
    assert_eq!(extracted.endpoints, contact.endpoints);

    // 2. Embed with secret passphrase
    let encoded_with_pw = StegoCarrier::embed_contact_into_image(&dyn_img, &contact, Some("secret_pass123")).unwrap();
    
    // Wrong password fails
    let fail_res = StegoCarrier::extract_contact_from_image(&encoded_with_pw, Some("wrong_pw"));
    assert!(fail_res.is_err());

    // Correct password succeeds
    let ok_res = StegoCarrier::extract_contact_from_image(&encoded_with_pw, Some("secret_pass123")).unwrap();
    assert_eq!(ok_res.display_name, "CyberAlice");
}

#[test]
fn test_onion_2hop_blind_routing_roundtrip() {
    let mut rng = rand::rngs::OsRng;
    let relay_secret = StaticSecret::random_from_rng(&mut rng);
    let relay_pub = PublicKey::from(&relay_secret);
    let relay_addr: SocketAddr = "198.51.100.5:8000".parse().unwrap();

    let dest_secret = StaticSecret::random_from_rng(&mut rng);
    let dest_pub = PublicKey::from(&dest_secret);
    let dest_addr: SocketAddr = "203.0.113.88:9000".parse().unwrap();
    let dest_id = NodeId::generate_random();

    let secret_payload = b"TOP_SECRET_NODE_X_E2EE_MESSAGE";

    // 1. Sender builds 2-hop onion packet
    let onion_packet = OnionRouter::build_2hop_onion(
        &dest_pub,
        &relay_pub,
        relay_addr,
        dest_addr,
        dest_id,
        secret_payload,
    ).unwrap();

    // 2. Relay receives packet: unwraps outer layer
    let relay_header = OnionRouter::process_at_relay(&relay_secret, &onion_packet).unwrap();
    assert_eq!(relay_header.next_hop_addr, dest_addr);
    assert_eq!(relay_header.next_hop_id, dest_id);

    // 3. Final Destination receives inner packet: unwraps secret payload
    let decrypted = OnionRouter::process_at_destination(&dest_secret, &relay_header.inner_packet).unwrap();
    assert_eq!(decrypted, secret_payload);
}

#[test]
fn test_mesh_discovery_table_friend_of_friend() {
    let mut table = MeshDiscoveryTable::new();

    let ann1 = BlindMeshAnnouncement {
        target_id_hex: "bob_node_id_hex".to_string(),
        target_display_name: "Bob".to_string(),
        target_x25519_pub_hex: "aabb".to_string(),
        relay_id_hex: "carol_node_id_hex".to_string(),
        relay_addr: "192.168.1.50:8000".parse().unwrap(),
        hops: 1,
        expires_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600,
    };

    table.insert_announcements(vec![ann1.clone()], 3);

    let route = table.find_best_route("bob_node_id_hex").unwrap();
    assert_eq!(route.target_display_name, "Bob");
    assert_eq!(route.relay_id_hex, "carol_node_id_hex");
    assert_eq!(route.hops, 1);
}
