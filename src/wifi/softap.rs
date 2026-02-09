use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::wifi::{AuthMethod, Configuration, EspWifi};
use log::info;

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

        let mut wifi = EspWifi::new(modem, sysloop, None)?;

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

        // Display network info
        if let Ok(ip_info) = wifi.ap_netif().get_ip_info() {
            info!("AP IP: {}", ip_info.ip);
            info!("Connect to WiFi: {}", ssid);
            info!("Then visit: http://{}", ip_info.ip);
        }

        Ok(Self { wifi })
    }

    #[allow(dead_code)]
    pub fn wifi(&self) -> &EspWifi<'a> {
        &self.wifi
    }

    #[allow(dead_code)]
    pub fn get_ip(&self) -> Option<std::net::Ipv4Addr> {
        self.wifi.ap_netif().get_ip_info().ok().map(|info| info.ip)
    }
}
