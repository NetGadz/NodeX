use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CallSignalType {
    Offer,
    Answer,
    IceCandidate(String),
    Ringing,
    Reject,
    Hangup,
    Busy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallSignalPayload {
    pub call_id: String,
    pub caller_id_hex: String,
    pub callee_id_hex: String,
    pub signal_type: CallSignalType,
    pub timestamp: u64,
}

impl CallSignalPayload {
    pub fn new(call_id: &str, caller: &str, callee: &str, signal: CallSignalType) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            call_id: call_id.to_string(),
            caller_id_hex: caller.to_string(),
            callee_id_hex: callee.to_string(),
            signal_type: signal,
            timestamp: ts,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallAudioChunk {
    pub call_id: String,
    pub pcm_base64: String,
    pub sample_rate: u32,
    pub seq: u64,
}
