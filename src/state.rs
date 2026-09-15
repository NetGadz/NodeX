use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use crate::node::{Contact, NodeId};

pub struct StateManager;

impl StateManager {
    pub fn load_state<P: AsRef<Path>>(path: P) -> Result<(NodeId, Vec<Contact>), String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Read state failed: {}", e))?;
        
        let node_id_hex = json_get_string(&content, "node_id")
            .ok_or_else(|| "Missing node_id in state file".to_string())?;
        
        let node_id = NodeId::from_hex(&node_id_hex)
            .map_err(|e| format!("Invalid node_id hex in state file: {}", e))?;

        let mut contacts = Vec::new();
        if let Some(entries) = json_get_contacts_array(&content) {
            for (id_hex, addr_str) in entries {
                if let (Ok(id), Ok(addr)) = (NodeId::from_hex(&id_hex), addr_str.parse::<SocketAddr>()) {
                    contacts.push(Contact::new(id, addr));
                }
            }
        }

        Ok((node_id, contacts))
    }

    pub fn save_state<P: AsRef<Path>>(node_id: NodeId, contacts: &[Contact], path: P) -> Result<(), String> {
        let contacts_json = contacts
            .iter()
            .map(|c| format!("{{\"id\": \"{}\", \"addr\": \"{}\"}}", c.id, c.addr))
            .collect::<Vec<_>>()
            .join(",\n    ");

        let content = format!(
            "{{\n  \"node_id\": \"{}\",\n  \"contacts\": [\n    {}\n  ]\n}}",
            node_id.to_hex(),
            contacts_json
        );

        fs::write(path, content).map_err(|e| format!("Save state failed: {}", e))
    }
}

fn json_get_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let rest = &json[pos + pattern.len()..];
        if let Some(q1) = rest.find('"') {
            let value_part = &rest[q1 + 1..];
            if let Some(q2) = value_part.find('"') {
                return Some(value_part[..q2].to_string());
            }
        }
    }
    None
}

fn json_get_contacts_array(json: &str) -> Option<Vec<(String, String)>> {
    let pattern = "\"contacts\":";
    if let Some(pos) = json.find(pattern) {
        let rest = &json[pos + pattern.len()..];
        if let Some(b1) = rest.find('[') {
            if let Some(b2) = rest.find(']') {
                let inner = &rest[b1 + 1..b2];
                let mut results = Vec::new();
                for block in inner.split('}') {
                    if let (Some(id), Some(addr)) = (json_get_string(block, "id"), json_get_string(block, "addr")) {
                        results.push((id, addr));
                    }
                }
                return Some(results);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_save_and_restore() {
        let node_id = NodeId::generate_random();
        let c1 = Contact::new(NodeId::generate_random(), "127.0.0.1:8001".parse().unwrap());
        let contacts = vec![c1.clone()];

        let temp_path = std::env::temp_dir().join("test_kademlia_state.json");
        StateManager::save_state(node_id, &contacts, &temp_path).unwrap();

        let (loaded_id, loaded_contacts) = StateManager::load_state(&temp_path).unwrap();
        assert_eq!(loaded_id, node_id);
        assert_eq!(loaded_contacts.len(), 1);
        assert_eq!(loaded_contacts[0].id, c1.id);
        assert_eq!(loaded_contacts[0].addr, c1.addr);

        let _ = fs::remove_file(temp_path);
    }
}
