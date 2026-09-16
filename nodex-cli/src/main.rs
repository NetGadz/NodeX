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
                println!("  info                          — Show node & identity info");
                println!("  nodes                         — List known routing table contacts");
                println!("  ping <ip:port>                — Ping a remote address");
                println!("  put <key> <value>             — Store value in DHT");
                println!("  get <key>                     — Lookup value in DHT");
                println!("  send <user_id_hex> <text...>  — Send E2EE message to peer");
                println!("  contacts                      — List local saved contacts");
                println!("  add <user_id_hex> <name>      — Discover and save contact");
                println!("  save                          — Save node state");
                println!("  quit / exit                   — Terminate node");
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
            "contacts" => {
                let db = messenger.db.read().await;
                println!("--- Saved Contacts ({}) ---", db.contacts.len());
                for (id, c) in &db.contacts {
                    println!("  {} — {} (addr: {})", &id[..10], c.name, c.last_seen_addr);
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
