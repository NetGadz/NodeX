use std::sync::mpsc::Receiver;
use eframe::egui::{self, Color32, Frame, Margin, Rounding, Stroke, Vec2};
use crate::db::{SavedChatMessage, SavedContact};

// Events sent from Tokio background worker to GUI thread
#[derive(Debug, Clone)]
pub enum UiEvent {
    NodeInfo {
        user_id: String,
        dht_node_id: String,
        display_name: String,
        listen_addr: String,
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
pub enum AppCommand {
    SendMessage { recipient_id: String, text: String },
    AddContact { user_id: String, name: String },
    RefreshInfo,
    RefreshContacts,
    SelectContact(String),
}

pub struct NodeXApp {
    // Communication channels
    ui_rx: Receiver<UiEvent>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<AppCommand>,

    // App State
    my_user_id: String,
    my_dht_node_id: String,
    my_display_name: String,
    my_listen_addr: String,
    dht_peers: usize,
    stored_keys: usize,

    contacts: Vec<SavedContact>,
    selected_contact: Option<SavedContact>,
    messages: Vec<SavedChatMessage>,

    // UI Input Buffers
    message_input: String,
    add_contact_modal: bool,
    new_contact_id: String,
    new_contact_name: String,
    status_msg: String,
}

impl NodeXApp {
    pub fn new(
        ui_rx: Receiver<UiEvent>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<AppCommand>,
    ) -> Self {
        // Trigger initial data refresh
        let _ = cmd_tx.send(AppCommand::RefreshInfo);
        let _ = cmd_tx.send(AppCommand::RefreshContacts);

        Self {
            ui_rx,
            cmd_tx,
            my_user_id: "Loading...".into(),
            my_dht_node_id: "Loading...".into(),
            my_display_name: "NodeX Peer".into(),
            my_listen_addr: "Connecting...".into(),
            dht_peers: 0,
            stored_keys: 0,
            contacts: Vec::new(),
            selected_contact: None,
            messages: Vec::new(),
            message_input: String::new(),
            add_contact_modal: false,
            new_contact_id: String::new(),
            new_contact_name: String::new(),
            status_msg: "P2P UDP Network Ready".into(),
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
                    listen_addr,
                    dht_peers,
                    stored_keys,
                } => {
                    self.my_user_id = user_id;
                    self.my_dht_node_id = dht_node_id;
                    self.my_display_name = display_name;
                    self.my_listen_addr = listen_addr;
                    self.dht_peers = dht_peers;
                    self.stored_keys = stored_keys;
                }
                UiEvent::ContactsList(list) => {
                    self.contacts = list;
                }
                UiEvent::MessagesList { contact_id, messages } => {
                    if let Some(ref sel) = self.selected_contact {
                        if sel.user_id_hex == contact_id {
                            self.messages = messages;
                        }
                    }
                }
                UiEvent::MessageSent(msg) => {
                    if let Some(ref sel) = self.selected_contact {
                        if sel.user_id_hex == msg.sender_id_hex || sel.user_id_hex == msg.recipient_id_hex {
                            self.messages.push(msg);
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

    fn setup_custom_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();

        // Slate & Electric Cyan Color Palette
        let bg_slate_900 = Color32::from_rgb(15, 23, 42);    // #0f172a
        let panel_slate_800 = Color32::from_rgb(30, 41, 59); // #1e293b
        let text_slate_50 = Color32::from_rgb(248, 250, 252);// #f8fafc
        let cyan_500 = Color32::from_rgb(6, 182, 212);       // #06b6d4
        let cyan_400 = Color32::from_rgb(34, 211, 238);      // #22d3ee
        let border_slate_700 = Color32::from_rgb(51, 65, 85); // #334155

        visuals.dark_mode = true;
        visuals.override_text_color = Some(text_slate_50);
        visuals.window_fill = bg_slate_900;
        visuals.panel_fill = panel_slate_800;

        // Custom Widget Geometry & Rounding (10px)
        let rounded = Rounding::same(10.0);

        visuals.widgets.noninteractive.bg_fill = panel_slate_800;
        visuals.widgets.noninteractive.rounding = rounded;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border_slate_700);

        visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 41, 59);
        visuals.widgets.inactive.rounding = rounded;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border_slate_700);

        visuals.widgets.hovered.bg_fill = cyan_400;
        visuals.widgets.hovered.rounding = rounded;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, bg_slate_900);

        visuals.widgets.active.bg_fill = cyan_500;
        visuals.widgets.active.rounding = rounded;
        visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, bg_slate_900);

        visuals.selection.bg_fill = cyan_500;

        ctx.set_visuals(visuals);
    }
}

impl eframe::App for NodeXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.setup_custom_theme(ctx);
        self.process_events(ctx);

        let bg_slate_900 = Color32::from_rgb(15, 23, 42);
        let panel_slate_800 = Color32::from_rgb(30, 41, 59);
        let text_slate_50 = Color32::from_rgb(248, 250, 252);
        let cyan_500 = Color32::from_rgb(6, 182, 212);
        let cyan_400 = Color32::from_rgb(34, 211, 238);
        let border_slate_700 = Color32::from_rgb(51, 65, 85);
        let muted_text = Color32::from_rgb(148, 163, 184);

        // Sidebar (Left Panel)
        egui::SidePanel::left("left_sidebar")
            .resizable(false)
            .default_width(320.0)
            .frame(Frame::none().fill(panel_slate_800).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 12.0);

                // App Brand Banner
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⚡ NodeX")
                            .size(22.0)
                            .strong()
                            .color(cyan_500),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("P2P E2EE")
                                .size(10.0)
                                .color(muted_text),
                        );
                    });
                });

                ui.add_space(4.0);
                ui.separator();

                // My Node & Messenger Identity Card
                Frame::none()
                    .fill(bg_slate_900)
                    .rounding(Rounding::same(10.0))
                    .stroke(Stroke::new(1.0_f32, border_slate_700))
                    .inner_margin(Margin::same(14.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("MY MESSENGER USER ID (E2EE KEY)")
                                .size(9.0)
                                .strong()
                                .color(cyan_500),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(&self.my_user_id)
                                .size(11.0)
                                .monospace()
                                .color(text_slate_50),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("DHT Transport Node ID:")
                                .size(9.0)
                                .color(muted_text),
                        );
                        ui.label(
                            egui::RichText::new(&self.my_dht_node_id)
                                .size(9.5)
                                .monospace()
                                .color(muted_text),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("●")
                                    .size(10.0)
                                    .color(Color32::from_rgb(16, 185, 129)),
                            );
                            ui.label(
                                egui::RichText::new(&self.status_msg)
                                    .size(11.0)
                                    .color(muted_text),
                            );
                        });
                    });

                ui.add_space(8.0);

                // Contacts Header
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("CONTACTS")
                            .size(11.0)
                            .strong()
                            .color(muted_text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let add_btn = ui.add(
                            egui::Button::new(egui::RichText::new("+ Add").size(12.0).color(bg_slate_900))
                                .fill(cyan_500)
                                .rounding(Rounding::same(8.0)),
                        );
                        if add_btn.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }
                        if add_btn.clicked() {
                            self.add_contact_modal = true;
                        }
                    });
                });

                ui.add_space(4.0);

                // Contacts List Scroll Area
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        if self.contacts.is_empty() {
                            ui.add_space(20.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("No contacts added yet.")
                                        .size(12.0)
                                        .color(muted_text),
                                );
                            });
                        } else {
                            for c in &self.contacts {
                                let is_selected = self
                                    .selected_contact
                                    .as_ref()
                                    .map(|s| s.user_id_hex == c.user_id_hex)
                                    .unwrap_or(false);

                                let fill_color = if is_selected {
                                    border_slate_700
                                } else {
                                    panel_slate_800
                                };

                                let item_frame = Frame::none()
                                    .fill(fill_color)
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(Margin::same(10.0));

                                let res = item_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let initial = c.name.chars().next().unwrap_or('U').to_uppercase().to_string();
                                        ui.label(
                                            egui::RichText::new(initial)
                                                .size(14.0)
                                                .strong()
                                                .color(cyan_500),
                                        );
                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(&c.name)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(text_slate_50),
                                            );
                                            let short_id = if c.user_id_hex.len() > 14 {
                                                format!("{}...", &c.user_id_hex[..14])
                                            } else {
                                                c.user_id_hex.clone()
                                            };
                                            ui.label(
                                                egui::RichText::new(short_id)
                                                    .size(10.0)
                                                    .monospace()
                                                    .color(muted_text),
                                            );
                                        });
                                    });
                                });

                                let response = res.response.interact(egui::Sense::click());
                                if response.hovered() {
                                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                                }
                                if response.clicked() {
                                    self.selected_contact = Some(c.clone());
                                    let _ = self.cmd_tx.send(AppCommand::SelectContact(c.user_id_hex.clone()));
                                }
                            }
                        }
                    });

                // Sidebar Footer Metrics
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    Frame::none()
                        .fill(bg_slate_900)
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("DHT Peers").size(10.0).color(muted_text));
                                    ui.label(
                                        egui::RichText::new(self.dht_peers.to_string())
                                            .size(14.0)
                                            .strong()
                                            .color(cyan_500),
                                    );
                                });
                                ui.add_space(20.0);
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Stored Keys").size(10.0).color(muted_text));
                                    ui.label(
                                        egui::RichText::new(self.stored_keys.to_string())
                                            .size(14.0)
                                            .strong()
                                            .color(cyan_500),
                                    );
                                });
                            });
                        });
                });
            });

        // Main Chat Panel (Right Side)
        egui::CentralPanel::default()
            .frame(Frame::none().fill(bg_slate_900).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 14.0);

                // Header Bar
                Frame::none()
                    .fill(panel_slate_800)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(14.0))
                    .stroke(Stroke::new(1.0_f32, border_slate_700))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(ref sel) = self.selected_contact {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(&sel.name)
                                            .size(16.0)
                                            .strong()
                                            .color(text_slate_50),
                                    );
                                    ui.label(
                                        egui::RichText::new(&sel.user_id_hex)
                                            .size(10.0)
                                            .monospace()
                                            .color(muted_text),
                                    );
                                });
                            } else {
                                ui.label(
                                    egui::RichText::new("Select a Contact to Start Chatting")
                                        .size(15.0)
                                        .strong()
                                        .color(muted_text),
                                );
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                Frame::none()
                                    .fill(Color32::from_rgba_unmultiplied(6, 182, 212, 35))
                                    .rounding(Rounding::same(12.0))
                                    .stroke(Stroke::new(1.0_f32, cyan_500))
                                    .inner_margin(Margin::symmetric(10.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("🔒 End-to-End Encrypted")
                                                .size(11.0)
                                                .strong()
                                                .color(cyan_400),
                                        );
                                    });
                            });
                        });
                    });

                // Chat Messages Container
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 70.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.selected_contact.is_none() {
                            ui.add_space(60.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("⚡ NodeX Pure Rust P2P Messenger")
                                        .size(18.0)
                                        .strong()
                                        .color(cyan_500),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Zero Web Stack • Decentralized Kademlia DHT • C-Core E2EE",
                                    )
                                    .size(12.0)
                                    .color(muted_text),
                                );
                            });
                        } else if self.messages.is_empty() {
                            ui.add_space(40.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("No messages yet. Send an encrypted message below.")
                                        .size(13.0)
                                        .color(muted_text),
                                );
                            });
                        } else {
                            for m in &self.messages {
                                let is_self = m.sender_id_hex == self.my_user_id;

                                let bubble_fill = if is_self {
                                    Color32::from_rgb(2, 132, 199) // #0284c7
                                } else {
                                    panel_slate_800
                                };

                                let align = if is_self {
                                    egui::Align::RIGHT
                                } else {
                                    egui::Align::LEFT
                                };

                                ui.with_layout(egui::Layout::top_down(align), |ui| {
                                    Frame::none()
                                        .fill(bubble_fill)
                                        .rounding(Rounding::same(10.0))
                                        .inner_margin(Margin::same(12.0))
                                        .show(ui, |ui| {
                                            ui.set_max_width(420.0);
                                            ui.label(
                                                egui::RichText::new(&m.text)
                                                    .size(13.0)
                                                    .color(text_slate_50),
                                            );
                                            ui.add_space(2.0);
                                            let route_label = if is_self {
                                                if m.delivered {
                                                    "Direct UDP"
                                                } else {
                                                    "DHT Mailbox"
                                                }
                                            } else {
                                                ""
                                            };
                                            ui.label(
                                                egui::RichText::new(route_label)
                                                    .size(9.0)
                                                    .color(Color32::from_rgba_unmultiplied(255, 255, 255, 180)),
                                            );
                                        });
                                });
                                ui.add_space(6.0);
                            }
                        }
                    });

                // Message Input Form
                ui.horizontal(|ui| {
                    let text_edit = ui.add_enabled(
                        self.selected_contact.is_some(),
                        egui::TextEdit::singleline(&mut self.message_input)
                            .hint_text("Type an end-to-end encrypted message...")
                            .desired_width(ui.available_width() - 90.0)
                            .margin(Margin::same(12.0)),
                    );

                    let send_btn = ui.add_enabled(
                        self.selected_contact.is_some() && !self.message_input.trim().is_empty(),
                        egui::Button::new(egui::RichText::new("Send").size(13.0).strong().color(bg_slate_900))
                            .fill(cyan_500)
                            .rounding(Rounding::same(10.0))
                            .min_size(Vec2::new(75.0, 38.0)),
                    );

                    if send_btn.hovered() && self.selected_contact.is_some() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                    }

                    if (send_btn.clicked() || (text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                        && self.selected_contact.is_some()
                        && !self.message_input.trim().is_empty()
                    {
                        if let Some(ref sel) = self.selected_contact {
                            let text = self.message_input.trim().to_string();
                            self.message_input.clear();
                            let _ = self.cmd_tx.send(AppCommand::SendMessage {
                                recipient_id: sel.user_id_hex.clone(),
                                text,
                            });
                        }
                    }
                });
            });

        // Add Contact Modal Popup
        if self.add_contact_modal {
            egui::Window::new("Add New Contact")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(
                    Frame::window(&ctx.style())
                        .fill(panel_slate_800)
                        .rounding(Rounding::same(12.0))
                        .stroke(Stroke::new(1.0_f32, border_slate_700))
                        .inner_margin(Margin::same(20.0)),
                )
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Enter recipient's Ed25519 Public Key Hash:")
                            .size(12.0)
                            .color(muted_text),
                    );
                    ui.add_space(6.0);

                    ui.text_edit_singleline(&mut self.new_contact_id);
                    ui.add_space(6.0);

                    ui.label(
                        egui::RichText::new("Display Name (e.g. Alice):")
                            .size(12.0)
                            .color(muted_text),
                    );
                    ui.add_space(6.0);
                    ui.text_edit_singleline(&mut self.new_contact_name);

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.add_contact_modal = false;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let add_confirm = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Add Contact")
                                        .size(12.0)
                                        .strong()
                                        .color(bg_slate_900),
                                )
                                .fill(cyan_500)
                                .rounding(Rounding::same(8.0)),
                            );
                            if add_confirm.hovered() {
                                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                            }
                            if add_confirm.clicked() && !self.new_contact_id.trim().is_empty() {
                                let name = if self.new_contact_name.trim().is_empty() {
                                    "Peer".to_string()
                                } else {
                                    self.new_contact_name.trim().to_string()
                                };
                                let _ = self.cmd_tx.send(AppCommand::AddContact {
                                    user_id: self.new_contact_id.trim().to_string(),
                                    name,
                                });
                                self.add_contact_modal = false;
                                self.new_contact_id.clear();
                                self.new_contact_name.clear();
                            }
                        });
                    });
                });
        }
    }
}
