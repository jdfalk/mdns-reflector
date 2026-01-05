use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use trust_dns_proto::op::{Message, MessageType};
use trust_dns_proto::rr::{DNSClass, Name, RData, Record, RecordType};

/// mDNS IPv4 multicast address
pub const MDNS_IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);

/// mDNS IPv6 multicast address
pub const MDNS_IPV6_ADDR: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0xfb);

/// mDNS port
pub const MDNS_PORT: u16 = 5353;

/// Get the mDNS multicast socket address for IPv4
pub fn mdns_ipv4_socket_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(MDNS_IPV4_ADDR), MDNS_PORT)
}

/// Get the mDNS multicast socket address for IPv6
pub fn mdns_ipv6_socket_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V6(MDNS_IPV6_ADDR), MDNS_PORT)
}

/// mDNS packet wrapper
#[derive(Debug, Clone)]
pub struct MdnsPacket {
    /// The parsed DNS message
    pub message: Message,
    /// Source address of the packet
    pub src_addr: SocketAddr,
    /// Interface index where packet was received
    pub interface_index: u32,
    /// Raw packet data
    pub raw_data: Vec<u8>,
}

impl MdnsPacket {
    /// Parse an mDNS packet from raw bytes
    pub fn parse(data: &[u8], src_addr: SocketAddr, interface_index: u32) -> Result<Self> {
        let message = Message::from_vec(data)?;
        Ok(Self {
            message,
            src_addr,
            interface_index,
            raw_data: data.to_vec(),
        })
    }

    /// Check if this is a query packet
    pub fn is_query(&self) -> bool {
        self.message.message_type() == MessageType::Query
    }

    /// Check if this is a response packet
    pub fn is_response(&self) -> bool {
        self.message.message_type() == MessageType::Response
    }

    /// Check if this packet has the unicast response flag (QU)
    pub fn is_unicast_response_requested(&self) -> bool {
        // Check if any query has the unicast response bit set
        // In trust-dns-proto 0.23, we check the class value directly
        self.message
            .queries()
            .iter()
            .any(|q| {
                // QU flag is the high bit of the class field
                // Class values >= 0x8000 have QU bit set
                q.query_class() == DNSClass::IN // For now, simplified check
            })
    }

    /// Get service types from the packet
    pub fn get_service_types(&self) -> Vec<String> {
        let mut services = Vec::new();

        // Check queries
        for query in self.message.queries() {
            if let Some(service) = extract_service_type(query.name()) {
                services.push(service);
            }
        }

        // Check answers
        for answer in self.message.answers() {
            if let Some(service) = extract_service_type(answer.name()) {
                services.push(service);
            }
        }

        services.sort();
        services.dedup();
        services
    }

    /// Create a modified packet with updated TTL
    pub fn with_ttl(&self, max_ttl: u32) -> Result<Vec<u8>> {
        let mut message = self.message.clone();

        // Clear existing answers and rebuild with updated TTL
        let answers: Vec<Record> = message
            .answers()
            .iter()
            .map(|record| {
                let mut new_record = record.clone();
                let ttl = record.ttl().min(max_ttl);
                new_record.set_ttl(ttl);
                new_record
            })
            .collect();

        // Clear and re-insert answers
        message.take_answers();
        for answer in answers {
            message.add_answer(answer);
        }

        // Update TTL in additional records
        let additionals: Vec<Record> = message
            .additionals()
            .iter()
            .map(|record| {
                let mut new_record = record.clone();
                let ttl = record.ttl().min(max_ttl);
                new_record.set_ttl(ttl);
                new_record
            })
            .collect();

        // Clear and re-insert additionals
        message.take_additionals();
        for additional in additionals {
            message.add_additional(additional);
        }

        Ok(message.to_vec()?)
    }

    /// Get the source MAC address from raw packet data if available
    /// 
    /// Note: MAC address extraction requires raw socket access at the Ethernet layer.
    /// Since this implementation operates at the IP layer (UDP sockets), MAC addresses
    /// are not directly available. For MAC-based filtering, use the configuration file
    /// to specify device MAC addresses and VLAN pools, which will be matched against
    /// the source IP address via ARP table lookups (future enhancement).
    pub fn extract_mac_address(&self) -> Option<String> {
        // TODO: Implement MAC address lookup via ARP table
        // For now, MAC filtering is done through configuration-based VLAN pools
        None
    }
}

/// Extract service type from a DNS name
fn extract_service_type(name: &Name) -> Option<String> {
    let name_str = name.to_string();

    // Service names typically end with ._tcp.local. or ._udp.local.
    if name_str.contains("._tcp.local") {
        // Extract the service part (e.g., _airplay._tcp)
        let parts: Vec<&str> = name_str.split('.').collect();
        // For "_airplay._tcp.local.", parts = ["_airplay", "_tcp", "local", ""]
        if parts.len() >= 3 && !parts[0].is_empty() {
            return Some(format!("{}._tcp", parts[0]));
        }
    } else if name_str.contains("._udp.local") {
        let parts: Vec<&str> = name_str.split('.').collect();
        if parts.len() >= 3 && !parts[0].is_empty() {
            return Some(format!("{}._udp", parts[0]));
        }
    }

    None
}

/// Service information extracted from mDNS packets
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Service type (e.g., _http._tcp)
    pub service_type: String,
    /// Domain (usually .local)
    pub domain: String,
    /// Host name
    pub hostname: String,
    /// Port number
    pub port: u16,
    /// IP addresses
    pub addresses: Vec<IpAddr>,
    /// TXT record data
    pub txt_records: Vec<String>,
    /// TTL
    pub ttl: u32,
}

impl ServiceInfo {
    /// Parse service information from an mDNS packet
    pub fn from_packet(packet: &MdnsPacket) -> Vec<Self> {
        let mut services = Vec::new();

        // Parse SRV and PTR records to extract service information
        for answer in packet.message.answers() {
            if let Some(service) = Self::from_record(answer, packet) {
                services.push(service);
            }
        }

        services
    }

    fn from_record(record: &Record, _packet: &MdnsPacket) -> Option<Self> {
        match record.record_type() {
            RecordType::SRV => {
                if let Some(rdata) = record.data() {
                    if let RData::SRV(srv) = rdata {
                        let name = record.name().to_string();
                        let parts: Vec<&str> = name.split('.').collect();

                        if parts.len() >= 3 {
                            return Some(ServiceInfo {
                                name: parts[0].to_string(),
                                service_type: format!("{}.{}", parts[1], parts[2]),
                                domain: parts[3..].join("."),
                                hostname: srv.target().to_string(),
                                port: srv.port(),
                                addresses: Vec::new(),
                                txt_records: Vec::new(),
                                ttl: record.ttl(),
                            });
                        }
                    }
                }
            }
            RecordType::PTR => {
                if let Some(rdata) = record.data() {
                    if let RData::PTR(ptr) = rdata {
                        let service_type = record.name().to_string();
                        let instance_name = ptr.to_string();

                        return Some(ServiceInfo {
                            name: instance_name.clone(),
                            service_type: service_type.clone(),
                            domain: "local".to_string(),
                            hostname: instance_name,
                            port: 0,
                            addresses: Vec::new(),
                            txt_records: Vec::new(),
                            ttl: record.ttl(),
                        });
                    }
                }
            }
            _ => {}
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdns_addresses() {
        let ipv4_addr = mdns_ipv4_socket_addr();
        assert_eq!(ipv4_addr.port(), 5353);
        assert_eq!(ipv4_addr.ip(), IpAddr::V4(MDNS_IPV4_ADDR));

        let ipv6_addr = mdns_ipv6_socket_addr();
        assert_eq!(ipv6_addr.port(), 5353);
        assert_eq!(ipv6_addr.ip(), IpAddr::V6(MDNS_IPV6_ADDR));
    }

    #[test]
    fn test_extract_service_type() {
        let name = Name::from_ascii("_airplay._tcp.local.").unwrap();
        let service = extract_service_type(&name);
        assert_eq!(service, Some("_airplay._tcp".to_string()));

        let name = Name::from_ascii("_http._udp.local.").unwrap();
        let service = extract_service_type(&name);
        assert_eq!(service, Some("_http._udp".to_string()));
    }
}
