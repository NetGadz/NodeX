mod app;
mod audio_player;
mod components;
mod mic_recorder;
mod state;
mod theme;
mod views;

use std::sync::mpsc;
use app::NodeXApp;
use clap::Parser;
use nodex_kademlia::config::NodeConfig;
use nodex_kademlia::KademliaNode;
use nodex_messenger::messenger::KadMessenger;
use state::AppState;

#[derive(Parser, Debug)]
#[command(name = "nodex-gui", author = "NodeX Developers", version = "2.0.0", about = "Decentralized P2P E2EE Messenger")]
struct CliArgs {
    #[arg(short, long, default_value = "8443")]
    port: u16,

    #[arg(short, long, default_value = "User")]
    name: String,

    #[arg(short, long)]
    bootstrap: Option<String>,
}

fn main() -> eframe::Result<()> {
    let args = CliArgs::parse();

    // 1. Setup tokio runtime for background P2P DHT networking
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    let (event_tx, event_rx) = mpsc::channel();

    let (messenger_arc, initial_state) = rt.block_on(async {
        let mut config = NodeConfig::default();
        config.port = args.port;
        if let Some(b) = &args.bootstrap {
            if let Ok(addr) = b.parse() {
                config.bootstrap.push(addr);
            }
        }
        for candidate_port in [8443, 8444, 8445, 8446, 8447, 8448, 8000, 8001] {
            if let Ok(addr) = format!("127.0.0.1:{}", candidate_port).parse() {
                if !config.bootstrap.contains(&addr) {
                    config.bootstrap.push(addr);
                }
            }
        }

        match KademliaNode::start_auto(config.clone()).await {
            Ok(node) => {
                let actual_port = node.network.local_addr.port();
                let is_onboarded = std::path::Path::new(&format!("nodex_account_{}.ready", actual_port)).exists();
                let db_path = format!("messenger_db_{}.json", actual_port);
                let mut state = AppState::new(args.name.clone(), is_onboarded, actual_port);

                if let Ok(messenger) = KadMessenger::start(node.clone(), db_path.clone(), args.name.clone()).await {
                    messenger.start_inbox_polling_task(Some(event_tx));
                    state.my_node_id = messenger.user_id_hex().await;
                    let mn = messenger.get_mnemonic().await;
                    if !mn.is_empty() {
                        state.mnemonic_seed = mn;
                    }

                    // Background localhost peer discovery
                    let disc_node = node.clone();
                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
                        let my_port = disc_node.network.local_addr.port();
                        loop {
                            interval.tick().await;
                            for port in 8443..=8450 {
                                if port == my_port {
                                    continue;
                                }
                                if let Ok(addr) = format!("127.0.0.1:{}", port).parse::<std::net::SocketAddr>() {
                                    let is_known = {
                                        let rt = disc_node.routing_table.read().await;
                                        rt.all_contacts().iter().any(|c| c.addr == addr)
                                    };
                                    if !is_known {
                                        if let Ok(reply) = disc_node.network.call(addr, nodex_kademlia::rpc::RpcPayload::Ping, std::time::Duration::from_millis(300), 1).await {
                                            {
                                                let mut rt = disc_node.routing_table.write().await;
                                                rt.update(nodex_kademlia::node::Contact::new(reply.sender_id, addr));
                                            }
                                            let _ = disc_node.bootstrap(addr).await;
                                        }
                                    }
                                }
                            }
                        }
                    });

                    (Some(messenger), state)
                } else {
                    (None, state)
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to start Kademlia node: {}", e);
                let is_onboarded = std::path::Path::new(&format!("nodex_account_{}.ready", args.port)).exists();
                let state = AppState::new(args.name.clone(), is_onboarded, args.port);
                (None, state)
            }
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1160.0, 750.0])
            .with_min_inner_size([880.0, 560.0])
            .with_title("NodeX — Decentralized P2P Messenger"),
        ..Default::default()
    };

    let rt_handle = rt.handle().clone();

    eframe::run_native(
        "NodeX — Decentralized P2P Messenger",
        native_options,
        Box::new(move |_cc| {
            egui_extras::install_image_loaders(&_cc.egui_ctx);
            Ok(Box::new(NodeXApp::new(
                initial_state,
                messenger_arc,
                Some(event_rx),
                rt_handle,
            )))
        }),
    )
}
