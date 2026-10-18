#![forbid(unsafe_code)]

pub struct BlockSyncer;

impl BlockSyncer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BlockSyncer {
    fn default() -> Self {
        Self::new()
    }
}
