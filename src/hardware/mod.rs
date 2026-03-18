//! Hardware control module
//!
//! This module provides hardware control services for:
//! - Time-of-Flight distance sensors (VL53L0X / VL53L1X)
//! - DS-S006L servo (GPIO3 and GPIO4)

pub mod servo;
pub mod continuous_servo;
pub mod vl53l0x;
pub mod vl53l1x;

pub use servo::ServoService;
pub use continuous_servo::ContinuousServoService;

/// 抽象 ToF 传感器接口，方便同时支持 VL53L0X / VL53L1X。
pub trait TofSensor: Send + Sync {
    /// 返回当前距离（单位 mm），异常时建议返回 0 或上次有效值。
    fn get_distance(&self) -> u16;
}

pub use vl53l0x::VL53L0XService;
