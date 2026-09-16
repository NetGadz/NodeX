use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use nodex_kademlia::KademliaNode;
use crate::errors::MessengerError;
use crate::presence::UserPresenceCard;

#[derive(Clone)]
pub struct PeerDiscovery {
    dht_node: KademliaNode,
    cache: Arc<RwLock<HashMap<String, UserPresenceCard>>>,
}

impl PeerDiscovery {
    pub fn new(dht_node: KademliaNode) -> Self {
        Self {
            dht_node,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn discover(&self, user_id_hex: &str) -> Result<UserPresenceCard, MessengerError> {
        let presence_key = format!("presence_{}", user_id_hex.trim());

        if let Ok(Some((bytes, _))) = self.dht_node.get(&presence_key).await {
            let card: UserPresenceCard = serde_json::from_slice(&bytes)
                .map_err(|e| MessengerError::CryptoError(format!("Failed to parse presence card: {}", e)))?;
            card.verify_signature()?;
            self.cache.write().await.insert(user_id_hex.to_string(), card.clone());
            Ok(card)
        } else {
            Err(MessengerError::PeerNotFound(user_id_hex.to_string()))
        }
    }

    pub async fn get_cached(&self, user_id_hex: &str) -> Option<UserPresenceCard> {
        self.cache.read().await.get(user_id_hex).cloned()
    }
}
