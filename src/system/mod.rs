//! Butterfly System - Main orchestration module
//!
//! This module provides the main application logic.
//! It coordinates all services (WiFi, DNS, Web, Hardware) and manages their lifecycle.

pub mod config;

use esp_idf_hal::{peripheral::Peripheral, peripherals::Peripherals};
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger, nvs::EspDefaultNvsPartition};
use log::info;
use std::sync::Arc;

use crate::{
    dns::DnsService,
    hardware::{MotorDirection, MotorService, ServoService, VL53L0XService},
    system::config::AP_IP_ADDRESS,
    web::{HardwareStatus, WebService},
    wifi::WifiService,
};

/// Main system that orchestrates all services
pub struct ButterflySystem {
    _nvs: EspDefaultNvsPartition,
    wifi: WifiService,
    dns: DnsService,
    web: WebService,
    sensor: Option<Arc<VL53L0XService>>,
    motor: Option<Arc<MotorService>>,
    servo: Option<Arc<ServoService>>,
}

impl ButterflySystem {
    /// Create a new Butterfly system with automatic fallback
    pub fn new(mut peripherals: Peripherals) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing Butterfly System...");

        // Initialize ESP32 system
        Self::init_system()?;

        // Initialize NVS (Non-Volatile Storage) - required for WiFi
        let nvs = EspDefaultNvsPartition::take()?;
        info!("NVS initialized");

        // Get system event loop
        let sys_loop = EspSystemEventLoop::take()?;

        // Try to initialize hardware FIRST (before moving modem to WiFi)
        let (sensor, motor, servo) = Self::try_init_hardware(&mut peripherals);

        // Now create core services (WiFi, DNS, Web)
        // These work independently of hardware
        let wifi = WifiService::new(peripherals.modem, sys_loop)?;
        info!("WiFi service created");

        let dns = DnsService::new(AP_IP_ADDRESS)?;
        info!("DNS service created");

        let web = WebService::new(AP_IP_ADDRESS)?;
        info!("Web service created");

        if sensor.is_some() || motor.is_some() || servo.is_some() {
            info!("✓ Hardware initialized successfully");
        } else {
            log::warn!("⚠ System running in CAPTIVE PORTAL ONLY mode");
            log::info!("→ WiFi and web interface still available");
        }

        Ok(Self {
            _nvs: nvs,
            wifi,
            dns,
            web,
            sensor,
            motor,
            servo,
        })
    }

    /// Try to initialize hardware services (VL53L0X + DRV8833 motor)
    fn try_init_hardware(
        peripherals: &mut Peripherals,
    ) -> (
        Option<Arc<VL53L0XService>>,
        Option<Arc<MotorService>>,
        Option<Arc<ServoService>>,
    ) {
        let sensor = {
            info!("Initializing VL53L0X sensor...");

            // Initialize I2C for VL53L0X sensor (takes i2c0, gpio6, gpio7)
            // Use unsafe clone to get ownership (peripherals are partially moved)
            let i2c_driver = match crate::hardware::vl53l0x::create_i2c_driver(
                unsafe { peripherals.i2c0.clone_unchecked() },
                unsafe { peripherals.pins.gpio6.clone_unchecked() },
                unsafe { peripherals.pins.gpio7.clone_unchecked() },
            ) {
                Ok(driver) => driver,
                Err(e) => {
                    log::warn!("VL53L0X I2C init failed: {}", e);
                    return (
                        None,
                        Self::try_init_motor(peripherals),
                        Self::try_init_servo(peripherals),
                    );
                }
            };

            match VL53L0XService::new(i2c_driver) {
                Ok(service) => {
                    info!("VL53L0X sensor ready");
                    Some(Arc::new(service))
                }
                Err(e) => {
                    log::warn!("VL53L0X sensor init failed: {}", e);
                    None
                }
            }
        };

        let motor = Self::try_init_motor(peripherals);
        let servo = Self::try_init_servo(peripherals);
        (sensor, motor, servo)
    }

    fn try_init_motor(peripherals: &mut Peripherals) -> Option<Arc<MotorService>> {
        info!("Initializing DRV8833 motor service...");
        match MotorService::new(
            unsafe { peripherals.pins.gpio4.clone_unchecked() },
            unsafe { peripherals.pins.gpio5.clone_unchecked() },
        ) {
            Ok(service) => {
                info!("DRV8833 motor ready");
                Some(Arc::new(service))
            }
            Err(e) => {
                log::warn!("DRV8833 motor init failed: {}", e);
                None
            }
        }
    }

    fn try_init_servo(peripherals: &mut Peripherals) -> Option<Arc<ServoService>> {
        info!("Initializing DS-S006L servo service...");
        match ServoService::new(
            unsafe { peripherals.ledc.channel0.clone_unchecked() },
            unsafe { peripherals.ledc.timer0.clone_unchecked() },
            unsafe { peripherals.pins.gpio3.clone_unchecked() },
        ) {
            Ok(service) => {
                info!("DS-S006L servo ready");
                Some(Arc::new(service))
            }
            Err(e) => {
                log::warn!("DS-S006L servo init failed: {}", e);
                None
            }
        }
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

        // Configure hardware status for web service
        if self.sensor.is_some() || self.motor.is_some() || self.servo.is_some() {
            let sensor = self.sensor.clone();
            let motor = self.motor.clone();
            let servo = self.servo.clone();

            let distance = Arc::new(move || sensor.as_ref().map(|s| s.get_distance()).unwrap_or(0));
            let motor_for_speed = self.motor.clone();
            let motor_speed =
                Arc::new(move || motor_for_speed.as_ref().map(|m| m.get_speed()).unwrap_or(0));
            let motor_for_direction = self.motor.clone();
            let motor_direction = Arc::new(move || {
                motor_for_direction
                    .as_ref()
                    .map(|m| m.get_direction())
                    .unwrap_or(MotorDirection::Coast)
            });
            let motor_set = Arc::new(move |speed: u8, direction: MotorDirection| {
                motor
                    .as_ref()
                    .ok_or_else(|| "motor unavailable".to_string())
                    .and_then(|m| m.set(speed, direction))
            });
            let servo_for_angle = self.servo.clone();
            let servo_angle = Arc::new(move || {
                servo_for_angle
                    .as_ref()
                    .map(|s| s.get_angle())
                    .unwrap_or(90)
            });
            let servo_set = Arc::new(move |angle: u16| {
                servo
                    .as_ref()
                    .ok_or_else(|| "servo unavailable".to_string())
                    .and_then(|s| s.set_angle(angle))
            });

            let hw_status = HardwareStatus {
                distance,
                motor_speed,
                motor_direction,
                motor_set,
                servo_angle,
                servo_set,
            };

            self.web.set_hardware_status(hw_status);
            info!("Hardware status configured for web interface");
        }

        // Start Web server - serves the captive portal
        self.web.start()?;
        info!("Web server started");

        // Print system status
        Self::print_status(&self);

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
    fn print_status(system: &Self) {
        use crate::system::config::WIFI_SSID;

        info!("========================================");
        info!("Butterfly Captive Portal is Ready!");
        info!("========================================");
        info!("WiFi SSID: {} (no password)", WIFI_SSID);
        info!("IP Address: {}", AP_IP_ADDRESS);
        info!("DNS Server: {}:53", AP_IP_ADDRESS);
        info!("HTTP Server: http://{}", AP_IP_ADDRESS);

        if system.sensor.is_some() || system.motor.is_some() || system.servo.is_some() {
            info!("Hardware: ENABLED");
            info!(
                "  - VL53L0X sensor: {}",
                if system.sensor.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!(
                "  - DRV8833 motor: {}",
                if system.motor.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!(
                "  - DS-S006L servo: {}",
                if system.servo.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!("API Endpoints:");
            info!("  - http://{}/api/status", AP_IP_ADDRESS);
            info!("  - http://{}/api/distance", AP_IP_ADDRESS);
            info!("  - http://{}/api/motor", AP_IP_ADDRESS);
            info!("  - http://{}/api/servo", AP_IP_ADDRESS);
        } else {
            info!("Hardware: DISABLED (captive portal only)");
        }

        info!("========================================");
        info!("Connect to '{}' WiFi", WIFI_SSID);
        info!("Captive portal should auto-open!");
        info!("========================================");
    }
}
