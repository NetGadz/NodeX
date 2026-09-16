use nodex_kademlia::port_manager::PortManager;

#[tokio::test]
async fn test_port_manager_fallback_on_in_use() {
    let base_port = 19800;
    // Bind first socket
    let (_sock1, addr1) = PortManager::bind_auto("127.0.0.1", base_port, 10)
        .await
        .expect("Should bind first port");
    assert_eq!(addr1.port(), base_port);

    // Attempt to bind on same preferred port -> should fallback to base_port + 1
    let (_sock2, addr2) = PortManager::bind_auto("127.0.0.1", base_port, 10)
        .await
        .expect("Should bind fallback port");
    assert_eq!(addr2.port(), base_port + 1);
}
