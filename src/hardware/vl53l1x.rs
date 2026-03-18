//! VL53L1X Distance Sensor Service
//!
//! 通过 vl53l1x-uld crate 驱动 VL53L1X 传感器，使其对上层暴露与 VL53L0XService 相同的接口（实现 TofSensor）。

use esp_idf_hal::{
    i2c::{I2cConfig, I2cDriver},
    peripheral::Peripheral,
    prelude::*,
};
use log::{error, info};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use vl53l1x_uld::{IOVoltage, RangeStatus, VL53L1X, DEFAULT_ADDRESS};

use crate::hardware::TofSensor;
use crate::system::config::VL53L0X_I2C_FREQUENCY as VL53L1X_I2C_FREQUENCY;

/// VL53L1X sensor service（对外实现 TofSensor）
pub struct VL53L1XService {
    distance: Arc<Mutex<u16>>,
}

impl VL53L1XService {
    /// 使用给定的 I2C 驱动创建 VL53L1X 服务
    pub fn new(i2c: I2cDriver<'static>) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing VL53L1X sensor...");

        let distance = Arc::new(Mutex::new(0u16));
        let distance_clone = Arc::clone(&distance);

        // VL53L1X::new 直接返回传感器实例（无 Result）
        let mut sensor = VL53L1X::new(i2c, DEFAULT_ADDRESS);

        // 初始化 + 启动测量
        sensor
            .init(IOVoltage::Volt2_8)
            .map_err(|e| format!("VL53L1X init failed: {:?}", e))?;
        sensor
            .start_ranging()
            .map_err(|e| format!("VL53L1X start ranging failed: {:?}", e))?;

        info!("VL53L1X initialized successfully in continuous mode");

        // 后台线程轮询测距
        thread::Builder::new()
            .name("vl53l1x".to_string())
            .spawn(move || {
                info!("VL53L1X reading thread started");
                loop {
                    match sensor.is_data_ready() {
                        Ok(true) => {
                            if let Ok(RangeStatus::Valid) = sensor.get_range_status() {
                                match sensor.get_distance() {
                                    Ok(mm) => {
                                        let mm_u16 = (mm as u32).min(u16::MAX as u32) as u16;
                                        if let Ok(mut dist) = distance_clone.lock() {
                                            *dist = mm_u16;
                                        }
                                    }
                                    Err(e) => {
                                        error!("VL53L1X get_distance failed: {:?}", e);
                                    }
                                }
                            }
                        }
                        Ok(false) => {}
                        Err(e) => {
                            error!("VL53L1X is_data_ready failed: {:?}", e);
                        }
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            })?;

        Ok(Self { distance })
    }
}

impl TofSensor for VL53L1XService {
    fn get_distance(&self) -> u16 {
        self.distance.lock().map(|d| *d).unwrap_or_else(|e| {
            error!("Failed to lock VL53L1X distance: {}", e);
            0
        })
    }
}

/// Helper：为 VL53L1X 创建 I2C 驱动（与 VL53L0X 共用频率配置）
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
    let config = I2cConfig::new().baudrate(VL53L1X_I2C_FREQUENCY.Hz());
    let driver = I2cDriver::new(i2c, sda, scl, &config)?;
    info!("I2C driver created for VL53L1X");
    Ok(driver)
}

