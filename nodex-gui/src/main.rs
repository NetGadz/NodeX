#![allow(float_literal_f32_fallback)]

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use clap::Parser;
use eframe::egui;

use nodex_kademlia::config::NodeConfig;
use nodex_kademlia::KademliaNode;
use nodex_messenger::db::{MessengerDb, SavedContact};
use nodex_messenger::{KadMessenger, MessengerEvent};

mod gui;
use gui::{AppCommand, NodeXApp, UiEvent};

#[derive(Parser, Debug)]
#[command(
    name = "nodex",
    about = "NodeX — Pure Rust Native P2P E2EE Messenger built on Kademlia DHT & eframe/egui"
)]
pub struct Cli {
    /// Optional JSON configuration file path
    #[arg(short, long)]
    pub config: Option<String>,

    /// UDP port to bind on (overrides config)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// IP address to bind to (overrides config)
    #[arg(long)]
    pub ip: Option<String>,

    /// Optional bootstrap node address (overrides config)
    #[arg(short, long)]
    pub bootstrap: Option<SocketAddr>,

    /// State file path to load/save persistent Node ID and contacts
    #[arg(long)]
    pub state_file: Option<String>,

    /// User Display Name for Messenger
    #[arg(long, default_value = "UserNode")]
    pub name: String,

    /// Launch CLI interactive mode instead of GUI
    #[arg(long, default_value_t = false)]
    pub cli: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    let mut config = if let Some(ref config_path) = cli.config {
        if Path::new(config_path).exists() {
            println!("[CONFIG] Loading configuration from {}", config_path);
            NodeConfig::load_from_json_file(config_path)?
        } else {
            eprintln!("[WARNING] Config file {} not found; using defaults.", config_path);
            NodeConfig::default()
        }
    } else {
        NodeConfig::default()
    };

    if let Some(port) = cli.port {
        config.port = port;
    }
    if let Some(ip) = cli.ip {
        config.ip = ip;
    }
    if let Some(bootstrap) = cli.bootstrap {
        config.bootstrap = vec![bootstrap];
    }
    if let Some(state_file) = cli.state_file {
        config.state_file = state_file;
    }

    println!("============================================================");
    println!("     NodeX — Pure Rust Native P2P Encrypted Messenger      ");
    println!("============================================================");

    // Channels for UI <-> Tokio async worker communication
    let (ui_tx, ui_rx) = std::sync::mpsc::channel::<UiEvent>();
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<AppCommand>();

    // Start Tokio Runtime for background P2P networking
    let rt = tokio::runtime::Runtime::new()?;
    let display_name = cli.name.clone();

    rt.spawn(async move {
        // Auto-bind to available port starting from config.port (8000..8050)
        let node = match KademliaNode::start_auto(config.clone()).await {
            Ok(n) => n,
            Err(e) => {
                eprintln!("[FATAL] Failed to start Kademlia Node: {}", e);
                return;
            }
        };

        let bound_port = node.network.local_addr.port();
        println!("[NET] Successfully bound Kademlia Node on port {}", bound_port);

        let data_dir = if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::PathBuf::from(appdata).join("NodeX");
            let _ = std::fs::create_dir_all(&p);
            p
        } else if let Ok(home) = std::env::var("HOME") {
            let p = std::path::PathBuf::from(home).join(".config").join("nodex");
            let _ = std::fs::create_dir_all(&p);
            p
        } else {
            std::path::PathBuf::from(".")
        };
        let db_path = data_dir.join(format!("messenger_db_{}.json", bound_port)).to_string_lossy().to_string();
        let messenger = match KadMessenger::start(node.clone(), db_path, display_name).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[FATAL] Failed to start KadMessenger: {}", e);
                return;
            }
        };

        // INSTANTLY send initial node state to GUI (Zero latency!)
        let user_id = messenger.user_id_hex().await;
        let dht_node_id = node.node_id.to_string();
        let (saved_name, saved_bio, saved_mnemonic) = {
            let db = messenger.db.read().await;
            (db.display_name.clone(), db.bio.clone(), db.mnemonic.clone())
        };

        println!("[IDENTITY] User Messenger E2EE ID: {}", user_id);
        println!("[IDENTITY] Kademlia Transport Node ID: {}", dht_node_id);

        let _ = ui_tx.send(UiEvent::NodeInfo {
            user_id: user_id.clone(),
            dht_node_id: dht_node_id.clone(),
            display_name: saved_name,
            bio: saved_bio,
            mnemonic: String::new(),
            has_mnemonic: !saved_mnemonic.is_empty(),
            dht_peers: node.routing_table.read().await.total_contacts(),
            stored_keys: node.storage.read().await.len(),
        });

        // Send contacts list to GUI immediately
        {
            let db = messenger.db.read().await;
            let contacts: Vec<_> = db.contacts.values().cloned().collect();
            let _ = ui_tx.send(UiEvent::ContactsList(contacts));
        }

        // Start background inbox polling task
        let (msg_event_tx, msg_event_rx) = std::sync::mpsc::channel::<MessengerEvent>();
        messenger.start_inbox_polling_task(Some(msg_event_tx));

        let ui_tx_event_forwarder = ui_tx.clone();
        std::thread::spawn(move || {
            while let Ok(event) = msg_event_rx.recv() {
                match event {
                    MessengerEvent::ContactsUpdated(contacts) => {
                        let _ = ui_tx_event_forwarder.send(UiEvent::ContactsList(contacts));
                    }
                    MessengerEvent::MessageReceived(msg) => {
                        let _ = ui_tx_event_forwarder.send(UiEvent::MessageSent(msg));
                    }
                    MessengerEvent::MessageDeleted { contact_id, message_ids } => {
                        let _ = ui_tx_event_forwarder.send(UiEvent::MessagesDeleted { contact_id, message_ids });
                    }
                }
            }
        });

        // Auto-bootstrap in BACKGROUND so it never blocks GUI event loop
        let node_bg = node.clone();
        let msg_bg = Arc::clone(&messenger);
        let bootstrap_addrs = config.bootstrap.clone();
        tokio::spawn(async move {
            if bootstrap_addrs.is_empty() {
                for p in 8000..=8010 {
                    if p != bound_port {
                        if let Ok(baddr) = format!("127.0.0.1:{}", p).parse::<SocketAddr>() {
                            let _ = node_bg.bootstrap(baddr).await;
                        }
                    }
                }
            } else {
                for baddr in &bootstrap_addrs {
                    if let Err(e) = node_bg.bootstrap(*baddr).await {
                        eprintln!("[ERROR] Bootstrap to {} failed: {}", baddr, e);
                    }
                }
            }
            // Broadcast presence to all DHT peers immediately upon connection
            let _ = msg_bg.publish_presence().await;
        });

        // Background Command & Status Polling Loop
        let msg_worker = Arc::clone(&messenger);
        let ui_tx_worker = ui_tx.clone();

        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        AppCommand::SendMessage { local_id, recipient_id, text, image_base64 } => {
                            let worker = Arc::clone(&msg_worker);
                            let ui_tx_clone = ui_tx_worker.clone();
                            tokio::spawn(async move {
                                match worker.send_message(&recipient_id, &text, image_base64).await {
                                    Ok(saved) => {
                                        let _ = ui_tx_clone.send(UiEvent::MessageSent(saved));
                                    }
                                    Err(e) => {
                                        let _ = ui_tx_clone.send(UiEvent::MessageFailed {
                                            message_id: local_id,
                                            reason: e.to_string(),
                                        });
                                        let _ = ui_tx_clone.send(UiEvent::StatusLog(format!("Send error: {}", e)));
                                    }
                                }
                            });
                        }
                        AppCommand::AddContact { user_id, name } => {
                            match msg_worker.discover_peer(&user_id).await {
                                Ok(card) => {
                                    let final_name = if name.is_empty() { card.display_name } else { name };
                                    let mut db = msg_worker.db.write().await;
                                    db.add_contact(SavedContact {
                                        user_id_hex: user_id.clone(),
                                        name: final_name,
                                        bio: card.bio,
                                        ed25519_pub_hex: card.ed25519_pub.iter().map(|b| format!("{:02x}", b)).collect(),
                                        x25519_pub_hex: card.x25519_pub.iter().map(|b| format!("{:02x}", b)).collect(),
                                        last_seen_addr: card.socket_addr.to_string(),
                                    });
                                    db.save_to_file(&msg_worker.db_path).ok();
                                }
                                Err(_) => {
                                    let mut db = msg_worker.db.write().await;
                                    let existing = db.contacts.get(&user_id).cloned();
                                    let final_name = if name.is_empty() {
                                        existing.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| {
                                             let p: String = user_id.chars().take(6).collect();
                                             format!("Peer_{}", p)
                                        })
                                    } else {
                                        name
                                    };
                                    let c = existing.unwrap_or_else(|| SavedContact {
                                        user_id_hex: user_id.clone(),
                                        name: final_name,
                                        bio: String::new(),
                                        ed25519_pub_hex: "".into(),
                                        x25519_pub_hex: "".into(),
                                        last_seen_addr: "unknown".into(),
                                    });
                                    db.add_contact(c);
                                    db.save_to_file(&msg_worker.db_path).ok();
                                }
                            }
                            let db = msg_worker.db.read().await;
                            let contacts: Vec<_> = db.contacts.values().cloned().collect();
                            let _ = ui_tx_worker.send(UiEvent::ContactsList(contacts));
                        }
                        AppCommand::CreateInvite => {
                            if let Ok(invite_link) = msg_worker.create_invite_link(Some(7)).await {
                                let _ = ui_tx_worker.send(UiEvent::InviteGenerated(invite_link));
                            }
                        }
                        AppCommand::AddContactInvite(invite_str) => {
                            match msg_worker.add_contact_by_invite(&invite_str).await {
                                Ok(contact) => {
                                    let db = msg_worker.db.read().await;
                                    let contacts: Vec<_> = db.contacts.values().cloned().collect();
                                    let _ = ui_tx_worker.send(UiEvent::ContactsList(contacts));
                                    let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Добавлен контакт: {}", contact.name)));
                                }
                                Err(e) => {
                                    let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Ошибка инвайта: {}", e)));
                                }
                            }
                        }
                        AppCommand::SelectContact(contact_id) => {
                            let db = msg_worker.db.read().await;
                            let msgs = db.get_messages_for_contact(&contact_id);
                            let _ = ui_tx_worker.send(UiEvent::MessagesList {
                                contact_id,
                                messages: msgs,
                            });
                        }
                        AppCommand::RegisterAccount { display_name, bio, mnemonic } => {
                            if let Err(e) = msg_worker.restore_mnemonic(&mnemonic, Some(display_name.clone())).await {
                                eprintln!("[ERROR] Failed to set identity from mnemonic: {}", e);
                            }
                            if !bio.is_empty() {
                                let _ = msg_worker.update_profile(display_name.clone(), bio.clone()).await;
                            }
                            let uid = msg_worker.user_id_hex().await;
                            let _ = ui_tx_worker.send(UiEvent::NodeInfo {
                                user_id: uid,
                                dht_node_id: msg_worker.dht_node.node_id.to_string(),
                                display_name,
                                bio,
                                mnemonic: String::new(),
                                has_mnemonic: true,
                                dht_peers: msg_worker.dht_node.routing_table.read().await.total_contacts(),
                                stored_keys: msg_worker.dht_node.storage.read().await.len(),
                            });
                        }
                        AppCommand::UpdateProfile { display_name, bio } => {
                            let _ = msg_worker.update_profile(display_name.clone(), bio.clone()).await;
                            let uid = msg_worker.user_id_hex().await;
                            let _ = ui_tx_worker.send(UiEvent::NodeInfo {
                                user_id: uid,
                                dht_node_id: msg_worker.dht_node.node_id.to_string(),
                                display_name,
                                bio,
                                mnemonic: String::new(),
                                has_mnemonic: true,
                                dht_peers: msg_worker.dht_node.routing_table.read().await.total_contacts(),
                                stored_keys: msg_worker.dht_node.storage.read().await.len(),
                            });
                        }
                        AppCommand::RefreshInfo => {
                            let uid = msg_worker.user_id_hex().await;
                            let (name, bio, mn) = {
                                let db = msg_worker.db.read().await;
                                (db.display_name.clone(), db.bio.clone(), db.mnemonic.clone())
                            };
                            let _ = ui_tx_worker.send(UiEvent::NodeInfo {
                                user_id: uid,
                                dht_node_id: msg_worker.dht_node.node_id.to_string(),
                                display_name: name,
                                bio,
                                mnemonic: String::new(),
                                has_mnemonic: !mn.is_empty(),
                                dht_peers: msg_worker.dht_node.routing_table.read().await.total_contacts(),
                                stored_keys: msg_worker.dht_node.storage.read().await.len(),
                            });
                        }
                        AppCommand::RevealMnemonic => {
                            let mn = msg_worker.get_mnemonic().await;
                            let _ = ui_tx_worker.send(UiEvent::MnemonicRevealed(mn));
                        }
                        AppCommand::RefreshContacts => {
                            let db = msg_worker.db.read().await;
                            let contacts: Vec<_> = db.contacts.values().cloned().collect();
                            let _ = ui_tx_worker.send(UiEvent::ContactsList(contacts));
                        }
                        AppCommand::DeleteMessage { message_id, contact_id, for_everyone } => {
                            if for_everyone {
                                let worker = Arc::clone(&msg_worker);
                                let ui_tx_clone = ui_tx_worker.clone();
                                let m_id = message_id.clone();
                                let c_id = contact_id.clone();
                                tokio::spawn(async move {
                                    let _ = worker.delete_messages_for_everyone(&c_id, vec![m_id.clone()]).await;
                                    let _ = ui_tx_clone.send(UiEvent::MessagesDeleted {
                                        contact_id: c_id,
                                        message_ids: vec![m_id],
                                    });
                                });
                            } else {
                                msg_worker.delete_message(&message_id).await;
                                let _ = ui_tx_worker.send(UiEvent::MessagesDeleted {
                                    contact_id,
                                    message_ids: vec![message_id],
                                });
                            }
                        }
                        AppCommand::DeleteMessages { message_ids, contact_id, for_everyone } => {
                            if for_everyone {
                                let worker = Arc::clone(&msg_worker);
                                let ui_tx_clone = ui_tx_worker.clone();
                                let ids = message_ids.clone();
                                let c_id = contact_id.clone();
                                tokio::spawn(async move {
                                    let _ = worker.delete_messages_for_everyone(&c_id, ids.clone()).await;
                                    let _ = ui_tx_clone.send(UiEvent::MessagesDeleted {
                                        contact_id: c_id,
                                        message_ids: ids,
                                    });
                                });
                            } else {
                                msg_worker.delete_messages(&message_ids).await;
                                let _ = ui_tx_worker.send(UiEvent::MessagesDeleted {
                                    contact_id,
                                    message_ids,
                                });
                            }
                        }
                        AppCommand::ClearChat(contact_id) => {
                            msg_worker.clear_chat(&contact_id).await;
                            let _ = ui_tx_worker.send(UiEvent::MessagesList {
                                contact_id,
                                messages: Vec::new(),
                            });
                        }
                        AppCommand::DeleteContact(contact_id) => {
                            msg_worker.delete_contact(&contact_id).await;
                            let db = msg_worker.db.read().await;
                            let contacts: Vec<_> = db.contacts.values().cloned().collect();
                            let _ = ui_tx_worker.send(UiEvent::ContactsList(contacts));
                        }
                        AppCommand::RetryMessage { recipient_id, text, image_base64, .. } => {
                            let worker = Arc::clone(&msg_worker);
                            let ui_tx_clone = ui_tx_worker.clone();
                            tokio::spawn(async move {
                                match worker.send_message(&recipient_id, &text, image_base64).await {
                                    Ok(saved) => {
                                        let _ = ui_tx_clone.send(UiEvent::MessageSent(saved));
                                    }
                                    Err(e) => {
                                        let _ = ui_tx_clone.send(UiEvent::StatusLog(format!("Retry error: {}", e)));
                                    }
                                }
                            });
                        }
                        AppCommand::BlockContact(contact_id) => {
                            msg_worker.block_contact(&contact_id).await;
                            let p: String = contact_id.chars().take(6).collect();
                            let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Контакт {} заблокирован", p)));
                        }
                        AppCommand::UnblockContact(contact_id) => {
                            msg_worker.unblock_contact(&contact_id).await;
                            let p: String = contact_id.chars().take(6).collect();
                            let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Контакт {} разблокирован", p)));
                        }
                        AppCommand::Reconnect => {
                            let _ = msg_worker.publish_presence().await;
                            let _ = ui_tx_worker.send(UiEvent::StatusLog("Переподключение и публикация presence...".into()));
                        }
                        AppCommand::ToggleRelay { enabled } => {
                            let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Режим Relay: {}", enabled)));
                        }
                        AppCommand::SetRelayTrafficLimit { megabytes_per_hour } => {
                            let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Лимит трафика Relay: {} МБ/ч", megabytes_per_hour)));
                        }
                        AppCommand::SetRelayMaxSessions { sessions } => {
                            let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Максимум сессий Relay: {}", sessions)));
                        }
                        AppCommand::TestConnection(target) => {
                            let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Проверка соединения с {}...", target)));
                        }
                        AppCommand::ExportBackup { path, password } => {
                            let db = msg_worker.db.read().await;
                            match db.export_encrypted_backup(&path, &password) {
                                Ok(_) => {
                                    let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Зашифрованный бэкап сохранен в {}", path)));
                                }
                                Err(e) => {
                                    let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Ошибка экспорта бэкапа: {}", e)));
                                }
                            }
                        }
                        AppCommand::ImportBackup { path, password } => {
                            match MessengerDb::import_encrypted_backup(&path, &password) {
                                Ok(imported_db) => {
                                    {
                                        let mut db = msg_worker.db.write().await;
                                        *db = imported_db;
                                        db.save_to_file(&msg_worker.db_path).ok();
                                    }
                                    let _ = ui_tx_worker.send(UiEvent::StatusLog("Бэкап успешно восстановлен".into()));
                                    let db = msg_worker.db.read().await;
                                    let contacts: Vec<_> = db.contacts.values().cloned().collect();
                                    let _ = ui_tx_worker.send(UiEvent::ContactsList(contacts));
                                }
                                Err(e) => {
                                    let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Ошибка импорта бэкапа: {}", e)));
                                }
                            }
                        }
                        AppCommand::WipeLocalData => {
                            msg_worker.wipe_local_data().await;
                            let _ = ui_tx_worker.send(UiEvent::ContactsList(Vec::new()));
                            let _ = ui_tx_worker.send(UiEvent::StatusLog("Локальные данные и ключи полностью удалены".into()));
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
                    // Periodic stats sync to GUI (without exposing mnemonic)
                    let uid = msg_worker.user_id_hex().await;
                    let (name, bio, has_mn) = {
                        let db = msg_worker.db.read().await;
                        (db.display_name.clone(), db.bio.clone(), !db.mnemonic.is_empty())
                    };
                    let _ = ui_tx_worker.send(UiEvent::NodeInfo {
                        user_id: uid,
                        dht_node_id: msg_worker.dht_node.node_id.to_string(),
                        display_name: name,
                        bio,
                        mnemonic: String::new(),
                        has_mnemonic: has_mn,
                        dht_peers: msg_worker.dht_node.routing_table.read().await.total_contacts(),
                        stored_keys: msg_worker.dht_node.storage.read().await.len(),
                    });
                }
            }
        }
    });


    // Load App Icon from bundled assets
    let icon_bytes = include_bytes!("../assets/logo.png");
    let icon = if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        }
    } else {
        egui::IconData::default()
    };

    // Launch Native 100% Pure Rust eframe / egui Desktop GUI
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([840.0, 520.0])
            .with_title("NodeX — Pure Rust Native P2P Messenger")
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    println!("[GUI] Launching Pure Rust Native Desktop Window...");
    eframe::run_native(
        "NodeX — Pure Rust Native P2P Messenger",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(NodeXApp::new(ui_rx, cmd_tx)))
        }),
    )
    .map_err(|e| format!("eframe GUI error: {}", e))?;

    Ok(())
}
