use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::db::SavedContact;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContactsManager {
    contacts: HashMap<String, SavedContact>,
    blocked: HashMap<String, bool>,
}

impl ContactsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, contact: SavedContact) {
        self.contacts.insert(contact.user_id_hex.clone(), contact);
    }

    pub fn remove(&mut self, user_id_hex: &str) -> Option<SavedContact> {
        self.contacts.remove(user_id_hex)
    }

    pub fn get(&self, user_id_hex: &str) -> Option<&SavedContact> {
        self.contacts.get(user_id_hex)
    }

    pub fn block(&mut self, user_id_hex: &str) {
        self.blocked.insert(user_id_hex.to_string(), true);
    }

    pub fn unblock(&mut self, user_id_hex: &str) {
        self.blocked.remove(user_id_hex);
    }

    pub fn is_blocked(&self, user_id_hex: &str) -> bool {
        self.blocked.get(user_id_hex).copied().unwrap_or(false)
    }

    pub fn list(&self) -> Vec<SavedContact> {
        self.contacts.values().cloned().collect()
    }

    pub fn search(&self, query: &str) -> Vec<SavedContact> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.list();
        }
        self.contacts
            .values()
            .filter(|c| c.name.to_lowercase().contains(&q) || c.user_id_hex.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }
}
