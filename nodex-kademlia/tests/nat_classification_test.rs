use nodex_kademlia::nat_type::NatCategory;

#[test]
fn test_nat_categories_and_decision_logic() {
    let direct = NatCategory::DirectPossible;
    assert!(direct.can_accept_direct());
    assert!(direct.should_hole_punch());

    let cone = NatCategory::HolePunchLikely;
    assert!(!cone.can_accept_direct());
    assert!(cone.should_hole_punch());

    let complex = NatCategory::RelayRecommended;
    assert!(!complex.can_accept_direct());
    assert!(!complex.should_hole_punch());

    let blocked = NatCategory::UdpBlocked;
    assert!(!blocked.can_accept_direct());
    assert!(!blocked.should_hole_punch());

    let unknown = NatCategory::Unknown;
    assert_eq!(unknown.description(), "Assessing network...");
}
