use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    pub bio: String,
    pub avatar_base64: Option<String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: "NodeX User".into(),
            bio: String::new(),
            avatar_base64: None,
        }
    }
}
