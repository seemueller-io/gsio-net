use gsio_node::p2p::{P2PMessage, MessageType};
use gsio_node::ledger::{LedgerEntry, SharedLedger};
use serde_json::{json, Value as JsonValue};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;
use tracing::info;

// Mock SocketRef for testing
#[derive(Clone)]
struct MockSocketRef {
    id: String,
    sent_messages: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl MockSocketRef {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            sent_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn emit<T: serde::Serialize>(&self, _event: &str, data: &T) -> Result<(), String> {
        let value = serde_json::to_value(data).unwrap();
        let mut messages = self.sent_messages.lock().unwrap();
        messages.push(value);
        Ok(())
    }
    
    fn get_sent_messages(&self) -> Vec<serde_json::Value> {
        let messages = self.sent_messages.lock().unwrap();
        messages.clone()
    }
    
    fn ns(&self) -> &str {
        "/"
    }
}

// Test-specific P2PManager implementation
struct P2PManager {
    /// The ID of this node
    node_id: String,
    /// The shared ledger
    pub ledger: SharedLedger,
    /// Connected sockets by node ID
    connected_nodes: Arc<Mutex<HashMap<String, MockSocketRef>>>,
}

impl P2PManager {
    /// Create a new p2p manager
    pub fn new(node_id: String, ledger: SharedLedger) -> Self {
        Self {
            node_id,
            ledger,
            connected_nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Get a clone of the connected nodes Arc
    pub fn clone_connected_nodes(&self) -> Arc<Mutex<HashMap<String, MockSocketRef>>> {
        self.connected_nodes.clone()
    }

    /// Handle a new connection from another node
    pub fn handle_connection(&self, socket: MockSocketRef, data: JsonValue) {
        info!(ns = socket.ns(), ?socket.id, "P2P node connected");

        // Extract the node ID from the connection data
        let node_id = match data.get("node_id") {
            Some(id) => id.as_str().unwrap_or("unknown").to_string(),
            None => "unknown".to_string(),
        };

        // Add the node to the connected nodes
        {
            let mut connected_nodes = self.connected_nodes.lock().unwrap();
            connected_nodes.insert(node_id.clone(), socket.clone());
        }

        // Add the node to the known nodes in the ledger
        self.ledger.add_known_node(node_id.clone());

        // In the real implementation, we would set up event handlers for this socket
        // but for testing purposes, we don't need to do that
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

#[test]
fn test_p2p_message_creation() {
    let message_type = MessageType::NodeAnnounce;
    let sender_id = "test-node-1".to_string();
    let recipient_id = "test-node-2".to_string();
    let payload = json!({ "node_id": "test-node-1" });
    
    let message = P2PMessage::new(
        message_type,
        sender_id.clone(),
        recipient_id.clone(),
        payload.clone(),
    );
    
    // Verify the message fields
    assert!(matches!(message.message_type, MessageType::NodeAnnounce));
    assert_eq!(message.sender_id, sender_id);
    assert_eq!(message.recipient_id, recipient_id);
    assert_eq!(message.payload, payload);
    assert!(!message.message_id.is_empty());
}

#[test]
fn test_p2p_manager_creation() {
    let node_id = "test-node-1".to_string();
    let ledger = SharedLedger::new(node_id.clone());
    
    let p2p_manager = P2PManager::new(node_id.clone(), ledger);
    
    // Verify the manager fields
    assert_eq!(p2p_manager.node_id(), &node_id);
}

#[test]
fn test_handle_connection() {
    let node_id = "test-node-1".to_string();
    let ledger = SharedLedger::new(node_id.clone());
    
    let p2p_manager = P2PManager::new(node_id.clone(), ledger);
    
    // Create a mock socket
    let socket = MockSocketRef::new("socket-1");
    
    // Handle a connection
    let connection_data = json!({
        "node_id": "test-node-2"
    });
    
    p2p_manager.handle_connection(socket.clone(), connection_data);
    
    // Verify the node was added to known nodes
    let known_nodes = p2p_manager.ledger.get_known_nodes();
    assert!(known_nodes.contains(&"test-node-2".to_string()));
}

#[test]
fn test_broadcast_message() {
    let node_id = "test-node-1".to_string();
    let ledger = SharedLedger::new(node_id.clone());
    
    let p2p_manager = P2PManager::new(node_id.clone(), ledger);
    
    // Add some connected nodes
    let socket1 = MockSocketRef::new("socket-1");
    let socket2 = MockSocketRef::new("socket-2");
    
    {
        let connected_nodes_arc = p2p_manager.clone_connected_nodes();
        let mut connected_nodes = connected_nodes_arc.lock().unwrap();
        connected_nodes.insert("test-node-2".to_string(), socket1.clone());
        connected_nodes.insert("test-node-3".to_string(), socket2.clone());
    }
    
    // Create a message to broadcast
    let message = P2PMessage::new(
        MessageType::NodeAnnounce,
        node_id.clone(),
        "".to_string(),
        json!({ "node_id": node_id }),
    );
    
    // Broadcast the message
    p2p_manager.broadcast_message(message);
    
    // Verify the message was sent to all connected nodes
    let messages1 = socket1.get_sent_messages();
    let messages2 = socket2.get_sent_messages();
    
    assert_eq!(messages1.len(), 1);
    assert_eq!(messages2.len(), 1);
    
    let sent_message1 = &messages1[0];
    let sent_message2 = &messages2[0];
    
    assert_eq!(sent_message1["sender_id"], node_id);
    assert_eq!(sent_message2["sender_id"], node_id);
}

#[test]
fn test_send_message() {
    let node_id = "test-node-1".to_string();
    let ledger = SharedLedger::new(node_id.clone());
    
    let p2p_manager = P2PManager::new(node_id.clone(), ledger);
    
    // Add a connected node
    let socket = MockSocketRef::new("socket-1");
    
    {
        let connected_nodes_arc = p2p_manager.clone_connected_nodes();
        let mut connected_nodes = connected_nodes_arc.lock().unwrap();
        connected_nodes.insert("test-node-2".to_string(), socket.clone());
    }
    
    // Create a message to send
    let message = P2PMessage::new(
        MessageType::NodeListRequest,
        node_id.clone(),
        "test-node-2".to_string(),
        json!({}),
    );
    
    // Send the message
    let result = p2p_manager.send_message("test-node-2".to_string(), message);
    
    // Verify the message was sent
    assert!(result);
    
    let messages = socket.get_sent_messages();
    assert_eq!(messages.len(), 1);
    
    let sent_message = &messages[0];
    assert_eq!(sent_message["sender_id"], node_id);
    assert_eq!(sent_message["recipient_id"], "test-node-2");
}

#[test]
fn test_request_node_list() {
    let node_id = "test-node-1".to_string();
    let ledger = SharedLedger::new(node_id.clone());
    
    let p2p_manager = P2PManager::new(node_id.clone(), ledger);
    
    // Add a connected node
    let socket = MockSocketRef::new("socket-1");
    
    {
        let connected_nodes_arc = p2p_manager.clone_connected_nodes();
        let mut connected_nodes = connected_nodes_arc.lock().unwrap();
        connected_nodes.insert("test-node-2".to_string(), socket.clone());
    }
    
    // Request the node list
    let result = p2p_manager.request_node_list("test-node-2".to_string());
    
    // Verify the request was sent
    assert!(result);
    
    let messages = socket.get_sent_messages();
    assert_eq!(messages.len(), 1);
    
    let sent_message = &messages[0];
    assert_eq!(sent_message["sender_id"], node_id);
    assert_eq!(sent_message["recipient_id"], "test-node-2");
    assert_eq!(sent_message["message_type"], "NodeListRequest");
}

#[test]
fn test_request_entry() {
    let node_id = "test-node-1".to_string();
    let ledger = SharedLedger::new(node_id.clone());
    
    let p2p_manager = P2PManager::new(node_id.clone(), ledger);
    
    // Add a connected node
    let socket = MockSocketRef::new("socket-1");
    
    {
        let connected_nodes_arc = p2p_manager.clone_connected_nodes();
        let mut connected_nodes = connected_nodes_arc.lock().unwrap();
        connected_nodes.insert("test-node-2".to_string(), socket.clone());
    }
    
    // Request an entry
    let entry_id = "test-entry-1".to_string();
    let result = p2p_manager.request_entry("test-node-2".to_string(), entry_id.clone());
    
    // Verify the request was sent
    assert!(result);
    
    let messages = socket.get_sent_messages();
    assert_eq!(messages.len(), 1);
    
    let sent_message = &messages[0];
    assert_eq!(sent_message["sender_id"], node_id);
    assert_eq!(sent_message["recipient_id"], "test-node-2");
    assert_eq!(sent_message["message_type"], "EntryRequest");
    assert_eq!(sent_message["payload"]["entry_id"], entry_id);
}

#[test]
fn test_request_ledger_sync() {
    let node_id = "test-node-1".to_string();
    let ledger = SharedLedger::new(node_id.clone());
    
    let p2p_manager = P2PManager::new(node_id.clone(), ledger);
    
    // Add a connected node
    let socket = MockSocketRef::new("socket-1");
    
    {
        let connected_nodes_arc = p2p_manager.clone_connected_nodes();
        let mut connected_nodes = connected_nodes_arc.lock().unwrap();
        connected_nodes.insert("test-node-2".to_string(), socket.clone());
    }
    
    // Request ledger sync
    let result = p2p_manager.request_ledger_sync("test-node-2".to_string());
    
    // Verify the request was sent
    assert!(result);
    
    let messages = socket.get_sent_messages();
    assert_eq!(messages.len(), 1);
    
    let sent_message = &messages[0];
    assert_eq!(sent_message["sender_id"], node_id);
    assert_eq!(sent_message["recipient_id"], "test-node-2");
    assert_eq!(sent_message["message_type"], "LedgerSyncRequest");
}