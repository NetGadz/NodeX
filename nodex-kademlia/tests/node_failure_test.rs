use nodex_kademlia::node::{Contact, NodeId, RoutingTable};
use std::net::SocketAddr;

#[tokio::test]
async fn test_peer_eviction_on_failure() {
    let local_id = NodeId::generate_random();
    let mut rt = RoutingTable::new(local_id);

    let dummy_id = NodeId::generate_random();
    let dummy_addr: SocketAddr = "127.0.0.1:19899".parse().unwrap();
    let contact = Contact::new(dummy_id, dummy_addr);

    rt.update(contact);
    assert_eq!(rt.total_contacts(), 1);

    rt.remove(&dummy_id);
    assert_eq!(rt.total_contacts(), 0);
}
