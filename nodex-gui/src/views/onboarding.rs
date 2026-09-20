use egui::{vec2, Color32, Margin, Rounding, Stroke, Ui, Vec2};
use nodex_messenger::crypto::UserIdentity;
use nodex_messenger::mnemonic::MnemonicManager;
use crate::state::AppState;
use crate::theme::Theme;

pub fn render_onboarding(ui: &mut Ui, state: &mut AppState, theme: &Theme) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);

        // NodeX Photo Logo from Assets
        crate::components::icons::render_nodex_logo_widget(ui, 64.0);

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("NodeX")
                .size(24.0)
                .strong()
                .color(theme.text_primary),
        );
        ui.label(
            egui::RichText::new("Clean P2P & Quiet UX • Serverless Mesh")
                .size(12.0)
                .color(theme.text_muted),
        );

        ui.add_space(24.0);

        match state.onboarding_step {
            0 => {
                // STEP 0: Welcome Screen
                render_step_welcome(ui, state, theme);
            }
            1 => {
                // STEP 1: Choose Display Name & Bio & Word Count
                render_step_profile(ui, state, theme);
            }
            2 => {
                // STEP 2: BIP-39 12/24-word Seed Backup
                render_step_seed_generation(ui, state, theme);
            }
            3 => {
                // STEP 3: Restore flow
                render_step_seed_restore(ui, state, theme);
            }
            _ => {
                state.is_onboarded = true;
            }
        }
    });
}

fn render_step_welcome(ui: &mut Ui, state: &mut AppState, theme: &Theme) {
    egui::Frame::none()
        .fill(theme.card_bg)
        .rounding(Rounding::same(12.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.set_width(460.0);

            ui.label(
                egui::RichText::new("Serverless P2P Identity")
                    .size(15.0)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("NodeX requires zero phone numbers, emails, or central servers. Your identity is a cryptographic keypair owned solely by you.")
                    .size(12.0)
                    .color(theme.text_muted),
            );

            ui.add_space(18.0);

            // Feature Highlights
            feature_badge(ui, "[ E2EE ]", "End-to-End Encrypted", "Double Ratchet + XChaCha20-Poly1305 with Forward Secrecy", theme);
            ui.add_space(8.0);
            feature_badge(ui, "[ DHT  ]", "Direct P2P Kademlia Mesh", "NAT Hole-Punching & Opt-In Offline Store-and-Forward", theme);
            ui.add_space(8.0);
            feature_badge(ui, "[ STEGO]", "Steganography & Audio Chirp", "Hidden carriers & 3-second chirp invitations", theme);

            ui.add_space(20.0);

            // Primary action: Create new
            let create_btn = egui::Button::new(
                egui::RichText::new("[ Create New Account ]")
                    .size(13.0)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(theme.accent)
            .rounding(Rounding::same(8.0));

            let width = ui.available_width();
            if ui.add_sized(vec2(width, 38.0), create_btn).clicked() {
                // Generate fresh random mnemonic immediately on click
                let mn = if state.mnemonic_length == 12 {
                    MnemonicManager::generate_12_words()
                } else {
                    MnemonicManager::generate_24_words()
                }.unwrap_or_default();
                state.mnemonic_seed = mn;
                state.onboarding_step = 1;
            }

            ui.add_space(8.0);

            // Secondary action: Restore
            let restore_btn = egui::Button::new(
                egui::RichText::new("[ Restore from 12/24-word Seed ]")
                    .size(12.0)
                    .color(theme.text_primary),
            )
            .fill(theme.bubble_other_bg)
            .stroke(Stroke::new(1.0_f32, theme.border_color))
            .rounding(Rounding::same(8.0));

            if ui.add_sized(vec2(width, 34.0), restore_btn).clicked() {
                state.restore_input.clear();
                state.restore_error = None;
                state.onboarding_step = 3;
            }
        });
}

fn feature_badge(ui: &mut Ui, tag: &str, title: &str, subtitle: &str, theme: &Theme) {
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tag).size(11.0).monospace().color(theme.accent));
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).size(12.0).strong().color(theme.text_primary));
                    ui.label(egui::RichText::new(subtitle).size(10.5).color(theme.text_muted));
                });
            });
        });
}

fn render_step_profile(ui: &mut Ui, state: &mut AppState, theme: &Theme) {
    egui::Frame::none()
        .fill(theme.card_bg)
        .rounding(Rounding::same(12.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.set_width(460.0);

            ui.label(
                egui::RichText::new("Step 1 of 2: Set Up Profile")
                    .size(15.0)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Your profile data is signed and broadcast only to connected peers.")
                    .size(12.0)
                    .color(theme.text_muted),
            );

            ui.add_space(16.0);

            ui.label(egui::RichText::new("Display Name:").size(12.0).color(theme.text_primary));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut state.my_name)
                    .hint_text("Enter your nickname / handle")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(12.0);

            ui.label(egui::RichText::new("Bio / Status:").size(12.0).color(theme.text_primary));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut state.my_bio)
                    .hint_text("Optional bio (e.g. 'Cypherpunk enthusiast')")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(14.0);

            // Mnemonic phrase length choice
            ui.label(egui::RichText::new("Secret Phrase Length:").size(12.0).color(theme.text_primary));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for len in [12, 24] {
                    let is_sel = state.mnemonic_length == len;
                    let label = format!("{} Words", len);
                    if ui.selectable_label(is_sel, label).clicked() {
                        state.mnemonic_length = len;
                        let mn = if len == 12 {
                            MnemonicManager::generate_12_words()
                        } else {
                            MnemonicManager::generate_24_words()
                        }.unwrap_or_default();
                        state.mnemonic_seed = mn;
                    }
                }
            });

            ui.add_space(20.0);

            let can_proceed = !state.my_name.trim().is_empty();

            let next_btn = egui::Button::new(
                egui::RichText::new("[ Continue to Seed Backup ]")
                    .size(13.0)
                    .strong()
                    .color(if can_proceed { Color32::WHITE } else { theme.text_muted }),
            )
            .fill(if can_proceed { theme.accent } else { theme.bubble_other_bg })
            .rounding(Rounding::same(8.0));

            let width = ui.available_width();
            if ui.add_sized(vec2(width, 38.0), next_btn).clicked() && can_proceed {
                // Ensure seed phrase is freshly generated if empty
                if state.mnemonic_seed.trim().is_empty() {
                    state.mnemonic_seed = if state.mnemonic_length == 12 {
                        MnemonicManager::generate_12_words().unwrap_or_default()
                    } else {
                        MnemonicManager::generate_24_words().unwrap_or_default()
                    };
                }
                state.onboarding_step = 2;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("[ Back ]").color(theme.text_muted)).clicked() {
                    state.onboarding_step = 0;
                }
                if std::path::Path::new(&format!("nodex_account_{}.ready", state.port)).exists() {
                    if ui.button(egui::RichText::new("[ Cancel & Return ]").color(theme.text_muted)).clicked() {
                        state.is_onboarded = true;
                    }
                }
            });
        });
}

fn render_step_seed_generation(ui: &mut Ui, state: &mut AppState, theme: &Theme) {
    egui::Frame::none()
        .fill(theme.card_bg)
        .rounding(Rounding::same(12.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.set_width(520.0);

            let word_count = state.mnemonic_seed.split_whitespace().count();

            ui.label(
                egui::RichText::new(format!("Step 2 of 2: Backup Secret Mnemonic ({} Words)", word_count))
                    .size(15.0)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Write down these words on paper in exact order. They are the ONLY key to your account.")
                    .size(12.0)
                    .color(Color32::from_rgb(231, 76, 60)),
            );

            ui.add_space(14.0);

            // Words grid (4 columns)
            let words: Vec<&str> = state.mnemonic_seed.split_whitespace().collect();
            egui::Grid::new("seed_grid")
                .num_columns(4)
                .spacing(vec2(8.0, 6.0))
                .show(ui, |ui| {
                    for (i, word) in words.iter().enumerate() {
                        egui::Frame::none()
                            .fill(theme.bubble_other_bg)
                            .rounding(Rounding::same(6.0))
                            .stroke(Stroke::new(1.0_f32, theme.border_color))
                            .inner_margin(Margin::symmetric(8.0, 5.0))
                            .show(ui, |ui| {
                                ui.set_width(110.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("{:02}.", i + 1)).size(10.5).color(theme.accent));
                                    ui.label(egui::RichText::new(*word).size(12.0).monospace().color(theme.text_primary));
                                });
                            });
                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });

            ui.add_space(14.0);

            ui.horizontal(|ui| {
                let copy_btn = egui::Button::new(egui::RichText::new("[ Copy All Words ]").size(11.5).color(theme.text_primary))
                    .fill(theme.bubble_other_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(8.0));
                if ui.add_sized(Vec2::new(140.0, 28.0), copy_btn).clicked() {
                    ui.output_mut(|o| o.copied_text = state.mnemonic_seed.clone());
                }

                ui.add_space(8.0);

                let gen_btn = egui::Button::new(egui::RichText::new("[ Generate New Phrase ]").size(11.5).color(theme.text_primary))
                    .fill(theme.bubble_other_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(8.0));
                if ui.add_sized(Vec2::new(170.0, 28.0), gen_btn).clicked() {
                    state.mnemonic_seed = if state.mnemonic_length == 12 {
                        MnemonicManager::generate_12_words().unwrap_or_default()
                    } else {
                        MnemonicManager::generate_24_words().unwrap_or_default()
                    };
                }
            });

            ui.add_space(14.0);
            let is_confirmed = state.seed_confirmed_saved;
            ui.checkbox(
                &mut state.seed_confirmed_saved,
                egui::RichText::new("I confirm that I have safely written down and backed up my seed phrase")
                    .size(12.0)
                    .color(if is_confirmed { theme.online_indicator } else { theme.text_primary }),
            );

            ui.add_space(14.0);

            let can_finish = state.seed_confirmed_saved;
            let finish_btn = egui::Button::new(
                egui::RichText::new(format!("[ Create Account & Launch NodeX ({}) ]", word_count))
                    .size(13.0)
                    .strong()
                    .color(if can_finish { Color32::WHITE } else { theme.text_muted }),
            )
            .fill(if can_finish { theme.accent } else { theme.bubble_other_bg })
            .rounding(Rounding::same(8.0));

            let width = ui.available_width();
            if ui.add_sized(vec2(width, 38.0), finish_btn).clicked() && can_finish {
                if let Ok(identity) = UserIdentity::from_mnemonic(&state.mnemonic_seed) {
                    state.my_node_id = identity.user_id_hex();
                }
                let _ = std::fs::write(&format!("nodex_account_{}.ready", state.port), "registered");
                state.is_onboarded = true;
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let back_btn = egui::Button::new(egui::RichText::new("[ Back to Profile ]").size(11.5).color(theme.text_muted))
                    .fill(theme.bubble_other_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(8.0));
                if ui.add_sized(Vec2::new(130.0, 28.0), back_btn).clicked() {
                    state.onboarding_step = 1;
                }
                if std::path::Path::new(&format!("nodex_account_{}.ready", state.port)).exists() {
                    ui.add_space(8.0);
                    let cancel_btn = egui::Button::new(egui::RichText::new("[ Cancel & Return ]").size(11.5).color(theme.text_muted))
                        .fill(theme.bubble_other_bg)
                        .stroke(Stroke::new(1.0_f32, theme.border_color))
                        .rounding(Rounding::same(8.0));
                    if ui.add_sized(Vec2::new(130.0, 28.0), cancel_btn).clicked() {
                        state.is_onboarded = true;
                    }
                }
            });
        });
}

fn render_step_seed_restore(ui: &mut Ui, state: &mut AppState, theme: &Theme) {
    egui::Frame::none()
        .fill(theme.card_bg)
        .rounding(Rounding::same(12.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.set_width(480.0);

            ui.label(
                egui::RichText::new("Restore Identity from Mnemonic")
                    .size(15.0)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Paste your 12 or 24 BIP-39 seed words separated by spaces:")
                    .size(12.0)
                    .color(theme.text_muted),
            );

            ui.add_space(12.0);

            ui.add(
                egui::TextEdit::multiline(&mut state.restore_input)
                    .hint_text("word1 word2 word3 ...")
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );

            let words: Vec<&str> = state.restore_input.split_whitespace().collect();
            let count = words.len();
            let is_valid = MnemonicManager::validate(&state.restore_input);

            ui.add_space(8.0);
            if !state.restore_input.trim().is_empty() {
                if is_valid {
                    ui.label(egui::RichText::new(format!("[OK] Valid BIP-39 seed phrase ({} words)", count)).size(12.0).color(theme.online_indicator));
                } else if count == 12 || count == 24 {
                    ui.label(egui::RichText::new("[!] Checksum mismatch: Check for typos in words").size(12.0).color(Color32::from_rgb(231, 76, 60)));
                } else {
                    ui.label(egui::RichText::new(format!("Entered {}/12 or {}/24 words...", count, count)).size(11.5).color(theme.text_muted));
                }
            }

            if let Some(err) = &state.restore_error {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(err).size(11.5).color(Color32::from_rgb(231, 76, 60)));
            }

            ui.add_space(16.0);

            let restore_btn = egui::Button::new(
                egui::RichText::new("[ Restore Account & Enter ]")
                    .size(13.0)
                    .strong()
                    .color(if is_valid { Color32::WHITE } else { theme.text_muted }),
            )
            .fill(if is_valid { theme.accent } else { theme.bubble_other_bg })
            .rounding(Rounding::same(8.0));

            let restore_width = ui.available_width();
            if ui.add_sized(vec2(restore_width, 38.0), restore_btn).clicked() && is_valid {
                match UserIdentity::from_mnemonic(&state.restore_input) {
                    Ok(identity) => {
                        state.mnemonic_seed = state.restore_input.trim().to_string();
                        state.my_node_id = identity.user_id_hex();
                        state.restore_error = None;
                        let _ = std::fs::write(&format!("nodex_account_{}.ready", state.port), "registered");
                        state.is_onboarded = true;
                    }
                    Err(e) => {
                        state.restore_error = Some(format!("Failed to derive identity: {}", e));
                    }
                }
            }

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let back_btn = egui::Button::new(egui::RichText::new("[ Back to Welcome ]").size(11.5).color(theme.text_muted))
                    .fill(theme.bubble_other_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(8.0));
                if ui.add_sized(Vec2::new(140.0, 28.0), back_btn).clicked() {
                    state.onboarding_step = 0;
                }
                if std::path::Path::new(&format!("nodex_account_{}.ready", state.port)).exists() {
                    ui.add_space(8.0);
                    let cancel_btn = egui::Button::new(egui::RichText::new("[ Cancel & Return ]").size(11.5).color(theme.text_muted))
                        .fill(theme.bubble_other_bg)
                        .stroke(Stroke::new(1.0_f32, theme.border_color))
                        .rounding(Rounding::same(8.0));
                    if ui.add_sized(Vec2::new(130.0, 28.0), cancel_btn).clicked() {
                        state.is_onboarded = true;
                    }
                }
            });
        });
}


