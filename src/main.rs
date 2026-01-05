use anyhow::Result;
use clap::Parser;
use std::fs;
use std::process;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use mdns_reflector::cli::{Cli, Commands};
use mdns_reflector::config::Config;
use mdns_reflector::network::InterfaceInfo;
use mdns_reflector::Reflector;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        error!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(cli.log_level()));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Handle subcommands
    match cli.command {
        Some(Commands::ListInterfaces) => {
            list_interfaces()?;
            return Ok(());
        }
        Some(Commands::GenConfig { output }) => {
            generate_config(&output)?;
            return Ok(());
        }
        Some(Commands::ValidateConfig { config }) => {
            validate_config(&config)?;
            return Ok(());
        }
        Some(Commands::Run) | None => {
            // Continue to run the reflector
        }
    }

    // Load or create configuration
    let mut config = if let Some(config_file) = &cli.config_file {
        info!("Loading configuration from {:?}", config_file);
        Config::from_file(config_file)?
    } else if !cli.interfaces.is_empty() {
        info!("Using interfaces from command line: {:?}", cli.interfaces);
        Config::from_interfaces(cli.interfaces.clone())
    } else {
        error!("No configuration file or interfaces specified");
        error!("Use -f/--interfaces to specify interfaces or -c/--config for a config file");
        process::exit(1);
    };

    // Apply CLI overrides
    if cli.no_ipv4 {
        config.settings.enable_ipv4 = false;
    }
    if cli.no_ipv6 {
        config.settings.enable_ipv6 = false;
    }
    if cli.max_ttl != 255 {
        config.settings.max_ttl = cli.max_ttl;
    }
    if cli.rate_limit != 0 {
        config.settings.rate_limit = cli.rate_limit;
    }
    if cli.no_loop_prevention {
        config.settings.loop_prevention = false;
    }

    // Validate configuration
    config.validate()?;

    info!("Starting mDNS reflector");
    info!("IPv4: {}, IPv6: {}", config.settings.enable_ipv4, config.settings.enable_ipv6);
    info!("Interfaces: {:?}", config.interfaces);
    info!("Max TTL: {}", config.settings.max_ttl);
    info!("Rate limit: {} pps", config.settings.rate_limit);
    info!("Loop prevention: {}", config.settings.loop_prevention);

    // Create and run reflector
    let mut reflector = Reflector::new(config)?;
    reflector.run().await?;

    Ok(())
}

/// List available network interfaces
fn list_interfaces() -> Result<()> {
    let interfaces = InterfaceInfo::list_all()?;

    println!("Available network interfaces:");
    println!();

    for iface in interfaces {
        println!("Interface: {}", iface.name);
        println!("  Index: {}", iface.index);
        if let Some(ipv4) = iface.ipv4_addr {
            println!("  IPv4: {}", ipv4);
        }
        if let Some(ipv6) = iface.ipv6_addr {
            println!("  IPv6: {}", ipv6);
        }
        if let Some(mac) = iface.mac_addr {
            println!("  MAC: {}", mac);
        }
        println!();
    }

    Ok(())
}

/// Generate an example configuration file
fn generate_config(output: &std::path::Path) -> Result<()> {
    let example_config = r#"# mDNS Reflector Configuration File
# 
# This is an example configuration file for mdns-reflector.
# Uncomment and modify the settings as needed.

# Network interface to bind to (optional)
# net_interface = "eth0"

# Interfaces to reflect mDNS traffic between
interfaces = ["eth0", "eth1"]

# General settings
[settings]
# Enable IPv4 reflection
enable_ipv4 = true

# Enable IPv6 reflection
enable_ipv6 = true

# Maximum TTL for reflected packets (seconds)
max_ttl = 255

# Rate limit in packets per second (0 = unlimited)
rate_limit = 0

# Enable loop prevention
loop_prevention = true

# Enable service caching
enable_cache = true

# Cache TTL in seconds
cache_ttl = 300

# Zone-based configuration (optional)
# [[zones]]
# name = "trusted"
# interfaces = ["eth0"]
# allowed_zones = ["iot"]
#
# [[zones]]
# name = "iot"
# interfaces = ["eth1"]
# allowed_zones = ["trusted"]

# Device-specific rules with MAC address filtering (optional)
# [devices."AA:BB:CC:DD:EE:FF"]
# description = "Living Room Chromecast"
# origin_pool = 100
# shared_pools = [10, 20]
# enabled = true

# Service filtering rules (optional)
# [[service_filters]]
# service_type = "_airplay._tcp"
# action = "allow"
# zones = ["trusted", "iot"]
#
# [[service_filters]]
# service_type = "_ipp._tcp"
# action = "allow"
# zones = ["trusted"]
"#;

    fs::write(output, example_config)?;
    info!("Generated example configuration file: {:?}", output);
    println!("Configuration file generated: {:?}", output);

    Ok(())
}

/// Validate a configuration file
fn validate_config(config_path: &std::path::Path) -> Result<()> {
    info!("Validating configuration file: {:?}", config_path);

    let config = Config::from_file(config_path)?;
    config.validate()?;

    println!("Configuration is valid!");
    println!();
    println!("Interfaces: {:?}", config.interfaces);
    println!("Zones: {}", config.zones.len());
    println!("Devices: {}", config.devices.len());
    println!("Service filters: {}", config.service_filters.len());
    println!();
    println!("Settings:");
    println!("  IPv4: {}", config.settings.enable_ipv4);
    println!("  IPv6: {}", config.settings.enable_ipv6);
    println!("  Max TTL: {}", config.settings.max_ttl);
    println!("  Rate limit: {} pps", config.settings.rate_limit);
    println!("  Loop prevention: {}", config.settings.loop_prevention);

    Ok(())
}
