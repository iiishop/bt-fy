//! System configuration constants
//!
//! This module contains all system-wide configuration constants.
//! Centralizing configuration makes it easy to change settings in one place.

use std::net::Ipv4Addr;

/// AP IP address - the main IP of the captive portal
/// All services (DNS, HTTP) run on this IP
pub const AP_IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 71, 1);

/// WiFi SSID prefix; actual SSID is "{prefix}{MAC suffix}", e.g. BF_A1B2C3D4（与 App 发现约定一致）
pub const WIFI_SSID_PREFIX: &str = "BF_";

/// WiFi channel
pub const WIFI_CHANNEL: u8 = 1;

/// DNS server port
pub const DNS_PORT: u16 = 53;

/// 是否启用 DNS 截获（captive portal 将所有域名解析到 AP IP）。已关闭，相关 DNS 代码已标记为过时。
pub const ENABLE_DNS_CAPTIVE: bool = false;

/// HTTP server port  
pub const HTTP_PORT: u16 = 80;

/// AP 模式配网 TCP 端口（JSON 协议：identify / config）
pub const AP_TCP_PORT: u16 = 1234;

/// STA 模式：UDP 广播端口与 TCP 控制端口
pub const STA_UDP_PORT: u16 = 12345;
pub const STA_TCP_PORT: u16 = 12345;

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

// ========================================
// VL53L0X 距离触发舵机（test-mode）
// ========================================

/// 距离 < 此值(mm) 时开始挥动
pub const TOF_THRESHOLD_START_MM: u16 = 110;
/// 距离 ≤ 此值(mm) 时用较快周期（舵机最快速度）
pub const TOF_THRESHOLD_FAST_MM: u16 = 60;
/// 刚触发时的周期（ms），一整圈 25°↔130°↔25°
pub const TOF_PERIOD_SLOW_MS: u64 = 2000;
/// 较快周期（≤60mm 时），越小越快
pub const TOF_PERIOD_FAST_MS: u64 = 300;
