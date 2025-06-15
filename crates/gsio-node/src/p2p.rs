use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use socketioxide::extract::{Data, SocketRef};
use tracing::info;
use uuid::Uuid;
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{store::{Store, mem}, net_protocol::Blobs};

use crate::ledger::{LedgerEntry, SharedLedger};

/// Types of messages that can be sent between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Announce a new node joining the network
    NodeAnnounce,
    /// Request the list of known nodes
    NodeListRequest,
    /// Response with the list of known nodes
    NodeListResponse,
    /// Announce a new ledger entry
    EntryAnnounce,
    /// Request a specific ledger entry
    EntryRequest,
    /// Response with a requested ledger entry
    EntryResponse,
    /// Request all ledger entries
    LedgerSyncRequest,
    /// Response with all ledger entries
    LedgerSyncResponse,
}

/// A message sent between nodes in the p2p network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PMessage {
    /// Type of message
    pub message_type: MessageType,
    /// Unique ID for this message
    pub message_id: String,
    /// ID of the node that sent this message
    pub sender_id: String,
    /// ID of the node that should receive this message (empty for broadcast)
    pub recipient_id: String,
    /// The actual message payload
    pub payload: JsonValue,
}

impl P2PMessage {
    /// Create a new p2p message
    pub fn new(
        message_type: MessageType,
        sender_id: String,
        recipient_id: String,
        payload: JsonValue,
    ) -> Self {
        Self {
            message_type,
            message_id: Uuid::new_v4().to_string(),
            sender_id,
            recipient_id,
            payload,
        }
    }
}

/// Manages p2p communication between nodes
pub struct P2PManager {
    /// The ID of this node
    node_id: String,
    /// The shared ledger
    pub ledger: SharedLedger,
    /// Connected sockets by node ID
    connected_nodes: Arc<Mutex<HashMap<String, SocketRef>>>,
    /// Iroh endpoint for peer discovery and communication
    endpoint: Option<Arc<Endpoint>>,
    /// Iroh blobs for data storage and synchronization
    blobs: Option<Arc<Blobs<mem::Store>>>,
    /// Iroh router for handling connections
    router: Option<Arc<Router>>,
}

impl P2PManager {
    /// Create a new p2p manager
    pub fn new(node_id: String, ledger: SharedLedger) -> Self {
        Self {
            node_id,
            ledger,
            connected_nodes: Arc::new(Mutex::new(HashMap::new())),
            endpoint: None,
            blobs: None,
            router: None,
        }
    }

    /// Create a new p2p manager with iroh components
    pub fn new_with_iroh(
        node_id: String,
        ledger: SharedLedger,
        endpoint: Arc<Endpoint>,
        blobs: Arc<Blobs<mem::Store>>,
        router: Arc<Router>,
    ) -> Self {
        Self {
            node_id,
            ledger,
            connected_nodes: Arc::new(Mutex::new(HashMap::new())),
            endpoint: Some(endpoint),
            blobs: Some(blobs),
            router: Some(router),
        }
    }

    /// Get the node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Get a clone of the connected nodes Arc
    pub fn clone_connected_nodes(&self) -> Arc<Mutex<HashMap<String, SocketRef>>> {
        self.connected_nodes.clone()
    }

    /// Handle a new connection from another node
    pub fn handle_connection(&self, socket: SocketRef, data: JsonValue) {
        // Extract the node ID from the connection data
        let node_id = match data.get("node_id") {
            Some(id) => id.as_str().unwrap_or("unknown").to_string(),
            None => "unknown".to_string(),
        };

        info!(ns = socket.ns(), ?socket.id, node_id = node_id, "P2P node connected, establishing peering");

        // Add the node to the connected nodes
        {
            let mut connected_nodes = self.connected_nodes.lock().unwrap();
            connected_nodes.insert(node_id.clone(), socket.clone());
            info!(peer_id = node_id, "Successfully peered with node");
        }

        // Add the node to the known nodes in the ledger
        self.ledger.add_known_node(node_id.clone());

        // Send a node announce message to all other nodes
        self.broadcast_message(P2PMessage::new(
            MessageType::NodeAnnounce,
            self.node_id.clone(),
            "".to_string(),
            json!({ "node_id": node_id }),
        ));

        // Set up event handlers for this socket
        self.setup_socket_handlers(socket);
    }

    /// Set up event handlers for a socket
    fn setup_socket_handlers(&self, socket: SocketRef) {
        let p2p_manager = self.clone();

        // Handle p2p messages
        socket.on("p2p_message", move |socket: SocketRef, Data(data): Data<JsonValue>| {
            let p2p_manager = p2p_manager.clone();
            async move {
                info!(?data, "Received p2p message");

                // Parse the message
                let message: P2PMessage = match serde_json::from_value(data) {
                    Ok(msg) => msg,
                    Err(e) => {
                        info!("Error parsing p2p message: {}", e);
                        return;
                    }
                };

                // Handle the message
                p2p_manager.handle_message(socket, message);
            }
        });
    }

    /// Handle a p2p message
    fn handle_message(&self, socket: SocketRef, message: P2PMessage) {
        match message.message_type {
            MessageType::NodeAnnounce => self.handle_node_announce(message),
            MessageType::NodeListRequest => self.handle_node_list_request(socket, message),
            MessageType::EntryAnnounce => self.handle_entry_announce(message),
            MessageType::EntryRequest => self.handle_entry_request(socket, message),
            MessageType::LedgerSyncRequest => self.handle_ledger_sync_request(socket, message),
            _ => info!("Unhandled message type: {:?}", message.message_type),
        }
    }

    /// Handle a node announce message
    fn handle_node_announce(&self, message: P2PMessage) {
        // Extract the node ID from the message
        let node_id = match message.payload.get("node_id") {
            Some(id) => id.as_str().unwrap_or("unknown").to_string(),
            None => "unknown".to_string(),
        };

        // Add the node to the known nodes in the ledger
        self.ledger.add_known_node(node_id);
    }

    /// Handle a node list request message
    fn handle_node_list_request(&self, socket: SocketRef, message: P2PMessage) {
        // Get the list of known nodes
        let known_nodes = self.ledger.get_known_nodes();

        // Send the response
        let response = P2PMessage::new(
            MessageType::NodeListResponse,
            self.node_id.clone(),
            message.sender_id,
            json!({ "nodes": known_nodes }),
        );

        socket.emit("p2p_message", &serde_json::to_value(response).unwrap()).ok();
    }

    /// Handle an entry announce message
    fn handle_entry_announce(&self, message: P2PMessage) {
        // Extract the entry from the message
        let entry: LedgerEntry = match serde_json::from_value(message.payload.clone()) {
            Ok(entry) => entry,
            Err(e) => {
                info!("Error parsing entry announce: {}", e);
                return;
            }
        };

        // Add the entry to the pending entries
        self.ledger.add_pending_entry(entry);

        // Process pending entries
        let added_entries = self.ledger.process_pending_entries();

        // Announce any new entries that were added
        for entry in added_entries {
            self.broadcast_entry(entry);
        }
    }

    /// Handle an entry request message
    fn handle_entry_request(&self, socket: SocketRef, message: P2PMessage) {
        // Extract the entry ID from the message
        let entry_id = match message.payload.get("entry_id") {
            Some(id) => id.as_str().unwrap_or("").to_string(),
            None => "".to_string(),
        };

        // Find the entry in the ledger
        let entries = self.ledger.get_entries();
        let entry = entries.iter().find(|e| e.id == entry_id);

        // Send the response
        if let Some(entry) = entry {
            let response = P2PMessage::new(
                MessageType::EntryResponse,
                self.node_id.clone(),
                message.sender_id,
                serde_json::to_value(entry).unwrap(),
            );

            socket.emit("p2p_message", &serde_json::to_value(response).unwrap()).ok();
        }
    }

    /// Handle a ledger sync request message
    fn handle_ledger_sync_request(&self, socket: SocketRef, message: P2PMessage) {
        // Get all entries in the ledger
        let entries = self.ledger.get_entries();

        // Send the response
        let response = P2PMessage::new(
            MessageType::LedgerSyncResponse,
            self.node_id.clone(),
            message.sender_id,
            serde_json::to_value(entries).unwrap(),
        );

        socket.emit("p2p_message", &serde_json::to_value(response).unwrap()).ok();
    }

    /// Broadcast a message to all connected nodes
    pub fn broadcast_message(&self, message: P2PMessage) {
        let connected_nodes = self.connected_nodes.lock().unwrap();

        for (_, socket) in connected_nodes.iter() {
            socket.emit("p2p_message", &serde_json::to_value(message.clone()).unwrap()).ok();
        }
    }

    /// Broadcast a new ledger entry to all connected nodes
    pub fn broadcast_entry(&self, entry: LedgerEntry) {
        let message = P2PMessage::new(
            MessageType::EntryAnnounce,
            self.node_id.clone(),
            "".to_string(),
            serde_json::to_value(entry).unwrap(),
        );

        self.broadcast_message(message);
    }

    /// Send a message to a specific node
    pub fn send_message(&self, recipient_id: String, message: P2PMessage) -> bool {
        let connected_nodes = self.connected_nodes.lock().unwrap();

        if let Some(socket) = connected_nodes.get(&recipient_id) {
            socket.emit("p2p_message", &serde_json::to_value(message).unwrap()).is_ok()
        } else {
            false
        }
    }

    /// Request the list of known nodes from a specific node
    pub fn request_node_list(&self, recipient_id: String) -> bool {
        let message = P2PMessage::new(
            MessageType::NodeListRequest,
            self.node_id.clone(),
            recipient_id.clone(),
            json!({}),
        );

        self.send_message(recipient_id, message)
    }

    /// Request a specific ledger entry from a specific node
    pub fn request_entry(&self, recipient_id: String, entry_id: String) -> bool {
        let message = P2PMessage::new(
            MessageType::EntryRequest,
            self.node_id.clone(),
            recipient_id.clone(),
            json!({ "entry_id": entry_id }),
        );

        self.send_message(recipient_id, message)
    }

    /// Request all ledger entries from a specific node
    pub fn request_ledger_sync(&self, recipient_id: String) -> bool {
        let message = P2PMessage::new(
            MessageType::LedgerSyncRequest,
            self.node_id.clone(),
            recipient_id.clone(),
            json!({}),
        );

        self.send_message(recipient_id, message)
    }
}

impl Clone for P2PManager {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id.clone(),
            ledger: self.ledger.clone(),
            connected_nodes: self.connected_nodes.clone(),
            endpoint: self.endpoint.clone(),
            blobs: self.blobs.clone(),
            router: self.router.clone(),
        }
    }
}
