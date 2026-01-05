# TODO: Implementation Tasks

This document tracks remaining implementation tasks for the mdns-reflector project.

## High Priority

### 1. Complete Packet Reception and Reflection Loop

**Location**: `src/reflector.rs`, `spawn_receiver_task()`

**Current State**: Placeholder implementation that doesn't process packets

**Required Work**:
- Convert `socket2::Socket` to `tokio::net::UdpSocket` for async I/O
- Implement main receive loop:
  ```rust
  loop {
      let mut buf = [0u8; 9000]; // Support jumbo frames
      match socket.recv_from(&mut buf).await {
          Ok((size, src_addr)) => {
              // Parse packet
              // Apply filters
              // Reflect to other interfaces
          }
          Err(e) => { /* Handle error */ }
      }
  }
  ```
- Wire up packet parsing and reflection
- Add proper error handling and recovery

**Why Incomplete**: The socket2 crate provides synchronous sockets, but the reflector uses an async architecture. Need to either:
- Use `tokio::net::UdpSocket` from the start (but need raw socket capabilities)
- Use `tokio::net::UdpSocket::from_std()` to wrap the socket2 socket
- Use async-std or another async runtime compatible with socket2

### 2. MAC Address Extraction and ARP Lookup

**Location**: `src/mdns.rs`, `extract_mac_address()`

**Current State**: Returns `None` - not implemented

**Required Work**:
- Implement ARP table lookup to map IP addresses to MAC addresses
- Options:
  1. Parse `/proc/net/arp` on Linux
  2. Use `netlink` library for real-time ARP queries
  3. Use platform-specific APIs (Windows: `GetIpNetTable2`, macOS: `sysctl`)
- Cache MAC address mappings to avoid repeated lookups
- Handle missing or stale ARP entries gracefully

**Alternative Approach**: 
For production use, consider using raw sockets with packet capture to extract MAC addresses directly from Ethernet frames. This requires root/admin privileges but provides more reliable MAC filtering.

## Medium Priority

### 3. Async Socket Integration

**Location**: `src/network.rs`, `MdnsSocket`

**Current State**: Uses synchronous `socket2::Socket`

**Required Work**:
- Refactor to support async I/O
- Consider adding an `AsyncMdnsSocket` wrapper
- Maintain compatibility with tokio runtime

### 4. Service Cache Implementation

**Location**: `src/reflector.rs`, `ServiceCache`

**Current State**: Basic structure exists but not fully integrated

**Required Work**:
- Implement service discovery caching
- Add cache lookup/update logic
- Implement TTL-based expiration
- Add cache query API for troubleshooting

### 5. Configuration Hot Reload

**Location**: `src/main.rs` and `src/reflector.rs`

**Current State**: Configuration loaded once at startup

**Required Work**:
- Watch configuration file for changes
- Reload and apply new configuration without restart
- Validate configuration before applying
- Handle errors gracefully (keep old config on failure)

### 6. Metrics and Monitoring

**Location**: New module `src/metrics.rs`

**Required Work**:
- Add packet counters (received, reflected, dropped)
- Track rate limiting statistics
- Monitor cache hit rates
- Export metrics via:
  - Prometheus endpoint
  - Log output
  - Optional web dashboard

## Low Priority

### 7. Enhanced Logging

**Location**: Throughout the codebase

**Required Work**:
- Add structured logging with context
- Log important events (reflections, drops, errors)
- Add debug logging for troubleshooting
- Implement log levels per module

### 8. Performance Optimization

**Location**: Various

**Required Work**:
- Profile the application under load
- Optimize hot paths
- Consider zero-copy packet forwarding
- Batch packet processing if beneficial
- Use thread pools for CPU-intensive tasks

### 9. Additional Platform Support

**Location**: `src/network.rs` and platform-specific code

**Required Work**:
- Test on macOS (may need kqueue support)
- Test on Windows (different socket APIs)
- Add BSD support
- Document platform-specific requirements

### 10. Integration Tests

**Location**: `tests/` directory (to be created)

**Required Work**:
- Create integration test suite
- Test actual packet reflection between virtual interfaces
- Test filtering and access control
- Test configuration scenarios
- Add performance benchmarks

## Nice to Have

### 11. Web UI for Management

**Location**: New module or separate binary

**Features**:
- View current configuration
- See active interfaces and their status
- Monitor packet statistics
- View cached services
- Edit configuration via web interface

### 12. VLAN Interface Auto-detection

**Location**: `src/network.rs`

**Features**:
- Automatically detect VLAN interfaces
- Map to configured VLAN pools
- Update interface list on network changes

### 13. DNS-SD Browse Support

**Location**: `src/mdns.rs`

**Features**:
- Actively browse for services
- Provide service directory API
- Support service-specific queries

### 14. Packet Capture Mode

**Location**: New module

**Features**:
- Capture and log mDNS packets for debugging
- Export to pcap format
- Filter captured packets

## Documentation Tasks

- [ ] Add more code examples to README
- [ ] Create architecture documentation
- [ ] Document configuration options in detail
- [ ] Add troubleshooting guide
- [ ] Create deployment guide for different platforms
- [ ] Add FAQ section

## Security Tasks

- [ ] Security audit of all network code
- [ ] Fuzz testing of packet parsing
- [ ] Review for DOS vulnerabilities
- [ ] Add input validation tests
- [ ] Document security considerations
- [ ] Add security.md file

## Notes

The current implementation provides a solid foundation with:
- ✅ Complete configuration parsing
- ✅ Network interface management
- ✅ mDNS protocol handling
- ✅ Zone-based access control
- ✅ Service filtering framework
- ✅ Rate limiting
- ✅ Loop prevention
- ✅ CLI interface
- ✅ Comprehensive documentation

The main gap is the actual packet reception/reflection loop, which was left as a placeholder due to the complexity of integrating async I/O with low-level socket operations.

## Priority for Next Phase

1. **Complete packet reception loop** - This is the core functionality
2. **MAC address extraction** - Needed for MAC-based filtering
3. **Integration testing** - Verify the implementation works end-to-end
4. **Performance testing** - Ensure it can handle expected load

These four tasks would make the reflector fully functional and production-ready.
