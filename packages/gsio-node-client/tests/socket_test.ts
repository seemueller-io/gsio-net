import { expect, test, describe, mock, spyOn } from "bun:test";
import { Manager as SocketIO } from "socket.io-client";

// Mock the socket.io-client
mock.module("socket.io-client", () => {
  const mockEmit = mock(() => {});
  const mockOn = mock((event, callback) => {
    // Simulate the "connect" event
    if (event === "connect") {
      callback();
    }
    return mockSocket;
  });

  const mockSocket = {
    emit: mockEmit,
    on: mockOn,
    connected: true,
    id: "test-socket-id"
  };

  const mockSocketIO = {
    socket: mock(() => mockSocket)
  };

  return {
    Manager: mock(() => mockSocketIO)
  };
});

describe("Socket.IO Client", () => {
  test("should connect to the server", () => {
    // Import the module that uses socket.io-client
    const socketIO = new SocketIO("http://localhost:3000");
    const socket = socketIO.socket("/");

    // Verify the socket is connected
    expect(socket.connected).toBe(true);
  });

  test("should emit events", () => {
    const socketIO = new SocketIO("http://localhost:3000");
    const socket = socketIO.socket("/");

    // Spy on the emit method
    const emitSpy = spyOn(socket, "emit");

    // Emit an event
    socket.emit("ping", "ping message");

    // Verify the emit method was called with the correct arguments
    expect(emitSpy).toHaveBeenCalledWith("ping", "ping message");
  });

  test("should handle events", () => {
    const socketIO = new SocketIO("http://localhost:3000");
    const socket = socketIO.socket("/");

    // Create a mock callback
    const callback = mock(() => {});

    // Register the callback for an event
    socket.on("message", callback);

    // Verify the on method was called with the correct arguments
    expect(socket.on).toHaveBeenCalledWith("message", callback);
  });
});
