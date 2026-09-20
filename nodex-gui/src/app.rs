use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

use eframe::App;
use egui::{CentralPanel, SidePanel};

use crate::components::media_viewer::render_media_viewer;
use crate::state::{AppState, CallState};
use crate::theme::Theme;
use crate::views;
use crate::views::chat::ChatEventOut;
use nodex_messenger::db::{SavedChatMessage, SavedContact};
use nodex_messenger::groups::{GroupMember, GroupRole, P2PGroup};
use nodex_messenger::messenger::{KadMessenger, MessengerEvent};

pub struct NodeXApp {
    pub state: AppState,
    pub messenger: Option<Arc<KadMessenger>>,
    pub event_rx: Option<Receiver<MessengerEvent>>,
    pub contacts: Vec<SavedContact>,
    pub groups: Vec<P2PGroup>,
    pub messages: Vec<SavedChatMessage>,
    pub last_tick: Instant,
    #[allow(dead_code)]
    pub rt: tokio::runtime::Handle,
}

impl NodeXApp {
    pub fn new(
        state: AppState,
        messenger: Option<Arc<KadMessenger>>,
        event_rx: Option<Receiver<MessengerEvent>>,
        rt: tokio::runtime::Handle,
    ) -> Self {
        let (initial_contacts, initial_groups, initial_messages) = if let Some(msgr) = &messenger {
            let db_path = msgr.db_path.clone();
            if let Ok(db) = nodex_messenger::db::MessengerDb::load_from_file(&db_path) {
                (
                    db.contacts.into_values().collect(),
                    db.groups.into_values().collect(),
                    db.messages,
                )
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        Self {
            state,
            messenger,
            event_rx,
            contacts: initial_contacts,
            groups: initial_groups,
            messages: initial_messages,
            last_tick: Instant::now(),
            rt,
        }
    }

    fn handle_incoming_events(&mut self) {
        let events: Vec<_> = if let Some(rx) = &self.event_rx {
            let mut list = Vec::new();
            while let Ok(event) = rx.try_recv() {
                list.push(event);
            }
            list
        } else {
            Vec::new()
        };

        for event in events {
                match event {
                    MessengerEvent::ContactsUpdated(updated) => {
                        self.contacts = updated;
                    }
                    MessengerEvent::MessageReceived(saved_msg) => {
                        self.state.network_stats.messages_received += 1;
                        if self.state.notifications_enabled {
                            self.state.audio_player.play_notification_sound();
                        }
                        
                        let sender_id = saved_msg.sender_id_hex.clone();
                        let mut sender_name = format!("Peer {}", &sender_id.chars().take(8).collect::<String>());

                        if sender_id != self.state.my_node_id {
                            let mut exists = false;
                            for contact in &mut self.contacts {
                                if contact.user_id_hex == sender_id || contact.ed25519_pub_hex == sender_id {
                                    exists = true;
                                    sender_name = contact.name.clone();
                                    break;
                                }
                            }
                            if !exists {
                                let new_contact = nodex_messenger::db::SavedContact {
                                    user_id_hex: sender_id.clone(),
                                    name: sender_name.clone(),
                                    bio: "Direct Contact".to_string(),
                                    ed25519_pub_hex: sender_id.clone(),
                                    x25519_pub_hex: sender_id.clone(),
                                    last_seen_addr: String::new(),
                                };
                                self.contacts.insert(0, new_contact);

                                if let Some(msgr) = &self.messenger {
                                    let msgr_clone = msgr.clone();
                                    let peer = sender_id.clone();
                                    self.rt.spawn(async move {
                                        let _ = msgr_clone.discover_peer(&peer).await;
                                    });
                                }
                            }
                        }

                        if self.state.notifications_enabled {
                            self.state.toast_notification = Some(crate::state::ToastNotification {
                                sender_id: sender_id.clone(),
                                sender_name,
                                text: saved_msg.text.clone(),
                                created_at: std::time::Instant::now(),
                            });
                        }
                        
                        self.messages.push(saved_msg);
                    }
                    MessengerEvent::MessageDelivered { recipient_id: _, message_id } => {
                        self.state.network_stats.messages_delivered += 1;
                        if let Some(m) = self.messages.iter_mut().find(|m| m.id == message_id) {
                            m.delivered = true;
                        }
                    }
                    MessengerEvent::MessageEdited { message_id, new_text } => {
                        if let Some(m) = self.messages.iter_mut().find(|m| m.id == message_id) {
                            m.text = new_text;
                            m.is_edited = true;
                        }
                    }
                    MessengerEvent::ReactionAdded { message_id, emoji, reactor_id } => {
                        if let Some(m) = self.messages.iter_mut().find(|m| m.id == message_id) {
                            let entry = m.reactions.entry(emoji).or_default();
                            if !entry.contains(&reactor_id) {
                                entry.push(reactor_id);
                            }
                        }
                    }
                    MessengerEvent::CallSignalReceived(signal) => {
                        match signal.signal_type {
                            nodex_messenger::calls::CallSignalType::Offer => {
                                // If already in an active or ringing call, reject as busy
                                if let Some(existing_cs) = &self.state.call_state {
                                    if existing_cs.is_connected || !existing_cs.call_id.is_empty() {
                                        if let Some(msgr) = &self.messenger {
                                            let msgr_clone = msgr.clone();
                                            let target = signal.caller_id_hex.clone();
                                            let cid = signal.call_id.clone();
                                            self.rt.spawn(async move {
                                                let _ = msgr_clone.send_call_signal(&target, nodex_messenger::calls::CallSignalType::Reject, &cid).await;
                                            });
                                        }
                                        continue;
                                    }
                                }

                                self.state.audio_player.play_ringtone_loop();
                                let peer_name = self.contacts.iter()
                                    .find(|c| c.user_id_hex == signal.caller_id_hex || c.ed25519_pub_hex == signal.caller_id_hex)
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| format!("Peer {}", &signal.caller_id_hex.chars().take(8).collect::<String>()));

                                self.state.call_state = Some(CallState {
                                    call_id: signal.call_id.clone(),
                                    peer_id: signal.caller_id_hex.clone(),
                                    peer_name,
                                    is_incoming: true,
                                    is_connected: false,
                                    is_muted: false,
                                    duration_secs: 0.0,
                                    last_audio_send: std::time::Instant::now(),
                                    audio_seq: 0,
                                });
                            }
                            nodex_messenger::calls::CallSignalType::Answer => {
                                self.state.audio_player.start_call_audio();
                                if let Some(cs) = &mut self.state.call_state {
                                    cs.is_connected = true;
                                    let _ = self.state.mic_recorder.ensure_started();
                                    println!("[CALL] Call answered and connected with {}", cs.peer_id);
                                }
                            }
                            nodex_messenger::calls::CallSignalType::Reject => {
                                self.state.audio_player.stop();
                                self.state.audio_player.stop_call_audio();
                                self.state.audio_player.play_hangup_sound();
                                self.state.mic_recorder.stop();
                                if let Some(cs) = self.state.call_state.take() {
                                    let peer = if !cs.peer_id.is_empty() { cs.peer_id } else { signal.caller_id_hex.clone() };
                                    self.log_call_message(&peer, "🚫 Отклоненный звонок", false);
                                    println!("[CALL] Peer declined call, modal dismissed");
                                }
                            }
                            nodex_messenger::calls::CallSignalType::Hangup => {
                                self.state.audio_player.stop();
                                self.state.audio_player.stop_call_audio();
                                self.state.audio_player.play_hangup_sound();
                                self.state.mic_recorder.stop();
                                if let Some(cs) = self.state.call_state.take() {
                                    let peer = if !cs.peer_id.is_empty() { cs.peer_id } else { signal.caller_id_hex.clone() };
                                    if cs.is_connected {
                                        let mins = (cs.duration_secs as u64) / 60;
                                        let secs = (cs.duration_secs as u64) % 60;
                                        let label = if cs.is_incoming { "📞 Входящий звонок" } else { "📞 Исходящий звонок" };
                                        self.log_call_message(&peer, &format!("{} ({}:{:02})", label, mins, secs), cs.is_incoming);
                                    } else {
                                        let label = if cs.is_incoming { "📵 Пропущенный звонок" } else { "📵 Отмененный звонок" };
                                        self.log_call_message(&peer, label, cs.is_incoming);
                                    }
                                    println!("[CALL] Peer hung up, modal dismissed");
                                }
                            }
                            _ => {}
                        }
                    }
                    MessengerEvent::CallAudioReceived { sender_id: _, chunk } => {
                        if let Some(cs) = &self.state.call_state {
                            if cs.is_connected {
                                use base64::Engine;
                                if let Ok(pcm_bytes) = base64::engine::general_purpose::STANDARD.decode(&chunk.pcm_base64) {
                                    let mut samples = Vec::with_capacity(pcm_bytes.len() / 2);
                                    for frame in pcm_bytes.chunks_exact(2) {
                                        let sample = i16::from_le_bytes([frame[0], frame[1]]);
                                        samples.push(sample);
                                    }
                                    self.state.audio_player.play_call_audio_samples(&samples, chunk.sample_rate);
                                }
                            }
                        }
                    }
                    MessengerEvent::MessageDeleted { contact_id: _, message_ids } => {
                        self.messages.retain(|m| !message_ids.contains(&m.id));
                    }
                }
            }
    }

    fn log_call_message(&mut self, peer_id: &str, text: &str, incoming: bool) {
        let msg = nodex_messenger::db::SavedChatMessage {
            id: format!("call_log_{}_{:x}", chrono::Utc::now().timestamp_micros(), rand::random::<u32>()),
            sender_id_hex: if incoming { peer_id.to_string() } else { self.state.my_node_id.clone() },
            recipient_id_hex: if incoming { self.state.my_node_id.clone() } else { peer_id.to_string() },
            text: text.to_string(),
            image_base64: None,
            voice_note: None,
            reply_to_id: None,
            reply_snippet: None,
            is_edited: false,
            edit_timestamp: None,
            reactions: std::collections::HashMap::new(),
            group_id: None,
            is_pinned: false,
            expires_at: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
            incoming,
            delivered: true,
        };
        self.messages.push(msg.clone());
        if let Some(msgr) = &self.messenger {
            let msgr_clone = msgr.clone();
            self.rt.spawn(async move {
                let mut db = msgr_clone.db.write().await;
                db.add_message(msg);
                db.save_to_file(&msgr_clone.db_path).ok();
            });
        }
    }
}

impl App for NodeXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        // Advance active call timer & stream microphone audio
        if let Some(cs) = &mut self.state.call_state {
            if cs.is_connected {
                cs.duration_secs += dt;
                if let Err(e) = self.state.mic_recorder.ensure_started() {
                    eprintln!("[MIC ERROR] Failed to start microphone: {}", e);
                }

                if cs.last_audio_send.elapsed().as_millis() >= 40 {
                    cs.last_audio_send = std::time::Instant::now();
                    let samples = self.state.mic_recorder.drain_call_samples();
                    if !cs.is_muted && !samples.is_empty() {
                        cs.audio_seq += 1;
                        let seq = cs.audio_seq;
                        let call_id = cs.call_id.clone();
                        let target = cs.peer_id.clone();
                        if let Some(msgr) = &self.messenger {
                            let msgr_clone = msgr.clone();
                            self.rt.spawn(async move {
                                let _ = msgr_clone.send_call_audio(&target, &call_id, &samples, 16000, seq).await;
                            });
                        }
                    }
                }
            }
        }

        // Handle async backend events
        self.handle_incoming_events();

        // Sync audio playback lifecycle
        if let Some((msg_id, vn)) = self.state.voice_playback.requested_play.take() {
            self.state.audio_player.play_voice_note(&msg_id, &vn);
        }
        if self.state.voice_playback.requested_stop {
            self.state.voice_playback.requested_stop = false;
            self.state.audio_player.stop();
        }

        let (playing_id, progress) = self.state.audio_player.update_progress();
        self.state.voice_playback.playing_msg_id = playing_id;
        self.state.voice_playback.current_progress = progress;

        // Continuous redraw while audio is playing, recording voice message, or active call
        let in_active_call = self.state.call_state.as_ref().map_or(false, |c| c.is_connected);
        if self.state.voice_playback.playing_msg_id.is_some() || self.state.voice_recorder.is_recording || in_active_call {
            ctx.request_repaint();
        }

        let theme = Theme::from_mode(self.state.theme_mode);
        theme.apply_to_ctx(ctx);

        // Keyboard shortcut Ctrl+L to lock app
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::L)) {
            if self.state.passcode_hash.is_some() {
                self.state.is_app_locked = true;
            }
        }

        // Auto-lock timer enforcement
        if self.state.auto_lock_mins > 0 && !self.state.is_app_locked && self.state.passcode_hash.is_some() {
            if self.state.last_active.elapsed().as_secs() > (self.state.auto_lock_mins as u64 * 60) {
                self.state.is_app_locked = true;
            }
        }
        if ctx.input(|i| i.pointer.any_pressed() || i.pointer.is_moving() || !i.events.is_empty()) {
            self.state.last_active = Instant::now();
        }

        if self.state.is_app_locked {
            CentralPanel::default().show(ctx, |ui| {
                views::lock_screen::render_lock_screen(ui, &mut self.state, &theme);
            });
        } else if !self.state.is_onboarded {
            CentralPanel::default().show(ctx, |ui| {
                views::onboarding::render_onboarding(ui, &mut self.state, &theme);
            });
        } else {
            // Check Esc key to toggle off Zen Mode
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.state.zen_mode {
                self.state.zen_mode = false;
            }

            if !self.state.zen_mode {
                // 1. Far-left navigation icon rail
                SidePanel::left("nav_rail")
                    .resizable(false)
                    .exact_width(60.0)
                    .frame(egui::Frame::none().fill(theme.nav_bg))
                    .show(ctx, |ui| {
                        views::nav_rail::render_nav_rail(ui, &mut self.state, &theme);
                    });

                // 2. Chat sidebar list
                SidePanel::left("sidebar_panel")
                    .resizable(true)
                    .default_width(310.0)
                    .min_width(240.0)
                    .max_width(450.0)
                    .frame(egui::Frame::none().fill(theme.sidebar_bg))
                    .show(ctx, |ui| {
                        let mut sidebar_action_out = None;
                        views::sidebar::render_sidebar(
                            ui,
                            &mut self.state,
                            &theme,
                            &self.contacts,
                            &self.groups,
                            &self.messages,
                            &mut sidebar_action_out,
                        );

                        if let Some(action) = sidebar_action_out {
                            match action {
                                views::sidebar::SidebarAction::ClearHistory(contact_id) => {
                                    self.messages.retain(|m| m.sender_id_hex != contact_id && m.recipient_id_hex != contact_id);
                                    if let Some(msgr) = &self.messenger {
                                        let msgr_clone = msgr.clone();
                                        let cid = contact_id.clone();
                                        self.rt.spawn(async move {
                                            msgr_clone.clear_chat(&cid).await;
                                        });
                                    }
                                }
                                views::sidebar::SidebarAction::DeleteChat(contact_id) => {
                                    self.contacts.retain(|c| c.user_id_hex != contact_id);
                                    self.messages.retain(|m| m.sender_id_hex != contact_id && m.recipient_id_hex != contact_id);
                                    if self.state.selected_chat_id.as_deref() == Some(&contact_id) {
                                        self.state.selected_chat_id = None;
                                    }
                                    if let Some(msgr) = &self.messenger {
                                        let msgr_clone = msgr.clone();
                                        let cid = contact_id.clone();
                                        self.rt.spawn(async move {
                                            msgr_clone.delete_contact(&cid).await;
                                        });
                                    }
                                }
                                views::sidebar::SidebarAction::DeleteGroup(group_id) => {
                                    self.groups.retain(|g| g.group_id != group_id);
                                    self.messages.retain(|m| m.group_id.as_deref() != Some(&group_id) && m.recipient_id_hex != group_id);
                                    if self.state.selected_group_id.as_deref() == Some(&group_id) {
                                        self.state.selected_group_id = None;
                                    }
                                }
                            }
                        }
                    });
            }

            let is_saved_messages = self.state.selected_chat_id.as_deref() == Some("self_saved_messages");

            let active_contact = if !is_saved_messages {
                self.contacts
                    .iter()
                    .find(|c| self.state.selected_chat_id.as_deref() == Some(&c.user_id_hex))
            } else {
                None
            };

            let active_group = if !is_saved_messages {
                self.groups
                    .iter()
                    .find(|g| self.state.selected_group_id.as_deref() == Some(&g.group_id))
            } else {
                None
            };

            let active_target_id = if is_saved_messages {
                Some("self_saved_messages")
            } else {
                active_contact
                    .map(|c| c.user_id_hex.as_str())
                    .or_else(|| active_group.map(|g| g.group_id.as_str()))
            };

            let active_messages: Vec<SavedChatMessage> = if is_saved_messages {
                self.messages
                    .iter()
                    .filter(|m| m.recipient_id_hex == "self_saved_messages" || (m.sender_id_hex == self.state.my_node_id && m.recipient_id_hex == self.state.my_node_id))
                    .cloned()
                    .collect()
            } else if let Some(target_id) = active_target_id {
                self.messages
                    .iter()
                    .filter(|m| m.sender_id_hex == target_id || m.recipient_id_hex == target_id)
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };

            // 3. Right Drawer (User/Group Info & Shared Media)
            if self.state.show_info_drawer {
                SidePanel::right("info_drawer_panel")
                    .resizable(true)
                    .default_width(280.0)
                    .min_width(240.0)
                    .max_width(400.0)
                    .frame(egui::Frame::none().fill(theme.sidebar_bg))
                    .show(ctx, |ui| {
                        views::right_drawer::render_right_drawer(
                            ui,
                            &mut self.state,
                            &theme,
                            active_contact,
                            active_group,
                            &active_messages,
                        );
                    });
            }

            // 4. Central Chat / Stego Studio area
            CentralPanel::default()
                .frame(egui::Frame::none().fill(theme.window_bg))
                .show(ctx, |ui| {
                    if self.state.active_nav_tab == crate::state::ActiveNavTab::StegoStudio {
                        if let Some(msgr) = &self.messenger {
                            views::stego_studio::render_stego_studio(ui, &mut self.state, &theme, msgr);
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(egui::RichText::new("Connecting to P2P Node...").color(theme.text_muted));
                            });
                        }
                        return;
                    }

                    let mut chat_event_out: Option<ChatEventOut> = None;

                    views::chat::render_chat_view(
                        ui,
                        &mut self.state,
                        &theme,
                        active_contact,
                        active_group,
                        &active_messages,
                        &mut chat_event_out,
                    );

                    if let Some(event) = chat_event_out {
                        match event {
                            ChatEventOut::SendMessage { text, reply_to_id } => {
                                if let Some(target_id) = active_target_id {
                                    self.state.audio_player.play_sent_sound();
                                    let new_msg = SavedChatMessage {
                                        id: format!("msg_{}", hex::encode(&rand_bytes())),
                                        sender_id_hex: self.state.my_node_id.clone(),
                                        recipient_id_hex: target_id.to_string(),
                                        text: text.clone(),
                                        image_base64: None,
                                        voice_note: None,
                                        reply_to_id,
                                        reply_snippet: None,
                                        is_edited: false,
                                        edit_timestamp: None,
                                        reactions: std::collections::HashMap::new(),
                                        group_id: active_group.map(|g| g.group_id.clone()),
                                        is_pinned: false,
                                        expires_at: None,
                                        timestamp: chrono::Utc::now().timestamp() as u64,
                                        incoming: false,
                                        delivered: is_saved_messages,
                                    };
                                    self.messages.push(new_msg);
                                    self.state.network_stats.messages_sent += 1;

                                    if !is_saved_messages {
                                        if let Some(msgr) = &self.messenger {
                                            let msgr_clone = msgr.clone();
                                            let target = target_id.to_string();
                                            self.rt.spawn(async move {
                                                let _ = msgr_clone.send_message(&target, &text, None).await;
                                            });
                                        }
                                    }
                                }
                            }
                            ChatEventOut::SendVoiceNote { voice_note } => {
                                if let Some(target_id) = active_target_id {
                                    self.state.audio_player.play_sent_sound();
                                    let new_msg = SavedChatMessage {
                                        id: format!("msg_{}", hex::encode(&rand_bytes())),
                                        sender_id_hex: self.state.my_node_id.clone(),
                                        recipient_id_hex: target_id.to_string(),
                                        text: format!("🎙 Voice message ({})", voice_note.formatted_duration()),
                                        image_base64: None,
                                        voice_note: Some(voice_note.clone()),
                                        reply_to_id: None,
                                        reply_snippet: None,
                                        is_edited: false,
                                        edit_timestamp: None,
                                        reactions: std::collections::HashMap::new(),
                                        group_id: active_group.map(|g| g.group_id.clone()),
                                        is_pinned: false,
                                        expires_at: None,
                                        timestamp: chrono::Utc::now().timestamp() as u64,
                                        incoming: false,
                                        delivered: is_saved_messages,
                                    };
                                    self.messages.push(new_msg);
                                    self.state.network_stats.messages_sent += 1;

                                    if !is_saved_messages {
                                        if let Some(msgr) = &self.messenger {
                                            let msgr_clone = msgr.clone();
                                            let target = target_id.to_string();
                                            self.rt.spawn(async move {
                                                let _ = msgr_clone.send_voice_note(&target, voice_note).await;
                                            });
                                        }
                                    }
                                }
                            }
                            ChatEventOut::SendFile { filepath } => {
                                if let Some(target_id) = active_target_id {
                                    if let Ok(data) = std::fs::read(&filepath) {
                                        self.state.audio_player.play_sent_sound();
                                        let filename = std::path::Path::new(&filepath)
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "file.bin".to_string());

                                        let text_payload = format!("[File: {}]", filename);
                                        let new_msg = SavedChatMessage {
                                            id: format!("msg_{}", hex::encode(&rand_bytes())),
                                            sender_id_hex: self.state.my_node_id.clone(),
                                            recipient_id_hex: target_id.to_string(),
                                            text: text_payload.clone(),
                                            image_base64: Some(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data)),
                                            voice_note: None,
                                            reply_to_id: None,
                                            reply_snippet: None,
                                            is_edited: false,
                                            edit_timestamp: None,
                                            reactions: std::collections::HashMap::new(),
                                            group_id: active_group.map(|g| g.group_id.clone()),
                                            is_pinned: false,
                                            expires_at: None,
                                            timestamp: chrono::Utc::now().timestamp() as u64,
                                            incoming: false,
                                            delivered: is_saved_messages,
                                        };
                                        self.messages.push(new_msg);
                                        self.state.network_stats.messages_sent += 1;

                                        if !is_saved_messages {
                                            if let Some(msgr) = &self.messenger {
                                                let msgr_clone = msgr.clone();
                                                let target = target_id.to_string();
                                                self.rt.spawn(async move {
                                                    let _ = msgr_clone.send_message(&target, &text_payload, None).await;
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            ChatEventOut::ReactMessage { msg_id, emoji } => {
                                if let Some(m) = self.messages.iter_mut().find(|m| m.id == msg_id) {
                                    let my_id = self.state.my_node_id.clone();
                                    let already_had_this = m.reactions.get(&emoji).map_or(false, |r| r.contains(&my_id));

                                    for (_e, reactors) in m.reactions.iter_mut() {
                                        reactors.retain(|r| r != &my_id);
                                    }
                                    m.reactions.retain(|_, reactors| !reactors.is_empty());

                                    if !already_had_this {
                                        m.reactions.entry(emoji.clone()).or_default().push(my_id);
                                    }
                                }
                                if let Some(target_id) = active_target_id {
                                    if !is_saved_messages {
                                        if let Some(msgr) = &self.messenger {
                                            let msgr_clone = msgr.clone();
                                            let target = target_id.to_string();
                                            self.rt.spawn(async move {
                                                let _ = msgr_clone.react_to_message(&target, &msg_id, &emoji).await;
                                            });
                                        }
                                    }
                                }
                            }
                            ChatEventOut::EditMessage { msg_id, new_text } => {
                                if let Some(m) = self.messages.iter_mut().find(|m| m.id == msg_id) {
                                    m.text = new_text.clone();
                                    m.is_edited = true;
                                }
                                if let Some(target_id) = active_target_id {
                                    if !is_saved_messages {
                                        if let Some(msgr) = &self.messenger {
                                            let msgr_clone = msgr.clone();
                                            let target = target_id.to_string();
                                            self.rt.spawn(async move {
                                                let _ = msgr_clone.edit_message(&target, &msg_id, &new_text).await;
                                            });
                                        }
                                    }
                                }
                            }
                            ChatEventOut::PinMessage { msg_id } => {
                                if let Some(m) = self.messages.iter_mut().find(|m| m.id == msg_id) {
                                    m.is_pinned = !m.is_pinned;
                                }
                                if let Some(msgr) = &self.messenger {
                                    let msgr_clone = msgr.clone();
                                    self.rt.spawn(async move {
                                        let _ = msgr_clone.pin_message(&msg_id).await;
                                    });
                                }
                            }
                            ChatEventOut::DeleteMessage { msg_id } => {
                                let del_id = msg_id.clone();
                                self.messages.retain(|m| m.id != del_id);
                                if let Some(msgr) = &self.messenger {
                                    let msgr_clone = msgr.clone();
                                    self.rt.spawn(async move {
                                        msgr_clone.delete_message(&del_id).await;
                                    });
                                }
                            }
                            ChatEventOut::DeleteEveryone { msg_id } => {
                                let del_id = msg_id.clone();
                                self.messages.retain(|m| m.id != del_id);
                                if let Some(msgr) = &self.messenger {
                                    let msgr_clone = msgr.clone();
                                    let del_id_local = del_id.clone();
                                    self.rt.spawn(async move {
                                        msgr_clone.delete_message(&del_id_local).await;
                                    });
                                    if let Some(target_id) = active_target_id {
                                        if !is_saved_messages {
                                            let target = target_id.to_string();
                                            let ids = vec![del_id];
                                            let msgr_remote = msgr.clone();
                                            self.rt.spawn(async move {
                                                let _ = msgr_remote.delete_messages_for_everyone(&target, ids).await;
                                            });
                                        }
                                    }
                                }
                            }
                            ChatEventOut::StartCall { peer_id } => {
                                self.state.audio_player.play_dial_tone_loop();
                                let peer_name = self.contacts.iter()
                                    .find(|c| c.user_id_hex == peer_id || c.ed25519_pub_hex == peer_id)
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| format!("Peer {}", &peer_id.chars().take(8).collect::<String>()));

                                let call_id = format!("{}_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0), &peer_id.chars().take(8).collect::<String>());

                                self.state.call_state = Some(CallState {
                                    call_id: call_id.clone(),
                                    peer_id: peer_id.clone(),
                                    peer_name,
                                    is_incoming: false,
                                    is_connected: false,
                                    is_muted: false,
                                    duration_secs: 0.0,
                                    last_audio_send: std::time::Instant::now(),
                                    audio_seq: 0,
                                });
                                
                                if let Some(msgr) = &self.messenger {
                                    let msgr_clone = msgr.clone();
                                    let target = peer_id.clone();
                                    self.rt.spawn(async move {
                                        let _ = msgr_clone.send_call_signal(&target, nodex_messenger::calls::CallSignalType::Offer, &call_id).await;
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                });
        }

        // Render Modals & Popups
        let mut new_contact_out = None;
        if let Some(msgr) = &self.messenger {
            views::add_contact::render_add_contact_modal(ctx, &mut self.state, &theme, &mut new_contact_out, msgr);
        }
        if let Some((id, alias)) = new_contact_out {
            let contact = SavedContact {
                user_id_hex: id.clone(),
                name: alias.clone(),
                bio: String::new(),
                ed25519_pub_hex: id.clone(),
                x25519_pub_hex: id.clone(),
                last_seen_addr: String::new(),
            };
            self.contacts.retain(|c| c.user_id_hex != id);
            self.contacts.insert(0, contact.clone());
            self.state.selected_chat_id = Some(id.clone());
            self.state.selected_group_id = None;

            if let Some(msgr) = &self.messenger {
                let msgr_clone = msgr.clone();
                let peer_id = id.clone();
                let custom_alias = alias.clone();
                self.rt.spawn(async move {
                    if let Ok(card) = msgr_clone.discover_peer(&peer_id).await {
                        println!("[ADD_CONTACT] Discovered peer {} in DHT: {:?}", peer_id, card.display_name);
                    } else {
                        let mut db = msgr_clone.db.write().await;
                        db.add_contact(SavedContact {
                            user_id_hex: peer_id.clone(),
                            name: custom_alias,
                            bio: String::new(),
                            ed25519_pub_hex: peer_id.clone(),
                            x25519_pub_hex: peer_id.clone(),
                            last_seen_addr: String::new(),
                        });
                        db.save_to_file(&msgr_clone.db_path).ok();
                    }
                });
            }
        }

        let mut create_group_out = None;
        views::create_group::render_create_group_modal(ctx, &mut self.state, &theme, &self.contacts, &mut create_group_out);
        if let Some((title, member_ids)) = create_group_out {
            let mut group = P2PGroup::new(&title, "", &self.state.my_node_id, &self.state.my_name);
            for id in member_ids {
                if let Some(c) = self.contacts.iter().find(|c| c.user_id_hex == id) {
                    group.members.insert(
                        c.user_id_hex.clone(),
                        GroupMember {
                            user_id_hex: c.user_id_hex.clone(),
                            display_name: c.name.clone(),
                            role: GroupRole::Member,
                            joined_at: chrono::Utc::now().timestamp() as u64,
                        },
                    );
                }
            }

            let gid = group.group_id.clone();
            self.groups.insert(0, group);
            self.state.selected_group_id = Some(gid);
            self.state.selected_chat_id = None;
        }

        views::profile_modal::render_profile_modal(ctx, &mut self.state, &theme);
        views::network_modal::render_network_modal(ctx, &mut self.state, &theme);
        if let Some(event_out) = views::call_modal::render_call_modal(ctx, &mut self.state, &theme) {
            match event_out {
                crate::views::chat::ChatEventOut::AcceptCall { peer_id } => {
                    self.state.audio_player.start_call_audio();
                    let _ = self.state.mic_recorder.ensure_started();
                    let call_id = self.state.call_state.as_ref().map(|cs| cs.call_id.clone()).unwrap_or_else(|| {
                        format!("{}_{}", chrono::Utc::now().timestamp(), &peer_id.chars().take(8).collect::<String>())
                    });
                    if let Some(cs) = &mut self.state.call_state {
                        cs.is_connected = true;
                    }
                    if let Some(msgr) = &self.messenger {
                        let msgr_clone = msgr.clone();
                        let target = peer_id.clone();
                        self.rt.spawn(async move {
                            let _ = msgr_clone.send_call_signal(&target, nodex_messenger::calls::CallSignalType::Answer, &call_id).await;
                        });
                    }
                }
                crate::views::chat::ChatEventOut::DeclineCall { peer_id } => {
                    self.state.audio_player.stop();
                    self.state.audio_player.stop_call_audio();
                    self.state.audio_player.play_hangup_sound();
                    self.state.mic_recorder.stop();
                    let call_id = self.state.call_state.as_ref().map(|cs| cs.call_id.clone()).unwrap_or_default();
                    self.state.call_state = None;
                    self.log_call_message(&peer_id, "📵 Пропущенный звонок", true);
                    if let Some(msgr) = &self.messenger {
                        let msgr_clone = msgr.clone();
                        let target = peer_id.clone();
                        self.rt.spawn(async move {
                            let _ = msgr_clone.send_call_signal(&target, nodex_messenger::calls::CallSignalType::Reject, &call_id).await;
                        });
                    }
                }
                crate::views::chat::ChatEventOut::EndCall { peer_id } => {
                    self.state.audio_player.stop();
                    self.state.audio_player.stop_call_audio();
                    self.state.audio_player.play_hangup_sound();
                    self.state.mic_recorder.stop();
                    let call_id = self.state.call_state.as_ref().map(|cs| cs.call_id.clone()).unwrap_or_default();
                    if let Some(cs) = self.state.call_state.take() {
                        if cs.is_connected {
                            let mins = (cs.duration_secs as u64) / 60;
                            let secs = (cs.duration_secs as u64) % 60;
                            let label = if cs.is_incoming { "📞 Входящий звонок" } else { "📞 Исходящий звонок" };
                            self.log_call_message(&peer_id, &format!("{} ({}:{:02})", label, mins, secs), cs.is_incoming);
                        } else {
                            let label = if cs.is_incoming { "📵 Пропущенный звонок" } else { "📵 Отмененный звонок" };
                            self.log_call_message(&peer_id, label, cs.is_incoming);
                        }
                    }
                    if let Some(msgr) = &self.messenger {
                        let msgr_clone = msgr.clone();
                        let target = peer_id.clone();
                        self.rt.spawn(async move {
                            let _ = msgr_clone.send_call_signal(&target, nodex_messenger::calls::CallSignalType::Hangup, &call_id).await;
                        });
                    }
                }
                _ => {}
            }
        }
        if let Some(msgr) = &self.messenger {
            views::settings_modal::render_settings_modal(ctx, &mut self.state, &theme, msgr);
        }
        render_media_viewer(ctx, &mut self.state.modals.media_viewer, &theme);

        // Render In-App Floating Toast Notification
        if let Some(toast) = self.state.toast_notification.clone() {
            if toast.created_at.elapsed().as_secs() > 4 {
                self.state.toast_notification = None;
            } else {
                egui::Area::new(egui::Id::new("toast_notification_area"))
                    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-20.0, 20.0))
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(theme.card_bg)
                            .stroke(egui::Stroke::new(1.0_f32, theme.accent))
                            .rounding(12.0)
                            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("💬").size(16.0));
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&toast.sender_name).strong().size(12.0).color(theme.accent));
                                        let snippet = crate::theme::truncate_str(&toast.text, 40);
                                        ui.label(egui::RichText::new(snippet).size(12.0).color(theme.text_primary));
                                    });
                                    if ui.button(egui::RichText::new("Open").size(11.0).strong().color(egui::Color32::WHITE))
                                        .clicked()
                                    {
                                        self.state.selected_chat_id = Some(toast.sender_id.clone());
                                        self.state.selected_group_id = None;
                                        self.state.toast_notification = None;
                                    }
                                });
                            });
                    });
            }
        }

        // Continuous redraw for waveforms / active timers / smooth animations
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

fn rand_bytes() -> [u8; 8] {
    let mut b = [0u8; 8];
    let _ = getrandom::getrandom(&mut b);
    b
}
