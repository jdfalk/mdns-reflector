use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Main configuration structure for the mDNS reflector
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Network interface to bind to
    #[serde(default)]
    pub net_interface: Option<String>,

    /// Interfaces to reflect mDNS traffic between
    #[serde(default)]
    pub interfaces: Vec<String>,

    /// Zone-based configuration for interface grouping
    #[serde(default)]
    pub zones: Vec<Zone>,

    /// Device-specific rules
    #[serde(default)]
    pub devices: HashMap<String, DeviceConfig>,

    /// Service filtering rules
    #[serde(default)]
    pub service_filters: Vec<ServiceFilter>,

    /// General reflector settings
    #[serde(default)]
    pub settings: Settings,
}

/// Zone configuration for grouping interfaces
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Zone {
    /// Zone name
    pub name: String,

    /// Interfaces in this zone
    pub interfaces: Vec<String>,

    /// Zones that this zone can communicate with
    #[serde(default)]
    pub allowed_zones: Vec<String>,
}

/// Device-specific configuration with MAC-based filtering
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceConfig {
    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Origin VLAN tag where the device resides
    pub origin_pool: u16,

    /// VLAN tags where the device should be discoverable
    #[serde(default)]
    pub shared_pools: Vec<u16>,

    /// Whether this device is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Service filtering configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceFilter {
    /// Service type pattern (e.g., "_airplay._tcp", "_ipp._tcp")
    pub service_type: String,

    /// Action to take (allow or deny)
    pub action: FilterAction,

    /// Zones this filter applies to
    #[serde(default)]
    pub zones: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FilterAction {
    Allow,
    Deny,
}

/// General settings for the reflector
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    /// Enable IPv4 reflection
    #[serde(default = "default_true")]
    pub enable_ipv4: bool,

    /// Enable IPv6 reflection
    #[serde(default = "default_true")]
    pub enable_ipv6: bool,

    /// Maximum TTL for reflected packets
    #[serde(default = "default_max_ttl")]
    pub max_ttl: u32,

    /// Rate limit in packets per second (0 = unlimited)
    #[serde(default)]
    pub rate_limit: u64,

    /// Enable loop prevention
    #[serde(default = "default_true")]
    pub loop_prevention: bool,

    /// Enable service caching
    #[serde(default = "default_true")]
    pub enable_cache: bool,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enable_ipv4: true,
            enable_ipv6: true,
            max_ttl: 255,
            rate_limit: 0,
            loop_prevention: true,
            enable_cache: true,
            cache_ttl: 300,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            net_interface: None,
            interfaces: Vec::new(),
            zones: Vec::new(),
            devices: HashMap::new(),
            service_filters: Vec::new(),
            settings: Settings::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Create a default configuration with provided interfaces
    pub fn from_interfaces(interfaces: Vec<String>) -> Self {
        Self {
            interfaces,
            ..Default::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.interfaces.is_empty() && self.zones.is_empty() {
            anyhow::bail!("At least one interface or zone must be specified");
        }

        // Validate device MAC addresses
        for (mac, _device) in &self.devices {
            if !is_valid_mac_address(mac) {
                anyhow::bail!("Invalid MAC address: {}", mac);
            }
        }

        Ok(())
    }
}

fn default_true() -> bool {
    true
}

fn default_max_ttl() -> u32 {
    255
}

fn default_cache_ttl() -> u64 {
    300
}

/// Validate MAC address format
fn is_valid_mac_address(mac: &str) -> bool {
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        return false;
    }
    parts.iter().all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.settings.enable_ipv4);
        assert!(config.settings.enable_ipv6);
    }

    #[test]
    fn test_valid_mac_address() {
        assert!(is_valid_mac_address("AA:BB:CC:DD:EE:FF"));
        assert!(is_valid_mac_address("00:11:22:33:44:55"));
        assert!(!is_valid_mac_address("AA:BB:CC:DD:EE"));
        assert!(!is_valid_mac_address("GG:BB:CC:DD:EE:FF"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_err());

        config.interfaces.push("eth0".to_string());
        assert!(config.validate().is_ok());
    }
}
