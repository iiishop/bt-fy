use log::{info, warn};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

const DNS_PORT: u16 = 53;
const BUFFER_SIZE: usize = 512;

pub struct DnsServer {
    _handle: Option<std::thread::JoinHandle<()>>,
}

impl DnsServer {
    pub fn new(ap_ip: Ipv4Addr) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Starting DNS server on port {}...", DNS_PORT);
        info!("Will resolve all domains to: {}", ap_ip);

        let handle = std::thread::Builder::new()
            .name("dns-server".to_string())
            .stack_size(8192)
            .spawn(move || {
                info!("DNS server thread started");
                if let Err(e) = run_dns_server(ap_ip) {
                    warn!("DNS server error: {:?}", e);
                }
                warn!("DNS server thread exited");
            })?;

        // Give the thread time to bind
        std::thread::sleep(std::time::Duration::from_millis(200));

        info!("DNS server started successfully");
        Ok(Self {
            _handle: Some(handle),
        })
    }
}

fn run_dns_server(ap_ip: Ipv4Addr) -> Result<(), Box<dyn std::error::Error>> {
    // Create UDP socket and bind to port 53
    let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], DNS_PORT)))?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(1)))?;

    info!("DNS server listening on 0.0.0.0:{}", DNS_PORT);
    info!("All DNS queries will resolve to: {}", ap_ip);
    info!("Waiting for DNS queries...");

    let mut buffer = [0u8; BUFFER_SIZE];
    let mut query_count = 0u32;

    loop {
        // Receive DNS query
        match socket.recv_from(&mut buffer) {
            Ok((recv_len, client_addr)) => {
                query_count += 1;

                if recv_len < 12 {
                    warn!(
                        "[{}] Received invalid DNS packet (too short: {} bytes)",
                        query_count, recv_len
                    );
                    continue; // Invalid DNS packet
                }

                // Extract domain name for logging
                if let Some(domain) = extract_domain_name(&buffer[..recv_len]) {
                    info!(
                        "[{}] DNS query from {}: {} -> {}",
                        query_count, client_addr, domain, ap_ip
                    );
                } else {
                    info!(
                        "[{}] DNS query from {} (unparseable domain)",
                        query_count, client_addr
                    );
                }

                // Parse and respond
                if let Some(response) = create_dns_response(&buffer[..recv_len], ap_ip) {
                    match socket.send_to(&response, client_addr) {
                        Ok(sent) => {
                            info!("[{}] DNS response sent: {} bytes", query_count, sent);
                        }
                        Err(e) => {
                            warn!("[{}] Failed to send DNS response: {}", query_count, e);
                        }
                    }
                } else {
                    warn!("[{}] Failed to create DNS response", query_count);
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // Timeout, continue listening
                continue;
            }
            Err(e) => {
                warn!("DNS recv error: {}", e);
            }
        }
    }
}

fn create_dns_response(query: &[u8], ip: Ipv4Addr) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }

    // DNS Header structure:
    // Transaction ID (2 bytes)
    // Flags (2 bytes)
    // Questions (2 bytes)
    // Answer RRs (2 bytes)
    // Authority RRs (2 bytes)
    // Additional RRs (2 bytes)

    let mut response = Vec::with_capacity(512);

    // Copy transaction ID
    response.extend_from_slice(&query[0..2]);

    // Flags: Standard query response, no error
    response.push(0x81); // Response + Recursion Desired
    response.push(0x80); // Recursion Available + No Error

    // Questions count (copy from query)
    response.extend_from_slice(&query[4..6]);

    // Answer RRs count (1 answer)
    response.push(0x00);
    response.push(0x01);

    // Authority RRs (0)
    response.push(0x00);
    response.push(0x00);

    // Additional RRs (0)
    response.push(0x00);
    response.push(0x00);

    // Copy the question section
    let question_start = 12;
    let mut question_end = question_start;

    // Find end of question by parsing domain name
    while question_end < query.len() {
        let len = query[question_end];
        if len == 0 {
            question_end += 5; // null byte + qtype (2) + qclass (2)
            break;
        }
        question_end += len as usize + 1;
    }

    if question_end > query.len() {
        return None;
    }

    // Copy question section
    response.extend_from_slice(&query[question_start..question_end]);

    // Add answer section
    // Name: pointer to question name (0xC00C)
    response.push(0xC0);
    response.push(0x0C);

    // Type: A (0x0001)
    response.push(0x00);
    response.push(0x01);

    // Class: IN (0x0001)
    response.push(0x00);
    response.push(0x01);

    // TTL: 60 seconds
    response.push(0x00);
    response.push(0x00);
    response.push(0x00);
    response.push(0x3C);

    // Data length: 4 bytes (IPv4 address)
    response.push(0x00);
    response.push(0x04);

    // IP address
    let octets = ip.octets();
    response.extend_from_slice(&octets);

    Some(response)
}

fn extract_domain_name(query: &[u8]) -> Option<String> {
    if query.len() < 13 {
        return None;
    }

    let mut domain = String::new();
    let mut pos = 12; // Start after DNS header

    while pos < query.len() {
        let len = query[pos] as usize;
        if len == 0 {
            break;
        }
        if pos + 1 + len > query.len() {
            return None;
        }

        if !domain.is_empty() {
            domain.push('.');
        }

        let label = String::from_utf8_lossy(&query[pos + 1..pos + 1 + len]);
        domain.push_str(&label);
        pos += len + 1;
    }

    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}
