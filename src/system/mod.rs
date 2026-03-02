//! Butterfly System - Main orchestration module
//!
//! This module provides the main application logic.
//! It coordinates all services (WiFi, DNS, Web, Hardware) and manages their lifecycle.

pub mod config;

use esp_idf_hal::{
    ledc::{self, config::TimerConfig, LedcTimerDriver},
    peripheral::Peripheral,
    peripherals::Peripherals,
    prelude::*,
};
use esp_idf_hal::ledc::LowSpeed;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger, nvs::EspDefaultNvsPartition};
use log::info;
use std::sync::Arc;

use crate::{
    dns::DnsService,
    hardware::{ServoService, VL53L0XService},
    system::config::{AP_IP_ADDRESS, SERVO2_PIN, SERVO_PIN},
    web::{HardwareStatus, WebService},
    wifi::{WifiCommand, WifiResponse, WifiService},
};
use std::sync::mpsc;
use std::time::Duration;

/// Main system that orchestrates all services
pub struct ButterflySystem {
    _nvs: EspDefaultNvsPartition,
    wifi: WifiService,
    wifi_cmd_rx: Option<mpsc::Receiver<(WifiCommand, mpsc::Sender<WifiResponse>)>>,
    dns: DnsService,
    web: WebService,
    sensor: Option<Arc<VL53L0XService>>,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ServoService>>,
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
        let (sensor, servo, servo2) = Self::try_init_hardware(&mut peripherals);

        // Now create core services (WiFi, DNS, Web)
        let wifi = WifiService::new(peripherals.modem, sys_loop)?;
        info!("WiFi service created");

        let (wifi_cmd_tx, wifi_cmd_rx) = mpsc::channel();
        let mut web = WebService::new(AP_IP_ADDRESS)?;
        web.set_wifi_cmd_tx(Some(wifi_cmd_tx));
        info!("Web service created");

        let dns = DnsService::new(AP_IP_ADDRESS)?;
        info!("DNS service created");

        if sensor.is_some() || servo.is_some() || servo2.is_some() {
            info!("✓ Hardware initialized successfully");
        } else {
            log::warn!("⚠ System running in CAPTIVE PORTAL ONLY mode");
            log::info!("→ WiFi and web interface still available");
        }

        Ok(Self {
            _nvs: nvs,
            wifi,
            wifi_cmd_rx: Some(wifi_cmd_rx),
            dns,
            web,
            sensor,
            servo,
            servo2,
        })
    }

    /// Try to initialize hardware (VL53L0X + two servos on GPIO3 and GPIO4)
    /// Order: TOF first (I2C + background thread), then LEDC timer and servos, so I2C is stable.
    fn try_init_hardware(
        peripherals: &mut Peripherals,
    ) -> (
        Option<Arc<VL53L0XService>>,
        Option<Arc<ServoService>>,
        Option<Arc<ServoService>>,
    ) {
        // 1) Init VL53L0X first so I2C and its reading thread start before any LEDC/GPIO3/4 activity
        let sensor = {
            info!("Initializing VL53L0X sensor...");

            let i2c_driver = match crate::hardware::vl53l0x::create_i2c_driver(
                unsafe { peripherals.i2c0.clone_unchecked() },
                unsafe { peripherals.pins.gpio6.clone_unchecked() },
                unsafe { peripherals.pins.gpio7.clone_unchecked() },
            ) {
                Ok(driver) => driver,
                Err(e) => {
                    log::warn!("VL53L0X I2C init failed: {}", e);
                    return (None, None, None);
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

        // 2) Then shared LEDC timer and servos (GPIO3/4)
        const SERVO_PWM_HZ: u32 = 50;
        let timer_cfg = TimerConfig::new()
            .frequency(SERVO_PWM_HZ.Hz())
            .resolution(ledc::config::Resolution::Bits12);
        let timer_driver = match LedcTimerDriver::new(
            unsafe { peripherals.ledc.timer0.clone_unchecked() },
            &timer_cfg,
        ) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("LEDC timer init failed: {}", e);
                return (sensor, None, None);
            }
        };
        let shared_timer = Box::leak(Box::new(timer_driver));

        let servo = Self::try_init_servo(peripherals, shared_timer);
        let servo2 = Self::try_init_servo2(peripherals, shared_timer);
        (sensor, servo, servo2)
    }

    fn try_init_servo<T>(
        peripherals: &mut Peripherals,
        timer: &'static LedcTimerDriver<'static, T>,
    ) -> Option<Arc<ServoService>>
    where
        T: ledc::LedcTimer<SpeedMode = LowSpeed> + 'static,
    {
        match ServoService::new_with_shared_timer(
            unsafe { peripherals.ledc.channel0.clone_unchecked() },
            timer,
            unsafe { peripherals.pins.gpio3.clone_unchecked() },
            SERVO_PIN,
        ) {
            Ok(service) => {
                info!("DS-S006L servo (GPIO{}) ready", SERVO_PIN);
                Some(Arc::new(service))
            }
            Err(e) => {
                log::warn!("DS-S006L servo (GPIO{}) init failed: {}", SERVO_PIN, e);
                None
            }
        }
    }

    fn try_init_servo2<T>(
        peripherals: &mut Peripherals,
        timer: &'static LedcTimerDriver<'static, T>,
    ) -> Option<Arc<ServoService>>
    where
        T: ledc::LedcTimer<SpeedMode = LowSpeed> + 'static,
    {
        match ServoService::new_with_shared_timer(
            unsafe { peripherals.ledc.channel1.clone_unchecked() },
            timer,
            unsafe { peripherals.pins.gpio4.clone_unchecked() },
            SERVO2_PIN,
        ) {
            Ok(service) => {
                info!("DS-S006L servo (GPIO{}) ready", SERVO2_PIN);
                Some(Arc::new(service))
            }
            Err(e) => {
                log::warn!("DS-S006L servo (GPIO{}) init failed: {}", SERVO2_PIN, e);
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
        if self.sensor.is_some() || self.servo.is_some() || self.servo2.is_some() {
            let sensor = self.sensor.clone();
            let servo = self.servo.clone();
            let servo2 = self.servo2.clone();

            let distance = Arc::new(move || sensor.as_ref().map(|s| s.get_distance()).unwrap_or(0));
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
            let servo2_for_angle = self.servo2.clone();
            let servo2_angle = Arc::new(move || {
                servo2_for_angle
                    .as_ref()
                    .map(|s| s.get_angle())
                    .unwrap_or(90)
            });
            let servo2_set = Arc::new(move |angle: u16| {
                servo2
                    .as_ref()
                    .ok_or_else(|| "servo2 unavailable".to_string())
                    .and_then(|s| s.set_angle(angle))
            });

            let hw_status = HardwareStatus {
                distance,
                servo_angle,
                servo_set,
                servo2_angle,
                servo2_set,
            };

            self.web.set_hardware_status(hw_status);
            info!("Hardware status configured for web interface");
        }

        // Start Web server - serves the captive portal
        self.web.start()?;
        info!("Web server started");

        // Print system status
        Self::print_status(&self);

        // Command loop: process WiFi commands from HTTP handlers
        let mut wifi = self.wifi;
        let wifi_cmd_rx = self.wifi_cmd_rx.take().expect("wifi_cmd_rx");
        info!("System is running.");
        loop {
            match wifi_cmd_rx.recv_timeout(Duration::from_secs(1)) {
                Ok((cmd, reply_tx)) => {
                    let response = wifi.execute(cmd);
                    let _ = reply_tx.send(response);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
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
        info!("========================================");
        info!("Butterfly Captive Portal is Ready!");
        info!("========================================");
        info!("WiFi SSID: {} (no password)", system.wifi.ap_ssid());
        info!("IP Address: {}", AP_IP_ADDRESS);
        info!("DNS Server: {}:53", AP_IP_ADDRESS);
        info!("HTTP Server: http://{}", AP_IP_ADDRESS);

        if system.sensor.is_some() || system.servo.is_some() || system.servo2.is_some() {
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
                "  - Servo (GPIO3): {}",
                if system.servo.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!(
                "  - Servo (GPIO4): {}",
                if system.servo2.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!("API Endpoints:");
            info!("  - http://{}/api/status", AP_IP_ADDRESS);
            info!("  - http://{}/api/distance", AP_IP_ADDRESS);
            info!("  - http://{}/api/servo", AP_IP_ADDRESS);
            info!("  - http://{}/api/servo2", AP_IP_ADDRESS);
        } else {
            info!("Hardware: DISABLED (captive portal only)");
        }

        info!("========================================");
        info!("Connect to '{}' WiFi", system.wifi.ap_ssid());
        info!("Captive portal should auto-open!");
        info!("========================================");
    }
}
