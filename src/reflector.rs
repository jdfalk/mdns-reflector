use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

use crate::config::{Config, FilterAction};
use crate::mdns::MdnsPacket;
use crate::network::{InterfaceInfo, MdnsSocket};

/// Main reflector engine
pub struct Reflector {
    /// Configuration
    config: Arc<Config>,
    /// Network sockets mapped by interface name
    sockets: HashMap<String, MdnsSocket>,
    /// Packet cache for loop prevention
    packet_cache: Arc<RwLock<PacketCache>>,
    /// Service cache
    service_cache: Arc<RwLock<ServiceCache>>,
    /// Rate limiter
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl Reflector {
    /// Create a new reflector instance
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;

        let mut sockets = HashMap::new();

        // Create sockets for each configured interface
        for interface_name in &config.interfaces {
            if let Some(interface) = InterfaceInfo::find_by_name(interface_name)? {
                match MdnsSocket::new(
                    interface.clone(),
                    config.settings.enable_ipv4,
                    config.settings.enable_ipv6,
                ) {
                    Ok(socket) => {
                        sockets.insert(interface_name.clone(), socket);
                    }
                    Err(e) => {
                        warn!("Failed to create socket for {}: {}", interface_name, e);
                    }
                }
            } else {
                warn!("Interface {} not found", interface_name);
            }
        }

        if sockets.is_empty() {
            anyhow::bail!("No valid sockets created");
        }

        info!("Created {} network sockets", sockets.len());

        let packet_cache_ttl = Duration::from_secs(config.settings.packet_cache_ttl);

        Ok(Self {
            config: Arc::new(config),
            sockets,
            packet_cache: Arc::new(RwLock::new(PacketCache::new(packet_cache_ttl))),
            service_cache: Arc::new(RwLock::new(ServiceCache::new())),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new())),
        })
    }

    /// Start the reflector
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting mDNS reflector");

        // Start receiver tasks for each socket
        let mut tasks = Vec::new();

        for (interface_name, socket) in &self.sockets {
            // Spawn IPv4 receiver task
            if socket.ipv4_socket.is_some() {
                let task = self.spawn_receiver_task(
                    interface_name.clone(),
                    socket.interface.clone(),
                    false,
                );
                tasks.push(task);
            }

            // Spawn IPv6 receiver task
            if socket.ipv6_socket.is_some() {
                let task = self.spawn_receiver_task(
                    interface_name.clone(),
                    socket.interface.clone(),
                    true,
                );
                tasks.push(task);
            }
        }

        // Start cache cleanup task
        let cache_cleanup_task = self.spawn_cache_cleanup_task();
        tasks.push(cache_cleanup_task);

        info!("Reflector started with {} receiver tasks", tasks.len());

        // Wait for all tasks
        for task in tasks {
            if let Err(e) = task.await {
                warn!("Task failed: {}", e);
            }
        }

        Ok(())
    }

    /// Spawn a receiver task for an interface
    fn spawn_receiver_task(
        &self,
        interface_name: String,
        _interface: InterfaceInfo,
        is_ipv6: bool,
    ) -> tokio::task::JoinHandle<()> {
        let _config = Arc::clone(&self.config);
        let _packet_cache = Arc::clone(&self.packet_cache);
        let _service_cache = Arc::clone(&self.service_cache);
        let _rate_limiter = Arc::clone(&self.rate_limiter);

        tokio::spawn(async move {
            info!(
                "Receiver task started for {} (IPv{})",
                interface_name,
                if is_ipv6 { 6 } else { 4 }
            );

            // TODO: Implement actual packet reception and reflection
            // 
            // The full implementation would:
            // 1. Convert socket2::Socket to tokio::net::UdpSocket or use async I/O
            // 2. Set up a receive buffer (e.g., 9000 bytes for jumbo frames)
            // 3. Loop to receive packets:
            //    a. Await packet from socket
            //    b. Parse mDNS packet
            //    c. Apply filters (rate limit, service filters, MAC filters)
            //    d. Check for duplicate packets (loop prevention)
            //    e. Rewrite TTL if needed
            //    f. Send to other interfaces via reflector.reflect_packet()
            //    g. Update service cache if enabled
            // 4. Handle errors and reconnect if needed
            //
            // This is a placeholder that prevents the task from exiting
            // A production implementation would use async socket operations
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                trace!("Receiver task heartbeat for {}", interface_name);
            }
        })
    }

    /// Spawn cache cleanup task
    fn spawn_cache_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let packet_cache = Arc::clone(&self.packet_cache);
        let service_cache = Arc::clone(&self.service_cache);
        let cleanup_interval = self.config.settings.cache_cleanup_interval;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(cleanup_interval)).await;
                packet_cache.write().await.cleanup();
                service_cache.write().await.cleanup();
                trace!("Cache cleanup completed");
            }
        })
    }

    /// Reflect a packet to other interfaces
    pub async fn reflect_packet(
        &self,
        packet: &MdnsPacket,
        source_interface: &str,
    ) -> Result<()> {
        // Check rate limit
        {
            let mut limiter = self.rate_limiter.write().await;
            if !limiter.allow_packet(self.config.settings.rate_limit) {
                debug!("Packet rate limit exceeded, dropping packet");
                return Ok(());
            }
        }

        // Check for loops
        if self.config.settings.loop_prevention {
            let mut cache = self.packet_cache.write().await;
            if cache.contains(packet) {
                trace!("Packet already seen, dropping to prevent loop");
                return Ok(());
            }
            cache.insert(packet);
        }

        // Apply filters
        if !self.should_reflect_packet(packet, source_interface).await {
            trace!("Packet filtered, not reflecting");
            return Ok(());
        }

        // Rewrite TTL if needed
        let packet_data = if self.config.settings.max_ttl < 255 {
            packet.with_ttl(self.config.settings.max_ttl)?
        } else {
            packet.raw_data.clone()
        };

        // Reflect to all other interfaces
        let mut reflected = 0;
        for (interface_name, socket) in &self.sockets {
            if interface_name == source_interface {
                continue; // Don't reflect back to source
            }

            // Check zone restrictions
            if !self.is_reflection_allowed(source_interface, interface_name).await {
                continue;
            }

            // Send packet
            let result = match packet.src_addr.is_ipv4() {
                true => socket.send_ipv4(&packet_data),
                false => socket.send_ipv6(&packet_data),
            };

            match result {
                Ok(_) => {
                    reflected += 1;
                    trace!("Reflected packet from {} to {}", source_interface, interface_name);
                }
                Err(e) => {
                    warn!("Failed to reflect to {}: {}", interface_name, e);
                }
            }
        }

        if reflected > 0 {
            debug!(
                "Reflected packet from {} to {} interfaces",
                source_interface, reflected
            );
        }

        Ok(())
    }

    /// Check if packet should be reflected based on filters
    async fn should_reflect_packet(&self, packet: &MdnsPacket, _source_interface: &str) -> bool {
        // Check service filters
        if !self.config.service_filters.is_empty() {
            let service_types = packet.get_service_types();

            for service_type in service_types {
                for filter in &self.config.service_filters {
                    if filter.service_type == service_type || filter.service_type == "*" {
                        match filter.action {
                            FilterAction::Allow => return true,
                            FilterAction::Deny => return false,
                        }
                    }
                }
            }
        }

        true
    }

    /// Check if reflection is allowed between two interfaces based on zones
    async fn is_reflection_allowed(&self, source: &str, target: &str) -> bool {
        // If no zones are configured, allow all reflections
        if self.config.zones.is_empty() {
            return true;
        }

        // Find zones for source and target interfaces
        let source_zone = self.config.zones.iter().find(|z| z.interfaces.iter().any(|i| i == source));
        let target_zone = self.config.zones.iter().find(|z| z.interfaces.iter().any(|i| i == target));

        match (source_zone, target_zone) {
            (Some(src_zone), Some(tgt_zone)) => {
                // Check if source zone allows communication with target zone
                if src_zone.allowed_zones.is_empty() {
                    // Empty allowed_zones means allow all
                    return true;
                }
                src_zone.allowed_zones.contains(&tgt_zone.name)
            }
            (None, None) => {
                // Both interfaces not in any zone - allow if no zones configured
                true
            }
            _ => {
                // One interface is in a zone, the other is not - deny
                false
            }
        }
    }
}

/// Packet cache for loop prevention
struct PacketCache {
    cache: HashMap<PacketSignature, Instant>,
    max_age: Duration,
}

impl PacketCache {
    fn new(max_age: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            max_age,
        }
    }

    fn contains(&self, packet: &MdnsPacket) -> bool {
        let sig = PacketSignature::from_packet(packet);
        if let Some(&timestamp) = self.cache.get(&sig) {
            timestamp.elapsed() < self.max_age
        } else {
            false
        }
    }

    fn insert(&mut self, packet: &MdnsPacket) {
        let sig = PacketSignature::from_packet(packet);
        self.cache.insert(sig, Instant::now());
    }

    fn cleanup(&mut self) {
        self.cache.retain(|_, &mut timestamp| timestamp.elapsed() < self.max_age);
    }
}

/// Packet signature for duplicate detection
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct PacketSignature {
    id: u16,
    src_addr: SocketAddr,
    questions_hash: u64,
}

impl PacketSignature {
    fn from_packet(packet: &MdnsPacket) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for query in packet.message.queries() {
            query.name().hash(&mut hasher);
            query.query_type().hash(&mut hasher);
        }

        Self {
            id: packet.message.id(),
            src_addr: packet.src_addr,
            questions_hash: hasher.finish(),
        }
    }
}

/// Service cache for discovered services
struct ServiceCache {
    services: HashMap<String, CachedService>,
}

impl ServiceCache {
    fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        self.services.retain(|_, service| now < service.expires_at);
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CachedService {
    name: String,
    expires_at: Instant,
}

/// Rate limiter
struct RateLimiter {
    last_reset: Instant,
    count: u64,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            last_reset: Instant::now(),
            count: 0,
        }
    }

    fn allow_packet(&mut self, rate_limit: u64) -> bool {
        if rate_limit == 0 {
            return true; // No rate limit
        }

        // Reset counter every second
        if self.last_reset.elapsed() >= Duration::from_secs(1) {
            self.count = 0;
            self.last_reset = Instant::now();
        }

        if self.count < rate_limit {
            self.count += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new();

        // No limit
        assert!(limiter.allow_packet(0));
        assert!(limiter.allow_packet(0));

        // With limit
        limiter = RateLimiter::new();
        assert!(limiter.allow_packet(2));
        assert!(limiter.allow_packet(2));
        assert!(!limiter.allow_packet(2)); // Third packet should be rejected
    }
}
