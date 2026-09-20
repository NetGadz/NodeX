use nodex_kademlia::config::NodeConfig;
use nodex_kademlia::KademliaNode;

#[tokio::test]
async fn test_auto_bootstrap_between_nodes() {
    let cfg1 = NodeConfig {
        port: 19850,
        ..Default::default()
    };
    let node1 = KademliaNode::start_with_config(cfg1).await.expect("Node 1 start");

    let cfg2 = NodeConfig {
        port: 19851,
        bootstrap: vec![node1.network.local_addr],
        ..Default::default()
    };
    let node2 = KademliaNode::start_with_config(cfg2).await.expect("Node 2 start");

    // Bootstrap node2 to node1
    node2.bootstrap(node1.network.local_addr).await.expect("Bootstrap success");

    let count1 = node1.routing_table.read().await.total_contacts();
    let count2 = node2.routing_table.read().await.total_contacts();

    assert!(count1 >= 1, "Node 1 should know about Node 2");
    assert!(count2 >= 1, "Node 2 should know about Node 1");
}
