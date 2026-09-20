use serde::{Deserialize, Serialize};

/// Supported quick emoji reactions.
pub const DEFAULT_REACTIONS: &[&str] = &["👍", "❤️", "🔥", "😂", "🎉", "😮", "😢", "👏"];

/// A cryptographic reaction applied by a user to a specific message.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageReaction {
    /// ID of the message being reacted to.
    pub message_id: String,
    /// UTF-8 emoji string (e.g. "👍", "🔥").
    pub emoji: String,
    /// 40-character hex User ID of the reactor.
    pub reactor_user_id_hex: String,
    /// Unix timestamp when the reaction was placed.
    pub timestamp: u64,
}

impl MessageReaction {
    pub fn new(message_id: &str, emoji: &str, reactor_user_id_hex: &str) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            message_id: message_id.to_string(),
            emoji: emoji.to_string(),
            reactor_user_id_hex: reactor_user_id_hex.to_string(),
            timestamp,
        }
    }

    /// Check if the emoji is within reasonable length (single or double grapheme cluster).
    pub fn is_valid_emoji(&self) -> bool {
        !self.emoji.is_empty() && self.emoji.chars().count() <= 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reaction_validation() {
        let r1 = MessageReaction::new("msg_123", "🔥", "alice_id");
        assert!(r1.is_valid_emoji());
        assert_eq!(r1.emoji, "🔥");

        let r2 = MessageReaction::new("msg_123", "invalid_long_reaction_string", "alice_id");
        assert!(!r2.is_valid_emoji());
    }
}
