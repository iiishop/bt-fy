//! WiFi Service Module
//!
//! Manages the WiFi SoftAP and STA (scan, connect, stop AP).
//! Uses Mixed (APSTA) mode so we can scan while AP is running.

use esp_idf_svc::{
    eventloop::{EspSystemEventLoop, Wait},
    ipv4::{self, Mask, RouterConfiguration, Subnet},
    netif::{EspNetif, NetifConfiguration, NetifStack},
    wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiEvent},
};
use log::{info, warn};
use std::time::Duration;

use crate::system::config::{AP_IP_ADDRESS, SUBNET_MASK, WIFI_CHANNEL, WIFI_SSID_PREFIX};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// WiFi command sent from HTTP handlers to main thread
#[derive(Debug)]
pub enum WifiCommand {
    Scan,
    Connect {
        ssid: String,
        password: Option<String>,
        username: Option<String>,
        auth: String,
    },
    GetStatus,
    StopAp,
}

/// One AP entry for JSON (scan result)
#[derive(Debug)]
pub struct ApEntry {
    pub ssid: String,
    pub rssi: i8,
    pub auth: String,
}

/// STA connection info for JSON
#[derive(Clone, Debug)]
pub struct StaInfo {
    pub ip: String,
    pub ssid: String,
}

/// Response to WifiCommand
#[derive(Debug)]
pub enum WifiResponse {
    Scan(Vec<ApEntry>),
    Connect(Result<StaInfo, String>),
    Status(Option<StaInfo>),
    StopAp(Result<(), String>),
}

fn auth_to_str(auth: AuthMethod) -> &'static str {
    match auth {
        AuthMethod::None => "open",
        AuthMethod::WEP => "wep",
        AuthMethod::WPA => "wpa",
        AuthMethod::WPA2Personal => "wpa2",
        AuthMethod::WPAWPA2Personal => "wpa_wpa2",
        AuthMethod::WPA2Enterprise => "wpa2_enterprise",
        AuthMethod::WPA3Personal => "wpa3",
        AuthMethod::WPA2WPA3Personal => "wpa2_wpa3",
        AuthMethod::WAPIPersonal => "wapi",
        _ => "unknown",
    }
}

fn parse_auth(s: &str) -> AuthMethod {
    match s.to_ascii_lowercase().as_str() {
        "open" | "none" => AuthMethod::None,
        "wep" => AuthMethod::WEP,
        "wpa" => AuthMethod::WPA,
        "wpa2" | "wpa2_psk" => AuthMethod::WPA2Personal,
        "wpa_wpa2" => AuthMethod::WPAWPA2Personal,
        "wpa2_enterprise" | "enterprise" => AuthMethod::WPA2Enterprise,
        "wpa3" => AuthMethod::WPA3Personal,
        "wpa2_wpa3" => AuthMethod::WPA2WPA3Personal,
        _ => AuthMethod::WPA2Personal,
    }
}

trait ToAsciiLowercase {
    fn to_ascii_lowercase(&self) -> String;
}
impl ToAsciiLowercase for str {
    fn to_ascii_lowercase(&self) -> String {
        self.chars().map(|c| c.to_ascii_lowercase()).collect()
    }
}

/// Read MAC from eFuse (fallback when wifi.get_mac not yet available)
fn read_mac_efuse() -> Result<[u8; 6], Box<dyn std::error::Error>> {
    let mut mac = [0u8; 6];
    let err = unsafe {
        esp_idf_svc::sys::esp_read_mac(
            mac.as_mut_ptr() as *mut _,
            esp_idf_svc::sys::esp_mac_type_t_ESP_MAC_WIFI_STA,
        )
    };
    if err == 0 {
        Ok(mac)
    } else {
        Err(format!("esp_read_mac failed: {}", err).into())
    }
}

/// WiFi Service - SoftAP + scan/connect/stop AP
pub struct WifiService {
    wifi: EspWifi<'static>,
    event_loop: EspSystemEventLoop,
    ap_config: esp_idf_svc::wifi::AccessPointConfiguration,
}

impl WifiService {
    /// Create and start the WiFi service (Mixed mode: AP + STA for scan)
    pub fn new(
        modem: impl esp_idf_hal::peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
        sysloop: EspSystemEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating WiFi service...");

        let event_loop = sysloop.clone();

        let ap_netif = EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: Some(ipv4::Configuration::Router(RouterConfiguration {
                subnet: Subnet {
                    gateway: AP_IP_ADDRESS,
                    mask: Mask(SUBNET_MASK),
                },
                dhcp_enabled: true,
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

        // Use device MAC (factory-burned, immutable) to make SSID unique: butterfly-XXXXXXXX
        let mac = wifi
            .get_mac(esp_idf_svc::wifi::WifiDeviceId::Sta)
            .or_else(|_| read_mac_efuse())
            .unwrap_or([0u8; 6]);
        let ssid = format!(
            "{}{:02X}{:02X}{:02X}{:02X}",
            WIFI_SSID_PREFIX,
            mac[2],
            mac[3],
            mac[4],
            mac[5]
        );
        let ssid_hl =
            heapless::String::try_from(ssid.as_str()).unwrap_or_else(|_| heapless::String::new());

        let ap_config = esp_idf_svc::wifi::AccessPointConfiguration {
            ssid: ssid_hl.clone(),
            password: heapless::String::<64>::new(),
            channel: WIFI_CHANNEL,
            auth_method: AuthMethod::None,
            ..Default::default()
        };

        // Mixed mode so we can scan while AP is running
        let mixed = Configuration::Mixed(
            ClientConfiguration::default(),
            ap_config.clone(),
        );
        wifi.set_configuration(&mixed)?;
        wifi.start()?;

        std::thread::sleep(std::time::Duration::from_secs(2));
        info!(
            "WiFi SoftAP '{}' started on {} (Mixed mode for scan)",
            ssid, AP_IP_ADDRESS
        );

        Ok(Self {
            wifi,
            event_loop,
            ap_config,
        })
    }

    /// Start the WiFi service (already started in new)
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// AP SSID (with MAC suffix), e.g. ESP_A1B2C3D4
    pub fn ap_ssid(&self) -> &str {
        self.ap_config.ssid.as_str()
    }

    /// 设备唯一 ID（MAC 字符串，与 App 协议一致）
    pub fn get_device_id(&self) -> String {
        let mac = self
            .wifi
            .get_mac(esp_idf_svc::wifi::WifiDeviceId::Sta)
            .or_else(|_| read_mac_efuse())
            .unwrap_or([0u8; 6]);
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        )
    }

    /// Execute a WiFi command (called from main thread only)
    pub fn execute(&mut self, cmd: WifiCommand) -> WifiResponse {
        match cmd {
            WifiCommand::Scan => self.do_scan(),
            WifiCommand::Connect {
                ssid,
                password,
                username: _,
                auth,
            } => self.do_connect(ssid, password, auth),
            WifiCommand::GetStatus => self.do_status(),
            WifiCommand::StopAp => self.do_stop_ap(),
        }
    }

    fn do_scan(&mut self) -> WifiResponse {
        match self.wifi.scan() {
            Ok(list) => {
                let entries: Vec<ApEntry> = list
                    .into_iter()
                    .map(|ap| ApEntry {
                        ssid: ap.ssid.to_string(),
                        rssi: ap.signal_strength,
                        auth: auth_to_str(ap.auth_method.unwrap_or(AuthMethod::None)).to_string(),
                    })
                    .collect();
                WifiResponse::Scan(entries)
            }
            Err(e) => {
                warn!("WiFi scan failed: {:?}", e);
                WifiResponse::Scan(vec![])
            }
        }
    }

    fn do_connect(&mut self, ssid: String, password: Option<String>, auth: String) -> WifiResponse {
        let auth_method = parse_auth(&auth);
        let ssid_hl = heapless::String::try_from(ssid.as_str()).unwrap_or_else(|_| heapless::String::new());
        let password_hl = password
            .as_deref()
            .and_then(|s| heapless::String::try_from(s).ok())
            .unwrap_or_else(heapless::String::new);

        let sta_conf = ClientConfiguration {
            ssid: ssid_hl,
            password: password_hl,
            auth_method,
            ..Default::default()
        };

        let mixed = Configuration::Mixed(sta_conf, self.ap_config.clone());
        if let Err(e) = self.wifi.set_configuration(&mixed) {
            return WifiResponse::Connect(Err(format!("set_configuration: {:?}", e)));
        }
        if let Err(e) = self.wifi.start() {
            return WifiResponse::Connect(Err(format!("start: {:?}", e)));
        }
        self.wifi.connect().ok();

        let wait = match Wait::new::<WifiEvent>(&self.event_loop) {
            Ok(w) => w,
            Err(e) => return WifiResponse::Connect(Err(format!("Wait: {:?}", e))),
        };
        let ok = wait
            .wait_while(
                || self.wifi.is_connected().map(|c| !c),
                Some(CONNECT_TIMEOUT),
            )
            .is_ok();

        if !ok {
            return WifiResponse::Connect(Err("Connection timeout".to_string()));
        }

        // Wait for DHCP: poll until we get a non-zero IP (some routers need >2s)
        for _ in 0..12 {
            std::thread::sleep(Duration::from_secs(1));
            if let WifiResponse::Status(Some(ref sta)) = self.do_status() {
                if !sta.ip.is_empty() && sta.ip != "0.0.0.0" {
                    return WifiResponse::Connect(Ok(sta.clone()));
                }
            }
        }
        WifiResponse::Connect(Err("DHCP timeout (no IP)".to_string()))
    }

    fn do_status(&mut self) -> WifiResponse {
        let connected = match self.wifi.is_connected() {
            Ok(c) => c,
            Err(_) => return WifiResponse::Status(None),
        };
        if !connected {
            return WifiResponse::Status(None);
        }
        let ip_info = match self.wifi.sta_netif().get_ip_info() {
            Ok(info) => info,
            Err(_) => return WifiResponse::Status(None),
        };
        let ip = ip_info.ip;
        let ip_str = ip.to_string();
        let conf = match self.wifi.get_configuration() {
            Ok(c) => c,
            Err(_) => return WifiResponse::Status(None),
        };
        let ssid = match &conf {
            Configuration::Client(c) => c.ssid.to_string(),
            Configuration::Mixed(c, _) => c.ssid.to_string(),
            _ => String::new(),
        };
        WifiResponse::Status(Some(StaInfo {
            ip: ip_str,
            ssid,
        }))
    }

    fn do_stop_ap(&mut self) -> WifiResponse {
        let conf = match self.wifi.get_configuration() {
            Ok(c) => c,
            Err(e) => return WifiResponse::StopAp(Err(format!("get_config: {:?}", e))),
        };
        let sta_conf = match &conf {
            Configuration::Client(c) => c.clone(),
            Configuration::Mixed(c, _) => c.clone(),
            _ => return WifiResponse::StopAp(Err("No STA config".to_string())),
        };
        let sta_only = Configuration::Client(sta_conf);
        match self.wifi.set_configuration(&sta_only) {
            Ok(()) => {}
            Err(e) => return WifiResponse::StopAp(Err(format!("set_config: {:?}", e))),
        }
        match self.wifi.start() {
            Ok(()) => {
                info!("SoftAP stopped, STA only mode");
                WifiResponse::StopAp(Ok(()))
            }
            Err(e) => WifiResponse::StopAp(Err(format!("start: {:?}", e))),
        }
    }
}
