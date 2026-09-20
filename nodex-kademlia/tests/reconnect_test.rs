use nodex_kademlia::config::NodeConfig;
use nodex_kademlia::health::NodeHealthState;
use nodex_kademlia::KademliaNode;

#[tokio::test]
async fn test_node_health_state_transitions() {
    let cfg = NodeConfig {
        port: 19870,
        ..Default::default()
    };
    let node = KademliaNode::start_with_config(cfg).await.expect("Node start");

    assert_eq!(node.health.get_state(), NodeHealthState::Online);
    node.health.set_state(NodeHealthState::Degraded);
    assert_eq!(node.health.get_state(), NodeHealthState::Degraded);
    assert!(node.health.is_online());

    node.health.set_state(NodeHealthState::Offline);
    assert_eq!(node.health.get_state(), NodeHealthState::Offline);
    assert!(!node.health.is_online());
}
