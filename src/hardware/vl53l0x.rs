//! VL53L0X Distance Sensor Service
//!
//! This module provides a service for reading distance from VL53L0X sensor via I2C.

use esp_idf_hal::{
    i2c::{I2cConfig, I2cDriver},
    peripheral::Peripheral,
    prelude::*,
};
use log::{error, info};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use vl53l0x::VL53L0x;

use crate::system::config::{VL53L0X_I2C_FREQUENCY, VL53L0X_SCL_PIN, VL53L0X_SDA_PIN};

/// VL53L0X sensor service
pub struct VL53L0XService {
    distance: Arc<Mutex<u16>>,
}

impl VL53L0XService {
    /// Create a new VL53L0X service with pre-configured I2C driver
    pub fn new(i2c: I2cDriver<'static>) -> Result<Self, Box<dyn std::error::Error>> {
        info!(
            "Initializing VL53L0X sensor on SDA={}, SCL={}",
            VL53L0X_SDA_PIN, VL53L0X_SCL_PIN
        );

        let distance = Arc::new(Mutex::new(0u16));
        let distance_clone = Arc::clone(&distance);

        // Initialize VL53L0X
        let mut sensor =
            VL53L0x::new(i2c).map_err(|e| format!("Failed to initialize VL53L0X: {:?}", e))?;

        // Set measurement timing budget (200ms for accurate readings); unit is microseconds
        if !sensor
            .set_measurement_timing_budget(200_000)
            .map_err(|e| format!("Failed to set timing budget: {:?}", e))?
        {
            return Err("Timing budget 200ms too small for current config".into());
        }

        // Start continuous ranging mode
        sensor
            .start_continuous(0)
            .map_err(|e| format!("Failed to start continuous mode: {:?}", e))?;

        info!("VL53L0X initialized successfully in continuous mode");

        // Spawn background thread to read sensor
        thread::Builder::new()
            .name("vl53l0x".to_string())
            .spawn(move || {
                info!("VL53L0X reading thread started");

                loop {
                    // Wait for measurement to be ready and read it (value is in mm)
                    match sensor.read_range_continuous_millimeters_blocking() {
                        Ok(raw_mm) => {
                            // VL53L0X uses 8190/8191 as "no object" / out-of-range; treat as 0
                            let valid_mm = if raw_mm >= 8190 { 0u16 } else { raw_mm };
                            if let Ok(mut dist) = distance_clone.lock() {
                                *dist = valid_mm;
                            }
                        }
                        Err(e) => {
                            error!("Failed to read VL53L0X: {:?}", e);
                            // Clear stale value so UI doesn't show a stuck reading
                            if let Ok(mut dist) = distance_clone.lock() {
                                *dist = 0;
                            }
                            thread::sleep(Duration::from_millis(50));
                        }
                    }

                    // Small delay between readings (sensor updates at ~5Hz with 200ms budget)
                    thread::sleep(Duration::from_millis(50));
                }
            })?;

        Ok(Self { distance })
    }

    /// Get the current distance reading in millimeters
    pub fn get_distance(&self) -> u16 {
        self.distance.lock().map(|d| *d).unwrap_or_else(|e| {
            error!("Failed to lock distance: {}", e);
            0
        })
    }
}

/// Helper function to create I2C driver for VL53L0X
pub fn create_i2c_driver<I, SDA, SCL>(
    i2c: impl Peripheral<P = I> + 'static,
    sda: impl Peripheral<P = SDA> + 'static,
    scl: impl Peripheral<P = SCL> + 'static,
) -> Result<I2cDriver<'static>, Box<dyn std::error::Error>>
where
    I: esp_idf_hal::i2c::I2c,
    SDA: esp_idf_hal::gpio::InputPin + esp_idf_hal::gpio::OutputPin,
    SCL: esp_idf_hal::gpio::InputPin + esp_idf_hal::gpio::OutputPin,
{
    let config = I2cConfig::new().baudrate(VL53L0X_I2C_FREQUENCY.Hz());
    let driver = I2cDriver::new(i2c, sda, scl, &config)?;
    info!("I2C driver created for VL53L0X");
    Ok(driver)
}
