// GSIO-Node: a distributed-ledger node that interleaves iroh (discovery + blobs)
// and socketioxide (direct messaging) for fully-decentralized sync.
//
// - Iroh handles peer discovery and blob storage
// - Socketioxide handles live peer-to-peer messaging
// - Each node is an autonomous sync unit

use axum::{routing::get, Router};
use iroh::{protocol::Router as IrohRouter, Endpoint};
use iroh_blobs::{
    net_protocol::Blobs,
    rpc::client::blobs::MemClient,
    store::Store,
    ticket::BlobTicket,
    Hash, ALPN,
};
use serde_json::{json, Value as JsonValue};
use socketioxide::{
    extract::{AckSender, Data, SocketRef},
    SocketIo,
};
use std::{str::FromStr, sync::Arc};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

mod ledger;
mod p2p;

use ledger::{LedgerEntry, SharedLedger};
use p2p::P2PManager;

/// ========== Socket.io namespace helpers ==========
fn register_root_namespace(io: &SocketIo, p2p: Arc<P2PManager>) {
    let p2p_clone = p2p.clone();
    io.ns("/", move |s, d| on_connect(s, d, p2p_clone.clone()));
}

fn register_p2p_namespace(io: &SocketIo, p2p: Arc<P2PManager>) {
    let p2p_clone = p2p.clone();
    io.ns("/p2p", move |s, d| on_p2p_connect(s, d, p2p_clone.clone()));
}

fn register_peer_namespace<S>(io: &SocketIo, p2p: Arc<P2PManager>, blobs: Arc<Blobs<S>>)
where
    S: Store + Send + Sync + 'static,
{
    let p2p_clone = p2p.clone();
    let blobs_arc = blobs.clone();
    io.ns("/peers", async move |s, d| {
        let blobs_client = blobs_arc.client();
        on_peer_message(s, d, p2p_clone.clone(), &blobs_client.clone()).await.to_owned()
    });
}

/// ========== Periodic tasks ==========
fn spawn_advertisement_task(io: SocketIo, node_id: String) {
    tokio::spawn(async move {
        loop {
            if let Some(nsp) = io.of("/peers") {
                nsp.emit("advertise", &json!({ "type": "advertise", "peer_id": node_id }))
                    .await
                    .ok();
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}

fn spawn_peer_discovery_task<S>(
    endpoint: Endpoint,
    router: IrohRouter,
    io: SocketIo,
    node_id: String,
    blobs: Arc<Blobs<S>>,
) where
    S: Store + Send + Sync + 'static,
{
    tokio::spawn(async move {
        // Stub channel for future custom hooks
        let (_, mut _rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);

        // Periodically announce presence
        let router_clone = router.clone();
        let blobs_clone = blobs.clone();
        tokio::spawn(async move {
            loop {
                let announcement = format!("gsio-node:{node_id}");
                let res = blobs_clone.client().add_bytes(announcement).await.unwrap();

                let addr = router_clone.endpoint().node_addr().await.unwrap();
                let ticket = BlobTicket::new(addr, res.hash, res.format).unwrap();

                info!("serving hash: {}", ticket.hash());
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });

        // Additional discovery-message processing could be added here
        drop((_rx, endpoint)); // Silence unused warnings for now
    });
}

/// ========== Socket connection handlers ==========
async fn on_connect(socket: SocketRef, Data(data): Data<JsonValue>, p2p: Arc<P2PManager>) {
    info!(ns = socket.ns(), ?socket.id, "Socket.IO client connected");
    socket.emit("auth", &data).ok();
    register_basic_handlers(&socket);
    register_ledger_handlers(&socket, p2p).await;
}

fn register_basic_handlers(socket: &SocketRef) {
    socket.on("message", |socket: SocketRef, Data(d): Data<JsonValue>| async move {
        socket.emit("message-back", &d).ok();
    });
    socket.on("ping", |socket: SocketRef, Data(d): Data<JsonValue>| async move {
        socket.emit("pong", &d).ok();
    });
    socket.on(
        "message-with-ack",
        |Data(d): Data<JsonValue>, ack: AckSender| async move {
            ack.send(&d).ok();
        },
    );
}

async fn register_ledger_handlers(socket: &SocketRef, p2p: Arc<P2PManager>) {
    let add_clone = p2p.clone();
    socket.on(
        "add_ledger_entry",
        move |socket: SocketRef, Data(d): Data<JsonValue>| {
            let p2p = add_clone.clone();
            async move { handle_add_entry(socket, p2p, d).await }
        },
    );

    let get_clone = p2p.clone();
    socket.on("get_ledger", move |socket: SocketRef| {
        let p2p = get_clone.clone();
        async move {
            let entries = p2p.ledger.get_entries();
            socket.emit("ledger_entries", &json!(entries)).ok();
        }
    });

    let nodes_clone = p2p.clone();
    socket.on("get_known_nodes", move |socket: SocketRef| {
        let p2p = nodes_clone.clone();
        async move {
            let nodes = p2p.ledger.get_known_nodes();
            socket.emit("known_nodes", &json!({ "nodes": nodes })).ok();
        }
    });
}

async fn handle_add_entry(socket: SocketRef, p2p: Arc<P2PManager>, data: JsonValue) {
    match p2p.ledger.add_entry(data) {
        Ok(entry) => {
            p2p.broadcast_entry(entry.clone());
            socket.emit("ledger_entry_added", &json!(entry)).ok();
        }
        Err(e) => {
            socket.emit("error", &json!({ "error": e })).ok();
        }
    }
}

async fn on_p2p_connect(socket: SocketRef, Data(data): Data<JsonValue>, p2p: Arc<P2PManager>) {
    info!(ns = socket.ns(), ?socket.id, "P2P node connected");
    p2p.handle_connection(socket, data);
}

/// ========== Peer-to-peer message router ==========
async fn on_peer_message(
    socket: SocketRef,
    Data(data): Data<JsonValue>,
    p2p: Arc<P2PManager>,
    blobs_client: &MemClient,
) {
    if let Some(msg_type) = data.get("type").and_then(|t| t.as_str()) {
        match msg_type {
            "peer_discovered" => handle_peer_discovered(socket, p2p, &data).await,
            "advertise" => handle_advertise(socket, p2p, &data).await,
            "sync_request" => handle_sync_request(socket, p2p, &data).await,
            "sync_response" => handle_sync_response(socket, p2p, &data).await,
            "fetch_blob" => handle_fetch_blob(socket, p2p, &data, blobs_client).await,
            "entry_announce" => handle_entry_announce(socket, p2p, &data).await,
            "blob_available" => handle_blob_available(socket, p2p, &data).await,
            _ => info!("Unknown peer message type: {msg_type}"),
        }
    }
}

/// ---- Individual peer-message helpers ----
async fn handle_peer_discovered(socket: SocketRef, p2p: Arc<P2PManager>, data: &JsonValue) {
    if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
        p2p.ledger.add_known_node(peer_id.to_owned());
        socket
            .emit(
                "advertise",
                &json!({ "type": "advertise", "peer_id": p2p.node_id() }),
            )
            .ok();
    }
}

async fn handle_advertise(socket: SocketRef, p2p: Arc<P2PManager>, data: &JsonValue) {
    if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
        p2p.ledger.add_known_node(peer_id.to_owned());
        socket
            .emit("peer_ack", &json!({ "type": "ack", "peer_id": p2p.node_id() }))
            .ok();
        socket
            .emit(
                "peer_sync_request",
                &json!({ "type": "sync_request", "peer_id": p2p.node_id() }),
            )
            .ok();
    }
}

async fn handle_sync_request(socket: SocketRef, p2p: Arc<P2PManager>, _data: &JsonValue) {
    let entries = p2p.ledger.get_entries();
    socket
        .emit(
            "peer_sync_response",
            &json!({
                "type": "sync_response",
                "peer_id": p2p.node_id(),
                "entries": entries
            }),
        )
        .ok();
}

async fn handle_sync_response(_socket: SocketRef, p2p: Arc<P2PManager>, data: &JsonValue) {
    if let Some(entries_val) = data.get("entries") {
        if let Ok(entries) = serde_json::from_value::<Vec<LedgerEntry>>(entries_val.clone()) {
            for e in entries {
                p2p.ledger.add_pending_entry(e);
            }
            let added = p2p.ledger.process_pending_entries();
            info!("Added {} entries from peer sync", added.len());
        }
    }
}

async fn handle_fetch_blob(
    socket: SocketRef,
    p2p: Arc<P2PManager>,
    data: &JsonValue,
    _blobs_client: &MemClient,
) {
    if let Some(blob_hash) = data.get("blob_hash").and_then(|h| h.as_str()) {
        let hash_str = blob_hash.to_owned();
        let socket_clone = socket.clone();
        let node_id = p2p.node_id().to_owned();

        tokio::spawn(async move {
            match Hash::from_str(&hash_str) {
                Ok(_hash) => {
                    socket_clone
                        .emit(
                            "blob_fetch_ack",
                            &json!({
                                "type": "blob_fetch_ack",
                                "peer_id": node_id,
                                "blob_hash": hash_str,
                                "status": "success"
                            }),
                        )
                        .ok();
                }
                Err(e) => {
                    socket_clone
                        .emit(
                            "blob_fetch_ack",
                            &json!({
                                "type": "blob_fetch_ack",
                                "peer_id": node_id,
                                "blob_hash": hash_str,
                                "status": "error",
                                "error": format!("Invalid hash: {e}")
                            }),
                        )
                        .ok();
                }
            }
        });
    }
}

async fn handle_entry_announce(socket: SocketRef, p2p: Arc<P2PManager>, data: &JsonValue) {
    if let Some(entry_val) = data.get("entry") {
        if let Ok(entry) = serde_json::from_value::<LedgerEntry>(entry_val.clone()) {
            p2p.ledger.add_pending_entry(entry.clone());
            let added = p2p.ledger.process_pending_entries();
            for e in added {
                p2p.broadcast_entry(e);
            }

            let socket_clone = socket.clone();
            let node_id = p2p.node_id().to_owned();
            let entry_id = entry.id.clone();

            tokio::spawn(async move {
                let hash_str = format!("entry-{entry_id}-hash");
                if Hash::from_str(&hash_str).is_ok() {
                    socket_clone
                        .emit(
                            "blob_available",
                            &json!({
                                "type": "blob_available",
                                "peer_id": node_id,
                                "entry_id": entry_id,
                                "blob_hash": hash_str
                            }),
                        )
                        .ok();
                }
            });
        }
    }
}

async fn handle_blob_available(socket: SocketRef, p2p: Arc<P2PManager>, data: &JsonValue) {
    if let (Some(blob_hash), Some(entry_id)) = (
        data.get("blob_hash").and_then(|h| h.as_str()),
        data.get("entry_id").and_then(|id| id.as_str()),
    ) {
        socket
            .emit(
                "fetch_blob",
                &json!({
                    "type": "fetch_blob",
                    "peer_id": p2p.node_id(),
                    "blob_hash": blob_hash,
                    "entry_id": entry_id
                }),
            )
            .ok();
    }
}

/// ========== Application bootstrap ==========
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;

    // --- IROH SETUP --------------------------------------------------------
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;
    // Concrete store type inferred from the builder
    let blobs = Arc::new(Blobs::memory().build(&endpoint));
    let router = IrohRouter::builder(endpoint.clone())
        .accept(ALPN, blobs.clone())
        .spawn();

    // --- NODE & LEDGER -----------------------------------------------------
    let node_id = Uuid::new_v4();
    info!("Starting node with ID: {node_id}");
    let ledger = SharedLedger::new(node_id.to_string());
    let p2p = Arc::new(P2PManager::new(node_id.to_string(), ledger));

    // --- SOCKET.IO ---------------------------------------------------------
    let (layer, io) = SocketIo::new_layer();
    register_root_namespace(&io, p2p.clone());
    register_p2p_namespace(&io, p2p.clone());
    register_peer_namespace(&io, p2p.clone(), blobs.clone());

    spawn_advertisement_task(io.clone(), node_id.to_string());
    spawn_peer_discovery_task(endpoint, router, io.clone(), node_id.to_string(), blobs);

    // --- HTTP SERVER -------------------------------------------------------
    let app = Router::new()
        .route("/", get(|| async { "GSIO-Net Distributed Ledger Node" }))
        .layer(layer);

    info!("Server listening on 0.0.0.0:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
