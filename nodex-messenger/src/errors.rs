use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessengerError {
    IdentityNotFound,
    InvalidKeyLength(String),
    CryptoError(String),
    NetworkError(String),
    PeerNotFound(String),
    MailboxError(String),
    DatabaseError(String),
    MnemonicError(String),
    ContactBlocked(String),
    AttachmentTooLarge(usize, usize),
    ReplayDetected(String),
    Other(String),
}

impl fmt::Display for MessengerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessengerError::IdentityNotFound => write!(f, "Identity not found or not initialized"),
            MessengerError::InvalidKeyLength(k) => write!(f, "Invalid cryptographic key length: {}", k),
            MessengerError::CryptoError(e) => write!(f, "E2EE Cryptographic error: {}", e),
            MessengerError::NetworkError(e) => write!(f, "Network/DHT transport error: {}", e),
            MessengerError::PeerNotFound(p) => write!(f, "Peer {} was not found in the DHT", p),
            MessengerError::MailboxError(e) => write!(f, "DHT Mailbox error: {}", e),
            MessengerError::DatabaseError(e) => write!(f, "Local database error: {}", e),
            MessengerError::MnemonicError(e) => write!(f, "BIP-39 Mnemonic error: {}", e),
            MessengerError::ContactBlocked(c) => write!(f, "Contact {} is blocked", c),
            MessengerError::AttachmentTooLarge(size, max) => write!(f, "Attachment size {} exceeds maximum {}", size, max),
            MessengerError::ReplayDetected(id) => write!(f, "Replay attack detected for message {}", id),
            MessengerError::Other(e) => write!(f, "Messenger error: {}", e),
        }
    }
}

impl std::error::Error for MessengerError {}

impl From<String> for MessengerError {
    fn from(s: String) -> Self {
        MessengerError::Other(s)
    }
}
