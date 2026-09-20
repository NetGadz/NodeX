use std::collections::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Blocklist {
    blocked_user_ids: HashSet<String>,
}

impl Blocklist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block(&mut self, user_id_hex: &str) {
        self.blocked_user_ids.insert(user_id_hex.to_string());
    }

    pub fn unblock(&mut self, user_id_hex: &str) {
        self.blocked_user_ids.remove(user_id_hex);
    }

    pub fn is_blocked(&self, user_id_hex: &str) -> bool {
        self.blocked_user_ids.contains(user_id_hex)
    }
}
