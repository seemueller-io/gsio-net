# gsio-net
[![Tests](https://github.com/seemueller-io/gsio-net/actions/workflows/main.yml/badge.svg)](https://github.com/seemueller-io/gsio-net/actions/workflows/main.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)


gsio-net is a distributed ledger system with three main components:

1. **gsio-node**: A Rust-based Socket.IO server that handles real-time communication
2. **gsio-relay**: A Rust-based WebSocket server implemented as a Cloudflare Worker
3. **gsio-node-client**: A TypeScript client for connecting to the gsio-node server

## Project Overview

`gsio-net` provides a foundation for building distributed ledger applications with real-time communication capabilities. The system uses Socket.IO for client-server communication and WebSockets for peer-to-peer relay.

## Project Status

### Why it exists
- **1 AM couch spark → next-morning code.**
- **Big idea:** a **multipurpose private network**—built only with open-web tech—that gives any group trust (tamper-proof), transparency (everyone can verify), **and** anonymity (no personal data needed).

### What works today
1. **Real-time sync** – add a note on one computer, see it on every other in under a second.
2. **Cryptographic chain** – each note is linked to the last, so history can't be secretly edited.
3. **Auto-peer discovery** – nodes find each other through a tiny Cloudflare Worker relay.
4. **Runs almost anywhere** – thanks to **Socket.IO** (via Rust crate **`socketoxide`**) which swaps between WebSockets, HTTP long-polling, etc., to punch through most firewalls.

### What's next
- **Direct peer-to-peer connections with iroh**
    - When a node or client first connects, it will generate an *iroh dial-out address* (think of it as its "phone number").
    - That address is broadcast over the existing Socket.IO channel.
    - Other peers will then call that number to form a direct, encrypted link—skipping the relay for faster, more private transfers.
- A slick **web UI** so non-coders can join with one click.
- **One-command deploy** scripts so anyone can spin up their own private micro-network.
- A visual **history viewer** that shows who posted what (optionally under nicknames).

### Key Features

- Distributed ledger with cryptographic verification
- Peer-to-peer networking with node discovery and message propagation
- Consensus mechanism for ledger synchronization
- Real-time bidirectional communication using Socket.IO
- WebSocket relay for peer-to-peer messaging
- TypeScript client for easy integration with web applications
- Rust-based servers for high performance and reliability

## Network Architecture

`gsio-net` implements a hybrid network architecture combining client-server and peer-to-peer models:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  gsio-node  │◄────┤  gsio-relay │◄────│  gsio-node  │
│  (Server A) │     │ (WebSocket  │     │  (Server B) │
└──────┬──────┘     │    Relay)   │     └──────┬──────┘
       │            └─────────────┘            │
       │                                        │
       ▼                                        ▼
┌─────────────┐                         ┌─────────────┐
│ gsio-client │                         │ gsio-client │
│  (Client A) │                         │  (Client B) │
└─────────────┘                         └─────────────┘
```

### How the Network Works

1. **Node-Client Communication**:
   - Clients connect to gsio-node servers using Socket.IO
   - Each server maintains its own distributed ledger
   - Clients can add entries to the ledger and retrieve ledger data

2. **Node-Node Communication**:
   - Nodes communicate with each other through the gsio-relay server
   - The relay server acts as a message broker using WebSockets
   - Nodes can discover each other and synchronize their ledgers

3. **Distributed Ledger**:
   - Each ledger entry contains data, timestamps, and cryptographic hashes
   - Entries form a chain where each entry references the previous entry's hash
   - The ledger is synchronized across nodes using the P2P network

4. **Message Flow**:
   - When a client adds an entry to a node's ledger, the node broadcasts it to other nodes
   - Nodes validate incoming entries and add them to their ledgers
   - Nodes can request missing entries from other nodes to maintain synchronization

### Protocol Details

The P2P protocol uses the following message types:

1. **NodeAnnounce**: Announces a new node joining the network. Empty recipient_id indicates a broadcast message.
   ```json
   {
     "message_type": "NodeAnnounce",
     "message_id": "uuid-string",
     "sender_id": "node-id",
     "recipient_id": "",
     "payload": { "node_id": "node-id" }
   }
   ```

2. **NodeListRequest/Response**: Request and response for the list of known nodes
   ```json
   {
     "message_type": "NodeListResponse",
     "message_id": "uuid-string",
     "sender_id": "node-id",
     "recipient_id": "requesting-node-id",
     "payload": { "nodes": ["node-id-1", "node-id-2"] }
   }
   ```

3. **EntryAnnounce**: Announces a new ledger entry. Empty recipient_id indicates a broadcast message.
   ```json
   {
     "message_type": "EntryAnnounce",
     "message_id": "uuid-string",
     "sender_id": "node-id",
     "recipient_id": "",
     "payload": {
       "id": "entry-id",
       "timestamp": "2023-05-20T15:30:45Z",
       "data": { "message": "Example data" },
       "previous_hash": "hash-string",
       "hash": "hash-string",
       "creator_node_id": "node-id",
       "signatures": {}
     }
   }
   ```

4. **LedgerSyncRequest/Response**: Request and response for synchronizing the entire ledger. The payload contains an array of ledger entries.
   ```json
   {
     "message_type": "LedgerSyncResponse",
     "message_id": "uuid-string",
     "sender_id": "node-id",
     "recipient_id": "requesting-node-id",
     "payload": []
   }
   ```

### Security Considerations

1. **Cryptographic Verification**:
   - Each ledger entry is hashed using SHA-256
   - The hash includes the entry's ID, timestamp, data, previous hash, and creator node ID
   - This ensures the integrity of the ledger chain

2. **Chain Integrity**:
   - Each entry references the hash of the previous entry
   - This creates a cryptographic chain that is difficult to tamper with
   - Nodes validate the hash of each entry before adding it to their ledger

3. **Consensus Mechanism**:
   - Entries are ordered by timestamp to ensure deterministic ordering
   - Nodes only accept entries that link to their current last entry
   - This ensures that all nodes converge to the same ledger state

4. **Authentication**:
   - The current implementation uses simple node IDs
   - For production use, implement proper authentication and authorization
   - Consider adding digital signatures for entry validation

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Node.js (latest LTS version)
- Bun (for running TypeScript code and tests)
- Wrangler CLI (for Cloudflare Workers development)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/seemueller-io/gsio-net.git
   cd gsio-net
   ```

2. Install dependencies:
   ```bash
   npm install
   ```
   This will also build the Rust components due to the `postinstall` script.

### Running the Components

1. **gsio-node** (Socket.IO server):
   ```bash
   cd crates/gsio-node
   cargo run
   ```
   The server will start on port 3000. Each node has a unique ID and maintains its own distributed ledger.

2. **gsio-relay** (WebSocket relay):
   ```bash
   cd crates/gsio-relay
   wrangler dev
   ```
   The WebSocket server will start on port 8787. This relay enables p2p communication between nodes.

3. **gsio-node-client** (Socket.IO client):
   ```bash
   cd packages/gsio-node-client
   bun src/listeners/node_listener.ts
   ```
   This will connect to the gsio-node server and interact with the distributed ledger.

4. **gsio-node-client** (WebSocket p2p client):
   ```bash
   cd packages/gsio-node-client
   bun src/listeners/ws_client.ts
   ```
   This will connect to the relay server and participate in the p2p network.

### Using the Distributed Ledger

The Socket.IO client provides the following functions for interacting with the distributed ledger:

- **Adding an entry**:
  ```typescript
  // Add a new entry to the ledger
  socket.emit("add_ledger_entry", { message: "Hello, ledger!", timestamp: new Date().toISOString() });
  ```

- **Getting all entries**:
  ```typescript
  // Request all ledger entries
  socket.emit("get_ledger");

  // Handle the response
  socket.on("ledger_entries", (entries) => {
    console.log("Ledger entries:", entries);
  });
  ```

- **Getting known nodes**:
  ```typescript
  // Request all known nodes
  socket.emit("get_known_nodes");

  // Handle the response
  socket.on("known_nodes", (data) => {
    console.log("Known nodes:", data.nodes);
  });
  ```

### Using the P2P Network

The WebSocket client provides the following functions for participating in the p2p network:

- **Sending a p2p message**:
  ```typescript
  // Send a p2p message
  sendP2PMessage({
    message_type: "NodeAnnounce",
    sender_id: nodeId,
    recipient_id: "",
    payload: { node_id: nodeId }
  });
  ```

- **Handling p2p messages**:
  ```typescript
  // Handle incoming p2p messages
  websocket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    console.log("Received p2p message:", message);
  });
  ```

## Development

### Project Structure

- **crates/**: Contains Rust crates
  - **gsio-node/**: Socket.IO server implementation
    - **src/ledger.rs**: Distributed ledger implementation
    - **src/p2p.rs**: Peer-to-peer networking implementation
  - **gsio-relay/**: WebSocket server implemented as a Cloudflare Worker
- **packages/**: Contains TypeScript packages
  - **gsio-node-client/**: Client for connecting to the gsio-node server
    - **src/listeners/node_listener.ts**: Socket.IO client for ledger operations
    - **src/listeners/ws_client.ts**: WebSocket client for p2p communication
  - **scripts/**: Utility scripts for the project

### Distributed Ledger

The distributed ledger is implemented in `crates/gsio-node/src/ledger.rs` and provides the following features:

- **Ledger Entries**: Each entry contains data, timestamps, and cryptographic hashes
- **Chain Integrity**: Each entry references the hash of the previous entry
- **Validation**: Entries are validated using cryptographic hashes
- **Consensus**: Entries are synchronized across nodes in the network

### Peer-to-Peer Networking

The p2p networking is implemented in `crates/gsio-node/src/p2p.rs` and provides the following features:

- **Node Discovery**: Nodes announce themselves to the network
- **Message Propagation**: Messages are propagated to all connected nodes
- **Connection Management**: Connections are managed using Socket.IO
- **Ledger Synchronization**: Nodes can request and receive ledger entries from other nodes

### Building Individual Components

- **gsio-node**:
  ```bash
  cd crates/gsio-node
  cargo build
  ```

- **gsio-relay**:
  ```bash
  cd crates/gsio-relay
  cargo install -q worker-build && worker-build --release
  ```

- **gsio-node-client**:
  ```bash
  cd packages/gsio-node-client
  bun install
  ```

### Running Tests

1. **Rust Tests**:
   ```bash
   cd crates/gsio-node
   cargo test
   ```

2. **TypeScript Tests**:
   ```bash
   cd packages/gsio-node-client
   bun test
   ```

### Debugging

- **gsio-node**: Use `RUST_LOG=debug cargo run` for verbose logging.
- **gsio-relay**: Use `wrangler dev --verbose` for detailed logs.
- **gsio-node-client**: Add `console.log` statements for debugging.

### Cleaning the Project

To clean the project, run:
```bash
npm run clean
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
