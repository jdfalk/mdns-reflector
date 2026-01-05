# mdns-reflector

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

A secure, high-performance multicast DNS (mDNS) reflector written in Rust. This tool enables service discovery across VLANs and network segments by reflecting mDNS traffic between multiple network interfaces.

## Features

### Core Functionality
- **Multi-interface reflection**: Reflect mDNS traffic between multiple network interfaces
- **IPv4 and IPv6 support**: Full support for both IP protocols (224.0.0.251 and ff02::fb)
- **Zone-based configuration**: Group interfaces into zones with granular control
- **MAC address filtering**: Device-specific rules based on MAC addresses
- **VLAN support**: Origin pool and shared pool configuration for VLAN-based networks

### Security & Performance
- **Memory safe**: Written in Rust for guaranteed memory safety
- **Loop prevention**: Built-in duplicate packet detection
- **Rate limiting**: Configurable packet rate limits to prevent flooding
- **TTL control**: Automatic TTL rewriting for reflected packets
- **Rootless operation**: Can run without root privileges (with proper capabilities)
- **Efficient async I/O**: Built on Tokio for high-performance networking

### Filtering & Control
- **Service type filtering**: Allow or deny specific mDNS service types
- **MAC-based device filtering**: Per-device access control
- **Zone restrictions**: Control which zones can communicate
- **Service caching**: Optional caching for improved performance
- **Unicast query support**: Handles both multicast and unicast mDNS queries

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/jdfalk/mdns-reflector
cd mdns-reflector

# Build and install
cargo build --release
sudo cp target/release/mdns-reflector /usr/local/bin/
```

## Usage

### Basic Usage

Reflect mDNS traffic between two interfaces:

```bash
sudo mdns-reflector -f eth0 -f eth1
```

Or use the short form:

```bash
sudo mdns-reflector -f eth0 eth1
```

### List Available Interfaces

```bash
mdns-reflector list-interfaces
```

### Using a Configuration File

Create a configuration file:

```bash
mdns-reflector gen-config -o /etc/mdns-reflector.toml
```

Edit the configuration file as needed, then run:

```bash
sudo mdns-reflector -c /etc/mdns-reflector.toml
```

### Command Line Options

```
Options:
  -f, --interfaces <INTERFACES>  Network interfaces to reflect between
  -c, --config <FILE>            Configuration file path
  -n, --foreground               Run in foreground (do not daemonize)
  -v, --verbose                  Verbose logging (repeat for more verbosity)
      --no-ipv4                  Disable IPv4
      --no-ipv6                  Disable IPv6
      --max-ttl <SECONDS>        Maximum TTL for reflected packets [default: 255]
      --rate-limit <PPS>         Rate limit in packets per second [default: 0]
      --no-loop-prevention       Disable loop prevention
  -h, --help                     Print help
  -V, --version                  Print version

Commands:
  list-interfaces   List available network interfaces
  gen-config        Generate example configuration file
  validate-config   Validate configuration file
  run               Run the reflector (default)
```

## Configuration

### Example Configuration File

```toml
# Network interfaces to reflect mDNS traffic between
interfaces = ["eth0", "eth1", "wlan0"]

# General settings
[settings]
enable_ipv4 = true
enable_ipv6 = true
max_ttl = 255
rate_limit = 150  # packets per second
loop_prevention = true
enable_cache = true
cache_ttl = 300

# Zone-based configuration
[[zones]]
name = "trusted"
interfaces = ["eth0"]
allowed_zones = ["iot", "guest"]

[[zones]]
name = "iot"
interfaces = ["eth1"]
allowed_zones = ["trusted"]

[[zones]]
name = "guest"
interfaces = ["wlan0"]
allowed_zones = ["trusted"]

# Device-specific rules with MAC filtering
[devices."AA:BB:CC:DD:EE:FF"]
description = "Living Room Chromecast"
origin_pool = 100
shared_pools = [10, 20]
enabled = true

[devices."11:22:33:44:55:66"]
description = "Network Printer"
origin_pool = 10
shared_pools = [10, 100, 200]
enabled = true

# Service filtering
[[service_filters]]
service_type = "_airplay._tcp"
action = "allow"
zones = ["trusted", "iot"]

[[service_filters]]
service_type = "_ipp._tcp"
action = "allow"
zones = ["trusted"]

[[service_filters]]
service_type = "_ssh._tcp"
action = "deny"
zones = []
```

## Use Cases

### Home Network with VLANs

Separate IoT devices into a dedicated VLAN for security while maintaining service discovery:

```bash
# Reflect between main network (eth0) and IoT VLAN (eth0.100)
sudo mdns-reflector -f eth0 -f eth0.100
```

### Multi-Network Service Discovery

Enable discovery of services across different network segments:

- AirPlay speakers across VLANs
- Network printers accessible from multiple subnets
- Chromecast devices discoverable from guest networks (with restrictions)
- HomeKit devices across segmented networks

### Enterprise Networks

Use zone-based configuration to control service discovery between different departments or trust levels while maintaining network segmentation.

## Comparison to Other mDNS Reflectors

mdns-reflector combines the best features from existing tools with additional security and performance benefits:

| Feature | mdns-reflector | vfreex/mdns-reflector | Gandem/bonjour-reflector |
|---------|----------------|----------------------|--------------------------|
| Language | **Rust** | C | Go |
| Memory Safety | ✅ Built-in | ⚠️ Manual | ✅ Built-in |
| IPv4/IPv6 | ✅ Both | ✅ Both | ✅ Both |
| Zone-based | ✅ Yes | ✅ Yes | ❌ No |
| MAC filtering | ✅ Yes | ❌ No | ✅ Yes |
| Service filtering | ✅ Yes | ❌ No | ✅ Limited |
| Rate limiting | ✅ Yes | ❌ No | ❌ No |
| Loop prevention | ✅ Yes | ⚠️ Basic | ⚠️ Basic |
| Configuration file | ✅ TOML | ✅ Custom | ✅ TOML |
| Active maintenance | ✅ Yes | ⚠️ Stale | ⚠️ Stale |

## Security

This project is written in Rust to provide:

- **Memory safety**: No buffer overflows, use-after-free, or data races
- **Type safety**: Strong typing prevents common bugs
- **Thread safety**: Safe concurrent access to shared data
- **Input validation**: All inputs are validated and sanitized
- **Secure defaults**: Safe configuration defaults out of the box

### Running Without Root

While mdns-reflector typically requires elevated privileges to bind to multicast addresses, you can grant specific capabilities:

```bash
sudo setcap cap_net_raw,cap_net_bind_service=+ep /usr/local/bin/mdns-reflector
```

## Systemd Integration

Create `/etc/systemd/system/mdns-reflector.service`:

```ini
[Unit]
Description=mDNS Reflector
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/mdns-reflector -c /etc/mdns-reflector.toml
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable --now mdns-reflector
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

This project builds upon concepts from:
- [vfreex/mdns-reflector](https://github.com/vfreex/mdns-reflector) - Zone-based reflection
- [Gandem/bonjour-reflector](https://github.com/Gandem/bonjour-reflector) - MAC-based filtering

By combining these features with Rust's safety guarantees and modern async I/O, mdns-reflector provides a secure, performant solution for cross-VLAN mDNS service discovery.
