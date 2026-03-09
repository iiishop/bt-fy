//! Butterfly Captive Portal - Main Entry Point
//!
//! This is the main entry point for the Butterfly Captive Portal application.
//! It initializes the system with automatic hardware fallback and runs it.
//!
//! If hardware initialization fails, the system continues in captive portal mode.

mod dns;
mod hardware;
mod protocol;
mod system;
mod web;
mod wifi;

use esp_idf_hal::peripherals::Peripherals;
use system::ButterflySystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get ESP32 peripherals (can only be taken once!)
    let peripherals = Peripherals::take()?;

    // Create system with automatic hardware fallback
    // ButterflySystem::new() will gracefully degrade if hardware fails
    let system = ButterflySystem::new(peripherals)?;

    // Run the system (blocks forever)
    system.run()?;

    // Never reached
    Ok(())
}
