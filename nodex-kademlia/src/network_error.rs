use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    AddressInUse(u16),
    SocketBindFailed(String),
    Timeout(String),
    NodeUnreachable(String),
    SerializationError(String),
    BootstrapFailed(String),
    StorageError(String),
    LookupFailed(String),
    Other(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::AddressInUse(port) => write!(f, "UDP port {} is already in use (WSAEADDRINUSE)", port),
            NetworkError::SocketBindFailed(msg) => write!(f, "Failed to bind UDP socket: {}", msg),
            NetworkError::Timeout(msg) => write!(f, "Network operation timed out: {}", msg),
            NetworkError::NodeUnreachable(addr) => write!(f, "Node at {} is unreachable", addr),
            NetworkError::SerializationError(msg) => write!(f, "Serialization/wire error: {}", msg),
            NetworkError::BootstrapFailed(msg) => write!(f, "DHT Bootstrap failed: {}", msg),
            NetworkError::StorageError(msg) => write!(f, "Storage operation failed: {}", msg),
            NetworkError::LookupFailed(msg) => write!(f, "DHT lookup failed: {}", msg),
            NetworkError::Other(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for NetworkError {}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::AddrInUse {
            NetworkError::AddressInUse(0)
        } else {
            NetworkError::SocketBindFailed(err.to_string())
        }
    }
}
