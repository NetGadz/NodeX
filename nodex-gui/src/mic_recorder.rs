use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

pub struct MicRecorder {
    stream: Option<Stream>,
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    channels: u16,
    is_recording: bool,
}

impl MicRecorder {
    pub fn new() -> Self {
        Self {
            stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
            channels: 1,
            is_recording: false,
        }
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.stop();

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No default audio input device (microphone) found".to_string())?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        self.sample_rate = sample_rate;
        self.channels = channels;

        let samples_buf = Arc::new(Mutex::new(Vec::with_capacity((sample_rate * 5) as usize)));
        self.samples = Arc::clone(&samples_buf);

        let err_fn = |err| eprintln!("[MIC ERROR] An error occurred on audio input stream: {}", err);

        let stream = match config.sample_format() {
            SampleFormat::F32 => {
                let buf = Arc::clone(&samples_buf);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        let mut lock = buf.lock().unwrap();
                        for frame in data.chunks(channels as usize) {
                            let mono_sample = if frame.is_empty() {
                                0.0
                            } else {
                                frame.iter().sum::<f32>() / frame.len() as f32
                            };
                            let sample_i16 = (mono_sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                            lock.push(sample_i16);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I16 => {
                let buf = Arc::clone(&samples_buf);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &_| {
                        let mut lock = buf.lock().unwrap();
                        for frame in data.chunks(channels as usize) {
                            let mono_sample = if frame.is_empty() {
                                0
                            } else {
                                (frame.iter().map(|&s| s as i32).sum::<i32>() / frame.len() as i32) as i16
                            };
                            lock.push(mono_sample);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::U16 => {
                let buf = Arc::clone(&samples_buf);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &_| {
                        let mut lock = buf.lock().unwrap();
                        for frame in data.chunks(channels as usize) {
                            let mono_sample = if frame.is_empty() {
                                0
                            } else {
                                let sum: i32 = frame.iter().map(|&s| s as i32 - 32768).sum();
                                (sum / frame.len() as i32) as i16
                            };
                            lock.push(mono_sample);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err("Unsupported audio input sample format".to_string()),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start input stream: {}", e))?;

        self.stream = Some(stream);
        self.is_recording = true;
        println!("[MIC] Started recording from microphone (sample_rate: {}, channels: {})", sample_rate, channels);
        Ok(())
    }

    /// Stop recording and return (WAV bytes, 48-bar waveform, duration in seconds)
    pub fn stop(&mut self) -> (Vec<u8>, Vec<u8>, u32) {
        self.is_recording = false;
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }

        let raw_samples = {
            let mut lock = self.samples.lock().unwrap();
            std::mem::take(&mut *lock)
        };

        if raw_samples.is_empty() {
            return (Vec::new(), vec![20; 48], 0);
        }

        // Downsample to 16000 Hz mono for optimal voice quality and compact payload size
        let target_rate = 16000u32;
        let downsampled = if self.sample_rate > target_rate {
            let step = (self.sample_rate as f32 / target_rate as f32).max(1.0);
            let mut out = Vec::with_capacity((raw_samples.len() as f32 / step) as usize + 10);
            let mut idx = 0.0;
            while (idx as usize) < raw_samples.len() {
                out.push(raw_samples[idx as usize]);
                idx += step;
            }
            out
        } else {
            raw_samples
        };

        let duration_secs = ((downsampled.len() as f32 / target_rate as f32).ceil() as u32).max(1);

        // Generate waveform bars (48 samples normalized 0..100)
        let waveform = Self::generate_waveform_from_pcm(&downsampled, 48);

        // Convert PCM samples to valid standard WAV bytes
        let wav_bytes = Self::pcm_to_wav(&downsampled, target_rate);

        println!("[MIC] Stopped recording: {} samples ({} secs, {} WAV bytes)", downsampled.len(), duration_secs, wav_bytes.len());
        (wav_bytes, waveform, duration_secs)
    }

    /// Ensure microphone input is capturing (for active voice calls)
    pub fn ensure_started(&mut self) -> Result<(), String> {
        if !self.is_recording || self.stream.is_none() {
            self.start()
        } else {
            Ok(())
        }
    }

    /// Drain newly captured PCM audio samples since last call, downsampled to 16kHz mono with linear interpolation
    pub fn drain_call_samples(&self) -> Vec<i16> {
        let raw_samples = {
            let mut lock = self.samples.lock().unwrap();
            std::mem::take(&mut *lock)
        };

        if raw_samples.is_empty() {
            return Vec::new();
        }

        let target_rate = 16000u32;
        if self.sample_rate > target_rate {
            let ratio = self.sample_rate as f64 / target_rate as f64;
            let out_len = (raw_samples.len() as f64 / ratio).floor() as usize;
            let mut out = Vec::with_capacity(out_len);
            for i in 0..out_len {
                let src_pos = i as f64 * ratio;
                let idx = src_pos.floor() as usize;
                let next_idx = (idx + 1).min(raw_samples.len() - 1);
                let frac = (src_pos - idx as f64) as f32;

                let s0 = raw_samples[idx] as f32;
                let s1 = raw_samples[next_idx] as f32;
                let interp = s0 * (1.0 - frac) + s1 * frac;
                out.push(interp.clamp(-32768.0, 32767.0) as i16);
            }
            out
        } else {
            raw_samples
        }
    }

    fn generate_waveform_from_pcm(samples: &[i16], num_bars: usize) -> Vec<u8> {
        if samples.is_empty() {
            return vec![20; num_bars];
        }

        let chunk_size = (samples.len() / num_bars).max(1);
        let mut bars = Vec::with_capacity(num_bars);

        for chunk in samples.chunks(chunk_size) {
            if bars.len() >= num_bars {
                break;
            }
            let max_amp = chunk.iter().map(|&s| s.abs()).max().unwrap_or(0);
            let norm = ((max_amp as f32 / 32767.0) * 100.0).clamp(15.0, 100.0) as u8;
            bars.push(norm);
        }

        while bars.len() < num_bars {
            bars.push(18);
        }

        bars
    }

    fn pcm_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
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

        for &sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        wav
    }
}
