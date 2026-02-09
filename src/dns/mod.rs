//! DNS Service Module
//!
//! Provides DNS server functionality for the captive portal.
//! All DNS queries are intercepted and resolved to the AP's IP address.

mod simple;

use log::info;
use std::net::Ipv4Addr;

pub use simple::SimpleDns;

/// DNS Service - high-level wrapper around SimpleDns
///
/// This service handles DNS queries in a background thread.
/// When started, it spawns a thread that continuously polls for DNS queries.
pub struct DnsService {
    ap_ip: Ipv4Addr,
    _handle: Option<std::thread::JoinHandle<()>>,
}

impl DnsService {
    /// Create a new DNS service
    ///
    /// The service is created but not started until `start()` is called.
    pub fn new(ap_ip: Ipv4Addr) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating DNS service for IP: {}", ap_ip);

        Ok(Self {
            ap_ip,
            _handle: None,
        })
    }

    /// Start the DNS service
    ///
    /// Spawns a background thread that continuously handles DNS queries.
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting DNS server on {}:53...", self.ap_ip);

        // Create DNS server instance
        let mut dns = SimpleDns::try_new(self.ap_ip)?;

        // Spawn background thread
        let handle = std::thread::Builder::new()
            .name("dns-server".to_string())
            .stack_size(8192)
            .spawn(move || {
                info!("DNS server thread started");
                loop {
                    if let Err(e) = dns.poll() {
                        log::error!("DNS poll error: {:?}", e);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            })?;

        self._handle = Some(handle);
        info!("DNS server started successfully");

        Ok(())
    }
}
