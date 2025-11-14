# Multiplayer File Sync Architecture

## Overview

Three-tier connection strategy with automatic fallback:

```
┌──────────────────────────────────────────────────────────────┐
│  1. DIRECT P2P (Hole Punch)                                 │
│     ├─ STUN/ICE NAT traversal                                │
│     ├─ Direct TCP connection between peers                   │
│     └─ Git native pack protocol                              │
│     ✓ Fastest, lowest latency, no server bandwidth           │
├──────────────────────────────────────────────────────────────┤
│  2. BINARY PROXY (Through WebSocket Server)                 │
│     ├─ WebSocket binary frames                               │
│     ├─ Server tunnels raw git bytes                          │
│     └─ No JSON parsing overhead                              │
│     ✓ Fast, works through firewalls                          │
├──────────────────────────────────────────────────────────────┤
│  3. JSON FALLBACK (Serialized Objects)                      │
│     ├─ WebSocket text frames with JSON                       │
│     ├─ Git objects serialized as JSON                        │
│     └─ Fully parsed by server                                │
│     ✓ Works everywhere, highest overhead                     │
└──────────────────────────────────────────────────────────────┘
```

## Connection Flow

### Phase 1: Negotiation

```
Peer A (Host)                Server                 Peer B (Joiner)
    │                          │                          │
    │  Create Session         │                          │
    ├────────────────────────>│                          │
    │  SessionCreated         │                          │
    │<────────────────────────┤                          │
    │                          │       Join Session       │
    │                          │<─────────────────────────┤
    │                          │  SessionJoined           │
    │                          ├─────────────────────────>│
    │                          │                          │
    │  P2PConnectionRequest    │                          │
    ├────────────────────────>│  P2PConnectionRequest    │
    │                          ├─────────────────────────>│
```

### Phase 2: P2P Attempt (Try #1)

```
Peer A                                              Peer B
    │                                                  │
    │  Send public IP + ICE candidates                │
    ├─────────────────────────────────────────────────>│
    │                                                  │
    │           Try simultaneous TCP opens             │
    │<────────────────────────────────────────────────>│
    │                                                  │
    │  ✓ SUCCESS: Direct TCP connection established   │
    │  Run git fetch/push over native protocol        │
    │<═════════════════════════════════════════════════>│
```

### Phase 3: Binary Proxy (Try #2 if P2P fails)

```
Peer A                  Server                    Peer B
    │                      │                         │
    │  RequestBinaryProxy  │                         │
    ├─────────────────────>│                         │
    │                      │  RequestBinaryProxy     │
    │                      ├────────────────────────>│
    │                      │  BinaryProxyAccepted    │
    │                      │<────────────────────────┤
    │  BinaryProxyAccepted │                         │
    │<─────────────────────┤                         │
    │                      │                         │
    │  ╔═══════════════════╧═══════════════════╗    │
    │  ║  Binary WebSocket Tunnel              ║    │
    │  ║  Server proxies raw bytes             ║    │
    ║  ║  No parsing, just relay               ║    │
    ╚══╬════════════════════╬════════════════════╬═══╝
       │  Git pack data     │  Git pack data     │
       ├───────────────────>├───────────────────>│
       │<───────────────────┤<───────────────────┤
```

### Phase 4: JSON Fallback (Try #3)

```
Peer A                  Server                    Peer B
    │                      │                         │
    │  RequestGitObjects   │                         │
    ├─────────────────────>│  RequestGitObjects      │
    │                      ├────────────────────────>│
    │                      │                         │
    │                      │  Serialize git objects  │
    │                      │  (commit, trees, blobs) │
    │                      │                         │
    │                      │  GitObjectsChunk (JSON) │
    │  GitObjectsChunk     │<────────────────────────┤
    │<─────────────────────┤                         │
    │                      │                         │
    │  Reconstruct in      │                         │
    │  local git repo      │                         │
```

## Implementation Status

### ✅ Completed

- Git sync module (`git_sync.rs`)
  - Repo initialization
  - Commit creation
  - Object serialization for JSON fallback
- P2P connection manager skeleton (`p2p_connection.rs`)
- Protocol messages for all three modes
- Multiplayer window integration

### 🚧 Needs Implementation

#### Client-Side (Pulsar Engine)

1. **STUN/ICE Implementation**
   - Add `webrtc-rs` or `libnice` dependency
   - Query STUN server for public IP
   - Exchange ICE candidates between peers
   - Implement simultaneous TCP open for hole punch

2. **Binary WebSocket Handling**
   - Update `MultiuserClient` to handle binary frames
   - Add binary send/receive methods
   - Maintain sequence numbers for in-order delivery

3. **Git Protocol Transport**
   - Implement custom git transport using libgit2 callbacks
   - Route git pack protocol over P2PManager connection
   - Handle fetch/push operations

#### Server-Side (multiuser_server)

1. **Binary Proxy Mode**
   ```rust
   // In multiuser_server/src/main.rs or session.rs

   match message {
       Message::Text(text) => {
           // Existing JSON message handling
       }
       Message::Binary(data) => {
           // NEW: Binary proxy mode
           // Extract session_id + peer_id from first bytes
           // Relay to target peer without parsing
           relay_binary_to_peer(session_id, peer_id, data).await;
       }
   }
   ```

2. **P2P Connection Relay**
   - Relay P2PConnectionRequest/Response between peers
   - Don't need to understand ICE - just relay JSON

3. **Performance Monitoring**
   - Track bytes transferred per mode
   - Log which mode each session uses
   - Detect and handle connection upgrades/downgrades

## Message Protocol

### P2P Negotiation

```json
// ClientMessage
{
  "type": "p2p_connection_request",
  "session_id": "abc123",
  "peer_id": "peer-456",
  "public_ip": "203.0.113.5",
  "public_port": 54321
}

// ServerMessage (relayed)
{
  "type": "p2p_connection_response",
  "session_id": "abc123",
  "from_peer_id": "peer-789",
  "public_ip": "198.51.100.10",
  "public_port": 12345
}
```

### Binary Proxy

```json
// Request to enter binary mode
{
  "type": "request_binary_proxy",
  "session_id": "abc123",
  "peer_id": "peer-456"
}

// After this, switch to WebSocket binary frames
// Format: [session_id_len:u8][session_id][peer_id_len:u8][peer_id][sequence:u64][data...]
```

### JSON Fallback (Current)

```json
{
  "type": "request_git_objects",
  "session_id": "abc123",
  "peer_id": "peer-456",
  "commit_hash": "a1b2c3d4..."
}
```

## Performance Comparison

| Mode          | Latency | Bandwidth (Server) | Throughput | Use Case |
|---------------|---------|-------------------|------------|----------|
| Direct P2P    | ~10ms   | 0 bytes/s         | ~100 MB/s  | Same region |
| Binary Proxy  | ~50ms   | High (relayed)    | ~50 MB/s   | Firewalled |
| JSON Fallback | ~100ms  | Very High         | ~10 MB/s   | Last resort |

*Estimated values for 100MB project sync*

## Testing Checklist

- [ ] P2P connection between peers on same LAN
- [ ] P2P connection with symmetric NAT (should fail gracefully)
- [ ] Binary proxy through server
- [ ] JSON fallback mode
- [ ] Automatic fallback when P2P fails
- [ ] Large project sync (>100MB)
- [ ] Concurrent file editing detection
- [ ] Connection recovery after network drop

## Future Enhancements

1. **TURN Server Support** - Relay when hole punch fails
2. **Connection Upgrade** - Start with proxy, upgrade to P2P when possible
3. **Bandwidth Throttling** - Limit server bandwidth usage
4. **Compression** - Gzip for JSON fallback, native for git protocol
5. **Incremental Sync** - Only transfer changed objects, not full tree
