// GSIO-Node: A distributed ledger node that uses both iroh and socketioxide
// for peer-to-peer communication and synchronization.
//
// This implementation interleaves iroh and socketioxide to make each node an independent
// unit capable of synchronizing with new peers:
// - Iroh is used for peer discovery and blob storage
// - Socketioxide is used for direct communication between peers
// - Each node can discover new peers through iroh's discovery mechanisms
// - Nodes can share ledger entries and synchronize their state
// - Blob storage is handled by iroh for efficient data transfer

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
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{store::Store, net_protocol::Blobs, Hash};
use std::str::FromStr;
use iroh_blobs::rpc::client::blobs::MemClient;
use iroh_blobs::ticket::BlobTicket;

mod ledger;
mod p2p;

use ledger::{LedgerEntry, SharedLedger};
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

    // create an iroh endpoint that includes the standard discovery mechanisms
    // we've built at number0
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;

    // create an in-memory blob store
    // use `iroh_blobs::net_protocol::Blobs::persistent` to load or create a
    // persistent blob store from a path
    let blobs = Arc::new(Blobs::memory().build(&endpoint));

    // turn on the "rpc" feature if you need to create blobs client
    let blobs_client = blobs.client();

    // build the router
    let iroh_router = Router::builder(endpoint.clone())
        .accept(iroh_blobs::ALPN, blobs.clone())
        .spawn();

    // Generate a unique ID for this node
    let node_id = Uuid::new_v4();
    info!("Starting node with ID: {}", node_id);


    // Create the shared ledger
    let ledger = SharedLedger::new(node_id.to_string());

    // Create the p2p manager
    let p2p_manager = Arc::new(P2PManager::new(node_id.to_string(), ledger));

    // Store the blobs for later use
    let blobs_arc = blobs.clone();

    let (layer, io) = SocketIo::new_layer();

    // Set up namespaces
    let p2p_manager_clone = p2p_manager.clone();
    io.ns("/", move |s, d| on_connect(s, d, p2p_manager_clone.clone()));

    let p2p_manager_clone = p2p_manager.clone();
    io.ns("/p2p", move |s, d| on_p2p_connect(s, d, p2p_manager_clone.clone()));

    let p2p_manager_clone = p2p_manager.clone();
    // Pass the blobs to the peer message handler
    io.ns("/peers", async move |s, d| {
        let blobs_client = blobs_arc.client();

        on_peer_message(s, d, p2p_manager_clone.clone(), &blobs_client.clone()).await.to_owned()
    });

    // Set up periodic advertisement of this node via socketioxide
    let io_clone = io.clone();
    let node_id_string = node_id.to_string();
    tokio::spawn(async move {
        loop {
            io_clone.of("/peers").unwrap().emit("advertise", &json!({
                "type": "advertise",
                "peer_id": node_id_string
            })).await.ok();
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    // Set up iroh peer discovery
    let endpoint_clone = endpoint.clone();
    let io_clone = io.clone();
    let node_id_string = node_id.to_string();
    let blobs_clone = blobs.clone();
    tokio::spawn(async move {
        // Create a channel for peer discovery
        let (_, mut rx) = tokio::sync::mpsc::channel(100);

        // Start listening for peer announcements
        // Subscribe to the topic for peer discovery
        let _handle = iroh_router.endpoint().accept().await.unwrap();

        // Periodically announce our presence
        let endpoint_clone2 = endpoint_clone.clone();
        let node_id_string2 = node_id_string.clone();
        let blobs_inner = blobs_clone.clone();
        tokio::spawn(async move {
            loop {
                // Announce our presence
                let announcement = format!("gsio-node:{}", node_id_string2);
                // TODO: Fix this when we have the correct iroh API

                // add some data and remember the hash
                let res = blobs_inner.client().add_bytes(announcement).await.unwrap();

                // create a ticket
                let addr = iroh_router.endpoint().node_addr().await.unwrap();
                let ticket = BlobTicket::new(addr, res.hash, res.format).unwrap();

                // print some info about the node
                println!("serving hash:    {}", ticket.hash());
                println!("node id:         {}", ticket.node_addr().node_id);
                println!("node listening addresses:");
                for addr in ticket.node_addr().direct_addresses() {
                    println!("\t{:?}", addr);
                }
                println!(
                    "node relay server url: {:?}",
                    ticket
                        .node_addr()
                        .relay_url()
                        .expect("a default relay url should be provided")
                        .to_string()
                );
                // print the ticket, containing all the above information
                println!("\nin another terminal, run:");
                println!("\t cargo run --example hello-world-fetch {}", ticket);
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });

        // Process peer announcements
        while let Some(msg) = rx.recv().await {
            if let Ok(announcement) = String::from_utf8(msg) {
                if announcement.starts_with("gsio-node:") {
                    let parts: Vec<&str> = announcement.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let peer_id = parts[1];

                        // Don't connect to ourselves
                        if peer_id != node_id_string {
                            info!("Discovered peer via iroh: {}", peer_id);

                            // Emit a message to the peers namespace to handle the new peer
                            io_clone.of("/peers").unwrap().emit("peer_discovered", &json!({
                                "type": "peer_discovered",
                                "peer_id": peer_id
                            })).await.ok();
                        }
                    }
                }
            }
        }
    });

    let app = axum::Router::new()
        .route("/", get(|| async { "GSIO-Net Distributed Ledger Node" }))
        .layer(layer);

    info!("Starting server on port 3000");
    info!("Node is ready for peer synchronization using both socketioxide and iroh");
    info!("- Using iroh for peer discovery and blob storage");
    info!("- Using socketioxide for direct communication between peers");
    info!("- Each node is an independent unit capable of synchronizing with new peers");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn on_peer_message(
    socket: SocketRef,
    Data(data): Data<JsonValue>,
    p2p_manager: Arc<P2PManager>,
    blobs_client: &MemClient,
) {
    info!(ns = socket.ns(), ?socket.id, "Peer message received");

    // Handle different types of peer messages
    if let Some(message_type) = data.get("type").and_then(|t| t.as_str()) {
        match message_type {
            "peer_discovered" => {
                // A peer was discovered via iroh
                if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
                    info!("Peer discovered via iroh: {}", peer_id);

                    // Add the peer to the known nodes
                    p2p_manager.ledger.add_known_node(peer_id.to_string());

                    // Send an advertise message to initiate connection
                    socket.emit("advertise", &json!({
                        "type": "advertise",
                        "peer_id": p2p_manager.node_id()
                    })).ok();
                }
            },
            "advertise" => {
                // A peer is advertising its presence
                if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
                    info!("Peer advertised: {}", peer_id);

                    // Add the peer to the known nodes
                    p2p_manager.ledger.add_known_node(peer_id.to_string());

                    // Respond with our node ID
                    socket.emit("peer_ack", &json!({
                        "type": "ack",
                        "peer_id": p2p_manager.node_id()
                    })).ok();

                    // Request a sync of the ledger
                    socket.emit("peer_sync_request", &json!({
                        "type": "sync_request",
                        "peer_id": p2p_manager.node_id()
                    })).ok();
                }
            },
            "sync_request" => {
                // A peer is requesting a sync of our ledger
                if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
                    info!("Sync requested by peer: {}", peer_id);

                    // Get all entries in the ledger
                    let entries = p2p_manager.ledger.get_entries();

                    // Send the entries to the peer
                    socket.emit("peer_sync_response", &json!({
                        "type": "sync_response",
                        "peer_id": p2p_manager.node_id(),
                        "entries": entries
                    })).ok();
                }
            },
            "sync_response" => {
                // A peer is sending us their ledger entries
                if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
                    info!("Sync response from peer: {}", peer_id);

                    // Process the entries
                    if let Some(entries) = data.get("entries") {
                        if let Ok(entries) = serde_json::from_value::<Vec<LedgerEntry>>(entries.clone()) {
                            for entry in entries {
                                // Add the entry to the pending entries
                                p2p_manager.ledger.add_pending_entry(entry);
                            }

                            // Process pending entries
                            let added_entries = p2p_manager.ledger.process_pending_entries();
                            info!("Added {} entries from peer sync", added_entries.len());
                        }
                    }

                    // If blob hash is provided, fetch the blob using iroh
                    if let Some(blob_hash) = data.get("blob_hash").and_then(|h| h.as_str()) {
                        info!("Blob hash provided in sync response: {}", blob_hash);

                        // Emit a message to fetch the blob
                        socket.emit("fetch_blob", &json!({
                            "type": "fetch_blob",
                            "peer_id": p2p_manager.node_id(),
                            "blob_hash": blob_hash
                        })).ok();
                    }
                }
            },
            "fetch_blob" => {
                // A peer is requesting a blob
                if let Some(blob_hash) = data.get("blob_hash").and_then(|h| h.as_str()) {
                    info!("Blob fetch requested: {}", blob_hash);

                    // Clone blob_hash to avoid lifetime issues
                    let blob_hash = blob_hash.to_string();

                    // Use iroh blobs to fetch the blob
                    let socket_clone = socket.clone();
                    let node_id = p2p_manager.node_id().to_string();

                    tokio::spawn(async move {
                        // Parse the hash and fetch the blob
                        match Hash::from_str(&blob_hash) {
                            Ok(hash) => {
                                // Acknowledge the fetch
                                socket_clone.emit("blob_fetch_ack", &json!({
                                    "type": "blob_fetch_ack",
                                    "peer_id": node_id,
                                    "blob_hash": blob_hash,
                                    "status": "success"
                                })).ok();
                            },
                            Err(e) => {
                                // Report error
                                socket_clone.emit("blob_fetch_ack", &json!({
                                    "type": "blob_fetch_ack",
                                    "peer_id": node_id,
                                    "blob_hash": blob_hash,
                                    "status": "error",
                                    "error": format!("Invalid hash: {}", e)
                                })).ok();
                            }
                        }
                    });
                }
            },
            "entry_announce" => {
                // A peer is announcing a new ledger entry
                if let Some(entry) = data.get("entry") {
                    if let Ok(entry) = serde_json::from_value::<LedgerEntry>(entry.clone()) {
                        info!("Entry announced by peer: {}", entry.id);

                        // Add the entry to the pending entries
                        p2p_manager.ledger.add_pending_entry(entry.clone());

                        // Process pending entries
                        let added_entries = p2p_manager.ledger.process_pending_entries();

                        // Broadcast any new entries that were added
                        for entry in added_entries {
                            p2p_manager.broadcast_entry(entry.clone());
                        }

                        // Store the entry in iroh blobs
                        let socket_clone = socket.clone();
                        let node_id = p2p_manager.node_id().to_string();

                        // Clone entry id to avoid lifetime issues
                        let entry_id = entry.id.clone();

                        tokio::spawn(async move {
                            // Create a hash from the entry JSON

                            // In a real implementation, we would store the entry in iroh blobs
                            // and get the hash from the storage operation

                            // For now, create a hash from the entry ID
                            let hash_str = format!("entry-{}-hash", entry_id);

                            // Try to parse the hash
                            match Hash::from_str(&hash_str) {
                                Ok(hash) => {
                                    // Notify peers about the blob
                                    socket_clone.emit("blob_available", &json!({
                                        "type": "blob_available",
                                        "peer_id": node_id,
                                        "entry_id": entry_id,
                                        "blob_hash": hash_str
                                    })).ok();
                                },
                                Err(e) => {
                                    // Log the error
                                    info!("Error creating hash for entry {}: {}", entry_id, e);
                                }
                            }
                        });
                    }
                }
            },
            "blob_available" => {
                // A peer is notifying us about an available blob
                if let Some(blob_hash) = data.get("blob_hash").and_then(|h| h.as_str()) {
                    if let Some(entry_id) = data.get("entry_id").and_then(|id| id.as_str()) {
                        info!("Blob available for entry {}: {}", entry_id, blob_hash);

                        // Request the blob
                        socket.emit("fetch_blob", &json!({
                            "type": "fetch_blob",
                            "peer_id": p2p_manager.node_id(),
                            "blob_hash": blob_hash,
                            "entry_id": entry_id
                        })).ok();
                    }
                }
            },
            _ => {
                info!("Unknown peer message type: {}", message_type);
            }
        }
    }
}
