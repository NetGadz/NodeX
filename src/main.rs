use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use clap::Parser;
use eframe::egui;

use kademlia_dht::config::NodeConfig;
use kademlia_dht::gui::{AppCommand, NodeXApp, UiEvent};
use kademlia_dht::messenger::KadMessenger;
use kademlia_dht::KademliaNode;

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
        let node = match KademliaNode::start_with_config(config.clone()).await {
            Ok(n) => n,
            Err(e) => {
                eprintln!("[FATAL] Failed to start Kademlia Node: {}", e);
                return;
            }
        };

        let db_path = format!("messenger_db_{}.json", config.port);
        let messenger = match KadMessenger::start(node.clone(), db_path, display_name).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[FATAL] Failed to start KadMessenger: {}", e);
                return;
            }
        };

        messenger.start_inbox_polling_task(ui_tx.clone());

        for baddr in &config.bootstrap {
            if let Err(e) = node.bootstrap(*baddr).await {
                eprintln!("[ERROR] Bootstrap to {} failed: {}", baddr, e);
            }
        }

        // Send initial node state to GUI
        let user_id = messenger.identity.user_id_hex();
        let dht_node_id = node.node_id.to_string();
        let display_name = {
            let db = messenger.db.read().await;
            db.display_name.clone()
        };
        let listen_addr = node.network.local_addr.to_string();

        println!("[IDENTITY] User Messenger E2EE ID: {}", user_id);
        println!("[IDENTITY] Kademlia Transport Node ID: {}", dht_node_id);

        let _ = ui_tx.send(UiEvent::NodeInfo {
            user_id: user_id.clone(),
            dht_node_id: dht_node_id.clone(),
            display_name,
            listen_addr,
            dht_peers: node.routing_table.read().await.total_contacts(),
            stored_keys: node.storage.read().await.len(),
        });

        // Send contacts list to GUI
        {
            let db = messenger.db.read().await;
            let contacts: Vec<_> = db.contacts.values().cloned().collect();
            let _ = ui_tx.send(UiEvent::ContactsList(contacts));
        }

        // Background Command & Status Polling Loop
        let msg_worker = Arc::clone(&messenger);
        let ui_tx_worker = ui_tx.clone();

        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        AppCommand::SendMessage { recipient_id, text } => {
                            match msg_worker.send_message(&recipient_id, &text).await {
                                Ok(saved) => {
                                    let _ = ui_tx_worker.send(UiEvent::MessageSent(saved));
                                }
                                Err(e) => {
                                    let _ = ui_tx_worker.send(UiEvent::StatusLog(format!("Send error: {}", e)));
                                }
                            }
                        }
                        AppCommand::AddContact { user_id, name } => {
                            let _ = msg_worker.discover_peer(&user_id).await;
                            let mut db = msg_worker.db.write().await;
                            let c = kademlia_dht::db::SavedContact {
                                user_id_hex: user_id.clone(),
                                name: name.clone(),
                                ed25519_pub_hex: "".into(),
                                x25519_pub_hex: "".into(),
                                last_seen_addr: "unknown".into(),
                            };
                            db.add_contact(c);
                            db.save_to_file(&msg_worker.db_path).ok();
                            let contacts: Vec<_> = db.contacts.values().cloned().collect();
                            let _ = ui_tx_worker.send(UiEvent::ContactsList(contacts));
                        }
                        AppCommand::SelectContact(contact_id) => {
                            let db = msg_worker.db.read().await;
                            let msgs = db.get_messages_for_contact(&contact_id);
                            let _ = ui_tx_worker.send(UiEvent::MessagesList {
                                contact_id,
                                messages: msgs,
                            });
                        }
                        AppCommand::RefreshInfo => {
                            let _ = ui_tx_worker.send(UiEvent::NodeInfo {
                                user_id: msg_worker.identity.user_id_hex(),
                                dht_node_id: msg_worker.dht_node.node_id.to_string(),
                                display_name: msg_worker.db.read().await.display_name.clone(),
                                listen_addr: msg_worker.dht_node.network.local_addr.to_string(),
                                dht_peers: msg_worker.dht_node.routing_table.read().await.total_contacts(),
                                stored_keys: msg_worker.dht_node.storage.read().await.len(),
                            });
                        }
                        AppCommand::RefreshContacts => {
                            let db = msg_worker.db.read().await;
                            let contacts: Vec<_> = db.contacts.values().cloned().collect();
                            let _ = ui_tx_worker.send(UiEvent::ContactsList(contacts));
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
                    // Periodic stats sync to GUI
                    let _ = ui_tx_worker.send(UiEvent::NodeInfo {
                        user_id: msg_worker.identity.user_id_hex(),
                        dht_node_id: msg_worker.dht_node.node_id.to_string(),
                        display_name: msg_worker.db.read().await.display_name.clone(),
                        listen_addr: msg_worker.dht_node.network.local_addr.to_string(),
                        dht_peers: msg_worker.dht_node.routing_table.read().await.total_contacts(),
                        stored_keys: msg_worker.dht_node.storage.read().await.len(),
                    });
                }
            }
        }
    });

    // Launch Native 100% Pure Rust eframe / egui Desktop GUI
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("NodeX — Pure Rust Native P2P Messenger"),
        ..Default::default()
    };

    println!("[GUI] Launching Pure Rust Native Desktop Window...");
    eframe::run_native(
        "NodeX — Pure Rust Native P2P Messenger",
        native_options,
        Box::new(|_cc| Box::new(NodeXApp::new(ui_rx, cmd_tx))),
    )
    .map_err(|e| format!("eframe GUI error: {}", e))?;

    Ok(())
}
