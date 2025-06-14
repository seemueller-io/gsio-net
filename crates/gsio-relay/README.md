# gsio-relay

A Rust-based WebSocket server implemented as a Cloudflare Worker that serves as a relay in the GSIO-Net distributed ledger system.

## Overview

The gsio-relay component is a WebSocket server that:
- Accepts WebSocket connections from gsio-node instances
- Relays messages between nodes to enable peer-to-peer communication
- Runs as a Cloudflare Worker for global distribution and reliability
- Facilitates the synchronization of distributed ledgers across nodes

## Features

- **WebSocket Server**: Provides real-time bidirectional communication
- **Cloudflare Worker**: Runs on Cloudflare's edge network for low latency
- **Message Relay**: Forwards messages between nodes in the network
- **Lightweight**: Minimal implementation focused on efficient message passing
- **Scalable**: Can handle many concurrent connections

## Installation

### Prerequisites

- Rust (latest stable version)
- Wrangler CLI (for Cloudflare Workers development)
- Node.js (for running Wrangler)

### Building

```bash
# Clone the repository (if you haven't already)
git clone <repository-url>
cd gsio-net

# Install Wrangler CLI if you haven't already
npm install -g wrangler

# Build the gsio-relay component
cd crates/gsio-relay
cargo install -q worker-build && worker-build --release
```

## Usage

### Running Locally

```bash
# Run the worker locally
wrangler dev
```

The WebSocket server will start on port 8787 by default.

### Deploying to Cloudflare

```bash
# Deploy to Cloudflare
wrangler publish
```

### Configuration

The worker can be configured using the `wrangler.toml` file:

```toml
name = "gsio-relay"
type = "javascript"
account_id = "<your-account-id>"
workers_dev = true
compatibility_date = "2023-01-01"

[build]
command = "cargo install -q worker-build && worker-build --release"

[build.upload]
format = "modules"
main = "./worker/worker.mjs"
```

## WebSocket Protocol

The gsio-relay server accepts WebSocket connections and relays messages between connected clients. The protocol is simple:

1. Connect to the WebSocket server
2. Send messages as text
3. Receive echoed messages from the server

In the GSIO-Net system, nodes use this relay to exchange P2P messages for ledger synchronization and node discovery.

## Examples

### Connecting to the Relay

```javascript
// Using browser WebSocket API
const websocket = new WebSocket("wss://gsio-relay.your-worker.workers.dev");

websocket.addEventListener("open", () => {
  console.log("Connected to relay server");

  // Send a message
  websocket.send(JSON.stringify({
    message_type: "NodeAnnounce",
    sender_id: "node-id",
    recipient_id: "",
    payload: { node_id: "node-id" }
  }));
});

websocket.addEventListener("message", (event) => {
  console.log("Message received:", event.data);
});
```

## Architecture

The gsio-relay component is a simple WebSocket server that:

1. Accepts incoming WebSocket connections
2. Receives messages from connected clients
3. Echoes messages back to the sender (in the current implementation)
4. In a more advanced implementation, it would relay messages to the appropriate recipients

## Testing

```bash
# Run tests
wrangler dev --test
```

## License

[Add license information here]
