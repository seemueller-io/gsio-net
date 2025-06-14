#!/usr/bin/env bun

import { v4 as uuidv4 } from 'uuid';

// Generate a unique ID for this node
const nodeId = uuidv4();
console.log(`Starting p2p node with ID: ${nodeId}`);

// Connect to the relay server
const websocket = new WebSocket(
    "wss://localhost:8787",
);

// Set up event handlers
websocket.addEventListener("open", () => {
    console.log("Connected to relay server");

    // Announce this node to the network
    sendP2PMessage({
        message_type: "NodeAnnounce",
        sender_id: nodeId,
        recipient_id: "",
        payload: { node_id: nodeId }
    });

    // Request the list of known nodes
    setTimeout(() => {
        sendP2PMessage({
            message_type: "NodeListRequest",
            sender_id: nodeId,
            recipient_id: "",
            payload: {}
        });
    }, 1000);
});

websocket.addEventListener("message", (event) => {
    console.log("Message received from relay server");

    try {
        const message = JSON.parse(event.data);
        handleP2PMessage(message);
    } catch (error) {
        console.error("Error parsing message:", error);
        console.log("Raw message:", event.data);
    }
});

websocket.addEventListener("close", () => {
    console.log("Disconnected from relay server");
});

websocket.addEventListener("error", (error) => {
    console.error("WebSocket error:", error);
});

// Function to send a p2p message
function sendP2PMessage(message: any) {
    // Add a unique message ID if not provided
    if (!message.message_id) {
        message.message_id = uuidv4();
    }

    websocket.send(JSON.stringify(message));
    console.log("Sent p2p message:", message);
}

// Function to handle incoming p2p messages
function handleP2PMessage(message: any) {
    console.log("Handling p2p message:", message);

    switch (message.message_type) {
        case "NodeAnnounce":
            console.log(`Node announced: ${message.payload.node_id}`);
            break;

        case "NodeListResponse":
            console.log("Received node list:", message.payload.nodes);
            break;

        case "EntryAnnounce":
            console.log("New ledger entry announced:", message.payload);
            break;

        case "EntryResponse":
            console.log("Received ledger entry:", message.payload);
            break;

        case "LedgerSyncResponse":
            console.log("Received ledger sync:", message.payload);
            break;

        default:
            console.log("Unknown message type:", message.message_type);
    }
}

// Example: Create and announce a ledger entry after 3 seconds
setTimeout(() => {
    const entry = {
        id: `${nodeId}-${Date.now()}`,
        timestamp: new Date().toISOString(),
        data: { message: "Hello from p2p node!", timestamp: Date.now() },
        previous_hash: "0".repeat(64), // Genesis block has a hash of all zeros
        hash: "", // Will be calculated by the node
        creator_node_id: nodeId,
        signatures: {}
    };

    sendP2PMessage({
        message_type: "EntryAnnounce",
        sender_id: nodeId,
        recipient_id: "",
        payload: entry
    });
}, 3000);
