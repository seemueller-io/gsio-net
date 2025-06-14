use axum::routing::get;
use serde_json::{json, Value as JsonValue};
use socketioxide::{
    extract::{AckSender, Data, SocketRef},
    SocketIo,
};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

mod ledger;
mod p2p;

use ledger::SharedLedger;
use p2p::P2PManager;

// Handle regular client connections
async fn on_connect(socket: SocketRef, Data(data): Data<JsonValue>, p2p_manager: Arc<P2PManager>) {
    info!(ns = socket.ns(), ?socket.id, "Socket.IO client connected");
    socket.emit("auth", &data).ok();

    // Set up basic message handlers
    socket.on("message", |socket: SocketRef, Data(data): Data<JsonValue>| async move {
        info!(?data, "Received event:");
        socket.emit("message-back", &data).ok();
    });

    socket.on("ping", |socket: SocketRef, Data(data): Data<JsonValue>| async move {
        socket.emit("pong", &data).ok();
    });

    socket.on(
        "message-with-ack",
        |Data(data): Data<JsonValue>, ack: AckSender| async move {
            info!(?data, "Received event");
            ack.send(&data).ok();
        },
    );

    // Set up ledger-related handlers
    let p2p_manager_clone = p2p_manager.clone();
    socket.on(
        "add_ledger_entry",
        move |socket: SocketRef, Data(data): Data<JsonValue>| {
            let p2p_manager = p2p_manager_clone.clone();
            async move {
                info!(?data, "Adding ledger entry");

                // Add the entry to the ledger
                match p2p_manager.ledger.add_entry(data) {
                    Ok(entry) => {
                        // Broadcast the entry to all connected nodes
                        p2p_manager.broadcast_entry(entry.clone());

                        // Send the entry back to the client
                        socket.emit("ledger_entry_added", &serde_json::to_value(entry).unwrap()).ok();
                    },
                    Err(e) => {
                        socket.emit("error", &json!({ "error": e })).ok();
                    }
                }
            }
        },
    );

    let p2p_manager_clone = p2p_manager.clone();
    socket.on("get_ledger", move |socket: SocketRef| {
        let p2p_manager = p2p_manager_clone.clone();
        async move {
            info!("Getting ledger entries");

            // Get all entries in the ledger
            let entries = p2p_manager.ledger.get_entries();

            // Send the entries to the client
            socket.emit("ledger_entries", &serde_json::to_value(entries).unwrap()).ok();
        }
    });

    let p2p_manager_clone = p2p_manager.clone();
    socket.on("get_known_nodes", move |socket: SocketRef| {
        let p2p_manager = p2p_manager_clone.clone();
        async move {
            info!("Getting known nodes");

            // Get all known nodes
            let nodes = p2p_manager.ledger.get_known_nodes();

            // Send the nodes to the client
            socket.emit("known_nodes", &json!({ "nodes": nodes })).ok();
        }
    });
}

// Handle p2p node connections
async fn on_p2p_connect(socket: SocketRef, Data(data): Data<JsonValue>, p2p_manager: Arc<P2PManager>) {
    info!(ns = socket.ns(), ?socket.id, "P2P node connected");

    // Handle the connection in the p2p manager
    p2p_manager.handle_connection(socket, data);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;

    // Generate a unique ID for this node
    let node_id = Uuid::new_v4().to_string();
    info!("Starting node with ID: {}", node_id);

    // Create the shared ledger
    let ledger = SharedLedger::new(node_id.clone());

    // Create the p2p manager
    let p2p_manager = Arc::new(P2PManager::new(node_id, ledger));

    let (layer, io) = SocketIo::new_layer();

    // Set up namespaces
    let p2p_manager_clone = p2p_manager.clone();
    io.ns("/", move |s, d| on_connect(s, d, p2p_manager_clone.clone()));

    let p2p_manager_clone = p2p_manager.clone();
    io.ns("/p2p", move |s, d| on_p2p_connect(s, d, p2p_manager_clone.clone()));

    let app = axum::Router::new()
        .route("/", get(|| async { "GSIO-Net Distributed Ledger Node" }))
        .layer(layer);

    info!("Starting server on port 3000");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
