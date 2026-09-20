use std::net::SocketAddr;
use std::path::Path;
use clap::Parser;
use tokio::io::{AsyncBufReadExt, BufReader};

use nodex_kademlia::config::NodeConfig;
use nodex_kademlia::KademliaNode;
use nodex_messenger::db::SavedContact;
use nodex_messenger::{KadMessenger, MessengerEvent};

#[derive(Parser, Debug)]
#[command(
    name = "nodex-cli",
    about = "NodeX CLI — Headless P2P Node & Messenger Daemon"
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
    #[arg(long, default_value = "CliNode")]
    pub name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    println!("     NodeX CLI — Headless P2P Kademlia Node & Daemon       ");
    println!("============================================================");

    let node = KademliaNode::start_with_config(config.clone()).await?;

    let db_path = format!("messenger_db_{}.json", config.port);
    let messenger = KadMessenger::start(node.clone(), db_path.clone(), cli.name).await?;

    let (msg_event_tx, msg_event_rx) = std::sync::mpsc::channel::<MessengerEvent>();
    messenger.start_inbox_polling_task(Some(msg_event_tx));

    // Background listener for incoming messenger events
    std::thread::spawn(move || {
        while let Ok(event) = msg_event_rx.recv() {
            match event {
                MessengerEvent::ContactsUpdated(contacts) => {
                    println!("\n[EVENT] Contacts updated (total: {})", contacts.len());
                }
                MessengerEvent::MessageReceived(msg) => {
                    println!(
                        "\n\x1b[32m[MSG INCOMING]\x1b[0m From {}: {}",
                        msg.sender_id_hex, msg.text
                    );
                }
                MessengerEvent::MessageDeleted { contact_id, message_ids } => {
                    println!(
                        "\n\x1b[33m[MSG DELETED]\x1b[0m From/For {}: {} messages removed remotely",
                        contact_id, message_ids.len()
                    );
                }
                MessengerEvent::MessageDelivered { recipient_id, message_id } => {
                    println!(
                        "\n\x1b[32m[MSG DELIVERED]\x1b[0m Message {} delivered to {}",
                        message_id, recipient_id
                    );
                }
                MessengerEvent::MessageEdited { message_id, new_text } => {
                    println!(
                        "\n\x1b[33m[MSG EDITED]\x1b[0m Message {} edited: {}",
                        message_id, new_text
                    );
                }
                MessengerEvent::ReactionAdded { message_id, emoji, reactor_id } => {
                    println!(
                        "\n\x1b[35m[REACTION]\x1b[0m {} reacted with {} to {}",
                        reactor_id, emoji, message_id
                    );
                }
                MessengerEvent::CallSignalReceived(signal) => {
                    println!(
                        "\n\x1b[36m[CALL SIGNAL]\x1b[0m Type: {:?}, Call ID: {}, From: {}",
                        signal.signal_type, signal.call_id, signal.caller_id_hex
                    );
                }
                MessengerEvent::CallAudioReceived { sender_id: _, chunk: _ } => {
                    // In CLI, suppress noisy real-time audio chunk logs
                }
            }
        }
    });

    for baddr in &config.bootstrap {
        if let Err(e) = node.bootstrap(*baddr).await {
            eprintln!("[ERROR] Bootstrap to {} failed: {}", baddr, e);
        }
    }

    let user_id = messenger.user_id_hex().await;
    let dht_node_id = node.node_id.to_string();

    println!("[IDENTITY] User Messenger E2EE ID: {}", user_id);
    println!("[IDENTITY] Kademlia Transport Node ID: {}", dht_node_id);
    println!("[NET] Listening on: {}", node.network.local_addr);
    println!("\nType 'help' for available commands.\n");

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_lowercase();

        match cmd.as_str() {
            "help" => {
                println!("Available commands:");
                println!("  info                                — Show node, identity & DHT routing info");
                println!("  nodes                               — List known routing table contacts");
                println!("  ping <ip:port>                      — Ping a remote address");
                println!("  put <key> <value>                   — Store value in DHT");
                println!("  get <key>                           — Lookup value in DHT");
                println!("  send <user_id_hex> <text...>        — Send E2EE message via 5-tier delivery cascade");
                println!("  sendfile <user_id_hex> <filepath>   — Send file up to 100MB via chunked manifest");
                println!("  history <user_id_hex> [limit]       — View chat history with peer");
                println!("  contacts                            — List local saved contacts");
                println!("  add <user_id_hex> <name>            — Discover and save contact");
                println!("  block <user_id_hex>                 — Block a peer");
                println!("  unblock <user_id_hex>               — Unblock a peer");
                println!("  outbox                              — View pending outbox retry count");
                println!("  mnemonic                            — Display BIP-39 mnemonic recovery phrase");
                println!("  backup <filepath> <password>        — Export password-encrypted backup");
                println!("  wipe                                — Securely wipe all local keys and data");
                println!("  save                                — Save node routing state");
                println!("  quit / exit                         — Terminate node");
            }
            "info" => {
                let rt_count = node.routing_table.read().await.total_contacts();
                let st_count = node.storage.read().await.len();
                let db = messenger.db.read().await;
                let uid = messenger.user_id_hex().await;
                println!("--- Node Information ---");
                println!("  Display Name:      {}", db.display_name);
                println!("  User E2EE ID:      {}", uid);
                println!("  DHT Node ID:       {}", node.node_id);
                println!("  Listen Address:    {}", node.network.local_addr);
                println!("  Routing Table:     {} peers", rt_count);
                println!("  Stored Keys:       {} keys", st_count);
                println!("  Saved Contacts:    {}", db.contacts.len());
                println!("  Blocked Contacts:  {}", db.blocked_user_ids.len());
            }
            "nodes" => {
                node.list_nodes().await;
            }
            "ping" => {
                if parts.len() < 2 {
                    println!("Usage: ping <ip:port>");
                    continue;
                }
                if let Ok(addr) = parts[1].parse::<SocketAddr>() {
                    let _ = node.ping_addr(addr).await;
                } else {
                    println!("Invalid socket address: {}", parts[1]);
                }
            }
            "put" => {
                if parts.len() < 3 {
                    println!("Usage: put <key> <value>");
                    continue;
                }
                let key = parts[1];
                let value = parts[2..].join(" ").into_bytes();
                match node.put(key, value).await {
                    Ok(count) => println!("Stored on {} node(s)", count),
                    Err(e) => eprintln!("Put error: {}", e),
                }
            }
            "get" => {
                if parts.len() < 2 {
                    println!("Usage: get <key>");
                    continue;
                }
                let key = parts[1];
                match node.get(key).await {
                    Ok(Some((val, from))) => {
                        let text = String::from_utf8_lossy(&val);
                        println!("Found value: '{}' (from: {:?})", text, from);
                    }
                    Ok(None) => println!("Key '{}' not found in DHT", key),
                    Err(e) => eprintln!("Get error: {}", e),
                }
            }
            "send" => {
                if parts.len() < 3 {
                    println!("Usage: send <user_id_hex> <message text...>");
                    continue;
                }
                let recipient = parts[1];
                let text = parts[2..].join(" ");
                match messenger.send_message(recipient, &text, None).await {
                    Ok(saved) => println!("\x1b[32m[SENT]\x1b[0m ID: {} (delivered: {})", saved.id, saved.delivered),
                    Err(e) => eprintln!("Send error: {}", e),
                }
            }
            "sendfile" => {
                if parts.len() < 3 {
                    println!("Usage: sendfile <user_id_hex> <filepath>");
                    continue;
                }
                let recipient = parts[1];
                let filepath = parts[2];
                match std::fs::read(filepath) {
                    Ok(bytes) => {
                        let filename = std::path::Path::new(filepath)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "file.bin".to_string());
                        println!("[TRANSFER] Preparing {} bytes ({})...", bytes.len(), filename);
                        match messenger.send_file(recipient, &filename, &bytes).await {
                            Ok((msg, manifest)) => {
                                let short_hash = if manifest.sha256_root.len() >= 12 { &manifest.sha256_root[..12] } else { &manifest.sha256_root };
                                println!("\x1b[32m[FILE SENT]\x1b[0m ID: {} | Manifest SHA256: {} | Chunks: {}", 
                                    msg.id, short_hash, manifest.chunk_count);
                            }
                            Err(e) => eprintln!("File transfer error: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to read file '{}': {}", filepath, e),
                }
            }
            "history" => {
                if parts.len() < 2 {
                    println!("Usage: history <user_id_hex> [limit]");
                    continue;
                }
                let uid = parts[1];
                let limit = parts.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
                let db = messenger.db.read().await;
                let msgs = db.get_messages_paginated(uid, limit, None);
                println!("--- Message History with {} ({} shown) ---", &uid[..std::cmp::min(10, uid.len())], msgs.len());
                for m in msgs {
                    let dir = if m.incoming { "\x1b[36m<---\x1b[0m" } else { "\x1b[32m--->\x1b[0m" };
                    let status = if m.incoming { "" } else if m.delivered { " [Delivered]" } else { " [Queued]" };
                    println!("  {} [{}] {}{}", dir, m.timestamp, m.text, status);
                }
            }
            "contacts" => {
                let db = messenger.db.read().await;
                println!("--- Saved Contacts ({}) ---", db.contacts.len());
                for (id, c) in &db.contacts {
                    let short_id = if id.len() > 10 { &id[..10] } else { id };
                    let blocked = if db.is_blocked(id) { " \x1b[31m[BLOCKED]\x1b[0m" } else { "" };
                    println!("  {} — {} (addr: {}){}", short_id, c.name, c.last_seen_addr, blocked);
                }
            }
            "add" => {
                if parts.len() < 3 {
                    println!("Usage: add <user_id_hex> <name>");
                    continue;
                }
                let uid = parts[1];
                let name = parts[2..].join(" ");
                let _ = messenger.discover_peer(uid).await;
                let mut db = messenger.db.write().await;
                db.add_contact(SavedContact {
                    user_id_hex: uid.to_string(),
                    name,
                    bio: "".into(),
                    ed25519_pub_hex: "".into(),
                    x25519_pub_hex: "".into(),
                    last_seen_addr: "unknown".into(),
                });
                db.save_to_file(&db_path).ok();
                println!("Contact added: {}", uid);
            }
            "block" => {
                if parts.len() < 2 {
                    println!("Usage: block <user_id_hex>");
                    continue;
                }
                let uid = parts[1];
                messenger.block_contact(uid).await;
                println!("\x1b[31m[BLOCKED]\x1b[0m Peer {} is now blocked", uid);
            }
            "unblock" => {
                if parts.len() < 2 {
                    println!("Usage: unblock <user_id_hex>");
                    continue;
                }
                let uid = parts[1];
                messenger.unblock_contact(uid).await;
                println!("\x1b[32m[UNBLOCKED]\x1b[0m Peer {} is now unblocked", uid);
            }
            "outbox" => {
                let count = messenger.outbox.len().await;
                println!("Pending messages in Outbox queue: {}", count);
            }
            "mnemonic" => {
                let mnemonic = messenger.db.read().await.mnemonic.clone();
                if !mnemonic.is_empty() {
                    println!("\x1b[33m[BIP-39 MNEMONIC]\x1b[0m {}\n(Keep these 24 words safe!)", mnemonic);
                } else {
                    println!("No mnemonic phrase stored for this profile.");
                }
            }
            "backup" => {
                if parts.len() < 3 {
                    println!("Usage: backup <filepath> <password>");
                    continue;
                }
                let path = parts[1];
                let pass = parts[2];
                let db = messenger.db.read().await;
                match db.export_encrypted_backup(path, pass) {
                    Ok(()) => println!("\x1b[32m[BACKUP]\x1b[0m Encrypted backup written to {}", path),
                    Err(e) => eprintln!("Backup error: {}", e),
                }
            }
            "wipe" => {
                println!("\x1b[31m[CAUTION]\x1b[0m Wiping all local data and cryptographic keys...");
                messenger.wipe_local_data().await;
                println!("[WIPED] All credentials and database wiped.");
            }
            "save" => {
                let _ = node.save_state().await;
            }
            "quit" | "exit" => {
                println!("Shutting down node...");
                if config.save_state_on_exit {
                    let _ = node.save_state().await;
                }
                break;
            }
            _ => {
                println!("Unknown command: '{}'. Type 'help' for available commands.", cmd);
            }
        }
    }

    Ok(())
}
