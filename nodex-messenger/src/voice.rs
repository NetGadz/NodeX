use serde::{Deserialize, Serialize};

/// Maximum duration for voice notes (e.g., 5 minutes = 300 seconds).
pub const MAX_VOICE_NOTE_SECS: u32 = 300;

/// Number of amplitude samples in the waveform visualizer.
pub const WAVEFORM_SAMPLES_COUNT: usize = 48;

/// Metadata and payload for a voice note.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceNote {
    /// Duration of the voice note in seconds.
    pub duration_secs: u32,
    /// Array of normalized amplitude samples (0..100) for UI waveform visualizer.
    pub waveform: Vec<u8>,
    /// Audio MIME type (e.g., "audio/opus", "audio/wav", "audio/ogg").
    pub mime_type: String,
    /// Base64-encoded compressed audio bytes.
    pub audio_base64: String,
    /// Total audio file size in bytes before base64 encoding.
    pub file_size: usize,
}

impl VoiceNote {
    /// Create a new VoiceNote with audio bytes and duration.
    pub fn new(duration_secs: u32, audio_bytes: &[u8], mime_type: &str) -> Self {
        let waveform = Self::generate_waveform(audio_bytes, WAVEFORM_SAMPLES_COUNT);
        use base64::Engine;
        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);
        Self {
            duration_secs: duration_secs.min(MAX_VOICE_NOTE_SECS),
            waveform,
            mime_type: mime_type.to_string(),
            audio_base64,
            file_size: audio_bytes.len(),
        }
    }

    /// Create a simulated voice note with a realistic voice waveform pattern.
    pub fn create_simulated(duration_secs: u32, title_text: &str) -> Self {
        let mut waveform = Vec::with_capacity(WAVEFORM_SAMPLES_COUNT);
        let seed = title_text.bytes().fold(42u32, |acc, b| acc.wrapping_add(b as u32));
        for i in 0..WAVEFORM_SAMPLES_COUNT {
            // Harmonic wave formula with randomized speech bursts
            let base = (((i as f32 * 0.4 + (seed as f32 * 0.1)).sin() + 1.0) * 35.0) as u8;
            let noise = ((seed.wrapping_mul((i + 7) as u32)) % 30) as u8;
            let val = (base + noise + 10).min(100);
            waveform.push(val);
        }

        // Mock compressed audio stream payload
        let mock_audio = format!("VOICE_STREAM_DATA_DUR_{}s_HASH_{:x}", duration_secs, seed);
        use base64::Engine;
        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(mock_audio.as_bytes());

        Self {
            duration_secs: duration_secs.max(1).min(MAX_VOICE_NOTE_SECS),
            waveform,
            mime_type: "audio/opus".to_string(),
            audio_base64,
            file_size: mock_audio.len(),
        }
    }

    /// Extract waveform bars from audio payload bytes.
    pub fn generate_waveform(audio_bytes: &[u8], num_samples: usize) -> Vec<u8> {
        if audio_bytes.is_empty() {
            return vec![20; num_samples];
        }

        let chunk_size = (audio_bytes.len() / num_samples).max(1);
        let mut samples = Vec::with_capacity(num_samples);

        for chunk in audio_bytes.chunks(chunk_size) {
            if samples.len() >= num_samples {
                break;
            }
            let sum: usize = chunk.iter().map(|&b| (b as i16 - 128).abs() as usize).sum();
            let avg = sum / chunk.len();
            let normalized = ((avg as f32 / 128.0) * 100.0).clamp(10.0, 100.0) as u8;
            samples.push(normalized);
        }

        while samples.len() < num_samples {
            samples.push(15);
        }

        samples
    }

    /// Decode audio bytes from base64 representation.
    pub fn decode_audio_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(&self.audio_base64)
    }

    /// Format duration as MM:SS string (e.g., "0:42", "2:15").
    pub fn formatted_duration(&self) -> String {
        let mins = self.duration_secs / 60;
        let secs = self.duration_secs % 60;
        format!("{}:{:02}", mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_note_creation_and_waveform() {
        let sample_audio = b"OPUS_HEADER_AUDIO_STREAM_SAMPLE_PAYLOAD_DATA";
        let vn = VoiceNote::new(12, sample_audio, "audio/opus");
        assert_eq!(vn.duration_secs, 12);
        assert_eq!(vn.waveform.len(), WAVEFORM_SAMPLES_COUNT);
        assert_eq!(vn.formatted_duration(), "0:12");
        assert_eq!(vn.decode_audio_bytes().unwrap(), sample_audio);
    }

    #[test]
    fn test_simulated_voice_note() {
        let vn = VoiceNote::create_simulated(65, "Hello world voice message");
        assert_eq!(vn.duration_secs, 65);
        assert_eq!(vn.formatted_duration(), "1:05");
        assert_eq!(vn.waveform.len(), WAVEFORM_SAMPLES_COUNT);
        for &bar in &vn.waveform {
            assert!(bar <= 100);
        }
    }
}
