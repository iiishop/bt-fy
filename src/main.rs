//! Butterfly Captive Portal - Main Entry Point
//!
//! This is the main entry point for the Butterfly Captive Portal application.
//! It simply initializes the system and runs it.
//!
//! All complexity is hidden behind the ButterflySystem abstraction.

mod dns;
mod system;
mod web;
mod wifi;

use esp_idf_hal::peripherals::Peripherals;
use system::ButterflySystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get ESP32 peripherals
    let peripherals = Peripherals::take()?;

    // Create and run the system
    // All service initialization and orchestration happens inside ButterflySystem
    let system = ButterflySystem::new(peripherals)?;
    system.run()?;

    // Never reached (system.run() blocks forever)
    Ok(())
}
