use sha2::{Digest, Sha256};

pub fn calculate_merkle_root(hashes: &[[u8; 32]]) -> [u8; 32] {
    if hashes.is_empty() {
        return [0u8; 32];
    }
    if hashes.len() == 1 {
        return hashes[0];
    }

    let mut current_layer: Vec<[u8; 32]> = hashes.to_vec();

    while current_layer.len() > 1 {
        let mut next_layer = Vec::new();
        for chunk in current_layer.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(&chunk[0]);
            if chunk.len() == 2 {
                hasher.update(&chunk[1]);
            } else {
                hasher.update(&chunk[0]); // Duplicate odd node
            }
            let res = hasher.finalize();
            let mut parent = [0u8; 32];
            parent.copy_from_slice(&res);
            next_layer.push(parent);
        }
        current_layer = next_layer;
    }

    current_layer[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_root_single() {
        let h = [1u8; 32];
        assert_eq!(calculate_merkle_root(&[h]), h);
    }

    #[test]
    fn test_merkle_root_pair() {
        let h1 = [1u8; 32];
        let h2 = [2u8; 32];
        let root = calculate_merkle_root(&[h1, h2]);
        assert_ne!(root, [0u8; 32]);
    }
}
