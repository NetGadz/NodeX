use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rand::RngCore;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupRole {
    Creator,
    Admin,
    Member,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMember {
    pub user_id_hex: String,
    pub display_name: String,
    pub role: GroupRole,
    pub joined_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct P2PGroup {
    pub group_id: String,
    pub title: String,
    pub description: String,
    pub creator_id_hex: String,
    pub members: HashMap<String, GroupMember>,
    pub group_key_hex: String,
    pub created_at: u64,
    pub avatar_color_idx: u8,
}

impl P2PGroup {
    pub fn new(title: &str, description: &str, creator_id_hex: &str, creator_name: &str) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let group_key_hex: String = key_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let group_id = format!("grp_{}_{:x}", timestamp, rand::random::<u32>());

        let mut members = HashMap::new();
        members.insert(
            creator_id_hex.to_string(),
            GroupMember {
                user_id_hex: creator_id_hex.to_string(),
                display_name: creator_name.to_string(),
                role: GroupRole::Creator,
                joined_at: timestamp,
            },
        );

        let color_idx = rand::random::<u8>() % 6;

        Self {
            group_id,
            title: title.to_string(),
            description: description.to_string(),
            creator_id_hex: creator_id_hex.to_string(),
            members,
            group_key_hex,
            created_at: timestamp,
            avatar_color_idx: color_idx,
        }
    }

    pub fn add_member(&mut self, user_id_hex: &str, display_name: &str, role: GroupRole) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.members.insert(
            user_id_hex.to_string(),
            GroupMember {
                user_id_hex: user_id_hex.to_string(),
                display_name: display_name.to_string(),
                role,
                joined_at: timestamp,
            },
        );
    }

    pub fn remove_member(&mut self, user_id_hex: &str) -> bool {
        self.members.remove(user_id_hex).is_some()
    }

    pub fn is_member(&self, user_id_hex: &str) -> bool {
        self.members.contains_key(user_id_hex)
    }

    pub fn is_admin(&self, user_id_hex: &str) -> bool {
        match self.members.get(user_id_hex) {
            Some(m) => m.role == GroupRole::Creator || m.role == GroupRole::Admin,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_group_creation_and_membership() {
        let mut grp = P2PGroup::new("Rust Devs", "Decentralized P2P Group", "alice_id", "Alice");
        assert_eq!(grp.title, "Rust Devs");
        assert!(grp.is_member("alice_id"));
        assert!(grp.is_admin("alice_id"));

        grp.add_member("bob_id", "Bob", GroupRole::Member);
        assert_eq!(grp.members.len(), 2);
        assert!(grp.is_member("bob_id"));
        assert!(!grp.is_admin("bob_id"));

        grp.remove_member("bob_id");
        assert_eq!(grp.members.len(), 1);
        assert!(!grp.is_member("bob_id"));
    }
}
