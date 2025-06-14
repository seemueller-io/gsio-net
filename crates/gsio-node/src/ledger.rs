use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::{DateTime, Utc};

/// Represents a single entry in the distributed ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Unique identifier for the entry
    pub id: String,
    /// Timestamp when the entry was created
    pub timestamp: DateTime<Utc>,
    /// The actual data stored in the entry
    pub data: serde_json::Value,
    /// Hash of the previous entry in the chain
    pub previous_hash: String,
    /// Hash of this entry
    pub hash: String,
    /// Node ID that created this entry
    pub creator_node_id: String,
    /// Signatures from nodes that have validated this entry
    pub signatures: HashMap<String, String>,
}

impl LedgerEntry {
    /// Create a new ledger entry
    pub fn new(
        data: serde_json::Value,
        previous_hash: String,
        creator_node_id: String,
    ) -> Self {
        let timestamp = Utc::now();
        let id = format!("{}-{}", creator_node_id, timestamp.timestamp_millis());

        let mut entry = Self {
            id,
            timestamp,
            data,
            previous_hash,
            hash: String::new(),
            creator_node_id,
            signatures: HashMap::new(),
        };

        // Calculate the hash of this entry
        entry.hash = entry.calculate_hash();

        entry
    }

    /// Calculate the hash of this entry
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Hash the entry fields
        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.data.to_string().as_bytes());
        hasher.update(self.previous_hash.as_bytes());
        hasher.update(self.creator_node_id.as_bytes());

        // Convert the hash to a hex string
        format!("{:x}", hasher.finalize())
    }

    /// Add a signature from a node that has validated this entry
    pub fn add_signature(&mut self, node_id: String, signature: String) {
        self.signatures.insert(node_id, signature);
    }

    /// Verify that this entry is valid
    pub fn is_valid(&self) -> bool {
        // Check that the hash is correct
        self.hash == self.calculate_hash()
    }
}

/// The distributed ledger
#[derive(Debug)]
pub struct Ledger {
    /// The chain of entries in the ledger
    entries: Vec<LedgerEntry>,
    /// The ID of this node
    node_id: String,
    /// Pending entries that have been received but not yet added to the chain
    pending_entries: HashMap<String, LedgerEntry>,
    /// Set of node IDs that are known to this node
    known_nodes: HashSet<String>,
}

impl Ledger {
    /// Create a new ledger
    pub fn new(node_id: String) -> Self {
        let mut known_nodes = HashSet::new();
        known_nodes.insert(node_id.clone());

        Self {
            entries: Vec::new(),
            node_id,
            pending_entries: HashMap::new(),
            known_nodes,
        }
    }

    /// Add a new entry to the ledger
    pub fn add_entry(&mut self, data: serde_json::Value) -> Result<LedgerEntry, String> {
        let previous_hash = match self.entries.last() {
            Some(entry) => entry.hash.clone(),
            None => "0".repeat(64), // Genesis block has a hash of all zeros
        };

        let entry = LedgerEntry::new(data, previous_hash, self.node_id.clone());

        // Add the entry to the chain
        self.entries.push(entry.clone());

        Ok(entry)
    }

    /// Get all entries in the ledger
    pub fn get_entries(&self) -> &Vec<LedgerEntry> {
        &self.entries
    }

    /// Get the last entry in the ledger
    pub fn get_last_entry(&self) -> Option<&LedgerEntry> {
        self.entries.last()
    }

    /// Add a pending entry that has been received from another node
    pub fn add_pending_entry(&mut self, entry: LedgerEntry) {
        self.pending_entries.insert(entry.id.clone(), entry);
    }

    /// Process pending entries and add them to the chain if they are valid
    pub fn process_pending_entries(&mut self) -> Vec<LedgerEntry> {
        let mut added_entries = Vec::new();

        // Get the current last entry in the chain
        let last_entry = match self.entries.last() {
            Some(entry) => entry.clone(),
            None => return added_entries,
        };

        // Find pending entries that link to the last entry
        let mut entries_to_process: Vec<LedgerEntry> = self.pending_entries
            .values()
            .filter(|e| e.previous_hash == last_entry.hash)
            .cloned()
            .collect();

        // Sort by timestamp to ensure deterministic ordering
        entries_to_process.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Process each entry
        for entry in entries_to_process {
            if entry.is_valid() {
                // Add the entry to the chain
                self.entries.push(entry.clone());
                // Remove from pending
                self.pending_entries.remove(&entry.id);
                // Add to the list of added entries
                added_entries.push(entry);
            }
        }

        added_entries
    }

    /// Add a known node to the network
    pub fn add_known_node(&mut self, node_id: String) {
        self.known_nodes.insert(node_id);
    }

    /// Get all known nodes in the network
    pub fn get_known_nodes(&self) -> &HashSet<String> {
        &self.known_nodes
    }
}

/// Thread-safe wrapper around the ledger
#[derive(Clone)]
pub struct SharedLedger {
    ledger: Arc<Mutex<Ledger>>,
}

impl SharedLedger {
    /// Create a new shared ledger
    pub fn new(node_id: String) -> Self {
        Self {
            ledger: Arc::new(Mutex::new(Ledger::new(node_id))),
        }
    }

    /// Get a clone of the ledger Arc
    pub fn clone_ledger(&self) -> Arc<Mutex<Ledger>> {
        self.ledger.clone()
    }

    /// Add a new entry to the ledger
    pub fn add_entry(&self, data: serde_json::Value) -> Result<LedgerEntry, String> {
        let mut ledger = self.ledger.lock().unwrap();
        ledger.add_entry(data)
    }

    /// Get all entries in the ledger
    pub fn get_entries(&self) -> Vec<LedgerEntry> {
        let ledger = self.ledger.lock().unwrap();
        ledger.get_entries().clone()
    }

    /// Get the last entry in the ledger
    pub fn get_last_entry(&self) -> Option<LedgerEntry> {
        let ledger = self.ledger.lock().unwrap();
        ledger.get_last_entry().cloned()
    }

    /// Add a pending entry that has been received from another node
    pub fn add_pending_entry(&self, entry: LedgerEntry) {
        let mut ledger = self.ledger.lock().unwrap();
        ledger.add_pending_entry(entry);
    }

    /// Process pending entries and add them to the chain if they are valid
    pub fn process_pending_entries(&self) -> Vec<LedgerEntry> {
        let mut ledger = self.ledger.lock().unwrap();
        ledger.process_pending_entries()
    }

    /// Add a known node to the network
    pub fn add_known_node(&self, node_id: String) {
        let mut ledger = self.ledger.lock().unwrap();
        ledger.add_known_node(node_id);
    }

    /// Get all known nodes in the network
    pub fn get_known_nodes(&self) -> HashSet<String> {
        let ledger = self.ledger.lock().unwrap();
        ledger.get_known_nodes().clone()
    }
}
