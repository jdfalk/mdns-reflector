use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A secure, high-performance multicast DNS reflector written in Rust
#[derive(Parser, Debug)]
#[command(name = "mdns-reflector")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Network interfaces to reflect mDNS traffic between
    #[arg(short = 'f', long = "interfaces", value_name = "INTERFACES")]
    pub interfaces: Vec<String>,

    /// Configuration file path
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    pub config_file: Option<PathBuf>,

    /// Enable foreground mode (do not daemonize)
    #[arg(short = 'n', long = "foreground")]
    pub foreground: bool,

    /// Verbose logging (can be specified multiple times)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Disable IPv4
    #[arg(long = "no-ipv4")]
    pub no_ipv4: bool,

    /// Disable IPv6
    #[arg(long = "no-ipv6")]
    pub no_ipv6: bool,

    /// Maximum TTL for reflected packets
    #[arg(long = "max-ttl", value_name = "SECONDS", default_value = "255")]
    pub max_ttl: u32,

    /// Rate limit in packets per second (0 = unlimited)
    #[arg(long = "rate-limit", value_name = "PPS", default_value = "0")]
    pub rate_limit: u64,

    /// Disable loop prevention
    #[arg(long = "no-loop-prevention")]
    pub no_loop_prevention: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List available network interfaces
    ListInterfaces,

    /// Generate example configuration file
    GenConfig {
        /// Output file path
        #[arg(short = 'o', long = "output", default_value = "mdns-reflector.toml")]
        output: PathBuf,
    },

    /// Validate configuration file
    ValidateConfig {
        /// Configuration file to validate
        #[arg(value_name = "FILE")]
        config: PathBuf,
    },

    /// Run the reflector
    Run,
}

impl Cli {
    /// Get the log level based on verbosity
    pub fn log_level(&self) -> &str {
        match self.verbose {
            0 => "info",
            1 => "debug",
            _ => "trace",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse() {
        let cli = Cli::parse_from(&["mdns-reflector", "-f", "eth0", "-f", "eth1"]);
        assert_eq!(cli.interfaces, vec!["eth0", "eth1"]);
    }

    #[test]
    fn test_log_level() {
        let cli = Cli::parse_from(&["mdns-reflector"]);
        assert_eq!(cli.log_level(), "info");

        let cli = Cli::parse_from(&["mdns-reflector", "-v"]);
        assert_eq!(cli.log_level(), "debug");

        let cli = Cli::parse_from(&["mdns-reflector", "-vv"]);
        assert_eq!(cli.log_level(), "trace");
    }
}
