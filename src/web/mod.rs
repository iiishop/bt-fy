//! Web Service Module
//!
//! Manages the HTTP server and captive portal functionality.
//! Serves web pages and handles captive portal detection for various devices.

mod captive;
mod server;

use log::info;
use std::net::Ipv4Addr;

pub use server::ButterflyWeb;

/// Web Service - high-level wrapper around the HTTP server
///
/// Manages the HTTP server that serves the captive portal pages
/// and handles various device detection mechanisms.
pub struct WebService {
    ap_ip: Ipv4Addr,
    _server: Option<ButterflyWeb>,
}

impl WebService {
    /// Create a new Web service
    ///
    /// The service is created but the HTTP server is not started
    /// until `start()` is called.
    pub fn new(ap_ip: Ipv4Addr) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating Web service for IP: {}", ap_ip);

        Ok(Self {
            ap_ip,
            _server: None,
        })
    }

    /// Start the Web service
    ///
    /// Creates and starts the HTTP server with captive portal support.
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Web server on http://{}...", self.ap_ip);

        let server = ButterflyWeb::new(self.ap_ip)?;
        self._server = Some(server);

        info!("Web server started successfully");
        Ok(())
    }
}
