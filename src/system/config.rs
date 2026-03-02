//! System configuration constants
//!
//! This module contains all system-wide configuration constants.
//! Centralizing configuration makes it easy to change settings in one place.

use std::net::Ipv4Addr;

/// AP IP address - the main IP of the captive portal
/// All services (DNS, HTTP) run on this IP
pub const AP_IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 71, 1);

/// WiFi SSID prefix; actual SSID is "{prefix}-{MAC suffix}", e.g. butterfly-A1B2C3D4
pub const WIFI_SSID_PREFIX: &str = "butterfly";

/// WiFi channel
pub const WIFI_CHANNEL: u8 = 1;

/// DNS server port
pub const DNS_PORT: u16 = 53;

/// HTTP server port  
pub const HTTP_PORT: u16 = 80;

/// Subnet mask (24 = 255.255.255.0)
pub const SUBNET_MASK: u8 = 24;

// ========================================
// Hardware Configuration
// ========================================

/// VL53L0X I2C SDA pin (GPIO6 / D4 on board)
pub const VL53L0X_SDA_PIN: u8 = 6;

/// VL53L0X I2C SCL pin (GPIO7 / D5 on board)
pub const VL53L0X_SCL_PIN: u8 = 7;

/// I2C frequency for VL53L0X (400kHz = fast mode)
pub const VL53L0X_I2C_FREQUENCY: u32 = 400_000;

/// DS-S006L servo 1 signal pin (GPIO3 / D1)
pub const SERVO_PIN: u8 = 3;

/// DS-S006L servo 2 signal pin (GPIO4)
pub const SERVO2_PIN: u8 = 4;

/// DS-S006L angle range
pub const SERVO_ANGLE_MIN: u16 = 0;
pub const SERVO_ANGLE_MAX: u16 = 300;

/// DS-S006L pulse range in microseconds
pub const SERVO_PULSE_MIN_US: u32 = 500;
pub const SERVO_PULSE_MAX_US: u32 = 2500;
