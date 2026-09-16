use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};
use base64::prelude::*;
use eframe::egui::{self, Align, Color32, FontId, Frame, Id, Key, Layout, Margin, RichText, Rounding, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2};
use nodex_messenger::db::{SavedChatMessage, SavedContact};
use rand::RngCore;

pub const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");

// Events sent from Tokio background worker to GUI thread
#[derive(Debug, Clone)]
pub enum UiEvent {
    NodeInfo {
        user_id: String,
        dht_node_id: String,
        display_name: String,
        bio: String,
        mnemonic: String,
        dht_peers: usize,
        stored_keys: usize,
    },
    ContactsList(Vec<SavedContact>),
    MessagesList {
        contact_id: String,
        messages: Vec<SavedChatMessage>,
    },
    MessageSent(SavedChatMessage),
    StatusLog(String),
}

// Commands sent from GUI thread to Tokio background worker
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppCommand {
    SendMessage {
        recipient_id: String,
        text: String,
        image_base64: Option<String>,
    },
    AddContact {
        user_id: String,
        name: String,
    },
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
    RefreshContacts,
    SelectContact(String),
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum OnboardingTab {
    Create,
    Restore,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum EmojiCategory {
    Smiles,
    Gestures,
    Hearts,
    Tech,
    Food,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct PhotoPreviewData {
    pub title: String,
    pub sender_name: String,
    pub sender_id: String,
    pub timestamp: u64,
    pub caption: String,
    pub base64_data: String,
    pub incoming: bool,
}

pub struct NodeXApp {
    // Communication channels
    ui_rx: Receiver<UiEvent>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<AppCommand>,

    // App State
    my_user_id: String,
    my_dht_node_id: String,
    my_display_name: String,
    my_bio: String,
    my_mnemonic: String,
    dht_peers: usize,
    stored_keys: usize,

    // Profile Edit State
    edit_name_input: String,
    edit_bio_input: String,

    // Registration & Onboarding
    is_registered: bool,
    onboarding_tab: OnboardingTab,
    generated_mnemonic_words: Vec<String>,
    has_saved_seed: bool,
    create_name_input: String,
    create_bio_input: String,

    restore_mnemonic_input: String,
    restore_name_input: String,
    restore_bio_input: String,
    restore_error: Option<String>,

    // Contacts & Messages
    contacts: Vec<SavedContact>,
    selected_contact: Option<SavedContact>,
    messages: Vec<SavedChatMessage>,
    all_known_messages: HashMap<String, Vec<SavedChatMessage>>,
    last_messages: HashMap<String, SavedChatMessage>,
    unread_set: HashSet<String>,

    // Search and filters
    search_query: String,

    // UI Input Buffers & Modals
    message_input: String,
    pending_image_base64: Option<String>,
    pending_image_name: Option<String>,

    // Emoji Picker state
    emoji_picker_open: bool,
    emoji_category: EmojiCategory,

    // Photo Preview Lightbox
    preview_photo: Option<PhotoPreviewData>,

    add_contact_modal: bool,
    profile_drawer_open: bool,
    show_seed_in_profile: bool,
    new_contact_id: String,
    new_contact_name: String,
    status_msg: String,
    copied_feedback: Option<(String, f64)>, // (label, timestamp)
}

impl NodeXApp {
    pub fn new(
        ui_rx: Receiver<UiEvent>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<AppCommand>,
    ) -> Self {
        let _ = cmd_tx.send(AppCommand::RefreshInfo);
        let _ = cmd_tx.send(AppCommand::RefreshContacts);

        let initial_words = Self::generate_12_words();

        Self {
            ui_rx,
            cmd_tx,
            my_user_id: "Loading...".into(),
            my_dht_node_id: "Loading...".into(),
            my_display_name: "NodeX User".into(),
            my_bio: String::new(),
            my_mnemonic: String::new(),
            dht_peers: 0,
            stored_keys: 0,
            edit_name_input: String::new(),
            edit_bio_input: String::new(),
            is_registered: false,
            onboarding_tab: OnboardingTab::Create,
            generated_mnemonic_words: initial_words,
            has_saved_seed: false,
            create_name_input: String::new(),
            create_bio_input: String::new(),
            restore_mnemonic_input: String::new(),
            restore_name_input: String::new(),
            restore_bio_input: String::new(),
            restore_error: None,
            contacts: Vec::new(),
            selected_contact: None,
            messages: Vec::new(),
            all_known_messages: HashMap::new(),
            last_messages: HashMap::new(),
            unread_set: HashSet::new(),
            search_query: String::new(),
            message_input: String::new(),
            pending_image_base64: None,
            pending_image_name: None,
            emoji_picker_open: false,
            emoji_category: EmojiCategory::Smiles,
            preview_photo: None,
            add_contact_modal: false,
            profile_drawer_open: false,
            show_seed_in_profile: false,
            new_contact_id: String::new(),
            new_contact_name: String::new(),
            status_msg: "P2P Network Active".into(),
            copied_feedback: None,
        }
    }

    fn generate_12_words() -> Vec<String> {
        let mut entropy = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut entropy);
        if let Ok(phrase) = core_ffi::ffi_mnemonic_generate_12(&entropy) {
            phrase.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            vec![
                "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
                "absurd", "abuse", "access", "accident",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
        }
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        while let Ok(event) = self.ui_rx.try_recv() {
            updated = true;
            match event {
                UiEvent::NodeInfo {
                    user_id,
                    dht_node_id,
                    display_name,
                    bio,
                    mnemonic,
                    dht_peers,
                    stored_keys,
                } => {
                    self.my_user_id = user_id;
                    self.my_dht_node_id = dht_node_id;
                    self.my_bio = bio.clone();
                    if !mnemonic.is_empty() {
                        self.my_mnemonic = mnemonic.clone();
                        self.generated_mnemonic_words = mnemonic.split_whitespace().map(|s| s.to_string()).collect();
                    }
                    if !display_name.is_empty() && display_name != "UserNode" && display_name != "NodeX User" {
                        self.is_registered = true;
                    }
                    self.my_display_name = display_name.clone();
                    if self.edit_name_input.is_empty() {
                        self.edit_name_input = display_name;
                    }
                    if self.edit_bio_input.is_empty() {
                        self.edit_bio_input = bio;
                    }
                    self.dht_peers = dht_peers;
                    self.stored_keys = stored_keys;
                }
                UiEvent::ContactsList(list) => {
                    self.contacts = list;
                    if self.selected_contact.is_none() && !self.contacts.is_empty() {
                        let first = self.contacts[0].clone();
                        let _ = self.cmd_tx.send(AppCommand::SelectContact(first.user_id_hex.clone()));
                        self.selected_contact = Some(first);
                    } else if let Some(ref sel) = self.selected_contact {
                        if let Some(updated_c) = self.contacts.iter().find(|c| c.user_id_hex == sel.user_id_hex) {
                            self.selected_contact = Some(updated_c.clone());
                        }
                    }
                }
                UiEvent::MessagesList { contact_id, messages } => {
                    if let Some(last) = messages.last() {
                        self.last_messages.insert(contact_id.clone(), last.clone());
                    }
                    self.all_known_messages.insert(contact_id.clone(), messages.clone());
                    if let Some(ref sel) = self.selected_contact {
                        if sel.user_id_hex == contact_id {
                            self.messages = messages;
                        }
                    }
                }
                UiEvent::MessageSent(msg) => {
                    let peer_id = if msg.incoming {
                        msg.sender_id_hex.clone()
                    } else {
                        msg.recipient_id_hex.clone()
                    };

                    self.last_messages.insert(peer_id.clone(), msg.clone());
                    
                    let entry = self.all_known_messages.entry(peer_id.clone()).or_default();
                    if !entry.iter().any(|m| m.id == msg.id) {
                        entry.push(msg.clone());
                    }

                    if self.selected_contact.is_none() {
                        let contact = self.contacts.iter().find(|c| c.user_id_hex == peer_id).cloned().unwrap_or_else(|| SavedContact {
                            user_id_hex: peer_id.clone(),
                            name: format!("Peer_{}", &peer_id[..6.min(peer_id.len())]),
                            bio: String::new(),
                            ed25519_pub_hex: "".into(),
                            x25519_pub_hex: "".into(),
                            last_seen_addr: "unknown".into(),
                        });
                        self.selected_contact = Some(contact);
                        let _ = self.cmd_tx.send(AppCommand::SelectContact(peer_id.clone()));
                        self.messages.push(msg);
                    } else if let Some(ref sel) = self.selected_contact {
                        if sel.user_id_hex == peer_id {
                            if !self.messages.iter().any(|m| m.id == msg.id) {
                                self.messages.push(msg);
                            }
                        } else if msg.incoming {
                            self.unread_set.insert(peer_id);
                        }
                    }
                }
                UiEvent::StatusLog(log) => {
                    self.status_msg = log;
                }
            }
        }
        if updated {
            ctx.request_repaint();
        }
    }

    fn setup_telegram_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();

        let bg_slate_900 = Color32::from_rgb(15, 23, 42);     // #0f172a
        let panel_slate_800 = Color32::from_rgb(30, 41, 59);  // #1e293b
        let text_slate_50 = Color32::from_rgb(248, 250, 252); // #f8fafc
        let cyan_500 = Color32::from_rgb(6, 182, 212);        // #06b6d4
        let cyan_400 = Color32::from_rgb(34, 211, 238);       // #22d3ee
        let border_slate_700 = Color32::from_rgb(51, 65, 85); // #334155

        visuals.dark_mode = true;
        visuals.override_text_color = Some(text_slate_50);
        visuals.window_fill = bg_slate_900;
        visuals.panel_fill = panel_slate_800;
        visuals.extreme_bg_color = Color32::from_rgb(10, 15, 29);
        visuals.faint_bg_color = Color32::from_rgb(24, 32, 47);

        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);
        visuals.widgets.noninteractive.bg_fill = panel_slate_800;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border_slate_700);

        visuals.widgets.inactive.rounding = Rounding::same(8.0);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 51, 73);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border_slate_700);

        visuals.widgets.hovered.rounding = Rounding::same(8.0);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(51, 65, 85);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, cyan_400);

        visuals.widgets.active.rounding = Rounding::same(8.0);
        visuals.widgets.active.bg_fill = cyan_500;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, cyan_400);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(6, 182, 212, 60);
        visuals.selection.stroke = Stroke::new(1.0_f32, cyan_400);

        ctx.set_visuals(visuals);
    }

    fn avatar_color_for_name(name: &str) -> Color32 {
        let mut hash: u32 = 5381;
        for b in name.bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(b as u32);
        }
        let colors = [
            Color32::from_rgb(14, 165, 233),  // Sky 500
            Color32::from_rgb(6, 182, 212),   // Cyan 500
            Color32::from_rgb(16, 185, 129),  // Emerald 500
            Color32::from_rgb(139, 92, 246),  // Violet 500
            Color32::from_rgb(236, 72, 153),  // Pink 500
            Color32::from_rgb(245, 158, 11),  // Amber 500
            Color32::from_rgb(99, 102, 241),  // Indigo 500
            Color32::from_rgb(20, 184, 166),  // Teal 500
        ];
        colors[(hash as usize) % colors.len()]
    }

    fn render_avatar(ui: &mut Ui, name: &str, size: f32, is_online: bool) {
        let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
        let bg_color = Self::avatar_color_for_name(name);
        
        ui.painter().circle_filled(rect.center(), size / 2.0, bg_color);

        let initials: String = name
            .split_whitespace()
            .take(2)
            .filter_map(|w| w.chars().next())
            .collect();
        let initials = if initials.is_empty() {
            name.chars().take(2).collect::<String>().to_uppercase()
        } else {
            initials.to_uppercase()
        };

        let font_size = size * 0.42;
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            initials,
            FontId::proportional(font_size),
            Color32::WHITE,
        );

        if is_online {
            let dot_radius = size * 0.18;
            let dot_center = rect.right_bottom() - Vec2::splat(dot_radius * 0.8);
            ui.painter().circle_filled(dot_center, dot_radius + 1.5, Color32::from_rgb(30, 41, 59));
            ui.painter().circle_filled(dot_center, dot_radius, Color32::from_rgb(34, 197, 94));
        }
    }

    fn render_onboarding_screen(&mut self, ui: &mut Ui) {
        let cyan_500 = Color32::from_rgb(6, 182, 212);
        let border_color = Color32::from_rgb(51, 65, 85);
        let dark_input_bg = Color32::from_rgb(15, 23, 42);

        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.03);

            let card_frame = Frame::none()
                .fill(Color32::from_rgb(30, 41, 59))
                .rounding(Rounding::same(16.0))
                .stroke(Stroke::new(1.0_f32, border_color))
                .inner_margin(Margin::same(28.0));

            card_frame.show(ui, |ui| {
                ui.set_max_width(520.0);

                // Embedded NodeX Logo
                ui.add(
                    egui::Image::from_bytes("bytes://nodex_logo_onboarding.png", LOGO_BYTES)
                        .fit_to_exact_size(Vec2::new(72.0, 72.0))
                        .rounding(Rounding::same(14.0)),
                );

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Добро пожаловать в NodeX")
                        .size(23.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Децентрализованный P2P E2EE мессенджер на базе Kademlia DHT")
                        .size(13.0)
                        .color(Color32::from_rgb(148, 163, 184)),
                );

                ui.add_space(18.0);

                // Tab Switcher: [✨ Создать аккаунт] / [📥 Восстановить]
                ui.horizontal(|ui| {
                    let is_create = self.onboarding_tab == OnboardingTab::Create;
                    let create_btn = ui.add_sized(
                        [ui.available_width() / 2.0 - 6.0, 36.0],
                        egui::Button::new(
                            RichText::new("✨ Создать аккаунт")
                                .size(13.5)
                                .strong()
                                .color(if is_create { Color32::WHITE } else { Color32::from_rgb(148, 163, 184) }),
                        )
                        .fill(if is_create { cyan_500 } else { Color32::from_rgb(24, 32, 47) })
                        .rounding(Rounding::same(8.0)),
                    );
                    if create_btn.clicked() {
                        self.onboarding_tab = OnboardingTab::Create;
                    }

                    let is_restore = self.onboarding_tab == OnboardingTab::Restore;
                    let restore_btn = ui.add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new(
                            RichText::new("📥 Восстановить (BIP-39)")
                                .size(13.5)
                                .strong()
                                .color(if is_restore { Color32::WHITE } else { Color32::from_rgb(148, 163, 184) }),
                        )
                        .fill(if is_restore { cyan_500 } else { Color32::from_rgb(24, 32, 47) })
                        .rounding(Rounding::same(8.0)),
                    );
                    if restore_btn.clicked() {
                        self.onboarding_tab = OnboardingTab::Restore;
                    }
                });

                ui.add_space(16.0);

                match self.onboarding_tab {
                    OnboardingTab::Create => {
                        ui.label(
                            RichText::new("🔑 Ваша секретная сид-фраза восстановления (12 слов):")
                                .size(13.5)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new("Сохраните эти 12 слов в надёжном месте. Они дают полный доступ к вашему P2P Identity.")
                                .size(12.0)
                                .color(Color32::from_rgb(148, 163, 184)),
                        );

                        ui.add_space(10.0);

                        // 3x4 Grid of Mnemonic Words
                        egui::Grid::new("mnemonic_grid_create")
                            .num_columns(3)
                            .spacing([10.0, 8.0])
                            .show(ui, |ui| {
                                for (i, word) in self.generated_mnemonic_words.iter().enumerate() {
                                    let word_frame = Frame::none()
                                        .fill(dark_input_bg)
                                        .rounding(Rounding::same(8.0))
                                        .stroke(Stroke::new(1.0_f32, border_color))
                                        .inner_margin(Margin::symmetric(10.0, 7.0));

                                    word_frame.show(ui, |ui| {
                                        ui.set_min_width(140.0);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:02}.", i + 1))
                                                    .size(12.0)
                                                    .color(Color32::from_rgb(34, 211, 238)),
                                            );
                                            ui.label(
                                                RichText::new(word)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            );
                                        });
                                    });

                                    if (i + 1) % 3 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });

                        ui.add_space(10.0);

                        // Copy Mnemonic Button
                        let full_phrase = self.generated_mnemonic_words.join(" ");
                        let copy_btn = ui.add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new(
                                RichText::new("📋 Скопировать сид-фразу (12 слов)")
                                    .size(12.5)
                                    .color(Color32::from_rgb(203, 213, 225)),
                            )
                            .fill(Color32::from_rgb(24, 32, 47))
                            .rounding(Rounding::same(6.0)),
                        );
                        if copy_btn.clicked() {
                            ui.output_mut(|o| o.copied_text = full_phrase.clone());
                            self.copied_feedback = Some(("Сид-фраза скопирована в буфер обмена".into(), current_time_secs()));
                        }

                        ui.add_space(12.0);

                        // Checkbox confirmation
                        ui.horizontal(|ui| {
                            ui.checkbox(
                                &mut self.has_saved_seed,
                                RichText::new("Я надёжно сохранил сид-фразу из 12 слов")
                                    .size(13.0)
                                    .color(Color32::from_rgb(226, 232, 240)),
                            );
                        });

                        ui.add_space(12.0);

                        // Display Name Input
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("👤 Ваше имя:").size(13.0).strong().color(Color32::WHITE));
                        });
                        ui.add_space(4.0);

                        let input_frame = Frame::none()
                            .fill(dark_input_bg)
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.0_f32, border_color))
                            .inner_margin(Margin::symmetric(12.0, 8.0));

                        let mut enter_pressed = false;
                        input_frame.show(ui, |ui| {
                            let resp = ui.add(
                                TextEdit::singleline(&mut self.create_name_input)
                                    .hint_text("например, Alice или Satoshi")
                                    .desired_width(ui.available_width())
                                    .frame(false),
                            );
                            if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                enter_pressed = true;
                            }
                        });

                        ui.add_space(8.0);

                        // Optional Bio Input
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("📝 Описание (Bio / Статус):").size(13.0).color(Color32::from_rgb(203, 213, 225)));
                        });
                        ui.add_space(4.0);

                        let bio_frame = Frame::none()
                            .fill(dark_input_bg)
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.0_f32, border_color))
                            .inner_margin(Margin::symmetric(12.0, 8.0));

                        bio_frame.show(ui, |ui| {
                            ui.add(
                                TextEdit::singleline(&mut self.create_bio_input)
                                    .hint_text("например, P2P enthusiast | Rust developer")
                                    .desired_width(ui.available_width())
                                    .frame(false),
                            );
                        });

                        ui.add_space(16.0);

                        let can_submit = self.has_saved_seed && !self.create_name_input.trim().is_empty();
                        let submit_btn = ui.add_enabled_ui(can_submit, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 42.0],
                                egui::Button::new(
                                    RichText::new("🚀 Создать Identity и войти")
                                        .size(15.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(if can_submit { cyan_500 } else { Color32::from_rgb(51, 65, 85) })
                                .rounding(Rounding::same(10.0)),
                            )
                        });

                        if (submit_btn.inner.clicked() || (enter_pressed && can_submit)) && can_submit {
                            let final_name = self.create_name_input.trim().to_string();
                            let final_bio = self.create_bio_input.trim().to_string();
                            let phrase = self.generated_mnemonic_words.join(" ");

                            self.my_display_name = final_name.clone();
                            self.my_bio = final_bio.clone();
                            self.my_mnemonic = phrase.clone();
                            self.edit_name_input = final_name.clone();
                            self.edit_bio_input = final_bio.clone();
                            self.is_registered = true;

                            let _ = self.cmd_tx.send(AppCommand::RegisterAccount {
                                display_name: final_name,
                                bio: final_bio,
                                mnemonic: phrase,
                            });
                        }
                    }
                    OnboardingTab::Restore => {
                        ui.label(
                            RichText::new("📥 Введите 12 слов сид-фразы BIP-39:")
                                .size(13.5)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new("Слова должны быть разделены пробелами.")
                                .size(12.0)
                                .color(Color32::from_rgb(148, 163, 184)),
                        );

                        ui.add_space(10.0);

                        let input_frame = Frame::none()
                            .fill(dark_input_bg)
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.0_f32, border_color))
                            .inner_margin(Margin::symmetric(12.0, 8.0));

                        input_frame.show(ui, |ui| {
                            ui.add(
                                TextEdit::multiline(&mut self.restore_mnemonic_input)
                                    .hint_text("вставьте 12 слов через пробел, например: abandon ability able about...")
                                    .desired_rows(3)
                                    .desired_width(ui.available_width())
                                    .frame(false),
                            );
                        });

                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("👤 Ваше имя:").size(13.0).strong().color(Color32::WHITE));
                        });
                        ui.add_space(4.0);

                        let name_frame = Frame::none()
                            .fill(dark_input_bg)
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.0_f32, border_color))
                            .inner_margin(Margin::symmetric(12.0, 8.0));

                        let mut enter_restore = false;
                        name_frame.show(ui, |ui| {
                            let resp = ui.add(
                                TextEdit::singleline(&mut self.restore_name_input)
                                    .hint_text("например, Alice")
                                    .desired_width(ui.available_width())
                                    .frame(false),
                            );
                            if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                enter_restore = true;
                            }
                        });

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("📝 Описание (Bio / Статус):").size(13.0).color(Color32::from_rgb(203, 213, 225)));
                        });
                        ui.add_space(4.0);

                        let restore_bio_frame = Frame::none()
                            .fill(dark_input_bg)
                            .rounding(Rounding::same(8.0))
                            .stroke(Stroke::new(1.0_f32, border_color))
                            .inner_margin(Margin::symmetric(12.0, 8.0));

                        restore_bio_frame.show(ui, |ui| {
                            ui.add(
                                TextEdit::singleline(&mut self.restore_bio_input)
                                    .hint_text("например, P2P enthusiast")
                                    .desired_width(ui.available_width())
                                    .frame(false),
                            );
                        });

                        if let Some(ref err) = self.restore_error {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("⚠ {}", err))
                                    .size(12.5)
                                    .color(Color32::from_rgb(248, 113, 113)),
                            );
                        }

                        ui.add_space(16.0);

                        let restore_submit_btn = ui.add_sized(
                            [ui.available_width(), 42.0],
                            egui::Button::new(
                                RichText::new("📥 Восстановить Identity")
                                    .size(15.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(cyan_500)
                            .rounding(Rounding::same(10.0)),
                        );

                        if restore_submit_btn.clicked() || enter_restore {
                            let trimmed_words = self.restore_mnemonic_input.trim();
                            let words_vec: Vec<&str> = trimmed_words.split_whitespace().collect();

                            if words_vec.len() != 12 {
                                self.restore_error = Some(format!("Требуется ровно 12 слов (введено: {})", words_vec.len()));
                            } else if !core_ffi::ffi_mnemonic_validate(trimmed_words) {
                                self.restore_error = Some("Ошибка валидации сид-фразы: неизвестные слова или неверная контрольная сумма".into());
                            } else {
                                self.restore_error = None;
                                let name = if self.restore_name_input.trim().is_empty() {
                                    "UserNode".to_string()
                                } else {
                                    self.restore_name_input.trim().to_string()
                                };
                                let bio = self.restore_bio_input.trim().to_string();

                                self.my_display_name = name.clone();
                                self.my_bio = bio.clone();
                                self.my_mnemonic = trimmed_words.to_string();
                                self.edit_name_input = name.clone();
                                self.edit_bio_input = bio.clone();
                                self.is_registered = true;

                                let _ = self.cmd_tx.send(AppCommand::RegisterAccount {
                                    display_name: name,
                                    bio,
                                    mnemonic: trimmed_words.to_string(),
                                });
                            }
                        }
                    }
                }
            });
        });
    }

    fn render_telegram_sidebar(&mut self, ui: &mut Ui) {
        let panel_bg = Color32::from_rgb(30, 41, 59);
        let border_color = Color32::from_rgb(51, 65, 85);
        let cyan_accent = Color32::from_rgb(6, 182, 212);

        Frame::none()
            .fill(panel_bg)
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                // Top Action Bar in Sidebar: Hamburger Menu ☰, Search Bar 🔍, Add Contact ➕
                ui.horizontal(|ui| {
                    // Menu Button (Hamburger)
                    let menu_btn = ui.add_sized(
                        [34.0, 34.0],
                        egui::Button::new(RichText::new("☰").size(18.0).color(Color32::from_rgb(203, 213, 225)))
                            .rounding(Rounding::same(8.0)),
                    );
                    if menu_btn.on_hover_text("Профиль и настройки").clicked() {
                        self.edit_name_input = self.my_display_name.clone();
                        self.edit_bio_input = self.my_bio.clone();
                        self.profile_drawer_open = !self.profile_drawer_open;
                    }

                    ui.add_space(4.0);

                    // Search Input Pill
                    let search_frame = Frame::none()
                        .fill(Color32::from_rgb(15, 23, 42))
                        .rounding(Rounding::same(16.0))
                        .stroke(Stroke::new(1.0_f32, border_color))
                        .inner_margin(Margin::symmetric(10.0, 6.0));

                    search_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🔍").size(12.0).color(Color32::from_rgb(148, 163, 184)));
                            ui.add(
                                TextEdit::singleline(&mut self.search_query)
                                    .hint_text("Поиск чатов...")
                                    .desired_width(ui.available_width() - 40.0)
                                    .frame(false),
                            );
                        });
                    });

                    ui.add_space(4.0);

                    // Add Contact Button
                    let add_btn = ui.add_sized(
                        [34.0, 34.0],
                        egui::Button::new(RichText::new("➕").size(14.0).color(Color32::WHITE))
                            .fill(cyan_accent)
                            .rounding(Rounding::same(17.0)),
                    );
                    if add_btn.on_hover_text("Добавить контакт").clicked() {
                        self.add_contact_modal = true;
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // User profile mini row in sidebar
                ui.horizontal(|ui| {
                    Self::render_avatar(ui, &self.my_display_name, 38.0, true);
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&self.my_display_name).size(14.0).strong().color(Color32::WHITE));
                        if !self.my_bio.is_empty() {
                            ui.label(
                                RichText::new(&self.my_bio)
                                    .size(11.5)
                                    .color(Color32::from_rgb(34, 211, 238)),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!("ID: {}...", &self.my_user_id[..8.min(self.my_user_id.len())]))
                                    .size(11.0)
                                    .monospace()
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                        }
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                let network_status = if self.dht_peers > 0 {
                    format!("● {} пиров в сети", self.dht_peers)
                } else {
                    "○ Ожидание подключения к сети".to_string()
                };
                ui.label(
                    RichText::new(network_status)
                        .size(11.0)
                        .color(if self.dht_peers > 0 {
                            Color32::from_rgb(34, 197, 94)
                        } else {
                            Color32::from_rgb(148, 163, 184)
                        }),
                );

                // Filter contacts by search query
                let query = self.search_query.trim().to_lowercase();
                let filtered_contacts: Vec<SavedContact> = self
                    .contacts
                    .iter()
                    .filter(|c| {
                        query.is_empty()
                            || c.name.to_lowercase().contains(&query)
                            || c.user_id_hex.to_lowercase().contains(&query)
                    })
                    .cloned()
                    .collect();

                // Chat list in ScrollArea
                if filtered_contacts.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(30.0);
                        ui.label(RichText::new("📭").size(24.0));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(if query.is_empty() {
                                "Список чатов пуст.\nНажмите ➕ чтобы добавить собеседника."
                            } else {
                                "Ничего не найдено"
                            })
                            .size(12.5)
                            .color(Color32::from_rgb(148, 163, 184)),
                        );
                    });
                } else {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);

                            for contact in filtered_contacts {
                                let is_selected = self
                                    .selected_contact
                                    .as_ref()
                                    .map(|s| s.user_id_hex == contact.user_id_hex)
                                    .unwrap_or(false);

                                let is_online = contact.last_seen_addr != "unknown" && !contact.last_seen_addr.is_empty();

                                let row_bg = if is_selected {
                                    Color32::from_rgb(14, 116, 144) // Active cyan background
                                } else {
                                    Color32::from_rgb(24, 32, 47)
                                };

                                let row_frame = Frame::none()
                                    .fill(row_bg)
                                    .rounding(Rounding::same(10.0))
                                    .inner_margin(Margin::symmetric(10.0, 8.0));

                                let row_resp = row_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        Self::render_avatar(ui, &contact.name, 40.0, is_online);
                                        ui.add_space(10.0);

                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&contact.name)
                                                        .size(14.0)
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                );

                                                if self.unread_set.contains(&contact.user_id_hex) {
                                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                        ui.painter().circle_filled(
                                                            ui.cursor().center(),
                                                            4.0,
                                                            Color32::from_rgb(34, 211, 238),
                                                        );
                                                    });
                                                }
                                            });

                                            ui.add_space(2.0);

                                            // Last message preview if any
                                            let preview_text = if let Some(last_msg) = self.last_messages.get(&contact.user_id_hex) {
                                                let prefix = if last_msg.incoming { "" } else { "Вы: " };
                                                if let Some(ref _img) = last_msg.image_base64 {
                                                    if last_msg.text.is_empty() {
                                                        format!("{}🖼 [Фотография]", prefix)
                                                    } else {
                                                        format!("{}🖼 {}", prefix, last_msg.text)
                                                    }
                                                } else {
                                                    format!("{}{}", prefix, last_msg.text)
                                                }
                                            } else if !contact.bio.is_empty() {
                                                contact.bio.clone()
                                            } else {
                                                format!("ID: {}...", &contact.user_id_hex[..8.min(contact.user_id_hex.len())])
                                            };

                                            ui.label(
                                                RichText::new(preview_text)
                                                    .size(12.0)
                                                    .color(if is_selected { Color32::from_rgb(207, 250, 254) } else { Color32::from_rgb(148, 163, 184) }),
                                            );
                                        });
                                    });
                                });

                                // Click selection
                                if row_resp.response.interact(Sense::click()).clicked() {
                                    self.selected_contact = Some(contact.clone());
                                    self.unread_set.remove(&contact.user_id_hex);
                                    let _ = self.cmd_tx.send(AppCommand::SelectContact(contact.user_id_hex.clone()));
                                }
                            }
                        });
                }
            });
    }

    fn render_chat_header(&mut self, ui: &mut Ui, contact: &SavedContact) {
        let header_bg = Color32::from_rgb(30, 41, 59);
        let border_color = Color32::from_rgb(51, 65, 85);
        let is_online = contact.last_seen_addr != "unknown" && !contact.last_seen_addr.is_empty();

        Frame::none()
            .fill(header_bg)
            .stroke(Stroke::new(1.0_f32, border_color))
            .inner_margin(Margin::symmetric(16.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Self::render_avatar(ui, &contact.name, 40.0, is_online);
                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new(&contact.name).size(16.0).strong().color(Color32::WHITE));
                        ui.horizontal(|ui| {
                            if !contact.bio.is_empty() {
                                ui.label(
                                    RichText::new(format!("{} • ", contact.bio))
                                        .size(12.0)
                                        .color(Color32::from_rgb(203, 213, 225)),
                                );
                            }
                            if is_online {
                                ui.label(
                                    RichText::new("🟢 Direct P2P Online")
                                        .size(12.0)
                                        .color(Color32::from_rgb(74, 222, 128)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("⚪ DHT Mailbox Relay")
                                        .size(12.0)
                                        .color(Color32::from_rgb(148, 163, 184)),
                                );
                            }
                        });
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let copy_btn = ui.button(RichText::new("📋 Копировать ID").size(12.0));
                        if copy_btn.clicked() {
                            ui.output_mut(|o| o.copied_text = contact.user_id_hex.clone());
                            self.copied_feedback = Some(("ID собеседника скопирован".into(), current_time_secs()));
                        }
                    });
                });
            });
    }

    fn render_message_bubble(&mut self, ui: &mut Ui, msg: &SavedChatMessage, _my_user_id: &str) {
        let is_incoming = msg.incoming;
        let bubble_bg = if is_incoming {
            Color32::from_rgb(30, 41, 59) // Slate 800
        } else {
            Color32::from_rgb(14, 116, 144) // Cyan 700 / Slate Cyan
        };

        let max_bubble_width = (ui.available_width() * 0.65).min(520.0).max(140.0);

        let outer_align = if is_incoming {
            Layout::left_to_right(Align::Min)
        } else {
            Layout::right_to_left(Align::Min)
        };

        let mut clicked_photo = None;

        ui.with_layout(outer_align, |ui| {
            let rounding = if is_incoming {
                Rounding { nw: 14.0, ne: 14.0, sw: 2.0, se: 14.0 }
            } else {
                Rounding { nw: 14.0, ne: 14.0, sw: 14.0, se: 2.0 }
            };

            Frame::none()
                .fill(bubble_bg)
                .rounding(rounding)
                .stroke(if is_incoming {
                    Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85))
                } else {
                    Stroke::NONE
                })
                .inner_margin(Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.set_max_width(max_bubble_width);
                    ui.vertical(|ui| {
                        // Render attached image if present
                        if let Some(ref b64) = msg.image_base64 {
                            if let Ok(img_bytes) = BASE64_STANDARD.decode(b64) {
                                let uri = format!("bytes://msg_{}_{}", msg.id, img_bytes.len());
                                let img_widget = egui::Image::from_bytes(uri, img_bytes)
                                    .max_width(320.0)
                                    .rounding(Rounding::same(8.0))
                                    .sense(Sense::click());

                                let img_resp = ui.add(img_widget);
                                if img_resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("🔍 Нажмите для полноэкранного просмотра").clicked() {
                                    let sender_name = if is_incoming {
                                        self.selected_contact.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Собеседник".into())
                                    } else {
                                        "Вы".into()
                                    };
                                    clicked_photo = Some(PhotoPreviewData {
                                        title: format!("Фото от {}", sender_name),
                                        sender_name,
                                        sender_id: msg.sender_id_hex.clone(),
                                        timestamp: msg.timestamp,
                                        caption: msg.text.clone(),
                                        base64_data: b64.clone(),
                                        incoming: is_incoming,
                                    });
                                }
                                ui.add_space(4.0);
                            }
                        }

                        // Render text if not empty
                        if !msg.text.is_empty() {
                            ui.label(
                                RichText::new(&msg.text)
                                    .size(14.0)
                                    .color(Color32::WHITE),
                            );
                        }

                        ui.add_space(3.0);

                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !is_incoming {
                                    let (tick_str, tick_color) = if msg.delivered {
                                        ("\u{2713}\u{2713}", Color32::from_rgb(34, 211, 238))
                                    } else {
                                        ("\u{2713}", Color32::from_rgb(148, 163, 184))
                                    };
                                    ui.label(RichText::new(tick_str).size(12.0).strong().color(tick_color));
                                }

                                let time_str = format_timestamp(msg.timestamp);
                                ui.label(RichText::new(time_str).size(10.5).color(Color32::from_rgb(203, 213, 225)));
                            });
                        });
                    });
                });
        });

        if let Some(photo) = clicked_photo {
            self.preview_photo = Some(photo);
        }
    }

    fn render_emoji_picker(&mut self, ui: &mut Ui) {
        let smiles_emojis = [
            "😀", "😃", "😄", "😁", "😆", "😅", "😂", "🤣", "😊", "😇",
            "🙂", "😉", "😌", "😍", "🥰", "😘", "😋", "😛", "😜", "🤪",
            "😎", "🥳", "😏", "😒", "😞", "😔", "😟", "😕", "🙁", "😣",
            "😖", "😫", "😩", "🥺", "😢", "😭", "😤", "😠", "😡", "🤬",
            "🤯", "😳", "🥵", "🥶", "😱", "😨", "😰", "😥", "😓", "🤗",
            "🤔", "🤭", "🤫", "🤥", "😶", "😐", "😑", "😬", "🙄", "😯",
        ];

        let gesture_emojis = [
            "👍", "👎", "👏", "🙌", "👐", "🤲", "🤝", "🙏", "✍", "🤳",
            "💪", "🦾", "✌", "🤞", "🤟", "🤘", "🤙", "👈", "👉", "👆",
            "👇", "☝", "✋", "🤚", "🖐", "🖖", "👋", "🤙", "✊", "👊",
            "🤛", "🤜", "👌", "🤌", "🤏", "👑", "🎯", "🎲", "🧩", "💎",
        ];

        let heart_emojis = [
            "❤️", "🧡", "💛", "💚", "💙", "💜", "🖤", "🤍", "🤎", "💔",
            "❣️", "💕", "💞", "💓", "💗", "💖", "💘", "💝", "🔥", "✨",
            "⭐", "🌟", "💫", "⚡", "💥", "💯", "🎉", "🎊", "🏆", "🥇",
        ];

        let tech_emojis = [
            "🔒", "🔑", "🛡", "💻", "📱", "💾", "🌐", "🛰", "📡", "⚡",
            "🚀", "🛸", "🪐", "🔋", "💡", "🔌", "⚙", "🔧", "📦", "📬",
            "🖥", "⌨", "🖱", "🖨", "💿", "🕹", "📷", "📹", "🔍", "🔎",
        ];

        let food_emojis = [
            "☕", "🍵", "🍕", "🍔", "🍟", "🌭", "🍿", "🍩", "🍪", "🍫",
            "🍰", "🎂", "🍎", "🍓", "🍉", "🍇", "🥑", "🥦", "🍣", "🍜",
        ];

        let current_emojis: &[&str] = match self.emoji_category {
            EmojiCategory::Smiles => &smiles_emojis,
            EmojiCategory::Gestures => &gesture_emojis,
            EmojiCategory::Hearts => &heart_emojis,
            EmojiCategory::Tech => &tech_emojis,
            EmojiCategory::Food => &food_emojis,
        };

        let picker_frame = Frame::none()
            .fill(Color32::from_rgb(24, 32, 47))
            .rounding(Rounding::same(12.0))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
            .inner_margin(Margin::same(10.0));

        picker_frame.show(ui, |ui| {
            ui.set_max_width(360.0);

            // Category Bar
            ui.horizontal(|ui| {
                let cats = [
                    (EmojiCategory::Smiles, "😀"),
                    (EmojiCategory::Gestures, "👍"),
                    (EmojiCategory::Hearts, "❤️"),
                    (EmojiCategory::Tech, "🚀"),
                    (EmojiCategory::Food, "🍕"),
                ];

                for (cat, icon) in cats {
                    let is_sel = self.emoji_category == cat;
                    let btn = ui.add_sized(
                        [30.0, 26.0],
                        egui::Button::new(RichText::new(icon).size(14.0))
                            .fill(if is_sel { Color32::from_rgb(14, 116, 144) } else { Color32::from_rgb(30, 41, 59) })
                            .rounding(Rounding::same(6.0)),
                    );
                    if btn.clicked() {
                        self.emoji_category = cat;
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("✖").clicked() {
                        self.emoji_picker_open = false;
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    egui::Grid::new("emoji_grid")
                        .num_columns(10)
                        .spacing([3.0, 3.0])
                        .show(ui, |ui| {
                            for (i, emoji) in current_emojis.iter().enumerate() {
                                let btn = ui.add_sized(
                                    [28.0, 28.0],
                                    egui::Button::new(RichText::new(*emoji).size(16.0))
                                        .fill(Color32::from_rgb(30, 41, 59))
                                        .rounding(Rounding::same(6.0)),
                                );
                                if btn.clicked() {
                                    self.message_input.push_str(emoji);
                                }
                                if (i + 1) % 10 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
        });
    }

    fn render_chat_input_bar(&mut self, ui: &mut Ui, recipient_id: &str) {
        let input_bg = Color32::from_rgb(30, 41, 59);
        let border_color = Color32::from_rgb(51, 65, 85);
        let cyan_accent = Color32::from_rgb(6, 182, 212);

        if self.emoji_picker_open {
            ui.horizontal(|ui| {
                self.render_emoji_picker(ui);
            });
            ui.add_space(4.0);
        }

        Frame::none()
            .fill(input_bg)
            .stroke(Stroke::new(1.0_f32, border_color))
            .inner_margin(Margin::symmetric(14.0, 8.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Show pending image attachment chip if selected
                    let mut remove_image = false;
                    if let Some(ref name) = self.pending_image_name {
                        let name_clone = name.clone();
                        ui.horizontal(|ui| {
                            let chip_frame = Frame::none()
                                .fill(Color32::from_rgb(15, 23, 42))
                                .rounding(Rounding::same(8.0))
                                .stroke(Stroke::new(1.0_f32, cyan_accent))
                                .inner_margin(Margin::symmetric(8.0, 4.0));

                            chip_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("🖼 {}", name_clone)).size(12.0).color(Color32::WHITE));
                                    if ui.button("✖").on_hover_text("Удалить фото").clicked() {
                                        remove_image = true;
                                    }
                                });
                            });
                        });
                        ui.add_space(4.0);
                    }
                    if remove_image {
                        self.pending_image_base64 = None;
                        self.pending_image_name = None;
                    }

                    ui.horizontal(|ui| {
                        // File attachment button 📎
                        let attach_btn = ui.add_sized(
                            [34.0, 34.0],
                            egui::Button::new(RichText::new("📎").size(16.0).color(Color32::from_rgb(203, 213, 225)))
                                .fill(Color32::from_rgb(24, 32, 47))
                                .rounding(Rounding::same(17.0)),
                        );
                        if attach_btn.on_hover_text("Прикрепить фото / изображение").clicked() {
                            if let Some((fname, b64)) = pick_and_process_image() {
                                self.pending_image_name = Some(fname);
                                self.pending_image_base64 = Some(b64);
                            }
                        }

                        // Emoji button 😀
                        let emoji_btn = ui.add_sized(
                            [34.0, 34.0],
                            egui::Button::new(RichText::new("😀").size(16.0).color(Color32::from_rgb(203, 213, 225)))
                                .fill(Color32::from_rgb(24, 32, 47))
                                .rounding(Rounding::same(17.0)),
                        );
                        if emoji_btn.clicked() {
                            self.emoji_picker_open = !self.emoji_picker_open;
                        }

                        ui.add_space(4.0);

                        // Message Text Input Pill
                        let pill_frame = Frame::none()
                            .fill(Color32::from_rgb(15, 23, 42))
                            .rounding(Rounding::same(18.0))
                            .stroke(Stroke::new(1.0_f32, border_color))
                            .inner_margin(Margin::symmetric(14.0, 7.0));

                        let mut send_triggered = false;

                        pill_frame.show(ui, |ui| {
                            let text_edit = TextEdit::singleline(&mut self.message_input)
                                .hint_text("Напишите сообщение...")
                                .desired_width(ui.available_width() - 50.0)
                                .frame(false);

                            let resp = ui.add(text_edit);

                            if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                send_triggered = true;
                            }
                        });

                        ui.add_space(6.0);

                        // Send Button ➤
                        let send_btn = ui.add_sized(
                            [36.0, 36.0],
                            egui::Button::new(RichText::new("➤").size(15.0).color(Color32::WHITE))
                                .fill(cyan_accent)
                                .rounding(Rounding::same(18.0)),
                        );

                        if send_btn.clicked() || send_triggered {
                            let trimmed = self.message_input.trim().to_string();
                            let img_opt = self.pending_image_base64.take();
                            self.pending_image_name = None;

                            if !trimmed.is_empty() || img_opt.is_some() {
                                let _ = self.cmd_tx.send(AppCommand::SendMessage {
                                    recipient_id: recipient_id.to_string(),
                                    text: trimmed,
                                    image_base64: img_opt,
                                });
                                self.message_input.clear();
                                self.emoji_picker_open = false;
                            }
                        }
                    });
                });
            });
    }

    fn render_profile_modal(&mut self, ctx: &egui::Context) {
        if !self.profile_drawer_open {
            return;
        }

        let mut close_drawer = false;
        let cyan_500 = Color32::from_rgb(6, 182, 212);
        let border_color = Color32::from_rgb(51, 65, 85);
        let dark_input_bg = Color32::from_rgb(15, 23, 42);

        egui::Window::new("⚙ Мой профиль & Настройки")
            .open(&mut self.profile_drawer_open)
            .collapsible(false)
            .resizable(false)
            .constrain(false)
            .default_width(520.0)
            .default_pos(egui::pos2(300.0, 90.0))
            .show(ctx, |ui| {
                ui.add_space(6.0);

                // Profile Avatar & Logo
                ui.horizontal(|ui| {
                    Self::render_avatar(ui, &self.my_display_name, 56.0, true);
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&self.my_display_name).size(18.0).strong().color(Color32::WHITE));
                        if !self.my_bio.is_empty() {
                            ui.label(RichText::new(&self.my_bio).size(12.5).color(Color32::from_rgb(34, 211, 238)));
                        } else {
                            ui.label(RichText::new("NodeX P2P Messenger").size(12.0).color(Color32::from_rgb(148, 163, 184)));
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add(
                            egui::Image::from_bytes("bytes://nodex_logo_profile.png", LOGO_BYTES)
                                .fit_to_exact_size(Vec2::new(42.0, 42.0))
                                .rounding(Rounding::same(8.0)),
                        );
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Editable Fields: Display Name & Bio
                ui.label(RichText::new("✏ РЕДАКТИРОВАНИЕ ПРОФИЛЯ:").size(12.0).strong().color(cyan_500));
                ui.add_space(4.0);

                ui.label(RichText::new("Имя (Display Name):").size(12.0).color(Color32::from_rgb(203, 213, 225)));
                let name_frame = Frame::none()
                    .fill(dark_input_bg)
                    .rounding(Rounding::same(8.0))
                    .stroke(Stroke::new(1.0_f32, border_color))
                    .inner_margin(Margin::symmetric(10.0, 6.0));

                name_frame.show(ui, |ui| {
                    ui.add(
                        TextEdit::singleline(&mut self.edit_name_input)
                            .desired_width(ui.available_width())
                            .frame(false),
                    );
                });

                ui.add_space(6.0);
                ui.label(RichText::new("Описание / Статус (Bio):").size(12.0).color(Color32::from_rgb(203, 213, 225)));
                let bio_frame = Frame::none()
                    .fill(dark_input_bg)
                    .rounding(Rounding::same(8.0))
                    .stroke(Stroke::new(1.0_f32, border_color))
                    .inner_margin(Margin::symmetric(10.0, 6.0));

                bio_frame.show(ui, |ui| {
                    ui.add(
                        TextEdit::singleline(&mut self.edit_bio_input)
                            .hint_text("например, P2P enthusiast | Rust developer")
                            .desired_width(ui.available_width())
                            .frame(false),
                    );
                });

                ui.add_space(8.0);
                let save_profile_btn = ui.add_sized(
                    [ui.available_width(), 32.0],
                    egui::Button::new(RichText::new("💾 Сохранить изменения профиля").size(13.0).strong().color(Color32::WHITE))
                        .fill(cyan_500)
                        .rounding(Rounding::same(6.0)),
                );
                if save_profile_btn.clicked() {
                    let new_name = self.edit_name_input.trim().to_string();
                    let new_bio = self.edit_bio_input.trim().to_string();
                    if !new_name.is_empty() {
                        self.my_display_name = new_name.clone();
                        self.my_bio = new_bio.clone();
                        let _ = self.cmd_tx.send(AppCommand::UpdateProfile {
                            display_name: new_name,
                            bio: new_bio,
                        });
                        self.copied_feedback = Some(("Профиль успешно обновлён!".into(), current_time_secs()));
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // User Messenger E2EE ID
                ui.label(RichText::new("🔑 MESSENGER USER ID (E2EE PUBLIC KEY):").size(12.0).strong().color(cyan_500));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&self.my_user_id)
                            .size(11.5)
                            .monospace()
                            .color(Color32::from_rgb(226, 232, 240)),
                    );
                    if ui.button("📋").on_hover_text("Копировать User ID").clicked() {
                        ui.output_mut(|o| o.copied_text = self.my_user_id.clone());
                        self.copied_feedback = Some(("User ID скопирован в буфер".into(), current_time_secs()));
                    }
                });

                ui.add_space(8.0);

                // Kademlia Transport Node ID
                ui.label(RichText::new("🌐 KADEMLIA TRANSPORT NODE ID:").size(12.0).strong().color(Color32::from_rgb(148, 163, 184)));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&self.my_dht_node_id)
                            .size(11.5)
                            .monospace()
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                    if ui.button("📋").on_hover_text("Копировать Node ID").clicked() {
                        ui.output_mut(|o| o.copied_text = self.my_dht_node_id.clone());
                        self.copied_feedback = Some(("DHT Node ID скопирован".into(), current_time_secs()));
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Mnemonic Phrase Section
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔐 СИД-ФРАЗА ВОССТАНОВЛЕНИЯ (BIP-39):").size(12.0).strong().color(Color32::from_rgb(251, 191, 36)));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let toggle_label = if self.show_seed_in_profile { "🙈 Скрыть" } else { "👁 Показать" };
                        if ui.button(toggle_label).clicked() {
                            self.show_seed_in_profile = !self.show_seed_in_profile;
                        }
                    });
                });

                ui.add_space(4.0);

                let seed_box = Frame::none()
                    .fill(Color32::from_rgb(15, 23, 42))
                    .rounding(Rounding::same(8.0))
                    .stroke(Stroke::new(1.0_f32, border_color))
                    .inner_margin(Margin::same(10.0));

                seed_box.show(ui, |ui| {
                    ui.set_max_width(ui.available_width());
                    if self.show_seed_in_profile {
                        ui.label(
                            RichText::new(&self.my_mnemonic)
                                .size(12.5)
                                .monospace()
                                .color(Color32::from_rgb(226, 232, 240)),
                        );
                        ui.add_space(6.0);
                        if ui.button("📋 Скопировать сид-фразу").clicked() {
                            ui.output_mut(|o| o.copied_text = self.my_mnemonic.clone());
                            self.copied_feedback = Some(("Сид-фраза скопирована".into(), current_time_secs()));
                        }
                    } else {
                        ui.label(
                            RichText::new("•••• •••• •••• •••• •••• •••• •••• •••• •••• •••• •••• ••••")
                                .size(12.0)
                                .color(Color32::from_rgb(100, 116, 139)),
                        );
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Network Statistics
                ui.label(RichText::new("📊 Статистика P2P соединения:").size(12.5).strong().color(Color32::WHITE));
                ui.add_space(4.0);

                egui::Grid::new("net_stats_grid")
                    .num_columns(2)
                    .spacing([20.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("P2P Транспорт:").color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("🔒 Native UDP (Direct & DHT Relay)").strong().color(Color32::from_rgb(34, 197, 94)));
                        ui.end_row();

                        ui.label(RichText::new("Пиры в Kademlia DHT:").color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new(format!("{} активных узлов", self.dht_peers)).strong().color(Color32::from_rgb(34, 197, 94)));
                        ui.end_row();

                        ui.label(RichText::new("Ключи в DHT хранилище:").color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new(format!("{} записей", self.stored_keys)).strong().color(cyan_500));
                        ui.end_row();

                        ui.label(RichText::new("Шифрование локальной БД:").color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("ChaCha20-Poly1305 (NODEXENC)").strong().color(Color32::from_rgb(56, 189, 248)));
                        ui.end_row();
                    });

                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Закрыть").clicked() {
                        close_drawer = true;
                    }
                });
            });

        if close_drawer {
            self.profile_drawer_open = false;
        }
    }

    fn render_photo_lightbox_modal(&mut self, ctx: &egui::Context) {
        if self.preview_photo.is_none() {
            return;
        }

        let mut close_lightbox = false;
        let mut save_photo = false;
        let photo_data = self.preview_photo.clone().unwrap();

        let cyan_500 = Color32::from_rgb(6, 182, 212);

        egui::Window::new(&photo_data.title)
            .collapsible(false)
            .resizable(true)
            .constrain(false)
            .default_width(620.0)
            .default_pos(egui::pos2(250.0, 80.0))
            .show(ctx, |ui| {
                ui.add_space(4.0);

                if let Ok(img_bytes) = BASE64_STANDARD.decode(&photo_data.base64_data) {
                    let uri = format!("bytes://lightbox_{}_{}", photo_data.timestamp, img_bytes.len());
                    ui.vertical_centered(|ui| {
                        ui.add(
                            egui::Image::from_bytes(uri, img_bytes)
                                .max_width(580.0)
                                .max_height(420.0)
                                .rounding(Rounding::same(8.0)),
                        );
                    });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("Отправитель: {}", photo_data.sender_name)).size(13.0).strong().color(Color32::WHITE));
                        ui.label(RichText::new(format!("Время: {}", format_timestamp(photo_data.timestamp))).size(12.0).color(Color32::from_rgb(148, 163, 184)));
                        if !photo_data.caption.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("💬 {}", photo_data.caption)).size(13.0).color(Color32::from_rgb(226, 232, 240)));
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let save_btn = ui.add(
                            egui::Button::new(RichText::new("💾 Сохранить на диск").strong().color(Color32::WHITE))
                                .fill(cyan_500)
                                .rounding(Rounding::same(6.0)),
                        );
                        if save_btn.clicked() {
                            save_photo = true;
                        }

                        if ui.button("Закрыть").clicked() {
                            close_lightbox = true;
                        }
                    });
                });
            });

        if save_photo {
            if let Ok(img_bytes) = BASE64_STANDARD.decode(&photo_data.base64_data) {
                let default_name = format!("nodex_photo_{}.jpg", photo_data.timestamp);
                if let Some(save_path) = rfd::FileDialog::new().set_file_name(&default_name).save_file() {
                    if std::fs::write(&save_path, &img_bytes).is_ok() {
                        self.copied_feedback = Some(("Фото успешно сохранено на диск!".into(), current_time_secs()));
                    }
                }
            }
        }

        if close_lightbox {
            self.preview_photo = None;
        }
    }

    fn render_add_contact_dialog(&mut self, ctx: &egui::Context) {
        if !self.add_contact_modal {
            return;
        }

        let mut close_modal = false;
        let cyan_500 = Color32::from_rgb(6, 182, 212);

        egui::Window::new("➕ Добавить контакт в NodeX")
            .open(&mut self.add_contact_modal)
            .collapsible(false)
            .resizable(false)
            .constrain(false)
            .default_width(420.0)
            .default_pos(egui::pos2(360.0, 180.0))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Имя собеседника (или оставьте пустым для автопоиска):").size(13.0).color(Color32::WHITE));
                ui.add(TextEdit::singleline(&mut self.new_contact_name).hint_text("например, Bob"));

                ui.add_space(8.0);
                ui.label(RichText::new("User ID собеседника (40 hex символов):").size(13.0).color(Color32::WHITE));
                ui.add(TextEdit::singleline(&mut self.new_contact_id).hint_text("вставьте 40-значный User ID..."));

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let add_btn = ui.add(
                        egui::Button::new(RichText::new("Добавить").strong().color(Color32::WHITE))
                            .fill(cyan_500),
                    );

                    if add_btn.clicked() {
                        let id = self.new_contact_id.trim();
                        let name = self.new_contact_name.trim();
                        if !id.is_empty() {
                            let _ = self.cmd_tx.send(AppCommand::AddContact {
                                user_id: id.to_string(),
                                name: name.to_string(),
                            });
                            self.new_contact_id.clear();
                            self.new_contact_name.clear();
                            close_modal = true;
                        }
                    }

                    if ui.button("Отмена").clicked() {
                        close_modal = true;
                    }
                });
            });

        if close_modal {
            self.add_contact_modal = false;
        }
    }
}

impl eframe::App for NodeXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events(ctx);
        self.setup_telegram_theme(ctx);

        let bg_slate_900 = Color32::from_rgb(15, 23, 42);

        // If not registered yet, show Welcome / Registration screen
        if !self.is_registered {
            egui::CentralPanel::default()
                .frame(Frame::none().fill(bg_slate_900))
                .show(ctx, |ui| {
                    self.render_onboarding_screen(ui);
                });
            return;
        }

        // Clean Telegram-Style Two-Column Layout (NO top panel)
        egui::SidePanel::left("telegram_sidebar")
            .exact_width(320.0)
            .resizable(false)
            .show(ctx, |ui| {
                self.render_telegram_sidebar(ui);
            });

        egui::CentralPanel::default()
            .frame(Frame::none().fill(bg_slate_900))
            .show(ctx, |ui| {
                if let Some(contact) = self.selected_contact.clone() {
                    // Top Chat Header
                    self.render_chat_header(ui, &contact);

                    // Messages Area
                    let available_height = (ui.available_height() - 75.0).max(100.0);
                    ScrollArea::vertical()
                        .max_height(available_height)
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add_space(10.0);
                            if self.messages.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(60.0);
                                    ui.label(RichText::new("🔒").size(28.0));
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new("Сквозное E2EE шифрование активно.")
                                            .size(13.0)
                                            .color(Color32::from_rgb(148, 163, 184)),
                                    );
                                    ui.label(
                                        RichText::new("Сообщения и фото передаются напрямую или через DHT Mailbox.")
                                            .size(12.0)
                                            .color(Color32::from_rgb(100, 116, 139)),
                                    );
                                });
                            } else {
                                ui.spacing_mut().item_spacing = Vec2::new(0.0, 8.0);
                                let my_uid = self.my_user_id.clone();
                                let msgs_clone = self.messages.clone();
                                for msg in &msgs_clone {
                                    self.render_message_bubble(ui, msg, &my_uid);
                                }
                            }
                            ui.add_space(10.0);
                        });

                    // Bottom Chat Input Bar
                    self.render_chat_input_bar(ui, &contact.user_id_hex);
                } else {
                    // Clean Empty Dashboard (no chat selected)
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.22);
                        ui.add(
                            egui::Image::from_bytes("bytes://nodex_logo_empty.png", LOGO_BYTES)
                                .fit_to_exact_size(Vec2::new(88.0, 88.0))
                                .rounding(Rounding::same(16.0)),
                        );
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new("NodeX Messenger")
                                .size(22.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Выберите диалог слева для начала безопасного общения")
                                .size(13.5)
                                .color(Color32::from_rgb(148, 163, 184)),
                        );
                        ui.label(
                            RichText::new("Все сообщения и фото защищены сквозным E2EE шифрованием ChaCha20-Poly1305")
                                .size(12.0)
                                .color(Color32::from_rgb(100, 116, 139)),
                        );

                    });
                }
            });

        // Modals & Drawers
        self.render_profile_modal(ctx);
        self.render_add_contact_dialog(ctx);
        self.render_photo_lightbox_modal(ctx);

        // Copied toast feedback
        if let Some((ref text, timestamp)) = self.copied_feedback {
            if current_time_secs() - timestamp < 2.5 {
                egui::Area::new(Id::new("copied_toast"))
                    .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -30.0])
                    .show(ctx, |ui| {
                        Frame::none()
                            .fill(Color32::from_rgba_premultiplied(15, 23, 42, 235))
                            .rounding(Rounding::same(16.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(6, 182, 212)))
                            .inner_margin(Margin::symmetric(16.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("✓ {}", text)).size(13.0).color(Color32::WHITE));
                            });
                    });
            }
        }
    }
}

fn pick_and_process_image() -> Option<(String, String)> {
    let path = rfd::FileDialog::new()
        .add_filter("Изображения", &["png", "jpg", "jpeg", "webp"])
        .pick_file()?;

    let file_name = path.file_name()?.to_string_lossy().to_string();
    let bytes = std::fs::read(&path).ok()?;

    let img = image::load_from_memory(&bytes).ok()?;
    // The image is nested in JSON, encrypted, and then sent through the
    // fixed-size UDP RPC value field. Keep the encoded attachment well below
    // that limit so the receiver gets the complete envelope.
    let resized = img.thumbnail(480, 480);
    let mut quality = 75;

    loop {
        let mut jpeg_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
        resized
            .write_to(&mut cursor, image::ImageOutputFormat::Jpeg(quality))
            .ok()?;

        let b64 = BASE64_STANDARD.encode(&jpeg_bytes);
        if b64.len() <= 32 * 1024 || quality <= 35 {
            return Some((file_name, b64));
        }
        quality -= 10;
    }
}

fn current_time_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn format_timestamp(timestamp: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let diff = now.saturating_sub(timestamp);
    if diff < 60 {
        "только что".into()
    } else if diff < 3600 {
        format!("{}м назад", diff / 60)
    } else {
        let hours = (timestamp / 3600) % 24;
        let mins = (timestamp / 60) % 60;
        format!("{:02}:{:02}", hours, mins)
    }
}
