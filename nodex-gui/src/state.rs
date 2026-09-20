use std::collections::HashSet;
use nodex_messenger::db::SavedChatMessage;
use crate::components::media_viewer::MediaViewerModalState;
use crate::components::voice_player::VoicePlaybackState;
use crate::components::voice_recorder::VoiceRecorderState;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveNavTab {
    Chats,
    Groups,
    SavedMessages,
    StegoStudio,
    OnionMesh,
    Calls,
    Network,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderFilter {
    All,
    Direct,
    Groups,
    Unread,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Midnight,
    Day,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedMediaTab {
    Media,
    Files,
    Voice,
    Links,
}

#[derive(Clone, Debug, Default)]
pub struct AppModalState {
    pub show_add_contact: bool,
    pub show_create_group: bool,
    pub show_profile: bool,
    pub show_network: bool,
    pub show_settings: bool,
    pub media_viewer: MediaViewerModalState,
}

#[derive(Clone, Debug)]
pub struct CallState {
    pub call_id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub is_incoming: bool,
    pub is_connected: bool,
    pub is_muted: bool,
    pub duration_secs: f32,
    pub last_audio_send: std::time::Instant,
    pub audio_seq: u64,
}

#[derive(Clone, Debug)]
pub struct NetworkStats {
    pub connected_peers: usize,
    pub known_nodes: usize,
    pub nat_type: String,
    pub upnp_active: bool,
    pub relay_active: bool,
    pub messages_sent: usize,
    pub messages_delivered: usize,
    pub messages_received: usize,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self {
            connected_peers: 14,
            known_nodes: 52,
            nat_type: "Full Cone (P2P Direct)".to_string(),
            upnp_active: true,
            relay_active: true,
            messages_sent: 0,
            messages_delivered: 0,
            messages_received: 0,
        }
    }
}

#[allow(dead_code)]
pub struct AppState {
    pub my_node_id: String,
    pub my_name: String,
    pub my_bio: String,
    pub mnemonic_seed: String,
    pub is_onboarded: bool,
    pub onboarding_step: u8,
    pub theme_mode: ThemeMode,
    pub ui_scale: f32,

    pub active_nav_tab: ActiveNavTab,
    pub folder_filter: FolderFilter,
    pub selected_chat_id: Option<String>,
    pub selected_group_id: Option<String>,
    pub search_query: String,
    pub message_input: String,
    pub saved_messages_tag: Option<String>,
    
    // Reply and Edit state
    pub replying_to: Option<SavedChatMessage>,
    pub editing_msg: Option<SavedChatMessage>,

    // Media & Voice state
    pub voice_playback: VoicePlaybackState,
    pub voice_recorder: VoiceRecorderState,
    pub mic_recorder: crate::mic_recorder::MicRecorder,

    // Right drawer
    pub show_info_drawer: bool,
    pub shared_media_tab: SharedMediaTab,

    // Dialog inputs
    pub new_contact_id: String,
    pub new_contact_alias: String,
    pub new_group_title: String,
    pub selected_group_members: HashSet<String>,
    pub file_downloads: std::collections::HashMap<String, f32>,
    pub port: u16,

    // Modals
    pub modals: AppModalState,
    pub call_state: Option<CallState>,
    pub network_stats: NetworkStats,
    pub audio_player: crate::audio_player::AudioPlayer,

    // Mnemonic configuration & validation
    pub mnemonic_length: usize, // 12 or 24
    pub seed_confirmed_saved: bool,
    pub restore_input: String,
    pub restore_error: Option<String>,

    // Passcode App Lock (Telegram Style)
    pub passcode_hash: Option<String>,
    pub is_app_locked: bool,
    pub auto_lock_mins: u32,
    pub last_active: std::time::Instant,
    pub passcode_input: String,
    pub passcode_error: Option<String>,
    pub show_passcode_setup: bool,

    // Stego & 2-Hop Onion Routing (Privacy Shield)
    pub onion_routing_enabled: bool,
    pub embed_node_identity: bool,
    pub stego_passphrase: String,
    pub stego_status_msg: Option<String>,
    pub stego_export_image_path: Option<String>,
    pub stego_exported_bytes: Option<Vec<u8>>,
    pub mesh_routes_count: usize,
    pub passcode_new_pin: String,
    pub passcode_confirm_pin: String,

    // Seed Reveal & Passcode Protection in Settings
    pub reveal_seed_in_settings: bool,
    pub seed_verify_passcode: String,
    pub seed_passcode_error: Option<String>,
    pub show_seed_passcode_prompt: bool,

    // Advanced Preferences & Zen Mode
    pub auto_download_media: bool,
    pub auto_download_voice: bool,
    pub auto_download_files: bool,
    pub notifications_enabled: bool,
    pub p2p_relay_enabled: bool,
    pub zen_mode: bool,
    pub mood_status: String,
    pub toast_notification: Option<ToastNotification>,

    // Additional Settings & Customization
    pub chat_font_size: f32,
    pub send_on_enter: bool,
    pub compact_bubbles: bool,
    pub session_pair_token: String,
    pub ringtone_enabled: bool,
    pub sound_volume: f32,
    pub auto_delete_secs: u64,
    pub voice_modulator: String,
    pub duress_pin_hash: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToastNotification {
    pub sender_id: String,
    pub sender_name: String,
    pub text: String,
    pub created_at: std::time::Instant,
}

impl AppState {
    pub fn new(display_name: String, is_onboarded: bool, port: u16) -> Self {
        let initial_mnemonic = nodex_messenger::mnemonic::MnemonicManager::generate_24_words()
            .unwrap_or_else(|_| "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art".to_string());
        
        let initial_id = nodex_messenger::crypto::UserIdentity::from_mnemonic(&initial_mnemonic)
            .map(|id| id.user_id_hex())
            .unwrap_or_else(|_| "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678".to_string());

        Self {
            my_node_id: initial_id,
            my_name: display_name,
            my_bio: "Decentralized P2P User".to_string(),
            mnemonic_seed: initial_mnemonic,
            is_onboarded,
            onboarding_step: 0,
            theme_mode: ThemeMode::Dark,
            ui_scale: 1.0,
            active_nav_tab: ActiveNavTab::Chats,
            folder_filter: FolderFilter::All,
            selected_chat_id: Some("self_saved_messages".to_string()),
            selected_group_id: None,
            search_query: String::new(),
            message_input: String::new(),
            saved_messages_tag: None,
            replying_to: None,
            editing_msg: None,
            voice_playback: VoicePlaybackState::default(),
            voice_recorder: VoiceRecorderState::default(),
            mic_recorder: crate::mic_recorder::MicRecorder::new(),
            show_info_drawer: false,
            shared_media_tab: SharedMediaTab::Media,
            new_contact_id: String::new(),
            new_contact_alias: String::new(),
            new_group_title: String::new(),
            selected_group_members: HashSet::new(),
            file_downloads: std::collections::HashMap::new(),
            port,
            modals: AppModalState::default(),
            call_state: None,
            network_stats: NetworkStats::default(),
            audio_player: crate::audio_player::AudioPlayer::new(),
            mnemonic_length: 24,
            seed_confirmed_saved: false,
            restore_input: String::new(),
            restore_error: None,
            passcode_hash: None,
            is_app_locked: false,
            auto_lock_mins: 0,
            last_active: std::time::Instant::now(),
            passcode_input: String::new(),
            passcode_error: None,
            show_passcode_setup: false,
            passcode_new_pin: String::new(),
            passcode_confirm_pin: String::new(),
            reveal_seed_in_settings: false,
            seed_verify_passcode: String::new(),
            seed_passcode_error: None,
            show_seed_passcode_prompt: false,
            auto_download_media: true,
            auto_download_voice: true,
            auto_download_files: false,
            notifications_enabled: true,
            p2p_relay_enabled: false,
            onion_routing_enabled: true,
            embed_node_identity: false,
            stego_passphrase: String::new(),
            stego_status_msg: None,
            stego_export_image_path: None,
            stego_exported_bytes: None,
            mesh_routes_count: 3,
            zen_mode: false,
            mood_status: "☕ Ready".to_string(),
            toast_notification: None,
            chat_font_size: 13.5,
            send_on_enter: true,
            compact_bubbles: true,
            session_pair_token: String::new(),
            ringtone_enabled: true,
            sound_volume: 0.8,
            auto_delete_secs: 0,
            voice_modulator: "Оригинал".to_string(),
            duress_pin_hash: None,
        }
    }
}
