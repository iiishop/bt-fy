//! Web Service Module
//!
//! Manages the HTTP server and captive portal functionality.
//! Serves web pages and handles captive portal detection for various devices.

mod captive;
mod server;

use log::info;
use std::net::Ipv4Addr;

pub use server::{ButterflyWeb, HardwareStatus};

/// Web Service - high-level wrapper around the HTTP server
///
/// Manages the HTTP server that serves the captive portal pages
/// and handles various device detection mechanisms.
pub struct WebService {
    ap_ip: Ipv4Addr,
    hw_status: Option<HardwareStatus>,
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
            hw_status: None,
            _server: None,
        })
    }

    /// Set hardware status provider
    pub fn set_hardware_status(&mut self, hw_status: HardwareStatus) {
        self.hw_status = Some(hw_status);
    }

    /// Start the Web service
    ///
    /// Creates and starts the HTTP server with captive portal support.
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Web server on http://{}...", self.ap_ip);

        let server = ButterflyWeb::new(self.ap_ip, self.hw_status.clone())?;
        self._server = Some(server);

        info!("Web server started successfully");
        Ok(())
    }
}
