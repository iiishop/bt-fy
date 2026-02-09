mod web;
mod wifi;

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::info;

use web::ButterflyWeb;
use wifi::ButterflyAP;

fn main() {
    // Link runtime patches
    esp_idf_svc::sys::link_patches();

    // Initialize logger
    EspLogger::initialize_default();

    info!("Starting Butterfly...");

    // Get peripherals
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();

    // Initialize NVS (required for WiFi calibration data)
    let nvs = EspDefaultNvsPartition::take().unwrap();
    info!("NVS initialized");

    // Start SoftAP
    let _ap = ButterflyAP::new(peripherals.modem, sys_loop).expect("Failed to start SoftAP");

    // Start Web server (with Captive Portal support)
    let _web = ButterflyWeb::new().expect("Failed to start web server");

    info!("Butterfly is ready!");
    info!("Connect to WiFi: butterfly");
    info!("Your device should automatically show the welcome page");
    info!("Or visit: http://192.168.71.1");

    // Keep services alive and keep running
    let _nvs = nvs;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
