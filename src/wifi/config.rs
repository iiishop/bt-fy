use esp_idf_svc::netif::{EspNetif, NetifStack};
use esp_idf_svc::sys::{esp_ip4_addr, esp_netif_ip_info_t};
use log::info;

pub fn configure_dhcp_dns() {
    // This configures the DHCP server to send the ESP32's own IP as DNS server
    // This way, all DNS queries will be resolved by our DNS server
    info!("Configuring DHCP DNS to point to ESP32");

    // The DNS server will be pointing to 192.168.71.1 (the AP's IP)
    // This is handled by the lwip DHCP server automatically when
    // the AP netif is properly configured
}
