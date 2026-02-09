//! Butterfly System - Main orchestration module
//!
//! This module provides the main application logic.
//! It coordinates all services (WiFi, DNS, Web) and manages their lifecycle.

pub mod config;

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger, nvs::EspDefaultNvsPartition};
use log::info;

use crate::{
    dns::DnsService,
    system::config::{AP_IP_ADDRESS, WIFI_SSID},
    web::WebService,
    wifi::WifiService,
};

/// Main system that orchestrates all services
pub struct ButterflySystem {
    _nvs: EspDefaultNvsPartition,
    wifi: WifiService,
    dns: DnsService,
    web: WebService,
}

impl ButterflySystem {
    /// Create a new Butterfly system
    pub fn new(peripherals: Peripherals) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing Butterfly System...");

        // Initialize ESP32 system
        Self::init_system()?;

        // Initialize NVS (Non-Volatile Storage) - required for WiFi
        let nvs = EspDefaultNvsPartition::take()?;
        info!("NVS initialized");

        // Get system event loop
        let sys_loop = EspSystemEventLoop::take()?;

        // Create services - each service encapsulates its own logic
        let wifi = WifiService::new(peripherals.modem, sys_loop)?;
        let dns = DnsService::new(AP_IP_ADDRESS)?;
        let web = WebService::new(AP_IP_ADDRESS)?;

        info!("All services created");

        Ok(Self {
            _nvs: nvs,
            wifi,
            dns,
            web,
        })
    }

    /// Run the system
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Butterfly System...");

        // Start WiFi first - foundation for everything
        self.wifi.start()?;
        info!("WiFi started");

        // Start DNS server - required for captive portal
        self.dns.start()?;
        info!("DNS server started");

        // Start Web server - serves the captive portal
        self.web.start()?;
        info!("Web server started");

        // Print system status
        Self::print_status();

        // Keep running
        info!("System is running.");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    /// Initialize ESP32 system components
    fn init_system() -> Result<(), Box<dyn std::error::Error>> {
        // Link ESP-IDF runtime patches
        esp_idf_svc::sys::link_patches();

        // Initialize logger
        EspLogger::initialize_default();

        Ok(())
    }

    /// Print system status information
    fn print_status() {
        info!("========================================");
        info!("Butterfly Captive Portal is Ready!");
        info!("========================================");
        info!("WiFi SSID: {} (no password)", WIFI_SSID);
        info!("IP Address: {}", AP_IP_ADDRESS);
        info!("DNS Server: {}:53", AP_IP_ADDRESS);
        info!("HTTP Server: http://{}", AP_IP_ADDRESS);
        info!("========================================");
        info!("Connect to '{}' WiFi", WIFI_SSID);
        info!("Captive portal should auto-open!");
        info!("========================================");
    }
}
