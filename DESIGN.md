# Design Document: mdns-reflector

## Overview

mdns-reflector is a secure, high-performance multicast DNS reflector written in Rust. It enables service discovery across network segments (VLANs) by reflecting mDNS traffic between multiple network interfaces while providing fine-grained access control and security features.

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                           │
│  (main.rs, cli.rs)                                         │
│  - Argument parsing                                         │
│  - Configuration loading                                    │
│  - Command routing                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Configuration Layer                      │
│  (config.rs)                                               │
│  - TOML parsing                                            │
│  - Validation                                              │
│  - Settings management                                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Reflector Engine                         │
│  (reflector.rs)                                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │   Packet    │  │   Service   │  │    Rate     │       │
│  │   Cache     │  │   Cache     │  │   Limiter   │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
│                                                             │
│  - Packet reflection logic                                 │
│  - Filter application                                      │
│  - Loop prevention                                         │
│  - Zone enforcement                                        │
└─────────────────────────────────────────────────────────────┘
          │                    │                    │
          ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   Network       │  │   mDNS          │  │   Network       │
│   Layer         │  │   Protocol      │  │   Layer         │
│ (network.rs)    │  │ (mdns.rs)       │  │ (network.rs)    │
│                 │  │                 │  │                 │
│ - Socket mgmt   │  │ - Parsing       │  │ - Socket mgmt   │
│ - Multicast     │  │ - Serialization │  │ - Multicast     │
│ - IPv4/IPv6     │  │ - TTL rewrite   │  │ - IPv4/IPv6     │
└─────────────────┘  └─────────────────┘  └─────────────────┘
          │                                          │
          ▼                                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Operating System                         │
│  - Network interfaces                                       │
│  - UDP sockets                                              │
│  - Multicast groups                                         │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Configuration System

**File**: `src/config.rs`

The configuration system provides:
- TOML-based configuration file support
- Command-line configuration via arguments
- Validation of settings before use
- Support for multiple configuration scenarios

**Key Types**:
- `Config`: Main configuration container
- `Zone`: Interface grouping with access control
- `DeviceConfig`: MAC-based device filtering
- `ServiceFilter`: Service type filtering rules
- `Settings`: General reflector settings

### 2. mDNS Protocol Handler

**File**: `src/mdns.rs`

Handles mDNS protocol specifics:
- Packet parsing using trust-dns-proto
- Service type extraction (_airplay._tcp, etc.)
- TTL rewriting for reflected packets
- Unicast response detection
- Service information parsing

**Key Types**:
- `MdnsPacket`: Wrapper around parsed mDNS message
- `ServiceInfo`: Extracted service information

**Protocol Details**:
- IPv4: Multicast to 224.0.0.251:5353
- IPv6: Multicast to ff02::fb:5353
- Standard UDP port: 5353

### 3. Network Interface Management

**File**: `src/network.rs`

Manages network interfaces and sockets:
- Interface enumeration and information
- Multicast socket creation and configuration
- IPv4 and IPv6 socket support
- Multicast group membership
- Zero-copy packet reception (optimized)

**Key Types**:
- `InterfaceInfo`: Network interface metadata
- `MdnsSocket`: Multicast socket wrapper

**Socket Configuration**:
- Reuses addresses and ports (SO_REUSEADDR, SO_REUSEPORT)
- Joins multicast groups per interface
- Non-blocking I/O for async compatibility
- Multicast loopback enabled

### 4. Reflection Engine

**File**: `src/reflector.rs`

Core reflection logic:
- Spawns receiver tasks per interface
- Applies filters and access controls
- Enforces rate limits
- Prevents packet loops
- Manages service cache
- Implements zone-based restrictions

**Key Types**:
- `Reflector`: Main engine coordinator
- `PacketCache`: Loop prevention cache
- `ServiceCache`: Discovered services
- `RateLimiter`: Packet rate control

## Data Flow

### Packet Reception Flow

```
Network Interface
      │
      ▼
UDP Socket (multicast)
      │
      ▼
Receiver Task
      │
      ▼
Parse mDNS Packet
      │
      ▼
Rate Limit Check ───► [Drop if exceeded]
      │
      ▼
Loop Detection ─────► [Drop if duplicate]
      │
      ▼
Service Filtering ──► [Drop if denied]
      │
      ▼
Zone Check ─────────► [Drop if not allowed]
      │
      ▼
TTL Rewrite
      │
      ▼
Reflect to Other Interfaces
      │
      ▼
Send to Multicast Group
```

### Configuration Loading Flow

```
CLI Arguments / Config File
      │
      ▼
Parse TOML / Arguments
      │
      ▼
Create Config Object
      │
      ▼
Validate Configuration
      │
      ├─► [Invalid] ──► Error
      │
      ▼
Apply CLI Overrides
      │
      ▼
Create Reflector
      │
      ▼
Initialize Sockets
      │
      ▼
Start Receiver Tasks
```

## Security Features

### 1. Memory Safety
- Written in Rust for guaranteed memory safety
- No buffer overflows or use-after-free bugs
- Safe concurrent access to shared state

### 2. Input Validation
- All network input validated before processing
- Configuration validated before use
- MAC addresses checked for valid format

### 3. Access Control
- Zone-based interface restrictions
- Service type filtering (allow/deny)
- MAC-based device filtering
- Rate limiting to prevent DOS

### 4. Loop Prevention
- Packet signature caching
- Time-based cache expiration
- Configurable cache size limits

### 5. Privilege Management
- Can run without root (with proper capabilities)
- Systemd service with security hardening
- Capability bounding set restrictions

## Performance Considerations

### 1. Zero-Copy Operations
- Direct buffer reuse in socket operations
- Avoids unnecessary memory allocations
- Efficient MaybeUninit handling

### 2. Async I/O
- Tokio-based async runtime
- Non-blocking socket operations
- Efficient task scheduling

### 3. Caching
- Packet signature cache for loop detection
- Service discovery cache (optional)
- Time-based cache expiration

### 4. Rate Limiting
- Per-second packet counting
- Efficient token bucket algorithm
- Configurable limits per interface

## Extensibility Points

### 1. Custom Filters
Add new filter types in `config.rs` and implement in `reflector.rs`:
```rust
pub enum CustomFilter {
    IpRange { start: IpAddr, end: IpAddr },
    TimeWindow { start: Time, end: Time },
}
```

### 2. Metrics Collection
Add a metrics module to track:
- Packet counts (received, reflected, dropped)
- Rate limiting events
- Cache hit rates
- Service discoveries

### 3. Additional Protocols
Extend to support related protocols:
- SSDP (Simple Service Discovery Protocol)
- LLMNR (Link-Local Multicast Name Resolution)

### 4. Dynamic Configuration
Implement configuration hot-reload:
- Watch config file for changes
- Reload and apply new settings
- Graceful failover on errors

## Trade-offs and Decisions

### 1. Sync vs Async Sockets
**Decision**: Use socket2 for socket creation, plan for async I/O
**Rationale**: 
- socket2 provides low-level control needed for multicast
- Async I/O planned for packet processing
- Current implementation has placeholder for async receiver

**Trade-off**: Additional complexity in socket conversion

### 2. IP Layer vs Ethernet Layer
**Decision**: Operate at IP layer (UDP sockets)
**Rationale**:
- Simpler implementation
- Better portability across platforms
- No need for raw socket privileges initially

**Trade-off**: Cannot directly extract MAC addresses from packets

### 3. Service Caching
**Decision**: Optional service caching with TTL
**Rationale**:
- Improves performance for repeated queries
- Reduces network traffic
- Matches mDNS protocol TTL semantics

**Trade-off**: Additional memory usage, cache invalidation complexity

### 4. Configuration Format
**Decision**: TOML for configuration files
**Rationale**:
- Human-readable and editable
- Good Rust library support (serde)
- Structured data support

**Trade-off**: More complex than simple key=value format

## Future Enhancements

1. **Complete Async Implementation**: Finish the async packet reception loop
2. **MAC Address Lookup**: Implement ARP table lookup for MAC filtering
3. **Web Interface**: Optional web UI for monitoring and management
4. **Metrics Export**: Prometheus-compatible metrics endpoint
5. **Advanced Filtering**: More sophisticated filter expressions
6. **Service Discovery API**: Query cached services via API
7. **eBPF Support**: Use eBPF for high-performance packet filtering (Linux)

## Testing Strategy

### Unit Tests
- Configuration parsing and validation
- Packet parsing and serialization
- Rate limiting algorithm
- Loop detection logic

### Integration Tests
- End-to-end packet reflection
- Multi-interface scenarios
- Filter application
- Zone enforcement

### Performance Tests
- Packet throughput under load
- Memory usage profiling
- CPU usage under stress
- Latency measurements

### Security Tests
- Fuzzing of packet parser
- DOS resistance testing
- Invalid input handling
- Privilege escalation attempts

## Deployment Considerations

### System Requirements
- Linux, macOS, or Windows
- Rust 1.70+ for building
- Network interfaces to reflect between
- Optional: systemd for service management

### Privilege Requirements
- Root/admin for binding to port 5353
- Or: `CAP_NET_BIND_SERVICE` capability (Linux)
- Multicast group membership permissions

### Resource Usage
- Memory: Low (< 50MB typical)
- CPU: Low (< 5% typical)
- Network: Depends on mDNS traffic volume

### Monitoring
- Check service status via systemd
- Monitor log output for errors
- Optional: metrics endpoint for monitoring systems

## References

- RFC 6762: Multicast DNS
- RFC 6763: DNS-Based Service Discovery
- [vfreex/mdns-reflector](https://github.com/vfreex/mdns-reflector)
- [Gandem/bonjour-reflector](https://github.com/Gandem/bonjour-reflector)
