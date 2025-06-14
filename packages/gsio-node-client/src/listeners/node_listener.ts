#!/usr/bin/env bun

import { Manager as SocketIO } from "socket.io-client";

// Create a Socket.IO manager
const socketIO = new SocketIO("http://localhost:3000");

// Connect to the root namespace
const socket = socketIO.socket("/", {
    auth: {
        token: "abcd"
    }
});

// Basic Socket.IO event handlers
socket.on("connect", () => {
    console.log("Connected to GSIO-Net node");
});

socket.on("disconnect", () => {
    console.log("Disconnected from GSIO-Net node");
});

socket.on("error", (error) => {
    console.log("Socket error:", error);
});

socket.on("reconnect", (attemptNumber) => {
    console.log("Reconnected after", attemptNumber, "attempts");
});

socket.on("connect_error", (error) => {
    console.log("Connection error:", error);
});

// Ledger-related event handlers
socket.on("ledger_entry_added", (entry) => {
    console.log("New ledger entry added:");
    console.log(entry);
});

socket.on("ledger_entries", (entries) => {
    console.log("Received ledger entries:");
    console.log(entries);
});

socket.on("known_nodes", (data) => {
    console.log("Known nodes in the network:");
    console.log(data.nodes);
});

// Send a ping message every 4 seconds
setInterval(() => {
    socket.emit("ping", "ping message");
    console.log("Ping sent to server");
}, 4000);

// Function to add a new entry to the ledger
function addLedgerEntry(data: any) {
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

// Example usage
setTimeout(() => {
    // Add a sample ledger entry after 2 seconds
    addLedgerEntry({ message: "Hello, distributed ledger!", timestamp: new Date().toISOString() });

    // Get all ledger entries after 3 seconds
    setTimeout(getLedgerEntries, 1000);

    // Get all known nodes after 4 seconds
    setTimeout(getKnownNodes, 2000);
}, 2000);
