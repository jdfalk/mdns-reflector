# Quick Start Guide

Get started with mdns-reflector in minutes!

## Installation

### From Source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/jdfalk/mdns-reflector
cd mdns-reflector

# Build release version
cargo build --release

# Install binary
sudo cp target/release/mdns-reflector /usr/local/bin/
```

## Quick Test

### 1. List Available Interfaces

```bash
mdns-reflector list-interfaces
```

Example output:
```
Available network interfaces:

Interface: eth0
  Index: 2
  IPv4: 192.168.1.10

Interface: eth1
  Index: 3
  IPv4: 192.168.2.10
```

### 2. Test Reflection Between Two Interfaces

```bash
# Basic reflection between eth0 and eth1
sudo mdns-reflector -f eth0 -f eth1
```

This will start the reflector and log its activity.

### 3. Test with Verbose Logging

```bash
# See detailed packet information
sudo mdns-reflector -f eth0 -f eth1 -vv
```

## Configuration

### Generate Example Configuration

```bash
mdns-reflector gen-config -o mdns-reflector.toml
```

### Edit Configuration

Edit `mdns-reflector.toml` to customize:

```toml
# Simple two-interface configuration
interfaces = ["eth0", "eth1"]

[settings]
enable_ipv4 = true
enable_ipv6 = true
max_ttl = 255
rate_limit = 150  # packets per second
loop_prevention = true
```

### Run with Configuration

```bash
sudo mdns-reflector -c mdns-reflector.toml
```

## Common Use Cases

### Home Network with IoT VLAN

Reflect mDNS between main network and IoT devices:

```bash
sudo mdns-reflector -f eth0 -f eth0.100
```

Where `eth0` is your main network and `eth0.100` is VLAN 100 for IoT devices.

### Multi-Network Setup

Create a configuration file for complex setups:

```toml
interfaces = ["eth0", "eth1", "wlan0"]

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
```

This allows:
- Trusted network (eth0) can see IoT and guest devices
- IoT devices (eth1) can only be seen from trusted network
- Guest devices (wlan0) can only be seen from trusted network

## Testing Service Discovery

### From a Device on eth0

```bash
# Discover Chromecast devices
dns-sd -B _googlecast._tcp

# Discover AirPlay devices
dns-sd -B _airplay._tcp

# Discover printers
dns-sd -B _ipp._tcp
```

If the reflector is working, you should see devices from both networks!

## Systemd Service Setup

### 1. Create Configuration

```bash
sudo mkdir -p /etc/mdns-reflector
sudo cp mdns-reflector.toml /etc/mdns-reflector/mdns-reflector.toml
```

### 2. Install Service

```bash
sudo cp mdns-reflector.service /etc/systemd/system/
sudo systemctl daemon-reload
```

### 3. Enable and Start

```bash
sudo systemctl enable mdns-reflector
sudo systemctl start mdns-reflector
```

### 4. Check Status

```bash
sudo systemctl status mdns-reflector
```

### 5. View Logs

```bash
sudo journalctl -u mdns-reflector -f
```

## Troubleshooting

### No Packets Being Reflected

1. **Check interfaces exist:**
   ```bash
   mdns-reflector list-interfaces
   ```

2. **Verify multicast routing:**
   ```bash
   ip mroute show
   ```

3. **Check firewall rules:**
   ```bash
   # Allow mDNS
   sudo iptables -A INPUT -p udp --dport 5353 -j ACCEPT
   sudo iptables -A OUTPUT -p udp --dport 5353 -j ACCEPT
   ```

4. **Run with verbose logging:**
   ```bash
   sudo mdns-reflector -f eth0 -f eth1 -vv
   ```

### Permission Denied

The reflector needs elevated privileges to bind to port 5353 and join multicast groups.

**Option 1: Run as root**
```bash
sudo mdns-reflector -f eth0 -f eth1
```

**Option 2: Grant capabilities** (Linux only)
```bash
sudo setcap cap_net_raw,cap_net_bind_service=+ep /usr/local/bin/mdns-reflector
mdns-reflector -f eth0 -f eth1
```

### Services Not Appearing

1. **Check both interfaces are up:**
   ```bash
   ip link show eth0
   ip link show eth1
   ```

2. **Verify mDNS traffic exists:**
   ```bash
   sudo tcpdump -i eth0 port 5353
   ```

3. **Check zone configuration:**
   - Ensure zones allow communication between interfaces
   - Verify service filters aren't blocking traffic

## Performance Tips

### For High Traffic Networks

```toml
[settings]
rate_limit = 150        # Limit packets per second
cache_ttl = 300         # Cache discovered services
packet_cache_ttl = 2    # Prevent loops
```

### For Low Latency

```toml
[settings]
packet_cache_ttl = 1    # Faster reflection
cache_cleanup_interval = 30  # More frequent cleanup
```

## Next Steps

- Read the [README.md](README.md) for detailed documentation
- Check [DESIGN.md](DESIGN.md) for architecture details
- See [CONTRIBUTING.md](CONTRIBUTING.md) to contribute
- Review [TODO.md](TODO.md) for planned features

## Getting Help

1. Check the logs: `journalctl -u mdns-reflector`
2. Enable verbose mode: `-vv`
3. Verify configuration: `mdns-reflector validate-config config.toml`
4. Open an issue on GitHub

## Common Commands Reference

```bash
# List interfaces
mdns-reflector list-interfaces

# Generate config
mdns-reflector gen-config -o config.toml

# Validate config
mdns-reflector validate-config config.toml

# Run with interfaces
sudo mdns-reflector -f eth0 -f eth1

# Run with config
sudo mdns-reflector -c /etc/mdns-reflector/mdns-reflector.toml

# Run with verbose logging
sudo mdns-reflector -f eth0 -f eth1 -vv

# Disable IPv6
sudo mdns-reflector -f eth0 -f eth1 --no-ipv6

# Set rate limit
sudo mdns-reflector -f eth0 -f eth1 --rate-limit 100

# Set max TTL
sudo mdns-reflector -f eth0 -f eth1 --max-ttl 120
```

## Example Scenarios

### Scenario 1: Basic Home Setup
Two networks, want to discover Chromecasts from both:
```bash
sudo mdns-reflector -f eth0 -f eth1
```

### Scenario 2: Guest Network Isolation
Guest network can discover services but can't access them directly:
```toml
[[zones]]
name = "main"
interfaces = ["eth0"]
allowed_zones = ["guest"]

[[zones]]
name = "guest"
interfaces = ["eth1"]
allowed_zones = ["main"]

[[service_filters]]
service_type = "_airplay._tcp"
action = "deny"
zones = ["guest"]  # Block AirPlay from guest network
```

### Scenario 3: Multiple VLANs
Complex enterprise setup with multiple VLANs:
```toml
interfaces = ["eth0.10", "eth0.20", "eth0.30", "eth0.100"]

[[zones]]
name = "corporate"
interfaces = ["eth0.10"]
allowed_zones = ["devices"]

[[zones]]
name = "devices"
interfaces = ["eth0.100"]
allowed_zones = ["corporate"]
```

Happy reflecting! 🎉
