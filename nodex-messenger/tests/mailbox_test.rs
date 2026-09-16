use nodex_messenger::mailbox::MailboxManager;

#[test]
fn test_mailbox_key_formatting() {
    let user_id = "0123456789abcdef0123456789abcdef01234567";
    let key = MailboxManager::format_key(user_id, 1700000000);
    assert!(!key.is_empty());
    assert!(key.contains("mailbox"));
}
