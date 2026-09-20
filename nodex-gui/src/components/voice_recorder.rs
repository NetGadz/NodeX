use egui::{Color32, Sense, Ui, Vec2};
use nodex_messenger::voice::VoiceNote;
use crate::components::icons::draw_mic_icon;
use crate::theme::Theme;

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct VoiceRecorderState {
    pub is_recording: bool,
    pub start_time: Option<std::time::Instant>,
    pub live_amplitude: f32, // 0.0 .. 1.0
    pub captured_samples: Vec<f32>,
}

pub fn draw_voice_record_button(
    ui: &mut Ui,
    state: &mut VoiceRecorderState,
    mic: &mut crate::mic_recorder::MicRecorder,
    theme: &Theme,
    voice_note_out: &mut Option<VoiceNote>,
) {
    let btn_size = Vec2::splat(36.0);
    let (rect, resp) = ui.allocate_exact_size(btn_size, Sense::click());
    let painter = ui.painter_at(rect);

    if resp.clicked() {
        if state.is_recording {
            // Clicked while recording -> stop microphone & encode real voice WAV!
            state.is_recording = false;
            let duration = state.start_time.map_or(0.0, |t| t.elapsed().as_secs_f32());
            state.start_time = None;

            let (wav_bytes, waveform, duration_secs) = mic.stop();
            let final_dur = duration_secs.max(duration.ceil() as u32).max(1);

            use base64::Engine;
            let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);
            let vn = VoiceNote {
                duration_secs: final_dur,
                waveform,
                mime_type: "audio/wav".to_string(),
                audio_base64,
                file_size: wav_bytes.len(),
            };
            *voice_note_out = Some(vn);
        } else {
            // Clicked while not recording -> start real microphone capture!
            state.is_recording = true;
            state.start_time = Some(std::time::Instant::now());
            if let Err(e) = mic.start() {
                eprintln!("[MIC ERROR] Could not start microphone recording: {}", e);
            }
        }
    }

    if state.is_recording {
        let elapsed = state.start_time.map_or(0.0, |t| t.elapsed().as_secs_f32());
        
        // Pulsating red circle animation
        let pulse = (elapsed * 5.0).sin() * 0.5 + 0.5;
        let ring_radius = 18.0 + pulse * 6.0;
        painter.circle_filled(
            rect.center(),
            ring_radius,
            Color32::from_rgba_premultiplied(231, 76, 60, (70.0 * pulse) as u8),
        );
        painter.circle_filled(rect.center(), 17.0, Color32::from_rgb(231, 76, 60));
        draw_mic_icon(&painter, rect, Color32::WHITE);
    } else {
        let bg_color = if resp.hovered() { theme.bubble_other_bg } else { Color32::TRANSPARENT };
        if resp.hovered() {
            painter.circle_filled(rect.center(), 17.0, bg_color);
        }
        draw_mic_icon(&painter, rect, theme.text_muted);
    }
}
