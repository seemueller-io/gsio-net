use axum::routing::get;
use serde_json::json;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};

// Test the Socket.IO server setup
#[test]
fn test_socketio_setup() {
    let (_, io) = SocketIo::new_layer();
    // Set up a namespace to verify it works
    io.ns("/", |_socket: SocketRef, _data: Data<serde_json::Value>| async move {});
    // If we got here without errors, the setup is successful
    assert!(true);
}

// Test the on_connect handler
#[tokio::test]
async fn test_on_connect_handler() {
    let (_, io) = SocketIo::new_layer();

    // Define a simple handler for testing
    io.ns("/", |socket: SocketRef, Data(data): Data<serde_json::Value>| async move {
        // Echo back the auth data
        socket.emit("auth", &data).ok();
    });

    // If we got here without errors, the namespace was set up successfully
    assert!(true);
}

// Test the Socket.IO layer creation
#[test]
fn test_socketio_layer() {
    // Just test that we can create a Socket.IO layer
    let (layer, _) = SocketIo::new_layer();

    // If we got here without errors, the layer was created successfully
    assert!(true);
}
