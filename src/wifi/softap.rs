use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::ipv4::{self, Mask, RouterConfiguration, Subnet};
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStack};
use esp_idf_svc::wifi::{AuthMethod, Configuration, EspWifi};
use log::info;
use std::net::Ipv4Addr;

// Captive Portal IP address
pub const AP_IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 71, 1);

pub struct ButterflyAP<'a> {
    #[allow(dead_code)]
    wifi: EspWifi<'a>,
}

impl<'a> ButterflyAP<'a> {
    pub fn new(
        modem: impl esp_idf_hal::peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'a,
        sysloop: EspSystemEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing Butterfly SoftAP...");

        // Create netif with custom IP configuration including DNS server
        let ap_netif = EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: Some(ipv4::Configuration::Router(RouterConfiguration {
                subnet: Subnet {
                    gateway: AP_IP_ADDRESS,
                    mask: Mask(24), // 255.255.255.0
                },
                dhcp_enabled: true,
                // CRITICAL: Set DNS server to point to our own IP
                // This allows the DNS server to intercept all DNS queries
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
        let ssid = "butterfly";
        let ap_config = Configuration::AccessPoint(esp_idf_svc::wifi::AccessPointConfiguration {
            ssid: heapless::String::<32>::try_from(ssid).unwrap(),
            password: heapless::String::<64>::new(),
            channel: 1,
            auth_method: AuthMethod::None,
            ..Default::default()
        });

        info!("Setting up SoftAP: '{}'", ssid);

        wifi.set_configuration(&ap_config)?;
        wifi.start()?;

        // Wait for AP to start
        std::thread::sleep(std::time::Duration::from_secs(2));

        info!("SoftAP '{}' is running!", ssid);
        info!("AP IP: {}", AP_IP_ADDRESS);
        info!("DNS Server: {}", AP_IP_ADDRESS);
        info!("Connect to WiFi: {}", ssid);
        info!("Then visit: http://{}", AP_IP_ADDRESS);

        Ok(Self { wifi })
    }

    #[allow(dead_code)]
    pub fn wifi(&self) -> &EspWifi<'a> {
        &self.wifi
    }

    #[allow(dead_code)]
    pub fn get_ip(&self) -> Ipv4Addr {
        AP_IP_ADDRESS
    }
}
