//! Hardware control module
//!
//! This module provides hardware control services for:
//! - VL53L0X distance sensor
//! - DS-S006L servo (GPIO3 and GPIO4)

pub mod servo;
pub mod vl53l0x;

pub use servo::ServoService;
pub use vl53l0x::VL53L0XService;
