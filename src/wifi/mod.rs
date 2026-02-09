//! WiFi Service Module
//!
//! Manages the WiFi SoftAP functionality.
//! Creates and manages the wireless access point with captive portal configuration.

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    ipv4::{self, Mask, RouterConfiguration, Subnet},
    netif::{EspNetif, NetifConfiguration, NetifStack},
    wifi::{AuthMethod, Configuration, EspWifi},
};
use log::info;

use crate::system::config::{AP_IP_ADDRESS, SUBNET_MASK, WIFI_CHANNEL, WIFI_SSID};

/// WiFi Service - manages the SoftAP
///
/// Creates and manages a WiFi access point with DNS configuration
/// for captive portal functionality.
pub struct WifiService {
    _wifi: EspWifi<'static>,
}

impl WifiService {
    /// Create and start the WiFi service
    ///
    /// This creates the SoftAP with proper network configuration
    /// including DNS server settings.
    pub fn new(
        modem: impl esp_idf_hal::peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
        sysloop: EspSystemEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating WiFi service...");

        // Create netif with custom IP configuration including DNS server
        let ap_netif = EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: Some(ipv4::Configuration::Router(RouterConfiguration {
                subnet: Subnet {
                    gateway: AP_IP_ADDRESS,
                    mask: Mask(SUBNET_MASK),
                },
                dhcp_enabled: true,
                // Set DNS server to point to our own IP
                dns: Some(AP_IP_ADDRESS),
                secondary_dns: Some(AP_IP_ADDRESS),
            })),
            ..NetifConfiguration::wifi_default_router()
        })?;

        let mut wifi = EspWifi::wrap_all(
            esp_idf_svc::wifi::WifiDriver::new(modem, sysloop, None)?,
            EspNetif::new(NetifStack::Sta)?,
            ap_netif,
        )?;

        // Configure SoftAP
        let ap_config = Configuration::AccessPoint(esp_idf_svc::wifi::AccessPointConfiguration {
            ssid: heapless::String::<32>::try_from(WIFI_SSID).unwrap(),
            password: heapless::String::<64>::new(),
            channel: WIFI_CHANNEL,
            auth_method: AuthMethod::None,
            ..Default::default()
        });

        wifi.set_configuration(&ap_config)?;
        wifi.start()?;

        // Wait for AP to start
        std::thread::sleep(std::time::Duration::from_secs(2));

        info!("WiFi SoftAP '{}' started on {}", WIFI_SSID, AP_IP_ADDRESS);

        Ok(Self { _wifi: wifi })
    }

    /// Start the WiFi service (already started in new)
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // WiFi is already started in new()
        Ok(())
    }
}
