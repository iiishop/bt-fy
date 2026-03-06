//! Web Service Module
//!
//! Manages the HTTP server and captive portal functionality.
//! Serves web pages and handles captive portal detection for various devices.

mod captive;
mod server;

use log::info;
use std::net::Ipv4Addr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;

pub use server::{ButterflyWeb, HardwareStatus};

use crate::wifi::{WifiCommand, WifiResponse};

/// Sender for WiFi commands (main thread executes and sends WifiResponse back)
pub type WifiCmdTx = mpsc::Sender<(WifiCommand, mpsc::Sender<WifiResponse>)>;

/// Web Service - high-level wrapper around the HTTP server
pub struct WebService {
    ap_ip: Ipv4Addr,
    hw_status: Option<HardwareStatus>,
    wifi_cmd_tx: Option<WifiCmdTx>,
    test_mode: Option<Arc<AtomicBool>>,
    _server: Option<ButterflyWeb>,
}

impl WebService {
    pub fn new(ap_ip: Ipv4Addr) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating Web service for IP: {}", ap_ip);

        Ok(Self {
            ap_ip,
            hw_status: None,
            wifi_cmd_tx: None,
            test_mode: None,
            _server: None,
        })
    }

    pub fn set_hardware_status(&mut self, hw_status: HardwareStatus) {
        self.hw_status = Some(hw_status);
    }

    pub fn set_wifi_cmd_tx(&mut self, tx: Option<WifiCmdTx>) {
        self.wifi_cmd_tx = tx;
    }

    /// Set the test-mode flag (distance-triggered servo). Must be set before start().
    pub fn set_test_mode(&mut self, flag: Arc<AtomicBool>) {
        self.test_mode = Some(flag);
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Web server on http://{}...", self.ap_ip);

        let server = ButterflyWeb::new(
            self.ap_ip,
            self.hw_status.clone(),
            self.wifi_cmd_tx.take(),
            self.test_mode.take(),
        )?;
        self._server = Some(server);

        info!("Web server started successfully");
        Ok(())
    }
}
