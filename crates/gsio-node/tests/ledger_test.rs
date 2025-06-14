use gsio_node::ledger::{LedgerEntry, Ledger, SharedLedger};
use serde_json::json;

#[test]
fn test_ledger_entry_creation() {
    // Create a new ledger entry
    let data = json!({ "message": "Test entry" });
    let previous_hash = "0".repeat(64);
    let creator_node_id = "test-node-1".to_string();

    let entry = LedgerEntry::new(data.clone(), previous_hash.clone(), creator_node_id.clone());

    // Verify the entry fields
    assert_eq!(entry.data, data);
    assert_eq!(entry.previous_hash, previous_hash);
    assert_eq!(entry.creator_node_id, creator_node_id);
    assert!(!entry.hash.is_empty());
    assert!(entry.signatures.is_empty());

    // Verify the entry is valid
    assert!(entry.is_valid());
}

#[test]
fn test_ledger_add_entry() {
    // Create a new ledger
    let node_id = "test-node-1".to_string();
    let mut ledger = Ledger::new(node_id.clone());

    // Add an entry to the ledger
    let data = json!({ "message": "Test entry 1" });
    let result = ledger.add_entry(data.clone());

    // Verify the result
    assert!(result.is_ok());
    let entry = result.unwrap();
    assert_eq!(entry.data, data);
    assert_eq!(entry.creator_node_id, node_id);

    // Verify the ledger state
    let entries = ledger.get_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].data, data);
}

#[test]
fn test_ledger_chain_integrity() {
    // Create a new ledger
    let node_id = "test-node-1".to_string();
    let mut ledger = Ledger::new(node_id.clone());

    // Add multiple entries to the ledger
    let data1 = json!({ "message": "Test entry 1" });
    let data2 = json!({ "message": "Test entry 2" });
    let data3 = json!({ "message": "Test entry 3" });

    let _entry1 = ledger.add_entry(data1.clone()).unwrap();
    let _entry2 = ledger.add_entry(data2.clone()).unwrap();
    let _entry3 = ledger.add_entry(data3.clone()).unwrap();

    // Verify the chain integrity
    let entries = ledger.get_entries();
    assert_eq!(entries.len(), 3);

    // First entry should have a previous hash of all zeros
    assert_eq!(entries[0].previous_hash, "0".repeat(64));

    // Each entry's hash should match the next entry's previous_hash
    assert_eq!(entries[1].previous_hash, entries[0].hash);
    assert_eq!(entries[2].previous_hash, entries[1].hash);

    // All entries should be valid
    for entry in entries {
        assert!(entry.is_valid());
    }
}

#[test]
fn test_shared_ledger() {
    // Create a new shared ledger
    let node_id = "test-node-1".to_string();
    let shared_ledger = SharedLedger::new(node_id.clone());

    // Add entries to the shared ledger
    let data1 = json!({ "message": "Test entry 1" });
    let data2 = json!({ "message": "Test entry 2" });

    let _entry1 = shared_ledger.add_entry(data1.clone()).unwrap();
    let _entry2 = shared_ledger.add_entry(data2.clone()).unwrap();

    // Verify the entries
    let entries = shared_ledger.get_entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].data, data1);
    assert_eq!(entries[1].data, data2);

    // Verify the last entry
    let last_entry = shared_ledger.get_last_entry().unwrap();
    assert_eq!(last_entry.data, data2);
}

#[test]
fn test_pending_entries() {
    // Create a new shared ledger
    let node_id = "test-node-1".to_string();
    let shared_ledger = SharedLedger::new(node_id.clone());

    // Add an entry to the ledger
    let data1 = json!({ "message": "Test entry 1" });
    let entry1 = shared_ledger.add_entry(data1.clone()).unwrap();

    // Create a pending entry that links to the first entry
    let data2 = json!({ "message": "Test entry 2" });
    let entry2 = LedgerEntry::new(data2.clone(), entry1.hash.clone(), "test-node-2".to_string());

    // Add the pending entry
    shared_ledger.add_pending_entry(entry2.clone());

    // Process pending entries
    let added_entries = shared_ledger.process_pending_entries();

    // Verify the pending entry was added
    assert_eq!(added_entries.len(), 1);
    assert_eq!(added_entries[0].data, data2);

    // Verify the ledger state
    let entries = shared_ledger.get_entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].data, data1);
    assert_eq!(entries[1].data, data2);
}

#[test]
fn test_known_nodes() {
    // Create a new shared ledger
    let node_id = "test-node-1".to_string();
    let shared_ledger = SharedLedger::new(node_id.clone());

    // Add known nodes
    shared_ledger.add_known_node("test-node-2".to_string());
    shared_ledger.add_known_node("test-node-3".to_string());
    shared_ledger.add_known_node("test-node-2".to_string()); // Duplicate should be ignored

    // Verify the known nodes
    let known_nodes = shared_ledger.get_known_nodes();
    assert_eq!(known_nodes.len(), 3); // Including the original node
    assert!(known_nodes.contains(&node_id));
    assert!(known_nodes.contains(&"test-node-2".to_string()));
    assert!(known_nodes.contains(&"test-node-3".to_string()));
}
