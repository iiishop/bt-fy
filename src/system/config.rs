//! System configuration constants
//!
//! This module contains all system-wide configuration constants.
//! Centralizing configuration makes it easy to change settings in one place.

use std::net::Ipv4Addr;

/// AP IP address - the main IP of the captive portal
/// All services (DNS, HTTP) run on this IP
pub const AP_IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 71, 1);

/// WiFi SSID name
pub const WIFI_SSID: &str = "butterfly";

/// WiFi channel
pub const WIFI_CHANNEL: u8 = 1;

/// DNS server port
pub const DNS_PORT: u16 = 53;

/// HTTP server port  
pub const HTTP_PORT: u16 = 80;

/// Subnet mask (24 = 255.255.255.0)
pub const SUBNET_MASK: u8 = 24;
