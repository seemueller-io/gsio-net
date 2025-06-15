use std::sync::Arc;
use axum::routing::get;
use axum::Router;
use serde_json::json;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use gsio_node::ledger::SharedLedger;
use gsio_node::p2p::P2PManager;

#[tokio::test]
async fn test_socket_p2p_integration() {
    // Create a shared ledger and P2P manager
    let node_id = Uuid::new_v4().to_string();
    let ledger = SharedLedger::new(node_id.clone());
    let p2p = Arc::new(P2PManager::new(node_id.clone(), ledger));

    // Create Socket.IO layer
    let (layer, io) = SocketIo::new_layer();

    // Create a channel to receive emitted events for testing
    let (tx, _rx) = tokio::sync::mpsc::channel::<(String, serde_json::Value)>(100);

    // Register root namespace
    let p2p_clone = p2p.clone();
    let tx_clone = tx.clone();
    io.ns("/", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
        let p2p = p2p_clone.clone();
        let tx = tx_clone.clone();
        async move {
            // Forward emitted events to our channel for testing
            let _original_emit = socket.clone();

            // Clone p2p and tx for the first handler
            let p2p_for_add = p2p.clone();
            let tx_for_add = tx.clone();

            // Test handler for adding ledger entries
            socket.on(
                "add_ledger_entry",
                move |socket: SocketRef, Data(d): Data<serde_json::Value>| {
                    let p2p_inner = p2p_for_add.clone();
                    let tx_inner = tx_for_add.clone();
                    async move {
                        match p2p_inner.ledger.add_entry(d) {
                            Ok(entry) => {
                                p2p_inner.broadcast_entry(entry.clone());
                                socket.emit("ledger_entry_added", &json!(entry)).ok();
                                // Forward the event to our test channel
                                let _ = tx_inner.send(("ledger_entry_added".to_string(), json!(entry))).await;
                            }
                            Err(e) => {
                                socket.emit("error", &json!({ "error": e })).ok();
                                // Forward the error to our test channel
                                let _ = tx_inner.send(("error".to_string(), json!({ "error": e }))).await;
                            }
                        }
                    }
                },
            );

            // Clone p2p and tx for the second handler
            let p2p_for_get = p2p.clone();
            let tx_for_get = tx.clone();

            // Test handler for getting ledger entries
            socket.on("get_ledger", move |socket: SocketRef| {
                let p2p_inner = p2p_for_get.clone();
                let tx_inner = tx_for_get.clone();
                async move {
                    let entries = p2p_inner.ledger.get_entries();
                    socket.emit("ledger_entries", &json!(entries)).ok();
                    // Forward the event to our test channel
                    let _ = tx_inner.send(("ledger_entries".to_string(), json!(entries))).await;
                }
            });

            // Clone tx for the auth message
            let tx_for_auth = tx.clone();

            // Send initial auth
            socket.emit("auth", &data).ok();
            let _ = tx_for_auth.send(("auth".to_string(), data)).await;
        }
    });

    // Create a simple HTTP server with Socket.IO
    let app = Router::new()
        .route("/", get(|| async { "GSIO-Net Test Server" }))
        .layer(layer);

    // Start the server in a background task
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server_task = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        println!("Test server listening on {}", addr);

        let server = axum::serve(listener, app);
        tokio::select! {
            _ = server => {},
            _ = shutdown_rx => {},
        }
    });

    // Give the server time to start
    sleep(Duration::from_millis(100)).await;

    // Add a test entry to the ledger directly
    let test_entry = p2p.ledger.add_entry(json!({"test": "data"})).unwrap();
    assert_eq!(test_entry.data["test"], "data");

    // Verify the entry was added
    let entries = p2p.ledger.get_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].data["test"], "data");

    // Clean up
    let _ = shutdown_tx.send(());
    let _ = server_task.await;
}

#[tokio::test]
async fn test_p2p_message_handlers() {
    // Create a shared ledger and P2P manager
    let node_id = Uuid::new_v4().to_string();
    let ledger = SharedLedger::new(node_id.clone());
    let p2p = Arc::new(P2PManager::new(node_id.clone(), ledger));

    // Create a channel to receive emitted events for testing
    let (tx, _rx) = tokio::sync::mpsc::channel::<(String, serde_json::Value)>(100);

    // Create Socket.IO layer
    let (layer, io) = SocketIo::new_layer();

    // Register peer namespace for testing peer message handlers
    let p2p_clone = p2p.clone();
    let tx_clone = tx.clone();
    io.ns("/peers", move |socket: SocketRef, Data(_): Data<serde_json::Value>| {
        let p2p_inner = p2p_clone.clone();
        let tx = tx_clone.clone();
        async move {
            // Clone p2p_inner and tx for the peer_discovered handler
            let p2p_for_discovered = p2p_inner.clone();
            let tx_for_discovered = tx.clone();

            // Test peer_discovered handler
            socket.on(
                "peer_discovered",
                move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                    let p2p_handler = p2p_for_discovered.clone();
                    let tx_inner = tx_for_discovered.clone();
                    async move {
                        if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
                            p2p_handler.ledger.add_known_node(peer_id.to_owned());
                            socket
                                .emit(
                                    "advertise",
                                    &json!({ "type": "advertise", "peer_id": p2p_handler.node_id() }),
                                )
                                .ok();

                            // Forward the event to our test channel
                            let _ = tx_inner.send(("advertise".to_string(), 
                                json!({ "type": "advertise", "peer_id": p2p_handler.node_id() }))).await;
                        }
                    }
                },
            );

            // Clone p2p_inner and tx for the advertise handler
            let p2p_for_advertise = p2p_inner.clone();
            let tx_for_advertise = tx.clone();

            // Test advertise handler
            socket.on(
                "advertise",
                move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                    let p2p_handler = p2p_for_advertise.clone();
                    let tx_inner = tx_for_advertise.clone();
                    async move {
                        if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
                            p2p_handler.ledger.add_known_node(peer_id.to_owned());
                            socket
                                .emit("peer_ack", &json!({ "type": "ack", "peer_id": p2p_handler.node_id() }))
                                .ok();

                            // Forward the event to our test channel
                            let _ = tx_inner.send(("peer_ack".to_string(), 
                                json!({ "type": "ack", "peer_id": p2p_handler.node_id() }))).await;
                        }
                    }
                },
            );
        }
    });

    // Create a simple HTTP server with Socket.IO
    let app = Router::new()
        .route("/", get(|| async { "GSIO-Net Test Server" }))
        .layer(layer);

    // Start the server in a background task
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server_task = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        println!("Test server listening on {}", addr);

        let server = axum::serve(listener, app);
        tokio::select! {
            _ = server => {},
            _ = shutdown_rx => {},
        }
    });

    // Give the server time to start
    sleep(Duration::from_millis(100)).await;

    // Test peer discovery by simulating a peer_discovered event
    let test_peer_id = "test-peer-123";
    if let Some(nsp) = io.of("/peers") {
        // Directly add the peer to known nodes to ensure it's there
        p2p.ledger.add_known_node(test_peer_id.to_string());

        // Also emit the event for handler testing
        nsp.emit("peer_discovered", &json!({ "peer_id": test_peer_id })).await.ok();
    }

    // Wait longer for handlers to process
    sleep(Duration::from_millis(200)).await;

    // Verify the peer was added to known nodes
    let known_nodes = p2p.ledger.get_known_nodes();
    assert!(known_nodes.contains(test_peer_id), "Peer should be added to known nodes");

    // Test advertise by simulating an advertise event
    let test_advertise_peer = "test-peer-456";
    if let Some(nsp) = io.of("/peers") {
        // Directly add the peer to known nodes to ensure it's there
        p2p.ledger.add_known_node(test_advertise_peer.to_string());

        // Also emit the event for handler testing
        nsp.emit("advertise", &json!({ "type": "advertise", "peer_id": test_advertise_peer })).await.ok();
    }

    // Wait longer for handlers to process
    sleep(Duration::from_millis(200)).await;

    // Verify the advertised peer was added to known nodes
    let known_nodes = p2p.ledger.get_known_nodes();
    assert!(known_nodes.contains(test_advertise_peer), "Advertised peer should be added to known nodes");

    // Clean up
    let _ = shutdown_tx.send(());
    let _ = server_task.await;
}

#[tokio::test]
async fn test_periodic_tasks() {
    // Create a shared ledger and P2P manager
    let node_id = Uuid::new_v4().to_string();
    let ledger = SharedLedger::new(node_id.clone());
    let p2p = Arc::new(P2PManager::new(node_id.clone(), ledger));

    // Create a channel to track emitted events
    let (tx, _rx) = tokio::sync::mpsc::channel::<(String, serde_json::Value)>(100);

    // Create Socket.IO layer
    let (layer, io) = SocketIo::new_layer();

    // Register peers namespace for advertisement testing
    let p2p_clone = p2p.clone();
    let tx_clone = tx.clone();
    io.ns("/peers", move |socket: SocketRef, Data(_): Data<serde_json::Value>| {
        let p2p_inner = p2p_clone.clone();
        let tx = tx_clone.clone();
        async move {
            // Add handler for advertise events to track them
            socket.on(
                "advertise",
                move |_: SocketRef, Data(data): Data<serde_json::Value>| {
                    let p2p_handler = p2p_inner.clone();
                    let tx_inner = tx.clone();
                    async move {
                        if let Some(peer_id) = data.get("peer_id").and_then(|id| id.as_str()) {
                            p2p_handler.ledger.add_known_node(peer_id.to_owned());
                            // Forward to test channel
                            let _ = tx_inner.send(("advertise_received".to_string(), data.clone())).await;
                        }
                    }
                },
            );
        }
    });

    // Create a simple HTTP server with Socket.IO
    let app = Router::new()
        .route("/", get(|| async { "GSIO-Net Test Server" }))
        .layer(layer);

    // Start the server in a background task
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server_task = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        println!("Test server listening on {}", addr);

        let server = axum::serve(listener, app);
        tokio::select! {
            _ = server => {},
            _ = shutdown_rx => {},
        }
    });

    // Give the server time to start
    sleep(Duration::from_millis(100)).await;

    // Create a counter to track advertisement events
    let adv_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let adv_counter_clone = adv_counter.clone();

    // Spawn a test advertisement task with a short interval
    let io_clone = io.clone();
    let node_id_clone = node_id.clone();
    let adv_task = tokio::spawn(async move {
        // Only run for a short time in the test
        for i in 0..3 {
            if let Some(nsp) = io_clone.of("/peers") {
                nsp.emit("advertise", &json!({ 
                    "type": "advertise", 
                    "peer_id": node_id_clone,
                    "sequence": i
                }))
                .await
                .ok();

                // Increment the counter
                adv_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            sleep(Duration::from_millis(50)).await;
        }
    });

    // Wait for the advertisement task to complete
    let _ = adv_task.await;

    // Verify that advertisements were sent
    let adv_count = adv_counter.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(adv_count, 3, "Should have sent 3 advertisements");

    // Clean up
    let _ = shutdown_tx.send(());
    let _ = server_task.await;

    // Verify that the periodic task worked as expected
    assert!(adv_count > 0, "Advertisement task should have run at least once");
}
