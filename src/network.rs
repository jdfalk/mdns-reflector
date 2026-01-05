use anyhow::{Context, Result};
use if_addrs::IfAddr;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tracing::{debug, info, warn};

use crate::mdns::{mdns_ipv4_socket_addr, mdns_ipv6_socket_addr, MDNS_IPV4_ADDR, MDNS_IPV6_ADDR, MDNS_PORT};

/// Network interface information
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    /// Interface name
    pub name: String,
    /// Interface index
    pub index: u32,
    /// IPv4 address (if any)
    pub ipv4_addr: Option<Ipv4Addr>,
    /// IPv6 address (if any)
    pub ipv6_addr: Option<Ipv6Addr>,
    /// MAC address
    pub mac_addr: Option<String>,
}

impl InterfaceInfo {
    /// Get all available network interfaces
    pub fn list_all() -> Result<Vec<Self>> {
        let if_addrs = if_addrs::get_if_addrs()?;
        let mut interfaces: HashMap<String, InterfaceInfo> = HashMap::new();

        for iface in if_addrs {
            let entry = interfaces.entry(iface.name.clone()).or_insert(InterfaceInfo {
                name: iface.name.clone(),
                index: iface.index.unwrap_or(0),
                ipv4_addr: None,
                ipv6_addr: None,
                mac_addr: None,
            });

            match iface.addr {
                IfAddr::V4(ref addr) => {
                    entry.ipv4_addr = Some(addr.ip);
                }
                IfAddr::V6(ref addr) => {
                    entry.ipv6_addr = Some(addr.ip);
                }
            }
        }

        Ok(interfaces.into_values().collect())
    }

    /// Find interface by name
    pub fn find_by_name(name: &str) -> Result<Option<Self>> {
        let interfaces = Self::list_all()?;
        Ok(interfaces.into_iter().find(|iface| iface.name == name))
    }

    /// Get interface names only
    pub fn list_names() -> Result<Vec<String>> {
        let interfaces = Self::list_all()?;
        Ok(interfaces.into_iter().map(|iface| iface.name).collect())
    }
}

/// mDNS socket manager
pub struct MdnsSocket {
    /// IPv4 socket
    pub ipv4_socket: Option<Socket>,
    /// IPv6 socket
    pub ipv6_socket: Option<Socket>,
    /// Interface this socket is bound to
    pub interface: InterfaceInfo,
}

/// Helper function to safely convert a mutable byte slice to MaybeUninit slice
/// 
/// # Safety
/// 
/// This is safe because:
/// 1. MaybeUninit<u8> has the same size and alignment as u8 (guaranteed by Rust)
/// 2. MaybeUninit<T> is #[repr(transparent)] over T
/// 3. A slice of initialized u8 can be safely viewed as MaybeUninit<u8>
/// 4. The socket will initialize the bytes it writes to
/// 5. We only read the initialized portion (up to size returned by recv_from)
#[inline]
unsafe fn as_maybe_uninit_slice(buf: &mut [u8]) -> &mut [std::mem::MaybeUninit<u8>] {
    std::slice::from_raw_parts_mut(
        buf.as_mut_ptr() as *mut std::mem::MaybeUninit<u8>,
        buf.len()
    )
}

impl MdnsSocket {
    /// Create a new mDNS socket for an interface
    pub fn new(interface: InterfaceInfo, enable_ipv4: bool, enable_ipv6: bool) -> Result<Self> {
        let ipv4_socket = if enable_ipv4 && interface.ipv4_addr.is_some() {
            match Self::create_ipv4_socket(&interface) {
                Ok(socket) => Some(socket),
                Err(e) => {
                    warn!("Failed to create IPv4 socket for {}: {}", interface.name, e);
                    None
                }
            }
        } else {
            None
        };

        let ipv6_socket = if enable_ipv6 && interface.ipv6_addr.is_some() {
            match Self::create_ipv6_socket(&interface) {
                Ok(socket) => Some(socket),
                Err(e) => {
                    warn!("Failed to create IPv6 socket for {}: {}", interface.name, e);
                    None
                }
            }
        } else {
            None
        };

        if ipv4_socket.is_none() && ipv6_socket.is_none() {
            anyhow::bail!(
                "Failed to create any sockets for interface {}",
                interface.name
            );
        }

        info!(
            "Created mDNS sockets for interface {} (IPv4: {}, IPv6: {})",
            interface.name,
            ipv4_socket.is_some(),
            ipv6_socket.is_some()
        );

        Ok(Self {
            ipv4_socket,
            ipv6_socket,
            interface,
        })
    }

    /// Create IPv4 multicast socket
    fn create_ipv4_socket(interface: &InterfaceInfo) -> Result<Socket> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .context("Failed to create IPv4 socket")?;

        // Set socket options
        socket.set_reuse_address(true)?;
        #[cfg(not(windows))]
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;

        // Bind to mDNS port
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), MDNS_PORT);
        socket.bind(&bind_addr.into())?;

        // Join multicast group on this interface
        if let Some(ipv4_addr) = interface.ipv4_addr {
            socket
                .join_multicast_v4(&MDNS_IPV4_ADDR, &ipv4_addr)
                .context("Failed to join IPv4 multicast group")?;

            // Set outbound interface for multicast
            socket
                .set_multicast_if_v4(&ipv4_addr)
                .context("Failed to set IPv4 multicast interface")?;

            debug!(
                "Joined IPv4 multicast group on {} ({})",
                interface.name, ipv4_addr
            );
        }

        // Enable multicast loopback
        socket.set_multicast_loop_v4(true)?;

        Ok(socket)
    }

    /// Create IPv6 multicast socket
    fn create_ipv6_socket(interface: &InterfaceInfo) -> Result<Socket> {
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
            .context("Failed to create IPv6 socket")?;

        // Set socket options
        socket.set_reuse_address(true)?;
        #[cfg(not(windows))]
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;

        // Bind to mDNS port
        let bind_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), MDNS_PORT);
        socket.bind(&bind_addr.into())?;

        // Join multicast group on this interface
        socket
            .join_multicast_v6(&MDNS_IPV6_ADDR, interface.index)
            .context("Failed to join IPv6 multicast group")?;

        // Set outbound interface for multicast
        socket.set_multicast_if_v6(interface.index)?;

        debug!(
            "Joined IPv6 multicast group on {} (index: {})",
            interface.name, interface.index
        );

        // Enable multicast loopback
        socket.set_multicast_loop_v6(true)?;

        Ok(socket)
    }

    /// Send data to IPv4 multicast group
    pub fn send_ipv4(&self, data: &[u8]) -> Result<usize> {
        if let Some(ref socket) = self.ipv4_socket {
            let addr = mdns_ipv4_socket_addr();
            let sent = socket.send_to(data, &addr.into())?;
            Ok(sent)
        } else {
            anyhow::bail!("IPv4 socket not available for interface {}", self.interface.name);
        }
    }

    /// Send data to IPv6 multicast group
    pub fn send_ipv6(&self, data: &[u8]) -> Result<usize> {
        if let Some(ref socket) = self.ipv6_socket {
            let addr = mdns_ipv6_socket_addr();
            let sent = socket.send_to(data, &addr.into())?;
            Ok(sent)
        } else {
            anyhow::bail!("IPv6 socket not available for interface {}", self.interface.name);
        }
    }

    /// Receive data from IPv4 socket
    pub fn recv_ipv4(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        if let Some(ref socket) = self.ipv4_socket {
            // Convert buffer to MaybeUninit slice for socket2 API
            let uninit_buf = unsafe { as_maybe_uninit_slice(buf) };
            
            let (size, addr) = socket.recv_from(uninit_buf)?;
            
            // Data is now initialized up to size bytes
            // No need to copy - the data is already in buf
            
            let socket_addr = match addr.as_socket() {
                Some(addr) => addr,
                None => anyhow::bail!("Invalid socket address"),
            };
            Ok((size, socket_addr))
        } else {
            anyhow::bail!("IPv4 socket not available");
        }
    }

    /// Receive data from IPv6 socket
    pub fn recv_ipv6(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        if let Some(ref socket) = self.ipv6_socket {
            // Convert buffer to MaybeUninit slice for socket2 API
            let uninit_buf = unsafe { as_maybe_uninit_slice(buf) };
            
            let (size, addr) = socket.recv_from(uninit_buf)?;
            
            // Data is now initialized up to size bytes
            // No need to copy - the data is already in buf
            
            let socket_addr = match addr.as_socket() {
                Some(addr) => addr,
                None => anyhow::bail!("Invalid socket address"),
            };
            Ok((size, socket_addr))
        } else {
            anyhow::bail!("IPv6 socket not available");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_interfaces() {
        let result = InterfaceInfo::list_all();
        assert!(result.is_ok());
        let interfaces = result.unwrap();
        // At minimum, there should be a loopback interface
        assert!(!interfaces.is_empty());
    }

    #[test]
    fn test_list_interface_names() {
        let result = InterfaceInfo::list_names();
        assert!(result.is_ok());
        let names = result.unwrap();
        assert!(!names.is_empty());
    }
}
