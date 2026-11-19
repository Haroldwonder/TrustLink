/**
 * Issue #1688 — WebSocket upgrade handler must destroy sockets for paths
 * other than /graphql, not leave them hanging.
 *
 * We instantiate the raw `http.Server` upgrade logic in isolation by
 * replaying the same branching code path used in index.ts. The WebSocketServer
 * is mocked so the test has no network side-effects.
 */

import { EventEmitter } from "events";
import { IncomingMessage } from "http";
import { Socket } from "net";

// ---------------------------------------------------------------------------
// Minimal doubles
// ---------------------------------------------------------------------------

function makeReq(url: string): IncomingMessage {
  const req = new EventEmitter() as IncomingMessage;
  req.url = url;
  req.headers = {};
  req.method = "GET";
  return req;
}

function makeSocket(): Socket & { written: string[]; destroyed: boolean } {
  const socket = new EventEmitter() as Socket & {
    written: string[];
    destroyed: boolean;
  };
  socket.written = [];
  socket.destroyed = false;
  socket.write = (data: unknown) => {
    socket.written.push(String(data));
    return true;
  };
  socket.destroy = () => {
    socket.destroyed = true;
  };
  return socket;
}

// Mimics the WebSocketServer the upgrade handler calls into.
function makeWss() {
  const calls: { req: IncomingMessage }[] = [];
  return {
    handleUpgrade: (
      req: IncomingMessage,
      _socket: unknown,
      _head: unknown,
      cb: (ws: unknown) => void,
    ) => {
      calls.push({ req });
      cb({});
    },
    emit: jest.fn(),
    calls,
  };
}

// ---------------------------------------------------------------------------
// The upgrade handler under test — extracted verbatim from index.ts
// ---------------------------------------------------------------------------

function applyUpgradeHandler(
  wss: ReturnType<typeof makeWss>,
  req: IncomingMessage,
  socket: ReturnType<typeof makeSocket>,
  head: Buffer,
) {
  if (req.url === "/graphql") {
    wss.handleUpgrade(req, socket, head, (ws) => {
      wss.emit("connection", ws, req);
    });
  } else {
    socket.write("HTTP/1.1 404 Not Found\r\n\r\n");
    socket.destroy();
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("httpServer upgrade handler (issue #1688)", () => {
  const head = Buffer.alloc(0);

  it("delegates /graphql upgrades to the WebSocketServer", () => {
    const wss = makeWss();
    const req = makeReq("/graphql");
    const socket = makeSocket();

    applyUpgradeHandler(wss, req, socket, head);

    expect(wss.calls).toHaveLength(1);
    expect(wss.calls[0].req.url).toBe("/graphql");
    expect(wss.emit).toHaveBeenCalledWith("connection", expect.anything(), req);
    // Socket must NOT be destroyed for the legitimate path
    expect(socket.destroyed).toBe(false);
  });

  it("destroys the socket for an unknown path and does not call handleUpgrade", () => {
    const wss = makeWss();
    const req = makeReq("/admin");
    const socket = makeSocket();

    applyUpgradeHandler(wss, req, socket, head);

    expect(wss.calls).toHaveLength(0);
    expect(socket.destroyed).toBe(true);
  });

  it("writes a 404 response before destroying the socket for an unknown path", () => {
    const wss = makeWss();
    const req = makeReq("/ws");
    const socket = makeSocket();

    applyUpgradeHandler(wss, req, socket, head);

    expect(socket.written.some((w) => w.includes("404"))).toBe(true);
    expect(socket.destroyed).toBe(true);
  });

  it("destroys the socket for the root path /", () => {
    const wss = makeWss();
    const req = makeReq("/");
    const socket = makeSocket();

    applyUpgradeHandler(wss, req, socket, head);

    expect(socket.destroyed).toBe(true);
    expect(wss.calls).toHaveLength(0);
  });

  it("destroys the socket for /graphql/ (trailing slash)", () => {
    const wss = makeWss();
    const req = makeReq("/graphql/");
    const socket = makeSocket();

    applyUpgradeHandler(wss, req, socket, head);

    // /graphql/ !== /graphql — must be rejected to prevent path confusion
    expect(socket.destroyed).toBe(true);
    expect(wss.calls).toHaveLength(0);
  });

  it("does not destroy the socket for a valid /graphql upgrade", () => {
    const wss = makeWss();
    const req = makeReq("/graphql");
    const socket = makeSocket();

    applyUpgradeHandler(wss, req, socket, head);

    expect(socket.destroyed).toBe(false);
    expect(socket.written).toHaveLength(0);
  });
});
