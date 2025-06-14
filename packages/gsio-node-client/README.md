# gsio-node-client

A TypeScript client library for connecting to the GSIO-Net distributed ledger system.

## Overview

The gsio-node-client component is a TypeScript client that:
- Connects to gsio-node servers using Socket.IO
- Interacts with the distributed ledger
- Participates in the peer-to-peer network
- Provides a simple API for adding and retrieving ledger entries

## Features

- **Socket.IO Client**: Connects to gsio-node servers for real-time communication
- **WebSocket Client**: Connects to gsio-relay for peer-to-peer communication
- **Ledger Interaction**: Add entries to and retrieve entries from the distributed ledger
- **Node Discovery**: Discover other nodes in the network
- **Event-Based API**: Simple event-based API for easy integration

## Installation

### Prerequisites

- Node.js (latest LTS version)
- Bun (for running TypeScript code and tests)

### Installing

```bash
# Clone the repository (if you haven't already)
git clone <repository-url>
cd gsio-net

# Install dependencies
cd packages/gsio-node-client
bun install
```

## Usage

### Connecting to a Node

```typescript
import { Manager as SocketIO } from "socket.io-client";

// Create a Socket.IO manager
const socketIO = new SocketIO("http://localhost:3000");

// Connect to the root namespace
const socket = socketIO.socket("/", {
    auth: {
        token: "your-auth-token"
    }
});

// Basic Socket.IO event handlers
socket.on("connect", () => {
    console.log("Connected to GSIO-Net node");
});

socket.on("disconnect", () => {
    console.log("Disconnected from GSIO-Net node");
});
```

### Interacting with the Ledger

```typescript
// Function to add a new entry to the ledger
function addLedgerEntry(data) {
    socket.emit("add_ledger_entry", data);
    console.log("Adding ledger entry:", data);
}

// Function to get all entries in the ledger
function getLedgerEntries() {
    socket.emit("get_ledger");
    console.log("Requesting ledger entries");
}

// Function to get all known nodes in the network
function getKnownNodes() {
    socket.emit("get_known_nodes");
    console.log("Requesting known nodes");
}

// Handle ledger-related events
socket.on("ledger_entry_added", (entry) => {
    console.log("New ledger entry added:", entry);
});

socket.on("ledger_entries", (entries) => {
    console.log("Received ledger entries:", entries);
});

socket.on("known_nodes", (data) => {
    console.log("Known nodes in the network:", data.nodes);
});

// Example usage
addLedgerEntry({ 
    message: "Hello, distributed ledger!", 
    timestamp: new Date().toISOString() 
});
```

### Connecting to the P2P Network

```typescript
import { v4 as uuidv4 } from 'uuid';

// Generate a unique ID for this node
const nodeId = uuidv4();

// Connect to the relay server
const websocket = new WebSocket("wss://localhost:8787");

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
});

websocket.addEventListener("message", (event) => {
    console.log("Message received from relay server");

    try {
        const message = JSON.parse(event.data);
        handleP2PMessage(message);
    } catch (error) {
        console.error("Error parsing message:", error);
    }
});

// Function to send a p2p message
function sendP2PMessage(message) {
    // Add a unique message ID if not provided
    if (!message.message_id) {
        message.message_id = uuidv4();
    }

    websocket.send(JSON.stringify(message));
}

// Function to handle incoming p2p messages
function handleP2PMessage(message) {
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
        // Handle other message types...
    }
}
```

## API Reference

### Socket.IO Events (Client to Server)

| Event | Description | Parameters |
|-------|-------------|------------|
| `add_ledger_entry` | Add a new entry to the ledger | JSON data to store |
| `get_ledger` | Get all entries in the ledger | None |
| `get_known_nodes` | Get all known nodes in the network | None |
| `ping` | Simple ping to check connection | Any data |

### Socket.IO Events (Server to Client)

| Event | Description | Data |
|-------|-------------|------|
| `ledger_entry_added` | Notification of a new ledger entry | The added entry |
| `ledger_entries` | Response with all ledger entries | Array of entries |
| `known_nodes` | Response with all known nodes | Object with nodes array |
| `pong` | Response to ping | Same data as ping |

### P2P Message Types

| Message Type | Description | Payload |
|--------------|-------------|---------|
| `NodeAnnounce` | Announce a new node | `{ node_id: string }` |
| `NodeListRequest` | Request the list of known nodes | `{}` |
| `NodeListResponse` | Response with the list of known nodes | `{ nodes: string[] }` |
| `EntryAnnounce` | Announce a new ledger entry | Ledger entry object |
| `EntryRequest` | Request a specific ledger entry | `{ entry_id: string }` |
| `EntryResponse` | Response with a requested ledger entry | Ledger entry object |
| `LedgerSyncRequest` | Request all ledger entries | `{}` |
| `LedgerSyncResponse` | Response with all ledger entries | Array of ledger entries |

## Examples

The package includes example scripts:

- `node_listener.ts`: Demonstrates connecting to a gsio-node server
- `ws_client.ts`: Demonstrates connecting to the gsio-relay server

To run these examples:

```bash
# Run the Socket.IO client example
bun src/listeners/node_listener.ts

# Run the WebSocket client example
bun src/listeners/ws_client.ts
```

## Testing

```bash
# Run tests
bun test
```

## License

[Add license information here]
