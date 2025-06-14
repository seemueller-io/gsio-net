# gsio-node

A Rust-based Socket.IO server that serves as a node in the GSIO-Net distributed ledger system.

## Overview

The gsio-node component is a Socket.IO server that:
- Maintains a distributed ledger with cryptographic verification
- Handles real-time communication with clients
- Participates in a peer-to-peer network with other nodes
- Provides APIs for adding and retrieving ledger entries

## Features

- **Socket.IO Server**: Provides real-time bidirectional communication
- **Distributed Ledger**: Maintains a chain of cryptographically linked entries
- **P2P Networking**: Communicates with other nodes to synchronize the ledger
- **Node Discovery**: Automatically discovers and connects to other nodes
- **Consensus Mechanism**: Ensures all nodes converge to the same ledger state

## Installation

### Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)

### Building

```bash
# Clone the repository (if you haven't already)
git clone <repository-url>
cd gsio-net

# Build the gsio-node component
cd crates/gsio-node
cargo build
```

## Usage

### Running the Server

```bash
# Run in development mode
cargo run

# Run in release mode
cargo run --release
```

The server will start on port 3000 by default.

### API Endpoints

The gsio-node server provides the following Socket.IO events:

#### Client Events (Namespace: "/")

| Event | Description | Parameters | Response Event |
|-------|-------------|------------|----------------|
| `add_ledger_entry` | Add a new entry to the ledger | JSON data to store | `ledger_entry_added` |
| `get_ledger` | Get all entries in the ledger | None | `ledger_entries` |
| `get_known_nodes` | Get all known nodes in the network | None | `known_nodes` |
| `ping` | Simple ping to check connection | Any data | `pong` |
| `message` | Send a message to the server | Any data | `message-back` |
| `message-with-ack` | Send a message with acknowledgement | Any data | Acknowledgement with same data |

#### P2P Events (Namespace: "/p2p")

| Event | Description | Parameters | Response Event |
|-------|-------------|------------|----------------|
| `p2p_message` | Send a message to other nodes | P2P message object | Varies based on message type |

## Examples

### Adding a Ledger Entry

```javascript
// Using Socket.IO client
socket.emit("add_ledger_entry", { 
  message: "Hello, ledger!", 
  timestamp: new Date().toISOString() 
});

// Handle the response
socket.on("ledger_entry_added", (entry) => {
  console.log("Entry added:", entry);
});
```

### Getting Ledger Entries

```javascript
// Using Socket.IO client
socket.emit("get_ledger");

// Handle the response
socket.on("ledger_entries", (entries) => {
  console.log("Ledger entries:", entries);
});
```

### Getting Known Nodes

```javascript
// Using Socket.IO client
socket.emit("get_known_nodes");

// Handle the response
socket.on("known_nodes", (data) => {
  console.log("Known nodes:", data.nodes);
});
```

## Architecture

The gsio-node component consists of the following modules:

- **main.rs**: Entry point and Socket.IO server setup
- **ledger.rs**: Implementation of the distributed ledger
- **p2p.rs**: Implementation of peer-to-peer communication

## Testing

```bash
# Run tests
cargo test
```

## License

[Add license information here]
