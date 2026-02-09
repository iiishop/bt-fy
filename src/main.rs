mod dns;
mod web;
mod wifi;

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::info;
use std::time::Duration;

use dns::SimpleDns;
use web::ButterflyWeb;
use wifi::{ButterflyAP, AP_IP_ADDRESS};

fn main() {
    // Link runtime patches
    esp_idf_svc::sys::link_patches();

    // Initialize logger
    EspLogger::initialize_default();

    info!("Starting Butterfly Captive Portal...");

    // Get peripherals
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();

    // Initialize NVS (required for WiFi calibration data)
    let nvs = EspDefaultNvsPartition::take().unwrap();
    info!("NVS initialized");

    // Start SoftAP with DNS configuration
    let _ap = ButterflyAP::new(peripherals.modem, sys_loop).expect("Failed to start SoftAP");

    // Start DNS server in background thread
    // CRITICAL: This must bind to the specific AP IP address, not 0.0.0.0
    info!("Starting DNS server...");
    let mut dns = SimpleDns::try_new(AP_IP_ADDRESS).expect("Failed to create DNS server");
    std::thread::spawn(move || {
        info!("DNS server thread started");
        loop {
            if let Err(e) = dns.poll() {
                log::error!("DNS poll error: {:?}", e);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    info!("DNS server started on {}:53", AP_IP_ADDRESS);

    // Start Web server with Captive Portal support
    let _web = ButterflyWeb::new(AP_IP_ADDRESS).expect("Failed to start web server");

    info!("========================================");
    info!("Butterfly Captive Portal is ready!");
    info!("========================================");
    info!("WiFi SSID: butterfly (no password)");
    info!("AP IP Address: {}", AP_IP_ADDRESS);
    info!("DNS Server: {}:53", AP_IP_ADDRESS);
    info!("HTTP Server: http://{}", AP_IP_ADDRESS);
    info!("========================================");
    info!("Connect any device to 'butterfly' WiFi");
    info!("The captive portal should auto-open!");
    info!("========================================");

    // Keep services alive and keep running
    let _nvs = nvs;
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
