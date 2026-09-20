use nodex_messenger::identity::UserIdentity;
use nodex_messenger::receipts::DeliveryReceipt;

#[test]
fn test_delivery_receipt_creation() {
    let identity = UserIdentity::generate();
    let receipt = DeliveryReceipt::create(&identity, "msg_12345", 1700000000);
    assert_eq!(receipt.message_id, "msg_12345");
    assert_eq!(receipt.recipient_id, identity.user_id_hex());
    assert_eq!(receipt.signature.len(), 64);
}
