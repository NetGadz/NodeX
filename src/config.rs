use std::fs;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub ip: String,
    pub port: u16,
    pub bootstrap: Vec<SocketAddr>,
    pub state_file: String,
    pub ttl_secs: u64,
    pub republish_secs: u64,
    pub rpc_timeout_ms: u64,
    pub rpc_max_retries: usize,
    pub save_state_on_exit: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".into(),
            port: 8000,
            bootstrap: Vec::new(),
            state_file: "node_state.json".into(),
            ttl_secs: 86400,        // 24 hours
            republish_secs: 3600,   // 1 hour
            rpc_timeout_ms: 5000,   // 5 seconds
            rpc_max_retries: 2,
            save_state_on_exit: true,
        }
    }
}

impl NodeConfig {
    pub fn load_from_json_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
        Self::from_json_str(&content)
    }

    pub fn from_json_str(s: &str) -> Result<Self, String> {
        let mut cfg = Self::default();
        // Parse simple JSON config fields
        if let Ok(v) = json_get_string(s, "ip") {
            cfg.ip = v;
        }
        if let Ok(v) = json_get_u64(s, "port") {
            cfg.port = v as u16;
        }
        if let Ok(v) = json_get_string(s, "state_file") {
            cfg.state_file = v;
        }
        if let Ok(v) = json_get_u64(s, "ttl_secs") {
            cfg.ttl_secs = v;
        }
        if let Ok(v) = json_get_u64(s, "republish_secs") {
            cfg.republish_secs = v;
        }
        if let Ok(v) = json_get_u64(s, "rpc_timeout_ms") {
            cfg.rpc_timeout_ms = v;
        }
        if let Ok(v) = json_get_u64(s, "rpc_max_retries") {
            cfg.rpc_max_retries = v as usize;
        }
        if let Ok(addrs_str) = json_get_array_strings(s, "bootstrap") {
            let mut list = Vec::new();
            for a in addrs_str {
                if let Ok(sa) = a.parse::<SocketAddr>() {
                    list.push(sa);
                }
            }
            cfg.bootstrap = list;
        }

        Ok(cfg)
    }

    pub fn to_json_string(&self) -> String {
        let bootstrap_json = self
            .bootstrap
            .iter()
            .map(|a| format!("\"{}\"", a))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "{{\n  \"ip\": \"{}\",\n  \"port\": {},\n  \"bootstrap\": [{}],\n  \"state_file\": \"{}\",\n  \"ttl_secs\": {},\n  \"republish_secs\": {},\n  \"rpc_timeout_ms\": {},\n  \"rpc_max_retries\": {}\n}}",
            self.ip,
            self.port,
            bootstrap_json,
            self.state_file,
            self.ttl_secs,
            self.republish_secs,
            self.rpc_timeout_ms,
            self.rpc_max_retries
        )
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = self.to_json_string();
        fs::write(path, content).map_err(|e| format!("Failed to write config file: {}", e))
    }
}

fn json_get_string(json: &str, key: &str) -> Result<String, ()> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let rest = &json[pos + pattern.len()..];
        if let Some(q1) = rest.find('"') {
            let value_part = &rest[q1 + 1..];
            if let Some(q2) = value_part.find('"') {
                return Ok(value_part[..q2].to_string());
            }
        }
    }
    Err(())
}

fn json_get_u64(json: &str, key: &str) -> Result<u64, ()> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let rest = &json[pos + pattern.len()..].trim_start();
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(val) = num_str.parse::<u64>() {
            return Ok(val);
        }
    }
    Err(())
}

fn json_get_array_strings(json: &str, key: &str) -> Result<Vec<String>, ()> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let rest = &json[pos + pattern.len()..];
        if let Some(b1) = rest.find('[') {
            if let Some(b2) = rest.find(']') {
                let inner = &rest[b1 + 1..b2];
                let items: Vec<String> = inner
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                return Ok(items);
            }
        }
    }
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_serialization_and_parsing() {
        let mut cfg = NodeConfig::default();
        cfg.port = 9050;
        cfg.bootstrap = vec!["127.0.0.1:8001".parse().unwrap()];

        let json = cfg.to_json_string();
        let parsed = NodeConfig::from_json_str(&json).unwrap();
        assert_eq!(parsed.port, 9050);
        assert_eq!(parsed.bootstrap.len(), 1);
        assert_eq!(parsed.bootstrap[0], "127.0.0.1:8001".parse::<SocketAddr>().unwrap());
    }
}
