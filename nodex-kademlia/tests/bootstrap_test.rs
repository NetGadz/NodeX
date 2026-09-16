use nodex_kademlia::config::NodeConfig;
use nodex_kademlia::KademliaNode;

#[tokio::test]
async fn test_auto_bootstrap_between_nodes() {
    let mut cfg1 = NodeConfig::default();
    cfg1.port = 19850;
    let node1 = KademliaNode::start_with_config(cfg1).await.expect("Node 1 start");

    let mut cfg2 = NodeConfig::default();
    cfg2.port = 19851;
    cfg2.bootstrap = vec![node1.network.local_addr];
    let node2 = KademliaNode::start_with_config(cfg2).await.expect("Node 2 start");

    // Bootstrap node2 to node1
    node2.bootstrap(node1.network.local_addr).await.expect("Bootstrap success");

    let count1 = node1.routing_table.read().await.total_contacts();
    let count2 = node2.routing_table.read().await.total_contacts();

    assert!(count1 >= 1, "Node 1 should know about Node 2");
    assert!(count2 >= 1, "Node 2 should know about Node 1");
}
