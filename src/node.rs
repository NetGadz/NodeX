use std::fmt;
use std::net::SocketAddr;
use std::time::Instant;

use rand::Rng;

#[link(name = "kademlia_cryptography")]
extern "C" {
    fn c_shared_prefix_bits(id1: *const u8, id2: *const u8, len: usize) -> u32;
    fn hash_node_id(input: *const u8, len: usize, out_id: *mut u8);
}

pub const ID_SIZE: usize = 20;
pub const K: usize = 20;
pub const NUM_BUCKETS: usize = 160;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct NodeId(pub [u8; ID_SIZE]);

impl NodeId {
    pub fn generate_random() -> Self {
        let mut id = [0u8; ID_SIZE];
        rand::thread_rng().fill(&mut id);
        Self(id)
    }

    pub fn from_key(key: &[u8]) -> Self {
        let mut out = [0u8; ID_SIZE];
        unsafe {
            hash_node_id(key.as_ptr(), key.len(), out.as_mut_ptr());
        }
        Self(out)
    }

    pub fn from_bytes(bytes: [u8; ID_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ID_SIZE] {
        &self.0
    }

    pub fn shared_prefix_len(&self, other: &NodeId) -> usize {
        unsafe { c_shared_prefix_bits(self.0.as_ptr(), other.0.as_ptr(), ID_SIZE) as usize }
    }

    pub fn distance(&self, other: &NodeId) -> NodeId {
        let mut out = [0u8; ID_SIZE];
        for i in 0..ID_SIZE {
            out[i] = self.0[i] ^ other.0[i];
        }
        NodeId(out)
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn from_hex(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.len() != ID_SIZE * 2 {
            return Err(format!("Expected {} hex chars, got {}", ID_SIZE * 2, s.len()));
        }
        let mut bytes = [0u8; ID_SIZE];
        for i in 0..ID_SIZE {
            bytes[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|e| format!("Invalid hex: {}", e))?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    pub id: NodeId,
    pub addr: SocketAddr,
    pub last_seen: Instant,
}

impl Contact {
    pub fn new(id: NodeId, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            last_seen: Instant::now(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateResult {
    Inserted,
    Updated,
    Full(Contact),
    SelfIgnored,
}

#[derive(Clone, Debug)]
pub struct KBucket {
    pub contacts: Vec<Contact>,
}

impl Default for KBucket {
    fn default() -> Self {
        Self {
            contacts: Vec::with_capacity(K),
        }
    }
}

impl KBucket {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_full(&self) -> bool {
        self.contacts.len() >= K
    }

    pub fn len(&self) -> usize {
        self.contacts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    pub fn contains(&self, id: &NodeId) -> bool {
        self.contacts.iter().any(|c| c.id == *id)
    }

    pub fn update(&mut self, contact: Contact) -> UpdateResult {
        if let Some(pos) = self.contacts.iter().position(|c| c.id == contact.id) {
            self.contacts.remove(pos);
            self.contacts.push(contact);
            UpdateResult::Updated
        } else if self.contacts.len() < K {
            self.contacts.push(contact);
            UpdateResult::Inserted
        } else {
            // Bucket is full: return the least-recently-seen contact (at head) for pinging
            UpdateResult::Full(self.contacts[0].clone())
        }
    }

    pub fn remove(&mut self, id: &NodeId) -> Option<Contact> {
        if let Some(pos) = self.contacts.iter().position(|c| c.id == *id) {
            Some(self.contacts.remove(pos))
        } else {
            None
        }
    }

    pub fn replace_oldest(&mut self, new_contact: Contact) {
        if !self.contacts.is_empty() {
            self.contacts.remove(0);
        }
        self.contacts.push(new_contact);
    }
}

#[derive(Clone, Debug)]
pub struct RoutingTable {
    pub local_id: NodeId,
    pub buckets: Vec<KBucket>,
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        let mut buckets = Vec::with_capacity(NUM_BUCKETS);
        for _ in 0..NUM_BUCKETS {
            buckets.push(KBucket::new());
        }
        Self { local_id, buckets }
    }

    pub fn bucket_index(&self, id: &NodeId) -> usize {
        let prefix = self.local_id.shared_prefix_len(id);
        if prefix >= NUM_BUCKETS {
            NUM_BUCKETS - 1
        } else {
            prefix
        }
    }

    pub fn update(&mut self, contact: Contact) -> UpdateResult {
        if contact.id == self.local_id {
            return UpdateResult::SelfIgnored;
        }
        let idx = self.bucket_index(&contact.id);
        self.buckets[idx].update(contact)
    }

    pub fn remove(&mut self, id: &NodeId) -> Option<Contact> {
        let idx = self.bucket_index(id);
        self.buckets[idx].remove(id)
    }

    pub fn replace_oldest(&mut self, new_contact: Contact) {
        let idx = self.bucket_index(&new_contact.id);
        self.buckets[idx].replace_oldest(new_contact);
    }

    pub fn find_closest(&self, target: &NodeId, count: usize) -> Vec<Contact> {
        let mut all = self.all_contacts();
        all.sort_by(|a, b| {
            let dist_a = a.id.distance(target);
            let dist_b = b.id.distance(target);
            dist_a.cmp(&dist_b)
        });
        all.truncate(count);
        all
    }

    pub fn all_contacts(&self) -> Vec<Contact> {
        let mut list = Vec::new();
        for b in &self.buckets {
            list.extend(b.contacts.clone());
        }
        list
    }

    pub fn total_contacts(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_generate_random_has_correct_size() {
        let id = NodeId::generate_random();
        assert_eq!(id.as_bytes().len(), ID_SIZE);
    }

    #[test]
    fn node_id_from_key_is_deterministic() {
        let k1 = NodeId::from_key(b"test_key_hello");
        let k2 = NodeId::from_key(b"test_key_hello");
        let k3 = NodeId::from_key(b"other_key");
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn xor_distance_properties() {
        let a = NodeId::generate_random();
        let b = NodeId::generate_random();
        let dist_ab = a.distance(&b);
        let dist_ba = b.distance(&a);
        assert_eq!(dist_ab, dist_ba);
        assert_eq!(a.distance(&a), NodeId::from_bytes([0u8; ID_SIZE]));
    }

    #[test]
    fn shared_prefix_bits_calculation() {
        let mut b1 = [0u8; ID_SIZE];
        let mut b2 = [0u8; ID_SIZE];
        b1[0] = 0b11110000;
        b2[0] = 0b11110011;
        let id1 = NodeId::from_bytes(b1);
        let id2 = NodeId::from_bytes(b2);
        // First 6 bits match (111100), 7th bit differs (0 vs 1)
        assert_eq!(id1.shared_prefix_len(&id2), 6);

        // Identical IDs: all 160 bits match
        assert_eq!(id1.shared_prefix_len(&id1), 160);
    }

    #[test]
    fn kbucket_respects_capacity() {
        let mut bucket = KBucket::new();
        for i in 0..(K + 5) {
            let mut bytes = [0u8; ID_SIZE];
            bytes[0] = (i + 1) as u8;
            let id = NodeId::from_bytes(bytes);
            let contact = Contact::new(id, "127.0.0.1:8080".parse().unwrap());
            bucket.update(contact);
        }
        assert_eq!(bucket.len(), K);
    }

    #[test]
    fn routing_table_closest_nodes() {
        let local_id = NodeId::from_bytes([0u8; ID_SIZE]);
        let mut rt = RoutingTable::new(local_id);

        let mut c1_bytes = [0u8; ID_SIZE];
        c1_bytes[19] = 1; // distance 1
        let c1 = Contact::new(NodeId::from_bytes(c1_bytes), "127.0.0.1:8001".parse().unwrap());

        let mut c2_bytes = [0u8; ID_SIZE];
        c2_bytes[19] = 10; // distance 10
        let c2 = Contact::new(NodeId::from_bytes(c2_bytes), "127.0.0.1:8002".parse().unwrap());

        let mut c3_bytes = [0u8; ID_SIZE];
        c3_bytes[0] = 1; // large distance
        let c3 = Contact::new(NodeId::from_bytes(c3_bytes), "127.0.0.1:8003".parse().unwrap());

        rt.update(c1.clone());
        rt.update(c2.clone());
        rt.update(c3.clone());

        let closest = rt.find_closest(&local_id, 2);
        assert_eq!(closest.len(), 2);
        assert_eq!(closest[0].id, c1.id);
        assert_eq!(closest[1].id, c2.id);
    }
}
