use std::io::Cursor;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use nodex_messenger::voice::VoiceNote;

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    call_audio_sink: Option<Sink>,
    fx_sink: Option<Sink>,
    current_msg_id: Option<String>,
    start_time: Option<std::time::Instant>,
    duration_secs: f32,
    is_looping: bool,
    loop_wav_bytes: Option<Vec<u8>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        // Lazily or safely attempt to get the default audio output stream
        let (stream, handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("[AUDIO] Could not open default audio output device: {:?}", e);
                (None, None)
            }
        };

        Self {
            _stream: stream,
            stream_handle: handle,
            sink: None,
            call_audio_sink: None,
            fx_sink: None,
            current_msg_id: None,
            start_time: None,
            duration_secs: 0.0,
            is_looping: false,
            loop_wav_bytes: None,
        }
    }

    pub fn play_voice_note(&mut self, msg_id: &str, voice_note: &VoiceNote) {
        self.stop();

        // If audio output was not initialized, try to re-initialize it
        if self.stream_handle.is_none() {
            if let Ok((s, h)) = OutputStream::try_default() {
                self._stream = Some(s);
                self.stream_handle = Some(h);
            }
        }

        let wav_bytes = match voice_note.decode_audio_bytes() {
            Ok(bytes) if bytes.len() >= 44 && &bytes[0..4] == b"RIFF" => bytes,
            _ => {
                // Synthesize a rich, pleasant melodic voice note audio from waveform
                Self::synthesize_wav(voice_note.duration_secs, &voice_note.waveform)
            }
        };

        let dur = voice_note.duration_secs.max(1) as f32;

        if let Some(handle) = &self.stream_handle {
            match Sink::try_new(handle) {
                Ok(sink) => {
                    let cursor = Cursor::new(wav_bytes);
                    match Decoder::new(cursor) {
                        Ok(source) => {
                            sink.set_volume(1.0);
                            sink.append(source);
                            sink.play();
                            self.sink = Some(sink);
                            self.current_msg_id = Some(msg_id.to_string());
                            self.start_time = Some(std::time::Instant::now());
                            self.duration_secs = dur;
                            println!("[AUDIO ENGINE] Playing voice note {}: duration {}s", msg_id, dur);
                        }
                        Err(e) => {
                            eprintln!("[AUDIO ENGINE] Decoder error: {:?}", e);
                            self.current_msg_id = Some(msg_id.to_string());
                            self.start_time = Some(std::time::Instant::now());
                            self.duration_secs = dur;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[AUDIO ENGINE] Failed to create Sink: {:?}", e);
                    self.current_msg_id = Some(msg_id.to_string());
                    self.start_time = Some(std::time::Instant::now());
                    self.duration_secs = dur;
                }
            }
        } else {
            // Fallback progress simulation when no hardware audio device is available
            self.current_msg_id = Some(msg_id.to_string());
            self.start_time = Some(std::time::Instant::now());
            self.duration_secs = dur;
        }
    }

    pub fn play_notification_sound(&mut self) {
        let wav = Self::synthesize_notification_tone();
        self.play_fx_wav(wav);
    }

    pub fn play_sent_sound(&mut self) {
        let wav = Self::synthesize_sent_tone();
        self.play_fx_wav(wav);
    }

    pub fn play_ringtone_loop(&mut self) {
        let wav = Self::synthesize_ringtone();
        self.play_raw_wav_loop(wav, 2.4);
    }

    pub fn play_dial_tone_loop(&mut self) {
        let wav = Self::synthesize_dial_tone();
        self.play_raw_wav_loop(wav, 1.6);
    }

    pub fn play_hangup_sound(&mut self) {
        self.stop();
        let wav = Self::synthesize_hangup_tone();
        self.play_raw_wav(wav, 0.5);
    }

    pub fn start_call_audio(&mut self) {
        self.stop();
        if self.stream_handle.is_none() {
            if let Ok((s, h)) = OutputStream::try_default() {
                self._stream = Some(s);
                self.stream_handle = Some(h);
            }
        }
        if let Some(handle) = &self.stream_handle {
            if let Ok(sink) = Sink::try_new(handle) {
                sink.set_volume(1.0);
                sink.play();
                self.call_audio_sink = Some(sink);
                println!("[AUDIO ENGINE] Initialized persistent call audio sink");
            }
        }
    }

    /// Play direct PCM audio chunk received in real-time from peer during an active voice call
    pub fn play_call_audio_samples(&mut self, pcm_samples: &[i16], sample_rate: u32) {
        if pcm_samples.is_empty() {
            return;
        }
        if self.call_audio_sink.is_none() {
            self.start_call_audio();
        }
        if let Some(sink) = &self.call_audio_sink {
            // Playout queue cap: allow smooth jitter buffering (up to 40 chunks ~ 1.6s)
            if sink.len() > 40 {
                return;
            }
            let buffer = rodio::buffer::SamplesBuffer::new(1, sample_rate, pcm_samples.to_vec());
            sink.append(buffer);
        }
    }

    fn play_fx_wav(&mut self, wav_bytes: Vec<u8>) {
        if self.stream_handle.is_none() {
            if let Ok((s, h)) = OutputStream::try_default() {
                self._stream = Some(s);
                self.stream_handle = Some(h);
            }
        }
        if let Some(handle) = &self.stream_handle {
            if self.fx_sink.as_ref().map_or(true, |s| s.empty()) {
                if let Ok(sink) = Sink::try_new(handle) {
                    sink.set_volume(0.85);
                    let cursor = Cursor::new(wav_bytes);
                    if let Ok(source) = Decoder::new(cursor) {
                        sink.append(source);
                        sink.play();
                        self.fx_sink = Some(sink);
                    }
                }
            } else if let Some(sink) = &self.fx_sink {
                let cursor = Cursor::new(wav_bytes);
                if let Ok(source) = Decoder::new(cursor) {
                    sink.append(source);
                }
            }
        }
    }

    fn play_raw_wav_loop(&mut self, wav_bytes: Vec<u8>, duration: f32) {
        self.stop();
        self.is_looping = true;
        self.loop_wav_bytes = Some(wav_bytes.clone());

        if self.stream_handle.is_none() {
            if let Ok((s, h)) = OutputStream::try_default() {
                self._stream = Some(s);
                self.stream_handle = Some(h);
            }
        }

        if let Some(handle) = &self.stream_handle {
            if let Ok(sink) = Sink::try_new(handle) {
                let cursor = Cursor::new(wav_bytes);
                if let Ok(source) = Decoder::new(cursor) {
                    sink.set_volume(0.9);
                    sink.append(source);
                    sink.play();
                    self.sink = Some(sink);
                    self.current_msg_id = Some("__system_sound_loop__".to_string());
                    self.start_time = Some(std::time::Instant::now());
                    self.duration_secs = duration;
                }
            }
        }
    }

    fn play_raw_wav(&mut self, wav_bytes: Vec<u8>, duration: f32) {
        self.stop();
        if self.stream_handle.is_none() {
            if let Ok((s, h)) = OutputStream::try_default() {
                self._stream = Some(s);
                self.stream_handle = Some(h);
            }
        }

        if let Some(handle) = &self.stream_handle {
            if let Ok(sink) = Sink::try_new(handle) {
                let cursor = Cursor::new(wav_bytes);
                if let Ok(source) = Decoder::new(cursor) {
                    sink.set_volume(0.85);
                    sink.append(source);
                    sink.play();
                    self.sink = Some(sink);
                    self.current_msg_id = Some("__system_sound__".to_string());
                    self.start_time = Some(std::time::Instant::now());
                    self.duration_secs = duration;
                }
            }
        }
    }

    pub fn stop(&mut self) {
        self.is_looping = false;
        self.loop_wav_bytes = None;
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.current_msg_id = None;
        self.start_time = None;
        self.duration_secs = 0.0;
    }

    pub fn stop_call_audio(&mut self) {
        if let Some(sink) = self.call_audio_sink.take() {
            sink.stop();
        }
    }

    #[allow(dead_code)]
    pub fn is_playing(&self, msg_id: &str) -> bool {
        self.current_msg_id.as_deref() == Some(msg_id)
    }

    pub fn update_progress(&mut self) -> (Option<String>, f32) {
        if let (Some(msg_id), Some(start)) = (self.current_msg_id.clone(), self.start_time) {
            let elapsed = start.elapsed().as_secs_f32();
            let progress = (elapsed / self.duration_secs.max(0.1)).clamp(0.0, 1.0);

            let is_finished = if let Some(sink) = &self.sink {
                progress >= 1.0 || (elapsed > 0.5 && sink.empty() && progress > 0.9)
            } else {
                progress >= 1.0
            };

            if is_finished {
                if self.is_looping {
                    if let Some(bytes) = self.loop_wav_bytes.clone() {
                        if let Some(handle) = &self.stream_handle {
                            if let Ok(sink) = Sink::try_new(handle) {
                                let cursor = Cursor::new(bytes);
                                if let Ok(source) = Decoder::new(cursor) {
                                    sink.set_volume(0.9);
                                    sink.append(source);
                                    sink.play();
                                    self.sink = Some(sink);
                                    self.start_time = Some(std::time::Instant::now());
                                    return (Some(msg_id), 0.0);
                                }
                            }
                        }
                    }
                }
                self.stop();
                (None, 0.0)
            } else {
                (Some(msg_id), progress)
            }
        } else {
            (None, 0.0)
        }
    }

    /// Convert raw PCM samples into valid WAV bytes
    fn samples_to_wav(pcm_samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_len = (pcm_samples.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_len as usize);

        // "RIFF"
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // "fmt " chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size
        wav.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 = PCM)
        wav.extend_from_slice(&1u16.to_le_bytes());  // NumChannels (1 = Mono)
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * 1 * 16 / 8;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());  // BlockAlign
        wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample

        // "data" chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());

        // Samples
        for &sample in pcm_samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        wav
    }

    /// Telegram-like incoming notification chime (two high, clear bell notes)
    pub fn synthesize_notification_tone() -> Vec<u8> {
        let sample_rate = 44100u32;
        let duration = 0.35f32;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let (freq, env) = if t < 0.12 {
                let note_t = t / 0.12;
                let env = (1.0 - note_t).powf(1.8);
                (783.99f32, env) // G5
            } else {
                let note_t = (t - 0.12) / (duration - 0.12);
                let env = (1.0 - note_t).powf(2.0);
                (1046.50f32, env) // C6
            };

            let wave = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.7
                + (2.0 * std::f32::consts::PI * (freq * 2.0) * t).sin() * 0.25;
            let sample = (wave * env * 22000.0).clamp(-32000.0, 32000.0) as i16;
            samples.push(sample);
        }

        Self::samples_to_wav(&samples, sample_rate)
    }

    /// Telegram-like outgoing message sent pop
    pub fn synthesize_sent_tone() -> Vec<u8> {
        let sample_rate = 44100u32;
        let duration = 0.08f32;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let norm = t / duration;
            let freq = 500.0 + (1.0 - norm) * 400.0;
            let env = (1.0 - norm).powf(2.5);
            let wave = (2.0 * std::f32::consts::PI * freq * t).sin();
            let sample = (wave * env * 18000.0).clamp(-32000.0, 32000.0) as i16;
            samples.push(sample);
        }

        Self::samples_to_wav(&samples, sample_rate)
    }

    /// Gentle melody ringtone for incoming calls
    pub fn synthesize_ringtone() -> Vec<u8> {
        let sample_rate = 44100u32;
        let duration = 2.4f32;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        // Sequence of notes: E5, G#5, B5, E6, B5, G#5
        let notes = [659.25, 830.61, 987.77, 1318.51, 987.77, 830.61];
        let note_dur = 0.35f32;

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let note_idx = ((t / note_dur) as usize).min(notes.len() - 1);
            let note_t = (t % note_dur) / note_dur;
            let freq = notes[note_idx];
            let env = (1.0 - note_t).powf(1.5) * 0.8;

            let wave = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.6
                + (2.0 * std::f32::consts::PI * (freq * 2.0) * t).sin() * 0.3
                + (2.0 * std::f32::consts::PI * (freq * 0.5) * t).sin() * 0.1;
            let sample = (wave * env * 20000.0).clamp(-32000.0, 32000.0) as i16;
            samples.push(sample);
        }

        Self::samples_to_wav(&samples, sample_rate)
    }

    /// Outgoing calling dial tone beeps
    pub fn synthesize_dial_tone() -> Vec<u8> {
        let sample_rate = 44100u32;
        let duration = 1.6f32;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let is_beep = (t % 1.2) < 0.6;
            if is_beep {
                let wave = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
                    + (2.0 * std::f32::consts::PI * 480.0 * t).sin() * 0.5;
                let sample = (wave * 14000.0).clamp(-32000.0, 32000.0) as i16;
                samples.push(sample);
            } else {
                samples.push(0);
            }
        }

        Self::samples_to_wav(&samples, sample_rate)
    }

    /// Call hangup / disconnect sound
    pub fn synthesize_hangup_tone() -> Vec<u8> {
        let sample_rate = 44100u32;
        let duration = 0.45f32;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let (freq, env) = if t < 0.18 {
                let note_t = t / 0.18;
                let env = (1.0 - note_t).powf(1.8);
                (480.0f32, env)
            } else if t < 0.22 {
                (0.0f32, 0.0f32)
            } else {
                let note_t = (t - 0.22) / (duration - 0.22);
                let env = (1.0 - note_t).powf(1.8);
                (360.0f32, env)
            };

            let wave = if freq > 0.0 {
                (2.0 * std::f32::consts::PI * freq * t).sin()
            } else {
                0.0
            };
            let sample = (wave * env * 18000.0).clamp(-32000.0, 32000.0) as i16;
            samples.push(sample);
        }

        Self::samples_to_wav(&samples, sample_rate)
    }

    /// Real-time live connected direct audio carrier ambience
    #[allow(dead_code)]
    pub fn synthesize_call_carrier_tone() -> Vec<u8> {
        let sample_rate = 44100u32;
        let duration = 2.0f32;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let wave = (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.05
                + (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.02;
            let sample = (wave * 4000.0).clamp(-32000.0, 32000.0) as i16;
            samples.push(sample);
        }

        Self::samples_to_wav(&samples, sample_rate)
    }

    /// Generate standard 16-bit 44.1kHz mono PCM WAV data matching the waveform pattern
    pub fn synthesize_wav(duration_secs: u32, waveform: &[u8]) -> Vec<u8> {
        let sample_rate = 44100u32;
        let dur = duration_secs.max(1).min(300);
        let total_samples = (sample_rate * dur) as usize;
        let mut pcm_samples: Vec<i16> = Vec::with_capacity(total_samples);

        let num_bars = waveform.len().max(1);
        let samples_per_bar = (total_samples / num_bars).max(1);

        // Vocal formant frequencies for a pleasant acoustic chime / voice note
        let base_freq = 220.0_f32; // A3
        let harmonic1 = 440.0_f32; // A4
        let harmonic2 = 659.25_f32; // E5

        for i in 0..total_samples {
            let bar_idx = (i / samples_per_bar).min(num_bars - 1);
            let amp_val = waveform.get(bar_idx).copied().unwrap_or(30) as f32 / 100.0;
            let t = i as f32 / sample_rate as f32;

            // Formant synthesis modulated by waveform amplitude
            let wave = (2.0 * std::f32::consts::PI * base_freq * t).sin() * 0.5
                + (2.0 * std::f32::consts::PI * harmonic1 * t).sin() * 0.3
                + (2.0 * std::f32::consts::PI * harmonic2 * t).sin() * 0.2;

            // Envelope smoothing
            let envelope = amp_val * 0.7;
            let sample_i16 = (wave * envelope * 16000.0).clamp(-32000.0, 32000.0) as i16;
            pcm_samples.push(sample_i16);
        }

        Self::samples_to_wav(&pcm_samples, sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_synthesis_and_decoder() {
        let wav = AudioPlayer::synthesize_wav(3, &[50; 48]);
        assert!(wav.len() > 44);
        let cursor = Cursor::new(wav);
        let decoder = Decoder::new(cursor);
        assert!(decoder.is_ok(), "Decoder must parse synthesized WAV");
    }
}
