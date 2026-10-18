use eframe::egui;
use egui::{Color32, Margin, Rounding, Stroke};
use ed25519_dalek::SigningKey;
use nodex_blockchain::{GenesisConfig, Transaction, NANO_NDX_PER_NDX, AccountState};
use nodex_consensus::{Mempool, PoSEngine};
use nodex_storage::BlockchainStorage;
use nodex_crypto::KeyEngine;
use rand::RngCore;
use std::sync::Arc;
use std::time::Instant;

pub fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("⚡ NodeX Chain — Sovereign Layer-1 Wallet & WASM Explorer")
            .with_inner_size([1150.0, 750.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "NodeX Wallet",
        options,
        Box::new(|_cc| Box::new(NodeXWalletApp::new())),
    )
}

#[derive(PartialEq, Eq)]
enum Tab {
    Wallet,
    Staking,
    Contracts,
    Explorer,
}

struct NodeXWalletApp {
    current_tab: Tab,
    mnemonic: String,
    signing_key: SigningKey,
    pubkey_hex: String,
    recipient_input: String,
    amount_input: String,
    fee_input: String,
    status_msg: String,
    storage: Arc<BlockchainStorage>,
    mempool: Arc<Mempool>,
    pos_engine: PoSEngine,
    last_block_tick: Instant,
}

impl NodeXWalletApp {
    fn new() -> Self {
        // Never fall back to a public, deterministic recovery phrase: that would
        // give every affected wallet the same publicly known signing key.
        let mnemonic = KeyEngine::generate_mnemonic()
            .expect("secure mnemonic generation failed; refusing to create a predictable wallet");
        let signing_key = KeyEngine::derive_key_from_mnemonic(&mnemonic, "").unwrap();
        let pubkey_bytes = signing_key.verifying_key().to_bytes();
        let pubkey_hex = hex::encode(pubkey_bytes);

        let storage = Arc::new(BlockchainStorage::open_temporary().unwrap());
        let mempool = Arc::new(Mempool::new());

        let genesis_block = GenesisConfig::build_genesis_block(&signing_key);
        storage.put_block(&genesis_block).unwrap();

        let initial_state = AccountState {
            balance: 1_000_000 * NANO_NDX_PER_NDX,
            nonce: 0,
            stake: 100_000 * NANO_NDX_PER_NDX,
            is_validator: true,
            validator_commission_bps: 500,
            storage_root: [0u8; 32],
            code_hash: None,
            storage: std::collections::HashMap::new(),
        };
        storage.put_account(&pubkey_bytes, &initial_state).unwrap();

        let pos_engine = PoSEngine::new(signing_key.clone(), storage.clone(), mempool.clone());

        Self {
            current_tab: Tab::Wallet,
            mnemonic,
            signing_key,
            pubkey_hex,
            recipient_input: String::new(),
            amount_input: "100".to_string(),
            fee_input: "1".to_string(),
            status_msg: "Layer-1 NodeX Chain Enterprise Engine Started.".to_string(),
            storage,
            mempool,
            pos_engine,
            last_block_tick: Instant::now(),
        }
    }
}

impl eframe::App for NodeXWalletApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_block_tick.elapsed().as_secs() >= 3 {
            if let Ok(block) = self.pos_engine.produce_next_block() {
                self.status_msg = format!("⚡ New Block #{} produced! Transactions: {}, Receipts: {}", block.header.block_number, block.transactions.len(), block.receipts.len());
            }
            self.last_block_tick = Instant::now();
        }
        ctx.request_repaint();

        let pubkey_bytes = self.signing_key.verifying_key().to_bytes();
        let account_state = self.storage.get_account(&pubkey_bytes).unwrap_or_default();
        let balance_ndx = account_state.balance as f64 / NANO_NDX_PER_NDX as f64;
        let stake_ndx = account_state.stake as f64 / NANO_NDX_PER_NDX as f64;
        let height = self.storage.get_latest_height().unwrap_or(0);

        egui::SidePanel::left("wallet_sidebar")
            .resizable(false)
            .exact_width(230.0)
            .show(ctx, |ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("⚡ NODEX CHAIN L1").strong().color(Color32::from_rgb(6, 182, 212)));
                ui.label(egui::RichText::new("Enterprise PoS & WASM Engine").size(11.0).color(Color32::GRAY));
                ui.add_space(20.0);

                if ui.selectable_label(self.current_tab == Tab::Wallet, "💳  P2P Wallet").clicked() {
                    self.current_tab = Tab::Wallet;
                }
                ui.add_space(6.0);
                if ui.selectable_label(self.current_tab == Tab::Staking, "🔒  PoS Staking").clicked() {
                    self.current_tab = Tab::Staking;
                }
                ui.add_space(6.0);
                if ui.selectable_label(self.current_tab == Tab::Contracts, "🧩  Smart Contracts").clicked() {
                    self.current_tab = Tab::Contracts;
                }
                ui.add_space(6.0);
                if ui.selectable_label(self.current_tab == Tab::Explorer, "🌐  Block Explorer").clicked() {
                    self.current_tab = Tab::Explorer;
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(format!("Block Height: #{}", height)).size(12.0).color(Color32::from_rgb(34, 197, 94)));
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                Tab::Wallet => {
                    ui.heading("P2P Wallet & Multi-Asset Transfer");
                    ui.add_space(10.0);

                    egui::Frame::none()
                        .fill(Color32::from_rgb(15, 23, 42))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::same(16.0))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(30, 41, 59)))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("AVAILABLE BALANCE").size(11.0).color(Color32::GRAY));
                            ui.heading(egui::RichText::new(format!("{:.4} NDX", balance_ndx)).size(28.0).strong().color(Color32::from_rgb(6, 182, 212)));
                            ui.label(egui::RichText::new(format!("Address: {}", &self.pubkey_hex)).size(11.0).color(Color32::LIGHT_GRAY));
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(format!("BIP-39 Mnemonic: {}", &self.mnemonic)).size(10.5).color(Color32::DARK_GRAY));
                        });

                    ui.add_space(20.0);
                    ui.heading("Send Transfer");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Recipient Public Key (Hex):");
                    });
                    ui.add(egui::TextEdit::singleline(&mut self.recipient_input).hint_text("Paste Ed25519 pubkey (64 hex chars)").desired_width(450.0));

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Amount (NDX):");
                        ui.add(egui::TextEdit::singleline(&mut self.amount_input).desired_width(100.0));
                        ui.label("Fee (NDX):");
                        ui.add(egui::TextEdit::singleline(&mut self.fee_input).desired_width(60.0));
                    });

                    ui.add_space(12.0);
                    if ui.button(egui::RichText::new("🚀 Execute Transfer").strong().color(Color32::WHITE)).clicked() {
                        if let Ok(rec_bytes) = hex::decode(&self.recipient_input) {
                            if rec_bytes.len() == 32 {
                                let mut recipient = [0u8; 32];
                                recipient.copy_from_slice(&rec_bytes);

                                let amount_ndx: f64 = self.amount_input.parse().unwrap_or(0.0);
                                let fee_ndx: f64 = self.fee_input.parse().unwrap_or(1.0);

                                let amount_nano = (amount_ndx * NANO_NDX_PER_NDX as f64) as u64;
                                let fee_nano = (fee_ndx * NANO_NDX_PER_NDX as f64) as u64;

                                let tx = Transaction::new_transfer(
                                    &self.signing_key,
                                    recipient,
                                    amount_nano,
                                    account_state.nonce + 1,
                                    fee_nano,
                                );

                                if let Err(e) = self.mempool.add_transaction(tx) {
                                    self.status_msg = format!("❌ Transaction rejected: {}", e);
                                } else {
                                    self.status_msg = "✅ Transaction submitted to Mempool!".to_string();
                                }
                            } else {
                                self.status_msg = "❌ Invalid recipient address length!".to_string();
                            }
                        } else {
                            self.status_msg = "❌ Invalid hex address format!".to_string();
                        }
                    }

                    ui.add_space(16.0);
                    ui.label(egui::RichText::new(&self.status_msg).color(Color32::YELLOW));
                }
                Tab::Staking => {
                    ui.heading("PoS Staking & Validator Management");
                    ui.add_space(10.0);
                    ui.label(format!("Active Stake: {} NDX", stake_ndx));
                    ui.label("Validator Status: Active (Producing Blocks)");
                }
                Tab::Contracts => {
                    ui.heading("WASM Smart Contracts Engine");
                    ui.add_space(10.0);
                    ui.label("Virtual Machine: Active (Gas limit: 30,000,000 per block)");
                }
                Tab::Explorer => {
                    ui.heading("Block Explorer");
                    ui.add_space(10.0);
                    ui.label(format!("Height: #{}", height));
                    ui.label(format!("Mempool Size: {} Txs", self.mempool.len()));
                }
            }
        });
    }
}
