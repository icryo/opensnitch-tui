//! Network formatting utilities

/// Format an address:port combination
pub fn format_address(host: &str, ip: &str, port: u32) -> String {
    let addr = if host.is_empty() { ip } else { host };
    format!("{}:{}", addr, port)
}

/// Truncate hostname to fit display
pub fn truncate_host(host: &str, max_len: usize) -> String {
    if host.len() <= max_len {
        host.to_string()
    } else {
        format!("{}...", &host[..max_len.saturating_sub(3)])
    }
}

/// Get protocol display name
pub fn protocol_name(proto: &str) -> String {
    match proto.to_uppercase().as_str() {
        "TCP" | "6" => "TCP".to_string(),
        "UDP" | "17" => "UDP".to_string(),
        "ICMP" | "1" => "ICMP".to_string(),
        "ICMP6" | "58" => "ICMPv6".to_string(),
        _ => proto.to_string(),
    }
}

/// Check if an IP address is IPv6
pub fn is_ipv6(ip: &str) -> bool {
    ip.contains(':')
}

/// Format IP address for display
pub fn format_ip(ip: &str) -> String {
    if is_ipv6(ip) && ip.len() > 20 {
        // Truncate long IPv6 addresses
        format!("{}...", &ip[..17])
    } else {
        ip.to_string()
    }
}
