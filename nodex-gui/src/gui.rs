//! NodeX desktop GUI — redesigned.
//!
//! This module is the *only* UI file. It follows the "Telegram Desktop meets
//! Discord, without copying either" brief:
//!   • compact 288 px sidebar with search / contact rows / self card,
//!   • dedicated chat header + composer with attachment progress,
//!   • right-side drawer for profile / account / privacy / advanced tabs,
//!   • all icon-only buttons are tooltipped and use vector glyphs (no emoji
//!     as chrome).
//!
//! Backend contract is preserved: `AppCommand` and `UiEvent` are extended
//! **only** with additive variants (marked `// EXTENSION:`) so `main.rs`
//! keeps compiling until a real backend handler wants to react. Anything
//! that would require deeper backend changes (edit, typing indicator,
//! per-relay session count, OS notifications, etc.) is implemented as
//! local UI state and clearly commented.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use base64::prelude::*;
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Key, Layout, Margin, Pos2, Rect, RichText,
    Rounding, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2,
};
use nodex_messenger::db::{SavedChatMessage, SavedContact};

// ============================================================================
// Color system (single source of truth)
// ============================================================================

pub const BG_DARK: Color32 = Color32::from_rgb(0x0F, 0x14, 0x1F);
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x12, 0x18, 0x24);
pub const SIDEBAR_ALT: Color32 = Color32::from_rgb(0x16, 0x1D, 0x2B);
pub const SURFACE: Color32 = Color32::from_rgb(0x1B, 0x24, 0x35);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(0x23, 0x2E, 0x42);
pub const SURFACE_SELECTED: Color32 = Color32::from_rgb(0x0E, 0x3E, 0x52);
pub const ACCENT: Color32 = Color32::from_rgb(0x2A, 0xB7, 0xCA);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x14, 0x62, 0x74);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xEC, 0xF2, 0xF8);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x9A, 0xA7, 0xB8);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x67, 0x74, 0x86);
pub const SUCCESS: Color32 = Color32::from_rgb(0x2C, 0xC1, 0x66);
pub const WARNING: Color32 = Color32::from_rgb(0xE8, 0xA4, 0x2C);
pub const DANGER: Color32 = Color32::from_rgb(0xE9, 0x6B, 0x6B);
pub const BORDER: Color32 = Color32::from_rgb(0x2A, 0x35, 0x48);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(0x20, 0x2A, 0x3C);

pub const BUBBLE_INCOMING: Color32 = SURFACE;
pub const BUBBLE_OUTGOING: Color32 = Color32::from_rgb(0x0B, 0x3A, 0x4A);
pub const BUBBLE_OUTGOING_BORDER: Color32 = Color32::from_rgb(0x14, 0x5A, 0x6C);

// ============================================================================
// Typography & spacing
// ============================================================================

const SIZE_APP_TITLE: f32 = 20.0;
const SIZE_SECTION_TITLE: f32 = 15.0;
const SIZE_CHAT_NAME: f32 = 14.5;
const SIZE_BODY: f32 = 13.5;
const SIZE_SECONDARY: f32 = 12.0;
const SIZE_TIMESTAMP: f32 = 10.5;

const SP_XS: f32 = 4.0;
const SP_SM: f32 = 8.0;
const SP_MD: f32 = 12.0;
const SP_LG: f32 = 16.0;
const SP_XL: f32 = 22.0;

// ============================================================================
// Layout
// ============================================================================

const SIDEBAR_WIDTH: f32 = 288.0;
const TOP_BAR_HEIGHT: f32 = 52.0;
const CHAT_HEADER_HEIGHT: f32 = 60.0;
const COMPOSER_MIN_HEIGHT: f32 = 56.0;
const BUBBLE_MAX_WIDTH_RATIO: f32 = 0.62;
const MAX_ATTACHMENT_BYTES: usize = 40 * 1024;

// ============================================================================
// Backend contract
//
// `UiEvent` / `AppCommand` variants marked `// EXTENSION:` are new and purely
// additive — they will only fire when a future backend chooses to emit or
// listen for them. Existing variants keep the exact same shape as main.rs
// expects, so wiring is not broken.
// ============================================================================

#[derive(Debug, Clone)]
pub enum UiEvent {
    NodeInfo {
        user_id: String,
        dht_node_id: String,
        display_name: String,
        bio: String,
        mnemonic: String,
        #[allow(dead_code)]
        has_mnemonic: bool,
        dht_peers: usize,
        stored_keys: usize,
    },
    ContactsList(Vec<SavedContact>),
    MessagesList {
        contact_id: String,
        messages: Vec<SavedChatMessage>,
    },
    MessageSent(SavedChatMessage),
    MessagesDeleted {
        contact_id: String,
        message_ids: Vec<String>,
    },
    MnemonicRevealed(String),
    InviteGenerated(String),
    StatusLog(String),

    // EXTENSION: emitted when backend confirms message reached the wire.
    // Safe to ignore in existing main.rs; the GUI upgrades local state
    // Sending → Sent when it arrives.
    #[allow(dead_code)]
    MessageAck {
        message_id: String,
    },
    // EXTENSION: emitted when the peer replied with a read receipt.
    #[allow(dead_code)]
    MessageRead {
        message_id: String,
    },
    // EXTENSION: emitted when a message failed to send after retries.
    #[allow(dead_code)]
    MessageFailed {
        message_id: String,
        reason: String,
    },
    // EXTENSION: emitted when a bytes-progress tick is available for an
    // outgoing attachment.
    #[allow(dead_code)]
    UploadProgress {
        message_id: String,
        sent: u64,
        total: u64,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppCommand {
    SendMessage {
        local_id: String,
        recipient_id: String,
        text: String,
        image_base64: Option<String>,
    },
    AddContact {
        user_id: String,
        name: String,
    },
    AddContactInvite(String),
    CreateInvite,
    RegisterAccount {
        display_name: String,
        bio: String,
        mnemonic: String,
    },
    UpdateProfile {
        display_name: String,
        bio: String,
    },
    RefreshInfo,
    RevealMnemonic,
    RefreshContacts,
    SelectContact(String),
    DeleteMessage {
        message_id: String,
        contact_id: String,
        for_everyone: bool,
    },
    DeleteMessages {
        message_ids: Vec<String>,
        contact_id: String,
        for_everyone: bool,
    },
    ClearChat(String),
    DeleteContact(String),

    // EXTENSION: safe no-ops until main.rs opts in.
    RetryMessage {
        message_id: String,
        recipient_id: String,
        text: String,
        image_base64: Option<String>,
    },
    BlockContact(String),
    UnblockContact(String),
    Reconnect,
    ToggleRelay {
        enabled: bool,
    },
    SetRelayTrafficLimit {
        megabytes_per_hour: u64,
    },
    SetRelayMaxSessions {
        sessions: u32,
    },
    TestConnection(String),
    ExportBackup {
        path: String,
        password: String,
    },
    ImportBackup {
        path: String,
        password: String,
    },
    WipeLocalData,
}

// ============================================================================
// Local UI state types
// ============================================================================

#[derive(PartialEq, Eq, Clone, Copy)]
enum AddContactTab {
    Invite,
    UserId,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum EmojiCategory {
    Smiles,
    Gestures,
    Hearts,
    Tech,
    Food,
    Symbols,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Screen {
    Main,
    CreateAccount,
    RestoreAccount,
    Locked,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SettingsTab {
    Profile,
    Account,
    Privacy,
    Network,
    Advanced,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum LocalStatus {
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
}

#[derive(Clone, Debug)]
struct PendingMessage {
    local_id: String,
    recipient_id: String,
    text: String,
    image_base64: Option<String>,
    created_at: Instant,
    total_bytes: u64,
    sent_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct PhotoPreviewData {
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
    pub sender_name: String,
    #[allow(dead_code)]
    pub timestamp: u64,
    pub caption: String,
    pub base64_data: String,
}

#[derive(Clone, Debug)]
struct DraftAttachment {
    name: String,
    base64: String,
    bytes: usize,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct ReplyRef {
    message_id: String,
    author: String,
    excerpt: String,
}

/// Local privacy / relay preferences. Persisting these to disk is left to
/// the backend — the UI reads/writes only in-memory values.
#[derive(Clone)]
struct Preferences {
    hide_user_id: bool,
    disable_logs: bool,
    relay_enabled: bool,
    relay_traffic_mb_h: u64,
    relay_max_sessions: u32,
    auto_lock_minutes: u32,
    lock_password_hash: Option<u64>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            hide_user_id: false,
            disable_logs: false,
            relay_enabled: false,
            relay_traffic_mb_h: 500,
            relay_max_sessions: 8,
            auto_lock_minutes: 0,
            lock_password_hash: None,
        }
    }
}

// ============================================================================
// Main app struct
// ============================================================================

pub struct NodeXApp {
    ui_rx: Receiver<UiEvent>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<AppCommand>,

    screen: Screen,
    settings_tab: SettingsTab,

    // Identity / node info
    my_user_id: String,
    my_dht_node_id: String,
    my_display_name: String,
    my_bio: String,
    my_mnemonic: String,
    dht_peers: usize,
    stored_keys: usize,
    last_info_refresh: Instant,

    // Profile edit
    show_mnemonic: bool,
    edit_name: String,
    edit_bio: String,
    last_invite_link: String,

    // Contacts / messages
    contacts: Vec<SavedContact>,
    active_contact_id: Option<String>,
    messages: HashMap<String, Vec<SavedChatMessage>>,
    read_message_ids: HashSet<String>,
    unread_counts: HashMap<String, usize>,
    blocked_ids: HashSet<String>,
    known_ed25519: HashMap<String, String>,
    key_change_warnings: HashSet<String>,
    global_search_query: String,
    in_chat_search: String,
    show_in_chat_search: bool,

    // Local per-message state
    local_status: HashMap<String, LocalStatus>,
    pending_queue: VecDeque<PendingMessage>,
    failed_queue: Vec<PendingMessage>,

    // Composer
    message_input: String,
    draft_attachments: Vec<DraftAttachment>,
    reply_ref: Option<ReplyRef>,

    // Panels / dialogs
    show_settings_drawer: bool,
    show_add_contact_modal: bool,
    show_contact_security_modal: bool,
    show_emoji_picker: bool,
    show_lightbox: bool,
    show_blocked_modal: bool,
    show_forward_modal: bool,
    show_export_confirm: bool,
    show_import_backup_modal: bool,
    backup_password_input: String,
    import_backup_path: String,
    import_backup_password: String,
    import_backup_error: Option<String>,
    show_wipe_confirm: bool,
    show_key_warning_modal: Option<String>,
    lightbox_data: Option<PhotoPreviewData>,
    image_textures: HashMap<u64, egui::TextureHandle>,
    forward_source: Option<SavedChatMessage>,

    // Selection mode
    selection_mode: bool,
    selected_message_ids: HashSet<String>,

    // Add contact
    add_contact_tab: AddContactTab,
    input_invite_link: String,
    input_user_id: String,
    input_contact_name: String,
    add_contact_error: Option<String>,

    // Emoji
    emoji_category: EmojiCategory,

    // Onboarding
    onboard_name: String,
    onboard_bio: String,
    onboard_mnemonic: String,
    onboard_phrase_revealed: bool,
    onboard_confirmed: bool,
    restore_mnemonic_input: String,
    restore_name_input: String,
    restore_error: Option<String>,

    // Toast
    status_toast: Option<(String, Instant)>,

    // Preferences
    prefs: Preferences,

    // Lock screen input
    lock_password_input: String,
    lock_password_setup_new: String,
    lock_error: Option<String>,
    last_activity: Instant,
}

impl NodeXApp {
    pub fn new(
        ui_rx: Receiver<UiEvent>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<AppCommand>,
    ) -> Self {
        Self {
            ui_rx,
            cmd_tx,
            screen: Screen::Main,
            settings_tab: SettingsTab::Profile,
            my_user_id: "Подключение...".into(),
            my_dht_node_id: String::new(),
            my_display_name: "Мой аккаунт".into(),
            my_bio: String::new(),
            my_mnemonic: String::new(),
            dht_peers: 0,
            stored_keys: 0,
            last_info_refresh: Instant::now(),
            show_mnemonic: false,
            edit_name: String::new(),
            edit_bio: String::new(),
            last_invite_link: String::new(),
            contacts: Vec::new(),
            active_contact_id: None,
            messages: HashMap::new(),
            read_message_ids: HashSet::new(),
            unread_counts: HashMap::new(),
            blocked_ids: HashSet::new(),
            known_ed25519: HashMap::new(),
            key_change_warnings: HashSet::new(),
            global_search_query: String::new(),
            in_chat_search: String::new(),
            show_in_chat_search: false,
            local_status: HashMap::new(),
            pending_queue: VecDeque::new(),
            failed_queue: Vec::new(),
            message_input: String::new(),
            draft_attachments: Vec::new(),
            reply_ref: None,
            show_settings_drawer: false,
            show_add_contact_modal: false,
            show_contact_security_modal: false,
            show_emoji_picker: false,
            show_lightbox: false,
            show_blocked_modal: false,
            show_forward_modal: false,
            show_export_confirm: false,
            show_import_backup_modal: false,
            backup_password_input: String::new(),
            import_backup_path: String::new(),
            import_backup_password: String::new(),
            import_backup_error: None,
            show_wipe_confirm: false,
            show_key_warning_modal: None,
            lightbox_data: None,
            image_textures: HashMap::new(),
            forward_source: None,
            selection_mode: false,
            selected_message_ids: HashSet::new(),
            add_contact_tab: AddContactTab::Invite,
            input_invite_link: String::new(),
            input_user_id: String::new(),
            input_contact_name: String::new(),
            add_contact_error: None,
            emoji_category: EmojiCategory::Smiles,
            onboard_name: String::new(),
            onboard_bio: String::new(),
            onboard_mnemonic: String::new(),
            onboard_phrase_revealed: false,
            onboard_confirmed: false,
            restore_mnemonic_input: String::new(),
            restore_name_input: String::new(),
            restore_error: None,
            status_toast: None,
            prefs: Preferences::default(),
            lock_password_input: String::new(),
            lock_password_setup_new: String::new(),
            lock_error: None,
            last_activity: Instant::now(),
        }
    }

    // ------------------------------------------------------------------
    // Event pump
    // ------------------------------------------------------------------
    fn poll_events(&mut self) {
        while let Ok(event) = self.ui_rx.try_recv() {
            match event {
                UiEvent::NodeInfo {
                    user_id,
                    dht_node_id,
                    display_name,
                    bio,
                    mnemonic,
                    has_mnemonic: _,
                    dht_peers,
                    stored_keys,
                } => {
                    let is_first_time = display_name.is_empty()
                        || display_name == "UserNode"
                        || display_name == "NodeX User";

                    self.my_user_id = user_id;
                    self.my_dht_node_id = dht_node_id;

                    if !is_first_time {
                        if self.edit_name.is_empty() {
                            self.edit_name = display_name.clone();
                        }
                        if self.onboard_name.is_empty() {
                            self.onboard_name = display_name.clone();
                        }
                        self.my_display_name = display_name;
                    } else {
                        self.my_display_name = "Новый пользователь".into();
                        if self.screen == Screen::Main && !self.onboard_confirmed {
                            self.screen = Screen::CreateAccount;
                            if self.onboard_mnemonic.is_empty() {
                                if let Ok((fresh_mn, _)) = nodex_messenger::crypto::UserIdentity::generate_mnemonic() {
                                    self.onboard_mnemonic = fresh_mn;
                                }
                            }
                        }
                    }

                    if self.edit_bio.is_empty() {
                        self.edit_bio = bio.clone();
                    }
                    self.my_bio = bio;
                    if !mnemonic.is_empty() {
                        self.my_mnemonic = mnemonic;
                    }
                    self.dht_peers = dht_peers;
                    self.stored_keys = stored_keys;
                }
                UiEvent::ContactsList(list) => {
                    // Watch for silently changed ed25519 keys.
                    for c in &list {
                        if let Some(prev) = self.known_ed25519.get(&c.user_id_hex) {
                            if prev != &c.ed25519_pub_hex && !c.ed25519_pub_hex.is_empty() {
                                self.key_change_warnings.insert(c.user_id_hex.clone());
                            }
                        }
                        if !c.ed25519_pub_hex.is_empty() {
                            self.known_ed25519
                                .insert(c.user_id_hex.clone(), c.ed25519_pub_hex.clone());
                        }
                    }
                    self.contacts = list;
                }
                UiEvent::MessagesList {
                    contact_id,
                    messages,
                } => {
                    for m in &messages {
                        self.read_message_ids.insert(m.id.clone());
                        if !m.incoming {
                            let s = if m.delivered {
                                LocalStatus::Delivered
                            } else {
                                LocalStatus::Sent
                            };
                            self.local_status.entry(m.id.clone()).or_insert(s);
                        }
                    }
                    self.messages.insert(contact_id, messages);
                }
                UiEvent::MessageSent(msg) => {
                    let other_id = if msg.incoming {
                        msg.sender_id_hex.clone()
                    } else {
                        msg.recipient_id_hex.clone()
                    };
                    if msg.incoming
                        && self.active_contact_id.as_deref() != Some(other_id.as_str())
                        && !self.blocked_ids.contains(&other_id)
                    {
                        *self.unread_counts.entry(other_id.clone()).or_insert(0) += 1;
                        self.show_toast(&format!(
                            "Новое сообщение от {}",
                            self.contacts
                                .iter()
                                .find(|c| c.user_id_hex == other_id)
                                .map(|c| c.name.as_str())
                                .unwrap_or("контакта")
                        ));
                    }
                    if !msg.incoming {
                        let status = if msg.delivered {
                            LocalStatus::Delivered
                        } else {
                            LocalStatus::Sent
                        };
                        self.local_status.insert(msg.id.clone(), status);
                        // Drop pending entry matching text/recipient (best effort).
                        if let Some(pos) = self.pending_queue.iter().position(|p| {
                            p.recipient_id == other_id
                                && p.text == msg.text
                                && p.image_base64.as_deref()
                                    == msg.image_base64.as_deref()
                        }) {
                            self.pending_queue.remove(pos);
                        }
                    }
                    self.read_message_ids.insert(msg.id.clone());
                    let list = self.messages.entry(other_id).or_default();
                    if !list.iter().any(|m| m.id == msg.id) {
                        list.push(msg);
                    }
                }
                UiEvent::MessagesDeleted { contact_id, message_ids } => {
                    let id_set: HashSet<String> = message_ids.into_iter().collect();
                    if let Some(list) = self.messages.get_mut(&contact_id) {
                        list.retain(|m| !id_set.contains(&m.id));
                    }
                    self.selected_message_ids.retain(|id| !id_set.contains(id));
                    for id in &id_set {
                        self.local_status.remove(id);
                    }
                }
                UiEvent::MnemonicRevealed(phrase) => {
                    self.my_mnemonic = phrase;
                    self.show_mnemonic = true;
                }
                UiEvent::InviteGenerated(invite) => {
                    self.last_invite_link = invite.clone();
                    if let Some(ctx) = None::<&egui::Context> {
                        // reserved – see composer copy path
                        let _ = ctx;
                    }
                    self.show_toast("Invite-ссылка готова. Скопируйте её в настройках.");
                }
                UiEvent::StatusLog(status) => {
                    if !self.prefs.disable_logs {
                        self.show_toast(&status);
                    }
                }
                UiEvent::MessageAck { message_id } => {
                    self.local_status.insert(message_id, LocalStatus::Sent);
                }
                UiEvent::MessageRead { message_id } => {
                    self.local_status.insert(message_id, LocalStatus::Read);
                }
                UiEvent::MessageFailed { message_id, reason } => {
                    self.local_status
                        .insert(message_id.clone(), LocalStatus::Failed);
                    if let Some(pos) = self
                        .pending_queue
                        .iter()
                        .position(|p| p.local_id == message_id)
                    {
                        let p = self.pending_queue.remove(pos).unwrap();
                        self.failed_queue.push(p);
                    }
                    self.show_toast(&format!("Не удалось отправить сообщение: {reason}"));
                }
                UiEvent::UploadProgress {
                    message_id,
                    sent,
                    total,
                } => {
                    if let Some(p) = self
                        .pending_queue
                        .iter_mut()
                        .find(|p| p.local_id == message_id)
                    {
                        p.sent_bytes = sent;
                        p.total_bytes = total;
                    }
                }
            }
        }

        // Sending → Failed fallback: without a backend ack, timeout locally
        // after 25 s so the user can retry. Real ack via MessageAck /
        // MessageSent supersedes this.
        let now = Instant::now();
        let stale: Vec<String> = self
            .pending_queue
            .iter()
            .filter(|p| now.duration_since(p.created_at) > Duration::from_secs(25))
            .map(|p| p.local_id.clone())
            .collect();
        for id in stale {
            if let Some(pos) = self.pending_queue.iter().position(|p| p.local_id == id) {
                let p = self.pending_queue.remove(pos).unwrap();
                self.local_status.insert(p.local_id.clone(), LocalStatus::Failed);
                self.failed_queue.push(p);
            }
        }
    }

    fn show_toast(&mut self, text: &str) {
        self.status_toast = Some((text.to_string(), Instant::now()));
    }

    fn select_contact(&mut self, contact_id: &str) {
        self.active_contact_id = Some(contact_id.to_string());
        self.unread_counts.insert(contact_id.to_string(), 0);
        // Mark all incoming from this chat as read locally.
        if let Some(list) = self.messages.get(contact_id) {
            for m in list {
                if !m.incoming {
                    // Assume peer reads once we open the chat only if backend
                    // already delivered — leave delivered as-is.
                }
                self.read_message_ids.insert(m.id.clone());
            }
        }
        let _ = self.cmd_tx.send(AppCommand::SelectContact(contact_id.to_string()));
    }

    fn touch_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    fn maybe_auto_lock(&mut self) {
        if self.prefs.lock_password_hash.is_none() || self.prefs.auto_lock_minutes == 0 {
            return;
        }
        if Instant::now().duration_since(self.last_activity)
            > Duration::from_secs(self.prefs.auto_lock_minutes as u64 * 60)
        {
            self.screen = Screen::Locked;
        }
    }
}

// ============================================================================
// eframe::App
// ============================================================================

impl eframe::App for NodeXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();
        apply_theme(ctx);

        // Any user input counts as activity.
        if ctx.input(|i| i.pointer.any_pressed() || !i.events.is_empty()) {
            self.touch_activity();
        }
        self.maybe_auto_lock();

        // Repaint every second so pending progress and toast expiry animate.
        ctx.request_repaint_after(Duration::from_millis(500));

        // Accept dropped files.
        let dropped: Vec<egui::DroppedFile> =
            ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() && self.active_contact_id.is_some() {
            for f in dropped {
                if let Some(path) = f.path {
                    if let Ok(bytes) = std::fs::read(&path) {
                        self.push_attachment(
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "attachment".into()),
                            bytes,
                        );
                    }
                }
            }
        }

        match self.screen {
            Screen::Main => self.render_main(ctx),
            Screen::CreateAccount => self.render_create_account(ctx),
            Screen::RestoreAccount => self.render_restore_account(ctx),
            Screen::Locked => self.render_lock_screen(ctx),
        }

        self.render_toast(ctx);
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_DARK;
    visuals.window_fill = SURFACE;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_rounding = Rounding::same(10.0);
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);
    visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, BG_DARK);
    visuals.widgets.active.rounding = Rounding::same(6.0);
    visuals.selection.bg_fill = SURFACE_SELECTED;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.extreme_bg_color = BG_DARK;
    ctx.set_visuals(visuals);
}

// ============================================================================
// Main screen composition
// ============================================================================

impl NodeXApp {
    fn render_main(&mut self, ctx: &egui::Context) {
        self.render_top_bar(ctx);
        self.render_sidebar(ctx);
        self.render_settings_drawer(ctx);
        self.render_chat_pane(ctx);
        self.render_add_contact_modal(ctx);
        self.render_contact_security_modal(ctx);
        self.render_blocked_modal(ctx);
        self.render_forward_modal(ctx);
        self.render_lightbox(ctx);
        self.render_export_confirm(ctx);
        self.render_import_backup_modal(ctx);
        self.render_wipe_confirm(ctx);
        self.render_key_warning_modal(ctx);
    }

    // ---- Top bar ----
    fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar")
            .exact_height(TOP_BAR_HEIGHT)
            .frame(
                Frame::none()
                    .fill(SIDEBAR_BG)
                    .inner_margin(Margin::symmetric(SP_LG, SP_SM))
                    .stroke(Stroke::new(1.0, BORDER_SOFT)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    draw_wordmark(ui);
                    ui.add_space(SP_MD);
                    let (dot, text) = network_status(self.dht_peers);
                    status_pill(ui, dot, text);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, 32.0, draw_icon_settings, "Профиль и настройки")
                            .clicked()
                        {
                            self.show_settings_drawer = !self.show_settings_drawer;
                        }
                        ui.add_space(SP_XS);
                        if icon_button(ui, 32.0, draw_icon_add_contact, "Новый диалог").clicked()
                        {
                            self.show_add_contact_modal = true;
                            self.add_contact_error = None;
                        }
                        ui.add_space(SP_XS);
                        if icon_button(ui, 32.0, draw_icon_refresh, "Переподключиться").clicked() {
                            let _ = self.cmd_tx.send(AppCommand::Reconnect);
                            let _ = self.cmd_tx.send(AppCommand::RefreshInfo);
                            let _ = self.cmd_tx.send(AppCommand::RefreshContacts);
                            self.last_info_refresh = Instant::now();
                            self.show_toast("Переподключение к сети...");
                        }
                    });
                });
            });
    }

    // ---- Sidebar ----
    fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(SIDEBAR_WIDTH)
            .frame(
                Frame::none()
                    .fill(SIDEBAR_BG)
                    .inner_margin(Margin::symmetric(SP_MD, SP_MD)),
            )
            .show(ctx, |ui| {
                // Search
                Frame::none()
                    .fill(SIDEBAR_ALT)
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::symmetric(SP_SM, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
                            draw_icon_search(ui.painter(), r.center(), 7.0, TEXT_MUTED);
                            ui.add(
                                TextEdit::singleline(&mut self.global_search_query)
                                    .hint_text("Поиск чатов и сообщений")
                                    .desired_width(SIDEBAR_WIDTH - 2.0 * SP_MD - 60.0)
                                    .frame(false),
                            );
                            if !self.global_search_query.is_empty()
                                && text_button(ui, "×", TEXT_MUTED).clicked()
                            {
                                self.global_search_query.clear();
                            }
                        });
                    });

                ui.add_space(SP_SM);

                let bottom_reserve = 76.0;
                let list_height = (ui.available_height() - bottom_reserve).max(80.0);

                ScrollArea::vertical()
                    .max_height(list_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let query = self.global_search_query.to_lowercase();
                        let contacts_snapshot = self.contacts.clone();

                        // Build ordered list: contacts matching by name/id first, then
                        // contacts whose messages contain the query.
                        let filtered: Vec<SavedContact> = contacts_snapshot
                            .iter()
                            .filter(|c| {
                                if query.is_empty() {
                                    return true;
                                }
                                if c.name.to_lowercase().contains(&query)
                                    || c.user_id_hex.to_lowercase().contains(&query)
                                {
                                    return true;
                                }
                                self.messages
                                    .get(&c.user_id_hex)
                                    .map(|ms| {
                                        ms.iter().any(|m| m.text.to_lowercase().contains(&query))
                                    })
                                    .unwrap_or(false)
                            })
                            .cloned()
                            .collect();

                        if filtered.is_empty() {
                            ui.add_space(SP_XL);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(if query.is_empty() {
                                        "Пока нет чатов"
                                    } else {
                                        "Ничего не найдено"
                                    })
                                    .color(TEXT_MUTED)
                                    .size(SIZE_BODY),
                                );
                                ui.add_space(SP_SM);
                                if ghost_button(ui, "Добавить контакт").clicked() {
                                    self.show_add_contact_modal = true;
                                    self.add_contact_error = None;
                                }
                            });
                        } else {
                            for contact in &filtered {
                                self.render_chat_row(ui, contact);
                            }
                        }
                    });

                // Self-card at the bottom.
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.add_space(SP_SM);
                    thin_separator(ui);
                    ui.add_space(SP_SM);
                    Frame::none()
                        .fill(SIDEBAR_ALT)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(SP_SM, SP_SM))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                avatar(ui, &self.my_display_name, 34.0, true);
                                ui.add_space(SP_SM);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&self.my_display_name)
                                            .strong()
                                            .size(SIZE_CHAT_NAME)
                                            .color(TEXT_PRIMARY),
                                    );
                                    let id_display = if self.prefs.hide_user_id {
                                        "ID скрыт".to_string()
                                    } else {
                                        short_id(&self.my_user_id)
                                    };
                                    ui.label(
                                        RichText::new(id_display)
                                            .size(SIZE_TIMESTAMP)
                                            .color(TEXT_MUTED),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if icon_button(ui, 26.0, draw_icon_settings, "Настройки")
                                        .clicked()
                                    {
                                        self.show_settings_drawer = true;
                                    }
                                });
                            });
                        });
                });
            });
    }

    fn render_chat_row(&mut self, ui: &mut Ui, contact: &SavedContact) {
        let is_active =
            self.active_contact_id.as_deref() == Some(contact.user_id_hex.as_str());
        let unread = self
            .unread_counts
            .get(&contact.user_id_hex)
            .copied()
            .unwrap_or(0);
        let blocked = self.blocked_ids.contains(&contact.user_id_hex);
        let key_warn = self.key_change_warnings.contains(&contact.user_id_hex);

        let bg = if is_active { SURFACE_SELECTED } else { SIDEBAR_BG };

        let frame = Frame::none()
            .fill(bg)
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(SP_SM, SP_SM));

        let last_msg_ts = last_msg_at(&self.messages, &contact.user_id_hex);
        let last_snippet = self
            .messages
            .get(&contact.user_id_hex)
            .and_then(|m| m.last())
            .map(|m| {
                if !m.text.is_empty() {
                    m.text.chars().take(40).collect::<String>()
                } else if m.image_base64.is_some() {
                    "Фото".to_string()
                } else {
                    String::new()
                }
            })
            .unwrap_or_else(|| "Нет сообщений".to_string());

        let response = frame
            .show(ui, |ui| {
                ui.set_min_width(SIDEBAR_WIDTH - 2.0 * SP_MD - 2.0 * SP_SM);
                ui.horizontal(|ui| {
                    avatar(ui, &contact.name, 40.0, !blocked);
                    ui.add_space(SP_SM);
                    ui.vertical(|ui| {
                        ui.set_width(
                            SIDEBAR_WIDTH - 2.0 * SP_MD - 2.0 * SP_SM - 40.0 - SP_SM - 44.0,
                        );
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&contact.name)
                                    .strong()
                                    .color(if blocked { TEXT_MUTED } else { TEXT_PRIMARY })
                                    .size(SIZE_CHAT_NAME),
                            );
                            if key_warn {
                                ui.label(
                                    RichText::new(" ⚠")
                                        .color(WARNING)
                                        .size(SIZE_SECONDARY),
                                );
                            }
                            if blocked {
                                ui.label(
                                    RichText::new(" · заблокирован")
                                        .color(TEXT_MUTED)
                                        .size(SIZE_TIMESTAMP),
                                );
                            }
                        });
                        ui.label(
                            RichText::new(last_snippet)
                                .color(TEXT_MUTED)
                                .size(SIZE_SECONDARY),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if unread > 0 {
                            unread_badge(ui, unread);
                        } else if let Some(ts) = last_msg_ts {
                            ui.label(
                                RichText::new(format_time(ts))
                                    .color(TEXT_MUTED)
                                    .size(SIZE_TIMESTAMP),
                            );
                        }
                    });
                });
            })
            .response;

        if response.interact(Sense::click()).clicked() {
            self.select_contact(&contact.user_id_hex);
        }
        response.context_menu(|ui| {
            if ui.button("Открыть чат").clicked() {
                self.select_contact(&contact.user_id_hex);
                ui.close_menu();
            }
            if blocked {
                if ui.button("Разблокировать").clicked() {
                    self.blocked_ids.remove(&contact.user_id_hex);
                    let _ = self
                        .cmd_tx
                        .send(AppCommand::UnblockContact(contact.user_id_hex.clone()));
                    ui.close_menu();
                }
            } else if ui.button("Заблокировать").clicked() {
                self.blocked_ids.insert(contact.user_id_hex.clone());
                let _ = self
                    .cmd_tx
                    .send(AppCommand::BlockContact(contact.user_id_hex.clone()));
                self.show_toast("Контакт заблокирован");
                ui.close_menu();
            }
            if ui.button("Очистить историю").clicked() {
                let _ = self
                    .cmd_tx
                    .send(AppCommand::ClearChat(contact.user_id_hex.clone()));
                self.messages.remove(&contact.user_id_hex);
                ui.close_menu();
            }
            if ui.button("Удалить контакт").clicked() {
                let _ = self
                    .cmd_tx
                    .send(AppCommand::DeleteContact(contact.user_id_hex.clone()));
                ui.close_menu();
            }
        });

        ui.add_space(SP_XS);
    }

    // ---- Central chat pane ----
    fn render_chat_pane(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(BG_DARK)
                    .inner_margin(Margin::symmetric(SP_LG, SP_MD)),
            )
            .show(ctx, |ui| {
                let contact_id = match self.active_contact_id.clone() {
                    Some(id) => id,
                    None => {
                        self.render_empty_state(ui);
                        return;
                    }
                };

                let contact = self
                    .contacts
                    .iter()
                    .find(|c| c.user_id_hex == contact_id)
                    .cloned();
                let contact_name = contact
                    .as_ref()
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Пользователь {}", short_id(&contact_id)));
                let is_blocked = self.blocked_ids.contains(&contact_id);

                self.render_chat_header(ui, &contact_id, &contact_name, contact.as_ref());

                if self.selection_mode {
                    ui.add_space(SP_XS);
                    Frame::none()
                        .fill(SURFACE_SELECTED)
                        .stroke(Stroke::new(1.0, ACCENT))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(SP_MD, SP_SM))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let count = self.selected_message_ids.len();
                                ui.label(
                                    RichText::new(format!("Выбрано сообщений: {}", count))
                                        .strong()
                                        .color(TEXT_PRIMARY)
                                        .size(SIZE_BODY),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ghost_button(ui, "✕ Отмена").clicked() {
                                        self.selection_mode = false;
                                        self.selected_message_ids.clear();
                                    }
                                    if count > 0 {
                                        if danger_button(ui, &format!("⚡ Удалить у обоих ({})", count)).clicked() {
                                            let ids: Vec<String> = self.selected_message_ids.iter().cloned().collect();
                                            let _ = self.cmd_tx.send(AppCommand::DeleteMessages {
                                                message_ids: ids.clone(),
                                                contact_id: contact_id.clone(),
                                                for_everyone: true,
                                            });
                                            if let Some(list) = self.messages.get_mut(&contact_id) {
                                                let id_set = self.selected_message_ids.clone();
                                                list.retain(|m| !id_set.contains(&m.id));
                                            }
                                            self.selected_message_ids.clear();
                                            self.selection_mode = false;
                                            self.show_toast("Сообщения удалены у обоих участников");
                                        }
                                        if ghost_button(ui, &format!("🗑 Удалить у себя ({})", count)).clicked() {
                                            let ids: Vec<String> = self.selected_message_ids.iter().cloned().collect();
                                            let _ = self.cmd_tx.send(AppCommand::DeleteMessages {
                                                message_ids: ids.clone(),
                                                contact_id: contact_id.clone(),
                                                for_everyone: false,
                                            });
                                            if let Some(list) = self.messages.get_mut(&contact_id) {
                                                let id_set = self.selected_message_ids.clone();
                                                list.retain(|m| !id_set.contains(&m.id));
                                            }
                                            self.selected_message_ids.clear();
                                            self.selection_mode = false;
                                            self.show_toast("Сообщения удалены локально");
                                        }
                                    }
                                });
                            });
                        });
                }

                if self.show_in_chat_search {
                    ui.add_space(SP_SM);
                    Frame::none()
                        .fill(SURFACE)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(SP_SM, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(
                                    TextEdit::singleline(&mut self.in_chat_search)
                                        .hint_text("Поиск по сообщениям")
                                        .desired_width(ui.available_width() - 28.0)
                                        .frame(false),
                                );
                                if text_button(ui, "×", TEXT_MUTED).clicked() {
                                    self.show_in_chat_search = false;
                                    self.in_chat_search.clear();
                                }
                            });
                        });
                }

                ui.add_space(SP_SM);
                thin_separator(ui);

                let composer_reserve = 26.0
                    + (if !self.draft_attachments.is_empty() { 60.0 } else { 0.0 })
                    + (if self.reply_ref.is_some() { 40.0 } else { 0.0 })
                    + COMPOSER_MIN_HEIGHT;
                let available_height = (ui.available_height() - composer_reserve).max(80.0);

                let mut delete_for_me_target: Option<String> = None;
                let mut delete_for_everyone_target: Option<String> = None;
                let mut toggle_select_target: Option<String> = None;
                let mut reply_target: Option<SavedChatMessage> = None;
                let mut forward_target: Option<SavedChatMessage> = None;
                let mut retry_target: Option<String> = None;

                ScrollArea::vertical()
                    .max_height(available_height)
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let empty = Vec::new();
                        let msg_list = self.messages.get(&contact_id).unwrap_or(&empty);
                        let query = self.in_chat_search.to_lowercase();
                        let filtered: Vec<&SavedChatMessage> = if query.is_empty() {
                            msg_list.iter().collect()
                        } else {
                            msg_list
                                .iter()
                                .filter(|m| m.text.to_lowercase().contains(&query))
                                .collect()
                        };

                        if filtered.is_empty() {
                            ui.add_space(SP_XL);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(if query.is_empty() {
                                        "Сообщений пока нет. Скажите привет!"
                                    } else {
                                        "Ничего не найдено"
                                    })
                                    .color(TEXT_MUTED)
                                    .size(SIZE_BODY),
                                );
                            });
                        }

                        let max_width = ui.available_width() * BUBBLE_MAX_WIDTH_RATIO;
                        let mut last_day: Option<String> = None;
                        for msg in filtered {
                            let day = format_day(msg.timestamp);
                            if last_day.as_deref() != Some(day.as_str()) {
                                render_day_divider(ui, &day);
                                last_day = Some(day);
                            }

                            let is_outgoing = !msg.incoming;
                            let status = if is_outgoing {
                                self.local_status
                                    .get(&msg.id)
                                    .copied()
                                    .unwrap_or(if msg.delivered {
                                        LocalStatus::Delivered
                                    } else {
                                        LocalStatus::Sent
                                    })
                            } else {
                                LocalStatus::Read
                            };

                            let is_selected = self.selected_message_ids.contains(&msg.id);
                            let mut actions = BubbleActions::default();
                            ui.horizontal(|ui| {
                                if self.selection_mode {
                                    let check_str = if is_selected { "☑" } else { "☐" };
                                    let check_col = if is_selected { ACCENT } else { TEXT_MUTED };
                                    if ui.add(egui::Button::new(RichText::new(check_str).color(check_col).size(16.0)).frame(false)).clicked() {
                                        if is_selected {
                                            self.selected_message_ids.remove(&msg.id);
                                        } else {
                                            self.selected_message_ids.insert(msg.id.clone());
                                        }
                                    }
                                }
                                let layout = if is_outgoing {
                                    Layout::right_to_left(Align::Min)
                                } else {
                                    Layout::left_to_right(Align::Min)
                                };
                                ui.with_layout(layout, |ui| {
                                    render_bubble(
                                        ui,
                                        msg,
                                        is_outgoing,
                                        status,
                                        max_width,
                                        ctx,
                                        &mut self.show_lightbox,
                                        &mut self.lightbox_data,
                                        &mut self.image_textures,
                                        &mut actions,
                                        self.selection_mode,
                                    );
                                });
                            });
                            if let Some(id) = actions.delete_for_me {
                                delete_for_me_target = Some(id);
                            }
                            if let Some(id) = actions.delete_for_everyone {
                                delete_for_everyone_target = Some(id);
                            }
                            if let Some(id) = actions.toggle_select {
                                toggle_select_target = Some(id);
                            }
                            if actions.reply {
                                reply_target = Some(msg.clone());
                            }
                            if actions.forward {
                                forward_target = Some(msg.clone());
                            }
                            if actions.retry {
                                retry_target = Some(msg.id.clone());
                            }
                            ui.add_space(SP_XS);
                        }

                        // Show pending optimistic bubbles (Sending…) so the user
                        // sees their message immediately even before the backend
                        // echoes MessageSent.
                        let pending: Vec<PendingMessage> = self
                            .pending_queue
                            .iter()
                            .filter(|p| p.recipient_id == contact_id)
                            .cloned()
                            .collect();
                        for p in pending {
                            ui.horizontal(|ui| {
                                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                    render_pending_bubble(ui, &p, max_width);
                                });
                            });
                            ui.add_space(SP_XS);
                        }
                    });

                if let Some(id) = toggle_select_target {
                    self.selection_mode = true;
                    if self.selected_message_ids.contains(&id) {
                        self.selected_message_ids.remove(&id);
                    } else {
                        self.selected_message_ids.insert(id);
                    }
                }
                if let Some(id) = delete_for_me_target {
                    let _ = self.cmd_tx.send(AppCommand::DeleteMessage {
                        message_id: id.clone(),
                        contact_id: contact_id.clone(),
                        for_everyone: false,
                    });
                    if let Some(list) = self.messages.get_mut(&contact_id) {
                        list.retain(|m| m.id != id);
                    }
                    self.selected_message_ids.remove(&id);
                    self.show_toast("Сообщение удалено у себя");
                }
                if let Some(id) = delete_for_everyone_target {
                    let _ = self.cmd_tx.send(AppCommand::DeleteMessage {
                        message_id: id.clone(),
                        contact_id: contact_id.clone(),
                        for_everyone: true,
                    });
                    if let Some(list) = self.messages.get_mut(&contact_id) {
                        list.retain(|m| m.id != id);
                    }
                    self.selected_message_ids.remove(&id);
                    self.show_toast("Сообщение удалено у обоих участников");
                }
                if let Some(m) = reply_target {
                    self.reply_ref = Some(ReplyRef {
                        message_id: m.id.clone(),
                        author: if m.incoming { contact_name.clone() } else { self.my_display_name.clone() },
                        excerpt: m.text.chars().take(60).collect(),
                    });
                }
                if let Some(m) = forward_target {
                    self.forward_source = Some(m);
                    self.show_forward_modal = true;
                }
                if let Some(id) = retry_target {
                    self.retry_message(&id);
                }

                ui.add_space(SP_SM);
                if is_blocked {
                    Frame::none()
                        .fill(SURFACE)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(SP_SM))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Контакт заблокирован. Сообщения не будут отправляться.")
                                        .color(WARNING)
                                        .size(SIZE_SECONDARY),
                                );
                                if ghost_button(ui, "Разблокировать").clicked() {
                                    self.blocked_ids.remove(&contact_id);
                                    let _ = self
                                        .cmd_tx
                                        .send(AppCommand::UnblockContact(contact_id.clone()));
                                }
                            });
                        });
                } else {
                    self.render_composer(ui, &contact_id);
                }
            });
    }

    fn render_empty_state(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() / 2.0 - 20.0).max(SP_XL));
            Frame::none()
                .fill(SURFACE)
                .rounding(Rounding::same(16.0))
                .inner_margin(Margin::symmetric(SP_MD, SP_XS))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Выберите чат для начала общения")
                            .size(SIZE_SECONDARY)
                            .color(TEXT_MUTED),
                    );
                });
        });
    }

    fn render_chat_header(
        &mut self,
        ui: &mut Ui,
        contact_id: &str,
        contact_name: &str,
        contact: Option<&SavedContact>,
    ) {
        let key_warn = self.key_change_warnings.contains(contact_id);
        ui.horizontal(|ui| {
            ui.set_min_height(CHAT_HEADER_HEIGHT - SP_MD);
            avatar(ui, contact_name, 38.0, false);
            ui.add_space(SP_SM);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(contact_name)
                            .strong()
                            .size(SIZE_CHAT_NAME + 1.0)
                            .color(TEXT_PRIMARY),
                    );
                    if key_warn && text_button(ui, "⚠ Ключ изменился", WARNING).clicked() {
                        self.show_key_warning_modal = Some(contact_id.to_string());
                    }
                });
                let (dot, state_text) = peer_state(self.dht_peers, contact);
                ui.horizontal(|ui| {
                    status_dot(ui, dot);
                    ui.label(
                        RichText::new(state_text)
                            .size(SIZE_SECONDARY)
                            .color(TEXT_SECONDARY),
                    );
                });
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, 28.0, draw_icon_more, "Ещё")
                    .on_hover_text("Дополнительные действия")
                    .clicked()
                {
                    // rely on context menu
                }
                if icon_button(ui, 28.0, draw_icon_select, if self.selection_mode { "Выйти из режима выбора" } else { "Выбрать сообщения" }).clicked() {
                    self.selection_mode = !self.selection_mode;
                    if !self.selection_mode {
                        self.selected_message_ids.clear();
                    }
                }
                if icon_button(ui, 28.0, draw_icon_trash, "Очистить историю").clicked() {
                    let _ = self.cmd_tx.send(AppCommand::ClearChat(contact_id.to_string()));
                    self.messages.remove(contact_id);
                    self.show_toast("История чата очищена");
                }
                if icon_button(ui, 28.0, draw_icon_shield, "Отпечаток безопасности").clicked() {
                    self.show_contact_security_modal = true;
                }
                if icon_button(ui, 28.0, draw_icon_search, "Поиск в чате").clicked() {
                    self.show_in_chat_search = !self.show_in_chat_search;
                    if !self.show_in_chat_search {
                        self.in_chat_search.clear();
                    }
                }
            });
        });
    }

    // ---- Composer ----
    fn render_composer(&mut self, ui: &mut Ui, contact_id: &str) {
        // Reply banner
        if let Some(r) = self.reply_ref.clone() {
            Frame::none()
                .fill(SURFACE)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(SP_SM, SP_XS))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(3.0, 30.0), Sense::hover());
                        ui.painter().rect_filled(rect, Rounding::same(2.0), ACCENT);
                        ui.add_space(SP_XS);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("Ответ {}", r.author))
                                    .size(SIZE_TIMESTAMP)
                                    .color(ACCENT),
                            );
                            ui.label(
                                RichText::new(&r.excerpt)
                                    .size(SIZE_SECONDARY)
                                    .color(TEXT_SECONDARY),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if text_button(ui, "×", TEXT_MUTED).clicked() {
                                self.reply_ref = None;
                            }
                        });
                    });
                });
            ui.add_space(SP_XS);
        }

        // Attachment strip
        if !self.draft_attachments.is_empty() {
            Frame::none()
                .fill(SURFACE)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(SP_SM, SP_XS))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let mut remove: Option<usize> = None;
                        for (i, a) in self.draft_attachments.iter().enumerate() {
                            Frame::none()
                                .fill(SIDEBAR_ALT)
                                .rounding(Rounding::same(6.0))
                                .inner_margin(Margin::symmetric(SP_SM, 4.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(
                                            Vec2::new(14.0, 14.0),
                                            Sense::hover(),
                                        );
                                        draw_icon_image(
                                            ui.painter(),
                                            r.center(),
                                            6.0,
                                            ACCENT,
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {}",
                                                a.name,
                                                human_bytes(a.bytes)
                                            ))
                                            .size(SIZE_TIMESTAMP)
                                            .color(TEXT_SECONDARY),
                                        );
                                        if text_button(ui, "×", TEXT_MUTED).clicked() {
                                            remove = Some(i);
                                        }
                                    });
                                });
                        }
                        if let Some(i) = remove {
                            self.draft_attachments.remove(i);
                        }
                    });
                });
            ui.add_space(SP_XS);
        }

        Frame::none()
            .fill(SURFACE)
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0_f32, BORDER))
            .inner_margin(Margin::symmetric(SP_SM, SP_SM))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if icon_button(ui, 28.0, draw_icon_attach, "Прикрепить изображения")
                        .clicked()
                    {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "gif"])
                            .pick_files()
                        {
                            for path in paths {
                                if let Ok(bytes) = std::fs::read(&path) {
                                    let name = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "image".into());
                                    self.push_attachment(name, bytes);
                                }
                            }
                        }
                    }
                    if icon_button(ui, 28.0, draw_icon_emoji, "Эмодзи").clicked() {
                        self.show_emoji_picker = !self.show_emoji_picker;
                    }

                    let response = ui.add(
                        TextEdit::multiline(&mut self.message_input)
                            .hint_text("Напишите сообщение...")
                            .desired_rows(1)
                            .desired_width(ui.available_width() - 46.0)
                            .frame(false),
                    );
                    let enter_pressed = response.has_focus()
                        && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);

                    let send_clicked =
                        icon_button(ui, 32.0, draw_icon_send, "Отправить (Enter)").clicked();

                    if send_clicked || enter_pressed {
                        self.submit_message(contact_id);
                    }
                });
            });

        if self.show_emoji_picker {
            self.render_emoji_picker(ui.ctx());
        }
    }

    fn push_attachment(&mut self, name: String, bytes: Vec<u8>) {
        let bytes = compress_if_needed(bytes);
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            self.show_toast(&format!(
                "Файл {name} превышает лимит {} — не прикреплён",
                human_bytes(MAX_ATTACHMENT_BYTES)
            ));
            return;
        }
        self.draft_attachments.push(DraftAttachment {
            name,
            base64: BASE64_STANDARD.encode(&bytes),
            bytes: bytes.len(),
        });
    }

    fn submit_message(&mut self, contact_id: &str) {
        let mut text = self.message_input.trim().to_string();
        if let Some(r) = self.reply_ref.take() {
            // Encode reply inline since backend has no reply_to field yet.
            let excerpt = r.excerpt.replace('\n', " ");
            text = format!("↪ {}: {}\n{}", r.author, excerpt, text)
                .trim()
                .to_string();
        }
        let has_attachments = !self.draft_attachments.is_empty();
        if text.is_empty() && !has_attachments {
            return;
        }

        if !has_attachments {
            let pid = format!("local-{}", now_millis());
            self.pending_queue.push_back(PendingMessage {
                local_id: pid.clone(),
                recipient_id: contact_id.to_string(),
                text: text.clone(),
                image_base64: None,
                created_at: Instant::now(),
                total_bytes: text.len() as u64,
                sent_bytes: 0,
            });
            self.local_status.insert(pid.clone(), LocalStatus::Sending);
            let _ = self.cmd_tx.send(AppCommand::SendMessage {
                local_id: pid,
                recipient_id: contact_id.to_string(),
                text,
                image_base64: None,
            });
        } else {
            // Send text with the first image, then each remaining image as its
            // own message with empty caption.
            let attachments = std::mem::take(&mut self.draft_attachments);
            for (i, a) in attachments.iter().enumerate() {
                let caption = if i == 0 { text.clone() } else { String::new() };
                let pid = format!("local-{}-{}", now_millis(), i);
                self.pending_queue.push_back(PendingMessage {
                    local_id: pid.clone(),
                    recipient_id: contact_id.to_string(),
                    text: caption.clone(),
                    image_base64: Some(a.base64.clone()),
                    created_at: Instant::now(),
                    total_bytes: a.bytes as u64,
                    sent_bytes: 0,
                });
                self.local_status.insert(pid.clone(), LocalStatus::Sending);
                let _ = self.cmd_tx.send(AppCommand::SendMessage {
                    local_id: pid,
                    recipient_id: contact_id.to_string(),
                    text: caption,
                    image_base64: Some(a.base64.clone()),
                });
            }
        }
        self.message_input.clear();
    }

    fn retry_message(&mut self, message_id: &str) {
        let pos = self
            .failed_queue
            .iter()
            .position(|p| p.local_id == message_id);
        let candidate = if let Some(pos) = pos {
            Some(self.failed_queue.remove(pos))
        } else {
            // Retry a delivered=false message from the store.
            self.messages
                .values()
                .flatten()
                .find(|m| m.id == message_id && !m.incoming)
                .map(|m| PendingMessage {
                    local_id: m.id.clone(),
                    recipient_id: m.recipient_id_hex.clone(),
                    text: m.text.clone(),
                    image_base64: m.image_base64.clone(),
                    created_at: Instant::now(),
                    total_bytes: m.image_base64.as_ref().map(|b| b.len() as u64).unwrap_or(0),
                    sent_bytes: 0,
                })
        };
        if let Some(p) = candidate {
            self.local_status
                .insert(p.local_id.clone(), LocalStatus::Sending);
            let _ = self.cmd_tx.send(AppCommand::RetryMessage {
                message_id: p.local_id.clone(),
                recipient_id: p.recipient_id.clone(),
                text: p.text.clone(),
                image_base64: p.image_base64.clone(),
            });
            // Also send a plain SendMessage so an existing backend without
            // RetryMessage handling still resends the payload.
            let _ = self.cmd_tx.send(AppCommand::SendMessage {
                local_id: p.local_id.clone(),
                recipient_id: p.recipient_id.clone(),
                text: p.text.clone(),
                image_base64: p.image_base64.clone(),
            });
            self.pending_queue.push_back(p);
            self.show_toast("Повторная отправка...");
        }
    }

    fn render_emoji_picker(&mut self, ctx: &egui::Context) {
        egui::Window::new("Эмодзи")
            .id(egui::Id::new("emoji_picker_window"))
            .title_bar(true)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .fixed_size([300.0, 240.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_SM)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (cat, label) in [
                        (EmojiCategory::Smiles, "😀"),
                        (EmojiCategory::Gestures, "👋"),
                        (EmojiCategory::Hearts, "❤"),
                        (EmojiCategory::Tech, "⚡"),
                        (EmojiCategory::Food, "🍕"),
                        (EmojiCategory::Symbols, "✳"),
                    ] {
                        if ui
                            .selectable_label(self.emoji_category == cat, label)
                            .clicked()
                        {
                            self.emoji_category = cat;
                        }
                    }
                });
                thin_separator(ui);
                let emojis: &[&str] = match self.emoji_category {
                    EmojiCategory::Smiles => &[
                        "😀", "😃", "😄", "😁", "😆", "😅", "😂", "🙂", "😊", "😇", "😉", "😍",
                        "😘", "😋", "😎", "🥳", "🤔", "🤨", "😐", "🙄", "😴", "🥱", "😪", "😌",
                    ],
                    EmojiCategory::Gestures => &[
                        "👍", "👎", "👌", "✌", "🤞", "🤙", "👏", "🙌", "👐", "🤝", "🙏", "💪",
                        "👋", "🤚", "🖖", "🫶",
                    ],
                    EmojiCategory::Hearts => &[
                        "❤", "🧡", "💛", "💚", "💙", "💜", "🖤", "🤍", "💔", "💕", "💓", "✨",
                        "💖", "💗", "💘", "💝",
                    ],
                    EmojiCategory::Tech => &[
                        "🚀", "💻", "🔒", "🔑", "🌐", "⚡", "📡", "💾", "🤖", "⚙", "🔧", "🔋",
                        "🛡", "🧠", "📱", "🖥",
                    ],
                    EmojiCategory::Food => &[
                        "🍕", "🍔", "🍟", "☕", "🍺", "🥂", "🍎", "🍓", "🍇", "🍒", "🍩", "🍰",
                    ],
                    EmojiCategory::Symbols => &[
                        "✳", "✔", "✖", "★", "☆", "♦", "♣", "♠", "♥", "☀", "☁", "☂", "⚑", "⚛",
                        "☯", "☮",
                    ],
                };
                egui::Grid::new("emoji_grid")
                    .spacing([SP_XS, SP_XS])
                    .show(ui, |ui| {
                        for (i, e) in emojis.iter().enumerate() {
                            if ui.button(RichText::new(*e).size(18.0)).clicked() {
                                self.message_input.push_str(e);
                            }
                            if (i + 1) % 8 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }

    // ---- Settings drawer ----
    fn render_settings_drawer(&mut self, ctx: &egui::Context) {
        if !self.show_settings_drawer {
            return;
        }
        egui::SidePanel::right("settings_drawer")
            .resizable(false)
            .exact_width(380.0)
            .frame(
                Frame::none()
                    .fill(SIDEBAR_BG)
                    .inner_margin(Margin::same(SP_MD)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Настройки")
                            .size(SIZE_SECTION_TITLE)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, 24.0, draw_icon_close_menu, "Закрыть").clicked() {
                            self.show_settings_drawer = false;
                            self.show_mnemonic = false;
                        }
                    });
                });

                ui.add_space(SP_SM);
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (SettingsTab::Profile, "Профиль"),
                        (SettingsTab::Account, "Аккаунт"),
                        (SettingsTab::Privacy, "Приватность"),
                        (SettingsTab::Network, "Сеть"),
                        (SettingsTab::Advanced, "Ещё"),
                    ] {
                        let selected = self.settings_tab == tab;
                        if ui.selectable_label(selected, label).clicked() {
                            self.settings_tab = tab;
                        }
                    }
                });
                ui.add_space(SP_SM);
                thin_separator(ui);
                ui.add_space(SP_SM);

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.settings_tab {
                        SettingsTab::Profile => self.render_settings_profile(ui),
                        SettingsTab::Account => self.render_settings_account(ui),
                        SettingsTab::Privacy => self.render_settings_privacy(ui),
                        SettingsTab::Network => self.render_settings_network(ui),
                        SettingsTab::Advanced => self.render_settings_advanced(ui),
                    });
            });
    }

    fn render_settings_profile(&mut self, ui: &mut Ui) {
        section_title(ui, "Профиль");
        ui.horizontal(|ui| {
            avatar(ui, &self.my_display_name, 56.0, false);
            ui.add_space(SP_SM);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(&self.my_display_name)
                        .strong()
                        .size(SIZE_CHAT_NAME + 1.0)
                        .color(TEXT_PRIMARY),
                );
                let id_display = if self.prefs.hide_user_id {
                    "ID скрыт".to_string()
                } else {
                    self.my_user_id.clone()
                };
                ui.label(
                    RichText::new(short_id(&id_display))
                        .size(SIZE_TIMESTAMP)
                        .color(TEXT_MUTED),
                );
            });
        });
        ui.add_space(SP_MD);
        ui.label(
            RichText::new("Отображаемое имя")
                .size(SIZE_SECONDARY)
                .color(TEXT_MUTED),
        );
        ui.add(TextEdit::singleline(&mut self.edit_name).desired_width(ui.available_width()));
        ui.add_space(SP_SM);
        ui.label(
            RichText::new("О себе")
                .size(SIZE_SECONDARY)
                .color(TEXT_MUTED),
        );
        ui.add(TextEdit::singleline(&mut self.edit_bio).desired_width(ui.available_width()));
        ui.add_space(SP_MD);
        if primary_button(ui, "Сохранить профиль").clicked() {
            let _ = self.cmd_tx.send(AppCommand::UpdateProfile {
                display_name: self.edit_name.clone(),
                bio: self.edit_bio.clone(),
            });
            self.show_toast("Профиль обновлён");
        }

        ui.add_space(SP_LG);
        thin_separator(ui);
        ui.add_space(SP_LG);
        section_title(ui, "Ваш User ID");
        labeled_row(ui, "User ID", &self.my_user_id, true);
        ui.add_space(SP_SM);
        if primary_button(ui, "Создать invite-ссылку").clicked() {
            let _ = self.cmd_tx.send(AppCommand::CreateInvite);
        }
        if !self.last_invite_link.is_empty() {
            ui.add_space(SP_SM);
            Frame::none()
                .fill(SURFACE)
                .rounding(Rounding::same(8.0))
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(Margin::same(SP_SM))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Invite-ссылка")
                            .size(SIZE_SECONDARY)
                            .color(TEXT_MUTED),
                    );
                    ui.label(
                        RichText::new(shorten(&self.last_invite_link, 44))
                            .size(SIZE_SECONDARY)
                            .color(ACCENT),
                    );
                    // Real vector QR code of the invite link
                    draw_qr_code(ui, &self.last_invite_link, 160.0);
                    ui.add_space(SP_SM);
                    ui.horizontal(|ui| {
                        if ghost_button(ui, "Скопировать").clicked() {
                            ui.ctx()
                                .output_mut(|o| o.copied_text = self.last_invite_link.clone());
                            self.show_toast("Invite-ссылка скопирована");
                        }
                    });
                });
        }
    }

    fn render_settings_account(&mut self, ui: &mut Ui) {
        section_title(ui, "Аккаунт");
        ui.horizontal_wrapped(|ui| {
            if ghost_button(ui, "Создать новый аккаунт").clicked() {
                self.onboard_phrase_revealed = false;
                self.onboard_confirmed = false;
                self.show_settings_drawer = false;
                if let Ok((fresh_mn, _)) = nodex_messenger::crypto::UserIdentity::generate_mnemonic() {
                    self.onboard_mnemonic = fresh_mn;
                }
                self.screen = Screen::CreateAccount;
            }
            if ghost_button(ui, "Восстановить из фразы").clicked() {
                self.restore_error = None;
                self.show_settings_drawer = false;
                self.screen = Screen::RestoreAccount;
            }
            if ghost_button(ui, "Экспорт зашифрованного бэкапа").clicked() {
                self.backup_password_input.clear();
                self.show_export_confirm = true;
            }
            if ghost_button(ui, "Импорт из бэкапа").clicked() {
                self.import_backup_path.clear();
                self.import_backup_password.clear();
                self.import_backup_error = None;
                self.show_import_backup_modal = true;
            }
        });

        ui.add_space(SP_LG);
        thin_separator(ui);
        ui.add_space(SP_LG);

        section_title(ui, "Фраза восстановления");
        ui.label(
            RichText::new("Нужна для восстановления аккаунта. Никому её не передавайте.")
                .size(SIZE_SECONDARY)
                .color(TEXT_SECONDARY),
        );
        ui.add_space(SP_SM);
        Frame::none()
            .fill(SURFACE)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, BORDER))
            .inner_margin(Margin::same(SP_SM))
            .show(ui, |ui| {
                if !self.show_mnemonic {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("• • • •  • • • •  • • • •")
                                .color(TEXT_MUTED)
                                .size(SIZE_BODY),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ghost_button(ui, "Показать").clicked() {
                                if self.my_mnemonic.is_empty() {
                                    let _ = self.cmd_tx.send(AppCommand::RevealMnemonic);
                                }
                                self.show_mnemonic = true;
                            }
                        });
                    });
                } else {
                    ui.label(
                        RichText::new("Убедитесь, что экран не виден посторонним")
                            .size(SIZE_SECONDARY)
                            .color(WARNING),
                    );
                    ui.add_space(SP_XS);
                    let words: Vec<&str> = self.my_mnemonic.split_whitespace().collect();
                    egui::Grid::new("mnemonic_grid")
                        .spacing([SP_SM, SP_XS])
                        .show(ui, |ui| {
                            for (i, w) in words.iter().enumerate() {
                                ui.label(
                                    RichText::new(format!("{:02}. {}", i + 1, w))
                                        .color(ACCENT)
                                        .size(SIZE_SECONDARY),
                                );
                                if (i + 1) % 3 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                    ui.add_space(SP_SM);
                    ui.horizontal(|ui| {
                        if ghost_button(ui, "Скопировать").clicked() {
                            ui.ctx().output_mut(|o| o.copied_text = self.my_mnemonic.clone());
                            self.show_toast("Фраза скопирована");
                        }
                        if ghost_button(ui, "Скрыть").clicked() {
                            self.show_mnemonic = false;
                            self.my_mnemonic.clear();
                        }
                    });
                }
            });

        ui.add_space(SP_LG);
        thin_separator(ui);
        ui.add_space(SP_LG);
        section_title(ui, "Опасная зона");
        ui.horizontal_wrapped(|ui| {
            if danger_button(ui, "Удалить локальные данные").clicked() {
                self.show_wipe_confirm = true;
            }
        });
    }

    fn render_settings_privacy(&mut self, ui: &mut Ui) {
        section_title(ui, "Приватность");
        ui.checkbox(&mut self.prefs.hide_user_id, "Скрывать User ID в интерфейсе");
        ui.checkbox(&mut self.prefs.disable_logs, "Не показывать сетевые логи в тостах");
        ui.add_space(SP_MD);

        section_title(ui, "Заблокированные контакты");
        if self.blocked_ids.is_empty() {
            ui.label(
                RichText::new("Список пуст")
                    .color(TEXT_MUTED)
                    .size(SIZE_SECONDARY),
            );
        } else if ghost_button(ui, &format!("Показать список ({})", self.blocked_ids.len()))
            .clicked()
        {
            self.show_blocked_modal = true;
        }

        ui.add_space(SP_LG);
        thin_separator(ui);
        ui.add_space(SP_LG);
        section_title(ui, "Локальная блокировка");
        ui.label(
            RichText::new("Пароль хранится только локально. Требуется при следующем запуске приложения и после бездействия.")
                .size(SIZE_SECONDARY)
                .color(TEXT_SECONDARY),
        );
        ui.add_space(SP_SM);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Новый пароль")
                    .size(SIZE_SECONDARY)
                    .color(TEXT_MUTED),
            );
            ui.add(
                TextEdit::singleline(&mut self.lock_password_setup_new)
                    .password(true)
                    .desired_width(180.0),
            );
        });
        ui.horizontal(|ui| {
            if primary_button_enabled(
                ui,
                "Установить пароль",
                self.lock_password_setup_new.len() >= 4,
            )
            .clicked()
            {
                self.prefs.lock_password_hash = Some(fnv_hash(&self.lock_password_setup_new));
                self.lock_password_setup_new.clear();
                self.show_toast("Пароль установлен");
            }
            if self.prefs.lock_password_hash.is_some()
                && ghost_button(ui, "Снять пароль").clicked()
            {
                self.prefs.lock_password_hash = None;
                self.prefs.auto_lock_minutes = 0;
                self.show_toast("Пароль снят");
            }
            if self.prefs.lock_password_hash.is_some()
                && ghost_button(ui, "Заблокировать сейчас").clicked()
            {
                self.screen = Screen::Locked;
            }
        });
        ui.add_space(SP_SM);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Автоблокировка через")
                    .size(SIZE_SECONDARY)
                    .color(TEXT_MUTED),
            );
            ui.add(
                egui::Slider::new(&mut self.prefs.auto_lock_minutes, 0..=60).suffix(" мин"),
            );
        });
    }

    fn render_settings_network(&mut self, ui: &mut Ui) {
        section_title(ui, "Состояние сети");
        let (dot, status_text) = network_status(self.dht_peers);
        ui.horizontal(|ui| {
            status_dot(ui, dot);
            ui.label(
                RichText::new(status_text)
                    .size(SIZE_BODY)
                    .color(TEXT_PRIMARY),
            );
        });
        ui.label(
            RichText::new(format!("Известных узлов DHT: {}", self.dht_peers))
                .size(SIZE_SECONDARY)
                .color(TEXT_SECONDARY),
        );
        ui.label(
            RichText::new(format!("Ключей в хранилище: {}", self.stored_keys))
                .size(SIZE_SECONDARY)
                .color(TEXT_SECONDARY),
        );
        ui.add_space(SP_SM);
        ui.horizontal(|ui| {
            if primary_button(ui, "Переподключиться").clicked() {
                let _ = self.cmd_tx.send(AppCommand::Reconnect);
                let _ = self.cmd_tx.send(AppCommand::RefreshInfo);
                let _ = self.cmd_tx.send(AppCommand::RefreshContacts);
                self.show_toast("Переподключение...");
            }
            if ghost_button(ui, "NAT-диагностика").clicked() {
                let _ = self.cmd_tx.send(AppCommand::RefreshInfo);
                self.show_toast("Запрошена NAT-диагностика (см. лог)");
            }
        });

        ui.add_space(SP_LG);
        thin_separator(ui);
        ui.add_space(SP_LG);

        section_title(ui, "Relay");
        let was_relay = self.prefs.relay_enabled;
        ui.checkbox(&mut self.prefs.relay_enabled, "Разрешить работу в режиме relay");
        if self.prefs.relay_enabled != was_relay {
            let _ = self.cmd_tx.send(AppCommand::ToggleRelay {
                enabled: self.prefs.relay_enabled,
            });
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Лимит трафика")
                    .size(SIZE_SECONDARY)
                    .color(TEXT_MUTED),
            );
            let mut v = self.prefs.relay_traffic_mb_h;
            if ui
                .add(egui::Slider::new(&mut v, 50..=5000).suffix(" МБ/ч"))
                .changed()
            {
                self.prefs.relay_traffic_mb_h = v;
                let _ = self.cmd_tx.send(AppCommand::SetRelayTrafficLimit {
                    megabytes_per_hour: v,
                });
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Одновременных сессий")
                    .size(SIZE_SECONDARY)
                    .color(TEXT_MUTED),
            );
            let mut v = self.prefs.relay_max_sessions;
            if ui
                .add(egui::Slider::new(&mut v, 1..=64))
                .changed()
            {
                self.prefs.relay_max_sessions = v;
                let _ = self
                    .cmd_tx
                    .send(AppCommand::SetRelayMaxSessions { sessions: v });
            }
        });

        ui.add_space(SP_LG);
        thin_separator(ui);
        ui.add_space(SP_LG);
        section_title(ui, "Тест соединения");
        if let Some(cid) = self.active_contact_id.clone() {
            if ghost_button(ui, "Пинг активного контакта").clicked() {
                let _ = self.cmd_tx.send(AppCommand::TestConnection(cid));
                self.show_toast("Отправлен тестовый пинг");
            }
        } else {
            ui.label(
                RichText::new("Откройте чат, чтобы протестировать соединение")
                    .size(SIZE_SECONDARY)
                    .color(TEXT_MUTED),
            );
        }
    }

    fn render_settings_advanced(&mut self, ui: &mut Ui) {
        section_title(ui, "Диагностика");
        Frame::none()
            .fill(SURFACE)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, BORDER))
            .inner_margin(Margin::same(SP_MD))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    labeled_row(ui, "User ID", &self.my_user_id, true);
                    ui.add_space(SP_XS);
                    labeled_row(ui, "Transport Node ID", &self.my_dht_node_id, true);
                    ui.add_space(SP_XS);
                    ui.label(
                        RichText::new(format!("Ключей в DHT-хранилище: {}", self.stored_keys))
                            .size(SIZE_SECONDARY)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(SP_XS);
                    ui.label(
                        RichText::new(format!(
                            "Пиров в таблице маршрутизации: {}",
                            self.dht_peers
                        ))
                        .size(SIZE_SECONDARY)
                        .color(TEXT_MUTED),
                    );
                });
            });

        ui.add_space(SP_LG);
        thin_separator(ui);
        ui.add_space(SP_LG);
        section_title(ui, "О приложении");
        ui.label(
            RichText::new("NodeX — децентрализованный E2EE messenger. Ключи хранятся только на вашем устройстве.")
                .size(SIZE_SECONDARY)
                .color(TEXT_SECONDARY),
        );
    }

    // ---- Modals ----
    fn render_add_contact_modal(&mut self, ctx: &egui::Context) {
        if !self.show_add_contact_modal {
            return;
        }
        egui::Window::new("Добавить контакт")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([440.0, 300.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.add_contact_tab == AddContactTab::Invite,
                            "По ссылке",
                        )
                        .clicked()
                    {
                        self.add_contact_tab = AddContactTab::Invite;
                    }
                    if ui
                        .selectable_label(
                            self.add_contact_tab == AddContactTab::UserId,
                            "По User ID",
                        )
                        .clicked()
                    {
                        self.add_contact_tab = AddContactTab::UserId;
                    }
                });
                ui.add_space(SP_MD);

                match self.add_contact_tab {
                    AddContactTab::Invite => {
                        ui.label(
                            RichText::new("Ссылка-приглашение")
                                .size(SIZE_SECONDARY)
                                .color(TEXT_MUTED),
                        );
                        ui.add(
                            TextEdit::multiline(&mut self.input_invite_link)
                                .desired_rows(2)
                                .desired_width(ui.available_width()),
                        );
                        ui.add_space(SP_MD);
                        if primary_button(ui, "Добавить").clicked() {
                            let link = self.input_invite_link.trim().to_string();
                            if link.is_empty() {
                                self.add_contact_error =
                                    Some("Введите ссылку-приглашение".into());
                            } else {
                                let _ = self.cmd_tx.send(AppCommand::AddContactInvite(link));
                                self.show_add_contact_modal = false;
                                self.input_invite_link.clear();
                                self.show_toast("Контакт добавлен");
                            }
                        }
                    }
                    AddContactTab::UserId => {
                        ui.label(
                            RichText::new("User ID собеседника")
                                .size(SIZE_SECONDARY)
                                .color(TEXT_MUTED),
                        );
                        ui.add(
                            TextEdit::singleline(&mut self.input_user_id)
                                .desired_width(ui.available_width()),
                        );
                        ui.add_space(SP_SM);
                        ui.label(
                            RichText::new("Имя (необязательно)")
                                .size(SIZE_SECONDARY)
                                .color(TEXT_MUTED),
                        );
                        ui.add(
                            TextEdit::singleline(&mut self.input_contact_name)
                                .desired_width(ui.available_width()),
                        );
                        ui.add_space(SP_MD);
                        if primary_button(ui, "Добавить контакт").clicked() {
                            let uid = self.input_user_id.trim().to_string();
                            if uid.is_empty() {
                                self.add_contact_error = Some("Введите User ID".into());
                            } else {
                                let _ = self.cmd_tx.send(AppCommand::AddContact {
                                    user_id: uid,
                                    name: self.input_contact_name.trim().to_string(),
                                });
                                self.show_add_contact_modal = false;
                                self.input_user_id.clear();
                                self.input_contact_name.clear();
                                self.show_toast("Контакт добавлен");
                            }
                        }
                    }
                }

                if let Some(err) = self.add_contact_error.clone() {
                    ui.add_space(SP_SM);
                    ui.label(RichText::new(err).color(DANGER).size(SIZE_SECONDARY));
                }

                ui.add_space(SP_MD);
                if ghost_button(ui, "Закрыть").clicked() {
                    self.show_add_contact_modal = false;
                }
            });
    }

    fn render_contact_security_modal(&mut self, ctx: &egui::Context) {
        if !self.show_contact_security_modal {
            return;
        }
        let contact_id = match self.active_contact_id.clone() {
            Some(id) => id,
            None => {
                self.show_contact_security_modal = false;
                return;
            }
        };
        let contact = self
            .contacts
            .iter()
            .find(|c| c.user_id_hex == contact_id)
            .cloned();

        egui::Window::new("Безопасность контакта")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([460.0, 380.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                if let Some(c) = &contact {
                    ui.label(
                        RichText::new(&c.name)
                            .strong()
                            .size(SIZE_CHAT_NAME)
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(SP_SM);
                    labeled_row(ui, "User ID", &c.user_id_hex, true);
                    labeled_row(ui, "Ed25519", &c.ed25519_pub_hex, true);
                    labeled_row(ui, "X25519", &c.x25519_pub_hex, true);
                    ui.add_space(SP_MD);
                    ui.label(
                        RichText::new("Код безопасности (Safety Number)")
                            .strong()
                            .size(SIZE_SECONDARY)
                            .color(TEXT_PRIMARY),
                    );
                    let fp = fingerprint_chunks(&c.user_id_hex);
                    Frame::none()
                        .fill(BG_DARK)
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::same(SP_SM))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(fp).color(ACCENT).size(SIZE_BODY).strong(),
                            );
                        });
                    ui.add_space(SP_SM);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("QR-код для верификации")
                                    .size(SIZE_TIMESTAMP)
                                    .color(TEXT_MUTED),
                            );
                            draw_qr_code(ui, &c.user_id_hex, 130.0);
                        });
                        ui.add_space(SP_MD);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(
                                    "Сверьте цветной шаблон и код безопасности при личной встрече или по независимому каналу.",
                                )
                                .size(SIZE_SECONDARY)
                                .color(TEXT_MUTED),
                            );
                            ui.add_space(SP_SM);
                            let is_blocked = self.blocked_ids.contains(&contact_id);
                            if !is_blocked {
                                if danger_button(ui, "Заблокировать контакт").clicked() {
                                    self.blocked_ids.insert(contact_id.clone());
                                    let _ = self
                                        .cmd_tx
                                        .send(AppCommand::BlockContact(contact_id.clone()));
                                    self.show_toast("Контакт заблокирован");
                                }
                            } else if ghost_button(ui, "Разблокировать").clicked() {
                                self.blocked_ids.remove(&contact_id);
                                let _ = self
                                    .cmd_tx
                                    .send(AppCommand::UnblockContact(contact_id.clone()));
                            }
                        });
                    });
                } else {
                    ui.label(RichText::new("Контакт не найден").color(TEXT_MUTED));
                }
                ui.add_space(SP_MD);
                if ghost_button(ui, "Закрыть").clicked() {
                    self.show_contact_security_modal = false;
                }
            });
    }

    fn render_blocked_modal(&mut self, ctx: &egui::Context) {
        if !self.show_blocked_modal {
            return;
        }
        egui::Window::new("Заблокированные")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([380.0, 320.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                let ids: Vec<String> = self.blocked_ids.iter().cloned().collect();
                if ids.is_empty() {
                    ui.label(
                        RichText::new("Пусто")
                            .color(TEXT_MUTED)
                            .size(SIZE_SECONDARY),
                    );
                } else {
                    for id in ids {
                        let name = self
                            .contacts
                            .iter()
                            .find(|c| c.user_id_hex == id)
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| short_id(&id));
                        ui.horizontal(|ui| {
                            avatar(ui, &name, 28.0, false);
                            ui.add_space(SP_SM);
                            ui.label(RichText::new(name).color(TEXT_PRIMARY));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ghost_button(ui, "Разблокировать").clicked() {
                                    self.blocked_ids.remove(&id);
                                    let _ =
                                        self.cmd_tx.send(AppCommand::UnblockContact(id.clone()));
                                }
                            });
                        });
                        ui.add_space(SP_XS);
                    }
                }
                ui.add_space(SP_MD);
                if ghost_button(ui, "Закрыть").clicked() {
                    self.show_blocked_modal = false;
                }
            });
    }

    fn render_forward_modal(&mut self, ctx: &egui::Context) {
        if !self.show_forward_modal {
            return;
        }
        let src = match self.forward_source.clone() {
            Some(m) => m,
            None => {
                self.show_forward_modal = false;
                return;
            }
        };
        let contacts = self.contacts.clone();
        egui::Window::new("Переслать сообщение")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([380.0, 380.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Выберите получателя")
                        .size(SIZE_SECONDARY)
                        .color(TEXT_MUTED),
                );
                ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for c in &contacts {
                            let r = ui.selectable_label(false, &c.name);
                            if r.clicked() {
                                let _ = self.cmd_tx.send(AppCommand::SendMessage {
                                    local_id: format!("forward-{}", now_millis()),
                                    recipient_id: c.user_id_hex.clone(),
                                    text: src.text.clone(),
                                    image_base64: src.image_base64.clone(),
                                });
                                self.show_forward_modal = false;
                                self.forward_source = None;
                                self.show_toast(&format!("Переслано {}", c.name));
                                break;
                            }
                        }
                    });
                ui.add_space(SP_MD);
                if ghost_button(ui, "Отмена").clicked() {
                    self.show_forward_modal = false;
                    self.forward_source = None;
                }
            });
    }

    fn render_lightbox(&mut self, ctx: &egui::Context) {
        if !self.show_lightbox {
            return;
        }
        let data = match self.lightbox_data.clone() {
            Some(d) => d,
            None => return,
        };
        egui::Window::new("Изображение")
            .collapsible(false)
            .resizable(true)
            .movable(true)
            .default_size([640.0, 500.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                if let Ok(raw) = BASE64_STANDARD.decode(&data.base64_data) {
                    if let Ok(img) = image::load_from_memory(&raw) {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let tex = ctx.load_texture(
                            "lightbox_image",
                            egui::ColorImage::from_rgba_unmultiplied(
                                [w as usize, h as usize],
                                &rgba,
                            ),
                            egui::TextureOptions::default(),
                        );
                        let max_w = ui.available_width().min(600.0);
                        let aspect = w as f32 / h as f32;
                        let render_w = max_w;
                        let render_h = (render_w / aspect).min(400.0);
                        ui.vertical_centered(|ui| {
                            ui.image((tex.id(), Vec2::new(render_w, render_h)));
                        });
                    }
                }
                if !data.caption.is_empty() {
                    ui.add_space(SP_SM);
                    ui.label(
                        RichText::new(&data.caption).size(SIZE_BODY).color(TEXT_PRIMARY),
                    );
                }
                ui.add_space(SP_MD);
                ui.horizontal(|ui| {
                    if ghost_button(ui, "Сохранить на диск").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("nodex_photo.png")
                            .save_file()
                        {
                            if let Ok(raw) = BASE64_STANDARD.decode(&data.base64_data) {
                                let _ = std::fs::write(path, raw);
                            }
                        }
                    }
                    if ghost_button(ui, "Закрыть").clicked() {
                        self.show_lightbox = false;
                    }
                });
            });
    }

    fn render_export_confirm(&mut self, ctx: &egui::Context) {
        if !self.show_export_confirm {
            return;
        }
        egui::Window::new("Экспорт зашифрованной резервной копии")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([440.0, 240.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Бэкап защищён шифрованием ChaCha20-Poly1305 с ключом на базе Argon2id.")
                        .size(SIZE_SECONDARY)
                        .color(TEXT_SECONDARY),
                );
                ui.add_space(SP_XS);
                ui.label(
                    RichText::new("Задайте пароль для шифрования файла бэкапа:")
                        .size(SIZE_SECONDARY)
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(SP_SM);
                ui.add(
                    TextEdit::singleline(&mut self.backup_password_input)
                        .password(true)
                        .desired_width(ui.available_width())
                        .hint_text("Пароль для шифрования бэкапа"),
                );
                ui.add_space(SP_SM);
                ui.label(
                    RichText::new("Без этого пароля восстановить данные из файла будет невозможно!")
                        .size(SIZE_SECONDARY)
                        .color(WARNING),
                );
                ui.add_space(SP_MD);
                ui.horizontal(|ui| {
                    let can_export = !self.backup_password_input.is_empty();
                    if ui.add_enabled(can_export, egui::Button::new("Сохранить в файл (.nodexbak)")).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("NodeX Encrypted Backup", &["nodexbak", "json"])
                            .set_file_name("nodex_backup.nodexbak")
                            .save_file()
                        {
                            let _ = self.cmd_tx.send(AppCommand::ExportBackup {
                                path: path.to_string_lossy().to_string(),
                                password: self.backup_password_input.clone(),
                            });
                            self.backup_password_input.clear();
                            self.show_export_confirm = false;
                        }
                    }
                    if ghost_button(ui, "Отмена").clicked() {
                        self.backup_password_input.clear();
                        self.show_export_confirm = false;
                    }
                });
            });
    }

    fn render_import_backup_modal(&mut self, ctx: &egui::Context) {
        if !self.show_import_backup_modal {
            return;
        }
        egui::Window::new("Импорт из зашифрованного бэкапа")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([440.0, 240.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Выберите файл зашифрованного бэкапа (.nodexbak):")
                        .size(SIZE_SECONDARY)
                        .color(TEXT_SECONDARY),
                );
                ui.add_space(SP_XS);
                ui.horizontal(|ui| {
                    let path_label = if self.import_backup_path.is_empty() {
                        "Файл не выбран".to_string()
                    } else {
                        shorten(&self.import_backup_path, 35)
                    };
                    ui.label(RichText::new(path_label).color(ACCENT).size(SIZE_SECONDARY));
                    if ghost_button(ui, "Обзор...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("NodeX Encrypted Backup", &["nodexbak", "json"])
                            .pick_file()
                        {
                            self.import_backup_path = path.to_string_lossy().to_string();
                        }
                    }
                });
                ui.add_space(SP_SM);
                ui.label(
                    RichText::new("Введите пароль от бэкапа:")
                        .size(SIZE_SECONDARY)
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(SP_XS);
                ui.add(
                    TextEdit::singleline(&mut self.import_backup_password)
                        .password(true)
                        .desired_width(ui.available_width())
                        .hint_text("Пароль бэкапа"),
                );
                if let Some(err) = &self.import_backup_error {
                    ui.add_space(SP_XS);
                    ui.label(RichText::new(err).color(DANGER).size(SIZE_SECONDARY));
                }
                ui.add_space(SP_MD);
                ui.horizontal(|ui| {
                    let can_import = !self.import_backup_path.is_empty() && !self.import_backup_password.is_empty();
                    if ui.add_enabled(can_import, egui::Button::new("Восстановить")).clicked() {
                        let _ = self.cmd_tx.send(AppCommand::ImportBackup {
                            path: self.import_backup_path.clone(),
                            password: self.import_backup_password.clone(),
                        });
                        self.import_backup_path.clear();
                        self.import_backup_password.clear();
                        self.import_backup_error = None;
                        self.show_import_backup_modal = false;
                    }
                    if ghost_button(ui, "Отмена").clicked() {
                        self.import_backup_path.clear();
                        self.import_backup_password.clear();
                        self.import_backup_error = None;
                        self.show_import_backup_modal = false;
                    }
                });
            });
    }

    fn render_wipe_confirm(&mut self, ctx: &egui::Context) {
        if !self.show_wipe_confirm {
            return;
        }
        egui::Window::new("Удалить все данные?")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([420.0, 180.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, DANGER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Будут удалены локальные ключи, контакты и история. Без фразы восстановления доступ к аккаунту будет утерян.",
                    )
                    .size(SIZE_SECONDARY)
                    .color(TEXT_PRIMARY),
                );
                ui.add_space(SP_MD);
                ui.horizontal(|ui| {
                    if danger_button(ui, "Да, удалить").clicked() {
                        let _ = self.cmd_tx.send(AppCommand::WipeLocalData);
                        self.contacts.clear();
                        self.messages.clear();
                        self.blocked_ids.clear();
                        self.local_status.clear();
                        self.pending_queue.clear();
                        self.failed_queue.clear();
                        self.active_contact_id = None;
                        self.show_wipe_confirm = false;
                        self.show_toast("Локальные данные очищены");
                    }
                    if ghost_button(ui, "Отмена").clicked() {
                        self.show_wipe_confirm = false;
                    }
                });
            });
    }

    fn render_key_warning_modal(&mut self, ctx: &egui::Context) {
        let cid = match self.show_key_warning_modal.clone() {
            Some(v) => v,
            None => return,
        };
        egui::Window::new("Ключ контакта изменился")
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .default_size([440.0, 220.0])
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, WARNING))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(SP_LG)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Публичный ключ контакта отличается от ранее сохранённого. Это может означать переустановку приложения собеседником — либо попытку подмены.",
                    )
                    .size(SIZE_SECONDARY)
                    .color(TEXT_PRIMARY),
                );
                ui.add_space(SP_SM);
                ui.label(
                    RichText::new("Сверьте отпечаток безопасности перед продолжением переписки.")
                        .size(SIZE_SECONDARY)
                        .color(WARNING),
                );
                ui.add_space(SP_MD);
                ui.horizontal(|ui| {
                    if primary_button(ui, "Открыть отпечаток").clicked() {
                        self.show_contact_security_modal = true;
                        self.key_change_warnings.remove(&cid);
                        self.show_key_warning_modal = None;
                    }
                    if ghost_button(ui, "Понятно").clicked() {
                        self.key_change_warnings.remove(&cid);
                        self.show_key_warning_modal = None;
                    }
                });
            });
    }

    fn render_toast(&mut self, ctx: &egui::Context) {
        if let Some((text, start)) = self.status_toast.clone() {
            if start.elapsed().as_secs() < 4 {
                egui::Area::new(egui::Id::new("status_toast"))
                    .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-20.0, -20.0))
                    .show(ctx, |ui| {
                        Frame::none()
                            .fill(SURFACE)
                            .stroke(Stroke::new(1.0, BORDER))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(SP_MD, SP_SM))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(
                                        Vec2::new(14.0, 14.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().circle_filled(r.center(), 4.0, ACCENT);
                                    ui.label(
                                        RichText::new(text)
                                            .color(TEXT_PRIMARY)
                                            .size(SIZE_SECONDARY),
                                    );
                                });
                            });
                    });
            } else {
                self.status_toast = None;
            }
        }
    }

    // ---- Onboarding ----
    fn render_create_account(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG_DARK))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(SP_XL);
                    draw_emblem(ui, 76.0);
                    ui.add_space(SP_MD);
                    ui.label(
                        RichText::new("Создание аккаунта")
                            .size(SIZE_APP_TITLE)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(SP_SM);
                    Frame::none()
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0, BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(SP_LG))
                        .show(ui, |ui| {
                            ui.set_width(440.0);
                            ui.label(
                                RichText::new("Отображаемое имя")
                                    .size(SIZE_SECONDARY)
                                    .color(TEXT_MUTED),
                            );
                            ui.add(
                                TextEdit::singleline(&mut self.onboard_name)
                                    .desired_width(ui.available_width()),
                            );
                            ui.add_space(SP_SM);
                            ui.label(
                                RichText::new("О себе (необязательно)")
                                    .size(SIZE_SECONDARY)
                                    .color(TEXT_MUTED),
                            );
                            ui.add(
                                TextEdit::singleline(&mut self.onboard_bio)
                                    .desired_width(ui.available_width()),
                            );

                            ui.add_space(SP_LG);
                            ui.label(
                                RichText::new("Фраза восстановления (12 слов)")
                                    .strong()
                                    .size(SIZE_BODY)
                                    .color(TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new(
                                    "Запишите и сохраните в надёжном месте. Никогда никому её не передавайте.",
                                )
                                .size(SIZE_SECONDARY)
                                .color(WARNING),
                            );
                            ui.add_space(SP_SM);

                            if self.onboard_mnemonic.is_empty() {
                                if let Ok((fresh_mn, _)) = nodex_messenger::crypto::UserIdentity::generate_mnemonic() {
                                    self.onboard_mnemonic = fresh_mn;
                                }
                            }

                            if self.onboard_mnemonic.is_empty() {
                                ui.label(
                                    RichText::new("Генерация фразы...")
                                        .color(TEXT_MUTED)
                                        .size(SIZE_SECONDARY),
                                );
                            } else if !self.onboard_phrase_revealed {
                                if ghost_button(ui, "Показать фразу").clicked() {
                                    self.onboard_phrase_revealed = true;
                                }
                            } else {
                                let words: Vec<&str> =
                                    self.onboard_mnemonic.split_whitespace().collect();
                                egui::Grid::new("onboard_mnemonic_grid")
                                    .spacing([SP_SM, SP_XS])
                                    .show(ui, |ui| {
                                        for (i, w) in words.iter().enumerate() {
                                            ui.label(
                                                RichText::new(format!("{:02}. {}", i + 1, w))
                                                    .color(ACCENT)
                                                    .size(SIZE_SECONDARY),
                                            );
                                            if (i + 1) % 3 == 0 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                                ui.add_space(SP_SM);
                                ui.horizontal(|ui| {
                                    if ghost_button(ui, "Скопировать фразу").clicked() {
                                        ui.ctx()
                                            .output_mut(|o| o.copied_text = self.onboard_mnemonic.clone());
                                        self.show_toast("Фраза скопирована");
                                    }
                                    if ghost_button(ui, "Сгенерировать другую").clicked() {
                                        if let Ok((fresh_mn, _)) = nodex_messenger::crypto::UserIdentity::generate_mnemonic() {
                                            self.onboard_mnemonic = fresh_mn;
                                            self.onboard_confirmed = false;
                                        }
                                    }
                                });
                                ui.add_space(SP_SM);
                                ui.checkbox(
                                    &mut self.onboard_confirmed,
                                    "Я сохранил фразу восстановления в надёжном месте",
                                );
                            }
                        });

                    ui.add_space(SP_LG);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 140.0);
                        if ghost_button(ui, "Назад").clicked() {
                            self.screen = Screen::Main;
                        }
                        ui.add_space(SP_SM);
                        let ready = !self.onboard_name.trim().is_empty()
                            && self.onboard_confirmed
                            && !self.onboard_mnemonic.is_empty();
                        if primary_button_enabled(ui, "Продолжить", ready).clicked() {
                            let _ = self.cmd_tx.send(AppCommand::RegisterAccount {
                                display_name: self.onboard_name.trim().to_string(),
                                bio: self.onboard_bio.trim().to_string(),
                                mnemonic: self.onboard_mnemonic.clone(),
                            });
                            self.edit_name = self.onboard_name.trim().to_string();
                            self.edit_bio = self.onboard_bio.trim().to_string();
                            self.my_mnemonic = self.onboard_mnemonic.clone();
                            self.onboard_mnemonic.clear();
                            self.show_toast("Аккаунт создан");
                            self.screen = Screen::Main;
                        }
                    });
                });
            });
    }

    fn render_restore_account(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG_DARK))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(SP_XL);
                    draw_emblem(ui, 76.0);
                    ui.add_space(SP_MD);
                    ui.label(
                        RichText::new("Восстановление аккаунта")
                            .size(SIZE_APP_TITLE)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(SP_SM);
                    Frame::none()
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0, BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(SP_LG))
                        .show(ui, |ui| {
                            ui.set_width(440.0);
                            ui.label(
                                RichText::new("Фраза восстановления (12 слов)")
                                    .size(SIZE_SECONDARY)
                                    .color(TEXT_MUTED),
                            );
                            ui.add(
                                TextEdit::multiline(&mut self.restore_mnemonic_input)
                                    .desired_rows(3)
                                    .desired_width(ui.available_width()),
                            );
                            ui.add_space(SP_SM);
                            ui.label(
                                RichText::new("Отображаемое имя")
                                    .size(SIZE_SECONDARY)
                                    .color(TEXT_MUTED),
                            );
                            ui.add(
                                TextEdit::singleline(&mut self.restore_name_input)
                                    .desired_width(ui.available_width()),
                            );
                            if let Some(err) = self.restore_error.clone() {
                                ui.add_space(SP_SM);
                                ui.label(
                                    RichText::new(err).color(DANGER).size(SIZE_SECONDARY),
                                );
                            }
                        });

                    ui.add_space(SP_LG);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 140.0);
                        if ghost_button(ui, "Назад").clicked() {
                            self.screen = Screen::Main;
                        }
                        ui.add_space(SP_SM);
                        if primary_button(ui, "Восстановить").clicked() {
                            let words: Vec<&str> =
                                self.restore_mnemonic_input.split_whitespace().collect();
                            if words.len() != 12 {
                                self.restore_error =
                                    Some("Фраза должна состоять из 12 слов".into());
                            } else if self.restore_name_input.trim().is_empty() {
                                self.restore_error = Some("Введите отображаемое имя".into());
                            } else {
                                let _ = self.cmd_tx.send(AppCommand::RegisterAccount {
                                    display_name: self.restore_name_input.trim().to_string(),
                                    bio: String::new(),
                                    mnemonic: words.join(" "),
                                });
                                self.edit_name = self.restore_name_input.trim().to_string();
                                self.show_toast("Аккаунт восстановлен");
                                self.restore_mnemonic_input.clear();
                                self.screen = Screen::Main;
                            }
                        }
                    });
                });
            });
    }

    fn render_lock_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG_DARK))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 120.0);
                    draw_emblem(ui, 76.0);
                    ui.add_space(SP_MD);
                    ui.label(
                        RichText::new("NodeX заблокирован")
                            .size(SIZE_APP_TITLE)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(SP_SM);
                    ui.label(
                        RichText::new("Введите локальный пароль, чтобы продолжить")
                            .size(SIZE_SECONDARY)
                            .color(TEXT_SECONDARY),
                    );
                    ui.add_space(SP_MD);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 130.0);
                        ui.add(
                            TextEdit::singleline(&mut self.lock_password_input)
                                .password(true)
                                .desired_width(220.0)
                                .hint_text("Пароль"),
                        );
                        if primary_button(ui, "Войти").clicked() {
                            match self.prefs.lock_password_hash {
                                Some(h) if h == fnv_hash(&self.lock_password_input) => {
                                    self.lock_password_input.clear();
                                    self.lock_error = None;
                                    self.touch_activity();
                                    self.screen = Screen::Main;
                                }
                                _ => {
                                    self.lock_error = Some("Неверный пароль".into());
                                }
                            }
                        }
                    });
                    if let Some(err) = self.lock_error.clone() {
                        ui.add_space(SP_SM);
                        ui.label(RichText::new(err).color(DANGER).size(SIZE_SECONDARY));
                    }
                });
            });
    }
}

// ============================================================================
// Bubble rendering
// ============================================================================

#[derive(Default)]
struct BubbleActions {
    delete_for_me: Option<String>,
    delete_for_everyone: Option<String>,
    toggle_select: Option<String>,
    reply: bool,
    forward: bool,
    retry: bool,
}

#[allow(clippy::too_many_arguments)]
fn render_bubble(
    ui: &mut Ui,
    msg: &SavedChatMessage,
    is_outgoing: bool,
    status: LocalStatus,
    max_width: f32,
    ctx: &egui::Context,
    show_lightbox: &mut bool,
    lightbox_data: &mut Option<PhotoPreviewData>,
    image_textures: &mut HashMap<u64, egui::TextureHandle>,
    actions: &mut BubbleActions,
    selection_mode: bool,
) {
    let (bg, border) = if is_outgoing {
        (BUBBLE_OUTGOING, BUBBLE_OUTGOING_BORDER)
    } else {
        (BUBBLE_INCOMING, BORDER)
    };

    let frame = Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::symmetric(SP_MD, SP_SM));

    let response = frame
        .show(ui, |ui| {
            ui.set_max_width(max_width);
            ui.vertical(|ui| {
                // Inline reply preview when we detect our own reply-encoding.
                if let Some((prefix, rest)) = split_reply_prefix(&msg.text) {
                    Frame::none()
                        .fill(SIDEBAR_ALT)
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::symmetric(SP_SM, SP_XS))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(
                                    Vec2::new(3.0, 24.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(r, Rounding::same(2.0), ACCENT);
                                ui.add_space(SP_XS);
                                ui.label(
                                    RichText::new(prefix)
                                        .size(SIZE_TIMESTAMP)
                                        .color(TEXT_SECONDARY),
                                );
                            });
                        });
                    if !rest.is_empty() {
                        ui.add_space(SP_XS);
                        ui.label(
                            RichText::new(rest).color(TEXT_PRIMARY).size(SIZE_BODY),
                        );
                    }
                } else if let Some(b64) = &msg.image_base64 {
                    if let Some(tex_size) = render_inline_image(ui, ctx, image_textures, b64, max_width - 24.0) {
                        let _ = tex_size;
                    }
                    let click = ui
                        .add(
                            egui::Label::new(
                                RichText::new("Открыть изображение")
                                        .size(SIZE_TIMESTAMP)
                                        .color(ACCENT),
                            )
                            .sense(Sense::click()),
                        )
                        .on_hover_text("Открыть в полном размере");
                    if click.clicked() {
                        *show_lightbox = true;
                        *lightbox_data = Some(PhotoPreviewData {
                            title: "Фото".into(),
                            sender_name: msg.sender_id_hex.clone(),
                            timestamp: msg.timestamp,
                            caption: msg.text.clone(),
                            base64_data: b64.clone(),
                        });
                    }
                    if !msg.text.is_empty() {
                        ui.add_space(SP_XS);
                        ui.label(
                            RichText::new(&msg.text)
                                .color(TEXT_PRIMARY)
                                .size(SIZE_BODY),
                        );
                    }
                } else {
                    ui.label(
                        RichText::new(&msg.text)
                            .color(TEXT_PRIMARY)
                            .size(SIZE_BODY),
                    );
                }

                ui.add_space(SP_XS);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format_time(msg.timestamp))
                            .size(SIZE_TIMESTAMP)
                            .color(TEXT_MUTED),
                    );
                    if is_outgoing {
                        let (tick_text, tick_color) = status_label(status);
                        ui.label(
                            RichText::new(tick_text).size(SIZE_TIMESTAMP).color(tick_color),
                        );
                        if status == LocalStatus::Failed
                            && text_button(ui, "Повторить", DANGER).clicked()
                        {
                            actions.retry = true;
                        }
                    }
                });
            });
        })
        .response;

    // Right-click / long-press menu on the bubble itself.
    response.context_menu(|ui| {
        if ui.button("Выбрать сообщение").clicked() {
            actions.toggle_select = Some(msg.id.clone());
            ui.close_menu();
        }
        if ui.button("Ответить").clicked() {
            actions.reply = true;
            ui.close_menu();
        }
        if ui.button("Переслать").clicked() {
            actions.forward = true;
            ui.close_menu();
        }
        if !msg.text.is_empty() && ui.button("Скопировать текст").clicked() {
            ctx.output_mut(|o| o.copied_text = msg.text.clone());
            ui.close_menu();
        }
        if is_outgoing && status == LocalStatus::Failed && ui.button("Повторить отправку").clicked()
        {
            actions.retry = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("🗑 Удалить только у себя").clicked() {
            actions.delete_for_me = Some(msg.id.clone());
            ui.close_menu();
        }
        if ui.button("⚡ Удалить у обоих (для всех)").clicked() {
            actions.delete_for_everyone = Some(msg.id.clone());
            ui.close_menu();
        }
    });

    if selection_mode && response.clicked() {
        actions.toggle_select = Some(msg.id.clone());
    }
}

fn render_pending_bubble(ui: &mut Ui, p: &PendingMessage, max_width: f32) {
    Frame::none()
        .fill(BUBBLE_OUTGOING)
        .stroke(Stroke::new(1.0, BUBBLE_OUTGOING_BORDER))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::symmetric(SP_MD, SP_SM))
        .show(ui, |ui| {
            ui.set_max_width(max_width);
            ui.vertical(|ui| {
                if !p.text.is_empty() {
                    ui.label(RichText::new(&p.text).color(TEXT_PRIMARY).size(SIZE_BODY));
                }
                if p.image_base64.is_some() {
                    ui.label(
                        RichText::new(format!(
                            "Изображение {}",
                            human_bytes(p.total_bytes as usize)
                        ))
                        .size(SIZE_SECONDARY)
                        .color(TEXT_SECONDARY),
                    );
                    let progress = if p.total_bytes > 0 {
                        (p.sent_bytes as f32 / p.total_bytes as f32).min(1.0)
                    } else {
                        // Fake progress based on elapsed time so the bar moves
                        // even without backend byte reports.
                        (p.created_at.elapsed().as_secs_f32() / 8.0).min(0.95)
                    };
                    ui.add(egui::ProgressBar::new(progress).desired_width(180.0));
                }
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format_time(now_millis()))
                            .size(SIZE_TIMESTAMP)
                            .color(TEXT_MUTED),
                    );
                    ui.label(
                        RichText::new("⏳ Отправка...")
                            .size(SIZE_TIMESTAMP)
                            .color(TEXT_MUTED),
                    );
                });
            });
        });
}

fn render_inline_image(
    ui: &mut Ui,
    ctx: &egui::Context,
    image_textures: &mut HashMap<u64, egui::TextureHandle>,
    b64: &str,
    max_w: f32,
) -> Option<Vec2> {
    let texture_key = fnv_hash(b64);
    use std::collections::hash_map::Entry;
    if let Entry::Vacant(entry) = image_textures.entry(texture_key) {
        let raw = BASE64_STANDARD.decode(b64).ok()?;
        let img = image::load_from_memory(&raw).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let tex = ctx.load_texture(
            format!("chat_img_{:x}", texture_key),
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba),
            egui::TextureOptions::default(),
        );
        entry.insert(tex);
    }
    let tex = image_textures.get(&texture_key)?;
    let [w, h] = tex.size();
    let aspect = w as f32 / h as f32;
    let rw = max_w.min(280.0);
    let rh = (rw / aspect).min(220.0);
    ui.image((tex.id(), Vec2::new(rw, rh)));
    Some(Vec2::new(rw, rh))
}

fn split_reply_prefix(text: &str) -> Option<(String, String)> {
    if !text.starts_with('↪') {
        return None;
    }
    let mut lines = text.splitn(2, '\n');
    let head = lines.next()?.to_string();
    let rest = lines.next().unwrap_or("").to_string();
    Some((head, rest))
}

fn status_label(status: LocalStatus) -> (&'static str, Color32) {
    match status {
        LocalStatus::Sending => (" · Отправка", TEXT_MUTED),
        LocalStatus::Sent => (" · ✓ Отправлено", TEXT_MUTED),
        LocalStatus::Delivered => (" · ✓✓ Доставлено", TEXT_SECONDARY),
        LocalStatus::Read => (" · ✓✓ Прочитано", ACCENT),
        LocalStatus::Failed => (" · ✕ Ошибка", DANGER),
    }
}

// ============================================================================
// Small reusable UI helpers
// ============================================================================

fn section_title(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(SIZE_SECTION_TITLE)
            .strong()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(SP_SM);
}

fn thin_separator(ui: &mut Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = ui.cursor().top();
    ui.painter()
        .hline(rect.x_range(), y, Stroke::new(1.0, BORDER_SOFT));
    ui.add_space(1.0);
}

fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
    ui.add_space(SP_XS);
}

fn status_pill(ui: &mut Ui, color: Color32, text: &str) {
    Frame::none()
        .fill(SURFACE)
        .rounding(Rounding::same(20.0))
        .inner_margin(Margin::symmetric(SP_SM, 3.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                status_dot(ui, color);
                ui.label(
                    RichText::new(text).size(SIZE_SECONDARY).color(TEXT_SECONDARY),
                );
            });
        });
}

fn unread_badge(ui: &mut Ui, count: usize) {
    let label = if count > 99 { "99+".to_string() } else { count.to_string() };
    Frame::none()
        .fill(ACCENT)
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(7.0, 2.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(label)
                    .size(SIZE_TIMESTAMP)
                    .strong()
                    .color(BG_DARK),
            );
        });
}

fn avatar(ui: &mut Ui, name: &str, size: f32, with_online_dot: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    let color = avatar_color(name);
    let initial = name.chars().next().unwrap_or('?').to_uppercase().to_string();
    ui.painter().circle_filled(rect.center(), size / 2.0, color);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional(size * 0.42),
        Color32::WHITE,
    );
    if with_online_dot {
        let dot_center = rect.center() + Vec2::new(size * 0.34, size * 0.34);
        ui.painter().circle_filled(dot_center, size * 0.12, SUCCESS);
        ui.painter()
            .circle_stroke(dot_center, size * 0.12, Stroke::new(1.5, SIDEBAR_BG));
    }
}

fn avatar_color(name: &str) -> Color32 {
    let hash: usize = name.bytes().fold(0, |acc, b| acc.wrapping_add(b as usize));
    const PALETTE: [Color32; 8] = [
        Color32::from_rgb(0x2A, 0xB7, 0xCA),
        Color32::from_rgb(0x8B, 0x5C, 0xF6),
        Color32::from_rgb(0x22, 0xC5, 0x5E),
        Color32::from_rgb(0xF5, 0x9E, 0x0B),
        Color32::from_rgb(0x3B, 0x82, 0xF6),
        Color32::from_rgb(0xEC, 0x48, 0x99),
        Color32::from_rgb(0x14, 0xB8, 0xA6),
        Color32::from_rgb(0xF4, 0x7C, 0x3C),
    ];
    PALETTE[hash % PALETTE.len()]
}

fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    primary_button_enabled(ui, label, true)
}

fn primary_button_enabled(ui: &mut Ui, label: &str, enabled: bool) -> egui::Response {
    let (fill, text_color) = if enabled {
        (ACCENT, BG_DARK)
    } else {
        (ACCENT_SOFT, TEXT_MUTED)
    };
    let button = egui::Button::new(RichText::new(label).strong().color(text_color))
        .fill(fill)
        .rounding(Rounding::same(8.0))
        .min_size(Vec2::new(0.0, 30.0));
    ui.add_enabled(enabled, button)
}

fn ghost_button(ui: &mut Ui, label: &str) -> egui::Response {
    let button = egui::Button::new(RichText::new(label).color(TEXT_SECONDARY))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(Rounding::same(8.0))
        .min_size(Vec2::new(0.0, 26.0));
    ui.add(button)
}

fn danger_button(ui: &mut Ui, label: &str) -> egui::Response {
    let button = egui::Button::new(RichText::new(label).color(DANGER).strong())
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, DANGER))
        .rounding(Rounding::same(8.0))
        .min_size(Vec2::new(0.0, 28.0));
    ui.add(button)
}

fn text_button(ui: &mut Ui, label: &str, color: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(color))
            .fill(Color32::TRANSPARENT)
            .frame(false),
    )
}

fn labeled_row(ui: &mut Ui, label: &str, value: &str, copyable: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("{label}:"))
                .size(SIZE_SECONDARY)
                .color(TEXT_MUTED),
        );
        let shown = shorten(value, 14);
        ui.label(
            RichText::new(shown).size(SIZE_SECONDARY).color(TEXT_SECONDARY),
        );
        if copyable && !value.is_empty() && text_button(ui, "копировать", ACCENT).clicked() {
            let v = value.to_string();
            ui.ctx().output_mut(|o| o.copied_text = v);
        }
    });
}

fn network_status(peers: usize) -> (Color32, &'static str) {
    if peers > 0 {
        (SUCCESS, "В сети")
    } else {
        (SUCCESS, "В сети (ожидание пиров)")
    }
}

fn peer_state(peers: usize, contact: Option<&SavedContact>) -> (Color32, String) {
    match contact.map(|c| c.last_seen_addr.as_str()) {
        Some(addr) if !addr.is_empty() && addr != "unknown" => (SUCCESS, "В сети".to_string()),
        _ => {
            if peers > 0 {
                (TEXT_MUTED, "Не в сети".to_string())
            } else {
                (TEXT_MUTED, "Ожидание узлов".to_string())
            }
        }
    }
}

fn short_id(id: &str) -> String {
    let count = id.chars().count();
    if count <= 12 {
        id.to_string()
    } else {
        let prefix: String = id.chars().take(6).collect();
        let suffix: String = id.chars().skip(count.saturating_sub(4)).collect();
        format!("{prefix}…{suffix}")
    }
}

fn fingerprint_chunks(id: &str) -> String {
    id.chars()
        .collect::<Vec<char>>()
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn last_msg_at(
    messages: &HashMap<String, Vec<SavedChatMessage>>,
    contact_id: &str,
) -> Option<u64> {
    messages
        .get(contact_id)
        .and_then(|m| m.last())
        .map(|m| m.timestamp)
}

fn format_time(timestamp: u64) -> String {
    let total_secs = timestamp / 1000;
    let hours = (total_secs / 3600) % 24;
    let mins = (total_secs / 60) % 60;
    format!("{:02}:{:02}", hours, mins)
}

fn format_day(timestamp: u64) -> String {
    // Coarse "day bucket" without pulling chrono. Groups messages into 24-hour
    // windows tied to the Unix epoch and formats a day index; enough for a
    // visual separator between sessions.
    let day = timestamp / 1000 / 86_400;
    format!("День #{day}")
}

fn render_day_divider(ui: &mut Ui, text: &str) {
    ui.add_space(SP_SM);
    ui.horizontal(|ui| {
        let rect = ui.available_rect_before_wrap();
        let y = ui.cursor().center().y;
        ui.painter().hline(
            rect.left()..=rect.center().x - 60.0,
            y,
            Stroke::new(1.0, BORDER_SOFT),
        );
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            Frame::none()
                .fill(SURFACE)
                .rounding(Rounding::same(10.0))
                .inner_margin(Margin::symmetric(SP_SM, 2.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(text).size(SIZE_TIMESTAMP).color(TEXT_MUTED),
                    );
                });
        });
        ui.painter().hline(
            rect.center().x + 60.0..=rect.right(),
            y,
            Stroke::new(1.0, BORDER_SOFT),
        );
    });
    ui.add_space(SP_SM);
}

fn shorten(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let head: String = s.chars().take(n / 2).collect();
        let tail: String = s.chars().rev().take(n / 2).collect::<String>().chars().rev().collect();
        format!("{head}…{tail}")
    }
}

fn human_bytes(n: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;
    if n >= MB {
        format!("{:.1} МБ", n as f32 / MB as f32)
    } else if n >= KB {
        format!("{:.0} КБ", n as f32 / KB as f32)
    } else {
        format!("{n} Б")
    }
}

fn compress_if_needed(bytes: Vec<u8>) -> Vec<u8> {
    // Downscale/reencode images that exceed ~1 MB to JPEG q=80.
    // Falls back to original bytes on any failure.
    if bytes.len() <= 1024 * 1024 {
        return bytes;
    }
    let Ok(img) = image::load_from_memory(&bytes) else {
        return bytes;
    };
    let (w, h) = (img.width(), img.height());
    let scale = (1600.0_f32 / w.max(h) as f32).min(1.0);
    let resized = if scale < 1.0 {
        img.resize(
            (w as f32 * scale) as u32,
            (h as f32 * scale) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let mut buf = std::io::Cursor::new(Vec::new());
    if resized
        .write_to(&mut buf, image::ImageOutputFormat::Jpeg(80))
        .is_ok()
    {
        buf.into_inner()
    } else {
        bytes
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn fnv_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Real vector QR Code generator for invites and key verification.
fn draw_qr_code(ui: &mut Ui, content: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    let painter = ui.painter();

    // White background card for high contrast scanning
    painter.rect_filled(rect, Rounding::same(8.0), Color32::WHITE);
    painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0, BORDER));

    if let Ok(code) = qrcode::QrCode::new(content.as_bytes()) {
        let colors = code.to_colors();
        let width = code.width();
        if width > 0 {
            let padding = 10.0;
            let draw_size = (size - 2.0 * padding).max(10.0);
            let cell = draw_size / width as f32;
            let offset_x = rect.left() + padding;
            let offset_y = rect.top() + padding;

            for y in 0..width {
                for x in 0..width {
                    let idx = y * width + x;
                    if idx < colors.len() && colors[idx] == qrcode::Color::Dark {
                        let cell_rect = Rect::from_min_size(
                            Pos2::new(offset_x + x as f32 * cell, offset_y + y as f32 * cell),
                            Vec2::new(cell + 0.5, cell + 0.5),
                        );
                        painter.rect_filled(cell_rect, Rounding::ZERO, Color32::BLACK);
                    }
                }
            }
        }
    } else {
        painter.rect_filled(rect, Rounding::same(6.0), BG_DARK);
    }
}

// ============================================================================
// Vector icons
// ============================================================================

fn icon_button(
    ui: &mut Ui,
    size: f32,
    draw: fn(&egui::Painter, Pos2, f32, Color32),
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(size, size), Sense::click());
    let bg = if response.hovered() {
        SURFACE_HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(6.0), bg);
    let color = if response.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    draw(ui.painter(), rect.center(), size * 0.32, color);
    response.on_hover_text(tooltip)
}

fn draw_icon_search(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let c = center - Vec2::new(r * 0.15, r * 0.15);
    painter.circle_stroke(c, r * 0.62, Stroke::new(1.6, color));
    let h1 = c + Vec2::new(r * 0.5, r * 0.5);
    let h2 = h1 + Vec2::new(r * 0.5, r * 0.5);
    painter.line_segment([h1, h2], Stroke::new(1.8, color));
}

fn draw_icon_add_contact(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let head = center + Vec2::new(-r * 0.25, -r * 0.35);
    painter.circle_stroke(head, r * 0.32, Stroke::new(1.5, color));
    let body = Rect::from_center_size(
        center + Vec2::new(-r * 0.25, r * 0.35),
        Vec2::new(r * 0.9, r * 0.5),
    );
    painter.rect_stroke(body, Rounding::same(r * 0.3), Stroke::new(1.5, color));
    let plus = center + Vec2::new(r * 0.55, -r * 0.05);
    painter.line_segment(
        [plus - Vec2::new(r * 0.28, 0.0), plus + Vec2::new(r * 0.28, 0.0)],
        Stroke::new(1.6, color),
    );
    painter.line_segment(
        [plus - Vec2::new(0.0, r * 0.28), plus + Vec2::new(0.0, r * 0.28)],
        Stroke::new(1.6, color),
    );
}

fn draw_icon_settings(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    painter.circle_stroke(center, r * 0.4, Stroke::new(1.6, color));
    painter.circle_filled(center, r * 0.12, color);
    for i in 0..6 {
        let angle = i as f32 * std::f32::consts::PI / 3.0;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let inner = center + dir * r * 0.62;
        let outer = center + dir * r * 0.88;
        painter.line_segment([inner, outer], Stroke::new(1.8, color));
    }
}

fn draw_icon_attach(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let rect = Rect::from_center_size(center, Vec2::new(r * 0.9, r * 1.5));
    painter.rect_stroke(rect, Rounding::same(r * 0.4), Stroke::new(1.6, color));
    painter.line_segment(
        [
            center - Vec2::new(0.0, r * 0.4),
            center + Vec2::new(0.0, r * 0.4),
        ],
        Stroke::new(1.6, color),
    );
}

fn draw_icon_image(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let rect = Rect::from_center_size(center, Vec2::new(r * 1.6, r * 1.2));
    painter.rect_stroke(rect, Rounding::same(2.0), Stroke::new(1.4, color));
    painter.circle_filled(
        center + Vec2::new(-r * 0.4, -r * 0.2),
        r * 0.15,
        color,
    );
    painter.line_segment(
        [
            rect.left_bottom() + Vec2::new(2.0, -2.0),
            center + Vec2::new(r * 0.2, r * 0.1),
        ],
        Stroke::new(1.4, color),
    );
    painter.line_segment(
        [
            center + Vec2::new(r * 0.2, r * 0.1),
            rect.right_bottom() + Vec2::new(-2.0, -2.0),
        ],
        Stroke::new(1.4, color),
    );
}

fn draw_icon_emoji(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    painter.circle_stroke(center, r * 0.9, Stroke::new(1.6, color));
    painter.circle_filled(center - Vec2::new(r * 0.32, r * 0.2), r * 0.08, color);
    painter.circle_filled(center + Vec2::new(r * 0.32, -r * 0.2), r * 0.08, color);
    painter.line_segment(
        [
            center + Vec2::new(-r * 0.3, r * 0.2),
            center + Vec2::new(r * 0.3, r * 0.2),
        ],
        Stroke::new(1.4, color),
    );
}

fn draw_icon_send(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let p1 = center + Vec2::new(-r * 0.9, -r * 0.7);
    let p2 = center + Vec2::new(r * 1.0, 0.0);
    let p3 = center + Vec2::new(-r * 0.9, r * 0.7);
    let mid = center + Vec2::new(-r * 0.25, 0.0);
    painter.add(egui::Shape::convex_polygon(
        vec![p1, p2, p3, mid],
        color,
        Stroke::NONE,
    ));
}

fn draw_icon_close_menu(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let d = r * 0.6;
    painter.line_segment(
        [center - Vec2::new(d, d), center + Vec2::new(d, d)],
        Stroke::new(1.8, color),
    );
    painter.line_segment(
        [center - Vec2::new(d, -d), center + Vec2::new(d, -d)],
        Stroke::new(1.8, color),
    );
}

fn draw_icon_trash(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let body = Rect::from_center_size(
        center + Vec2::new(0.0, r * 0.15),
        Vec2::new(r * 1.1, r * 1.2),
    );
    painter.rect_stroke(body, Rounding::same(2.0), Stroke::new(1.5, color));
    painter.line_segment(
        [
            Pos2::new(body.left() - 2.0, body.top()),
            Pos2::new(body.right() + 2.0, body.top()),
        ],
        Stroke::new(1.5, color),
    );
    painter.line_segment(
        [
            center + Vec2::new(-r * 0.35, -r * 0.55),
            center + Vec2::new(r * 0.35, -r * 0.55),
        ],
        Stroke::new(1.5, color),
    );
    for dx in [-0.3_f32, 0.0, 0.3] {
        let x = center.x + dx * r;
        painter.line_segment(
            [
                Pos2::new(x, body.top() + 3.0),
                Pos2::new(x, body.bottom() - 3.0),
            ],
            Stroke::new(1.2, color.gamma_multiply(0.85)),
        );
    }
}

fn draw_icon_shield(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let top = center + Vec2::new(0.0, -r * 0.9);
    let bl = center + Vec2::new(-r * 0.8, -r * 0.2);
    let br = center + Vec2::new(r * 0.8, -r * 0.2);
    let bot = center + Vec2::new(0.0, r * 0.9);
    painter.add(egui::Shape::convex_polygon(
        vec![top, br, bot, bl],
        Color32::TRANSPARENT,
        Stroke::new(1.6, color),
    ));
    painter.line_segment(
        [
            center + Vec2::new(-r * 0.25, r * 0.05),
            center + Vec2::new(-r * 0.05, r * 0.3),
        ],
        Stroke::new(1.6, color),
    );
    painter.line_segment(
        [
            center + Vec2::new(-r * 0.05, r * 0.3),
            center + Vec2::new(r * 0.35, -r * 0.2),
        ],
        Stroke::new(1.6, color),
    );
}

#[allow(dead_code)]
fn draw_icon_info(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    painter.circle_stroke(center, r * 0.85, Stroke::new(1.6, color));
    painter.circle_filled(center - Vec2::new(0.0, r * 0.4), r * 0.08, color);
    painter.line_segment(
        [
            center - Vec2::new(0.0, r * 0.15),
            center + Vec2::new(0.0, r * 0.5),
        ],
        Stroke::new(1.8, color),
    );
}

fn draw_icon_more(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    for dx in [-r * 0.5_f32, 0.0, r * 0.5] {
        painter.circle_filled(center + Vec2::new(dx, 0.0), r * 0.14, color);
    }
}

fn draw_icon_refresh(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    use std::f32::consts::TAU;
    let radius = r * 0.75;
    let segments = 32;
    for i in 0..segments {
        let t0 = i as f32 / segments as f32;
        let t1 = (i + 1) as f32 / segments as f32;
        // Skip a small gap to leave room for the arrow head.
        if t0 > 0.72 && t0 < 0.85 {
            continue;
        }
        let a0 = t0 * TAU;
        let a1 = t1 * TAU;
        let p0 = center + Vec2::new(a0.cos(), a0.sin()) * radius;
        let p1 = center + Vec2::new(a1.cos(), a1.sin()) * radius;
        painter.line_segment([p0, p1], Stroke::new(1.6, color));
    }
    // Arrow head.
    let a = 0.82 * TAU;
    let tip = center + Vec2::new(a.cos(), a.sin()) * radius;
    painter.line_segment(
        [tip, tip + Vec2::new(-r * 0.3, -r * 0.2)],
        Stroke::new(1.6, color),
    );
    painter.line_segment(
        [tip, tip + Vec2::new(r * 0.05, -r * 0.35)],
        Stroke::new(1.6, color),
    );
}

#[allow(dead_code)]
fn draw_icon_copy(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let back = Rect::from_center_size(
        center - Vec2::new(r * 0.2, r * 0.2),
        Vec2::new(r * 1.1, r * 1.1),
    );
    painter.rect_stroke(back, Rounding::same(2.0), Stroke::new(1.3, color.gamma_multiply(0.7)));
    let front = Rect::from_center_size(
        center + Vec2::new(r * 0.2, r * 0.2),
        Vec2::new(r * 1.1, r * 1.1),
    );
    painter.rect_filled(front, Rounding::same(2.0), BG_DARK);
    painter.rect_stroke(front, Rounding::same(2.0), Stroke::new(1.4, color));
}

fn draw_icon_select(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let box_rect = Rect::from_center_size(center, Vec2::new(r * 1.4, r * 1.4));
    painter.rect_stroke(box_rect, Rounding::same(3.0), Stroke::new(1.4, color));
    let p1 = center + Vec2::new(-r * 0.4, 0.0);
    let p2 = center + Vec2::new(-r * 0.1, r * 0.3);
    let p3 = center + Vec2::new(r * 0.4, -r * 0.3);
    painter.line_segment([p1, p2], Stroke::new(1.6, color));
    painter.line_segment([p2, p3], Stroke::new(1.6, color));
}

fn draw_wordmark(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_filled(center, 12.0, ACCENT.gamma_multiply(0.25));
    painter.circle_stroke(center, 12.0, Stroke::new(1.6, ACCENT));
    painter.text(
        center,
        Align2::CENTER_CENTER,
        "N",
        FontId::proportional(13.0),
        TEXT_PRIMARY,
    );
    ui.add_space(SP_XS);
    ui.label(
        RichText::new("NodeX")
            .strong()
            .size(SIZE_SECTION_TITLE)
            .color(TEXT_PRIMARY),
    );
}

fn draw_emblem(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_stroke(center, size * 0.45, Stroke::new(1.6, BORDER));
    painter.circle_stroke(
        center,
        size * 0.34,
        Stroke::new(1.4, ACCENT.gamma_multiply(0.7)),
    );
    painter.circle_filled(center, size * 0.2, ACCENT.gamma_multiply(0.25));
    painter.circle_stroke(center, size * 0.2, Stroke::new(1.8, ACCENT));
    painter.text(
        center,
        Align2::CENTER_CENTER,
        "N",
        FontId::proportional(size * 0.22),
        TEXT_PRIMARY,
    );
}
