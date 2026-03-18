use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::io::{EspIOError, Write};
use log::{info, warn};
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc;

use crate::wifi::{ApEntry, WifiCommand, WifiResponse};

use super::captive::CaptivePortal;
use super::WifiCmdTx;

const WELCOME_HTML: &str = include_str!("welcome.html");
const DEBUG_HTML: &str = include_str!("debug.html");
const DASHBOARD_HTML: &str = include_str!("dashboard.html");

pub struct ButterflyWeb {
    _server: EspHttpServer<'static>,
}

/// Hardware status data structure
#[derive(Clone)]
pub struct HardwareStatus {
    pub distance: Arc<dyn Fn() -> u16 + Send + Sync>,
    pub servo_angle: Arc<dyn Fn() -> u16 + Send + Sync>,
    pub servo_set: Arc<dyn Fn(u16) -> Result<(), String> + Send + Sync>,
    pub servo2_angle: Arc<dyn Fn() -> u16 + Send + Sync>,
    pub servo2_set: Arc<dyn Fn(u16) -> Result<(), String> + Send + Sync>,
}

impl ButterflyWeb {
    pub fn new(
        ap_ip: Ipv4Addr,
        hw_status: Option<HardwareStatus>,
        wifi_cmd_tx: Option<WifiCmdTx>,
        test_mode: Option<Arc<AtomicBool>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Starting HTTP server...");

        let config = Configuration {
            max_uri_handlers: 40,
            max_resp_headers: 8,
            max_sessions: 7,
            session_timeout: std::time::Duration::from_secs(20),
            stack_size: 8192,
            ..Default::default()
        };

        let mut server = EspHttpServer::new(&config)?;

        // Root - new welcome (WiFi list / setup)
        server.fn_handler("/", esp_idf_svc::http::Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write_all(WELCOME_HTML.as_bytes())?;
            Ok::<(), EspIOError>(())
        })?;

        server.fn_handler("/debug.html", esp_idf_svc::http::Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write_all(DEBUG_HTML.as_bytes())?;
            Ok::<(), EspIOError>(())
        })?;

        server.fn_handler("/dashboard.html", esp_idf_svc::http::Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write_all(DASHBOARD_HTML.as_bytes())?;
            Ok::<(), EspIOError>(())
        })?;

        // WiFi API (only if channel provided)
        if let Some(tx) = wifi_cmd_tx {
            let tx_scan = tx.clone();
            server.fn_handler(
                "/api/wifi/scan",
                esp_idf_svc::http::Method::Get,
                move |_request| {
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = tx_scan.send((WifiCommand::Scan, reply_tx));
                    let json = match reply_rx.recv() {
                        Ok(WifiResponse::Scan(list)) => wifi_scan_json(&list),
                        _ => r#"{"networks":[]}"#.to_string(),
                    };
                    let mut response = _request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            let tx_status = tx.clone();
            server.fn_handler(
                "/api/wifi/status",
                esp_idf_svc::http::Method::Get,
                move |_request| {
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = tx_status.send((WifiCommand::GetStatus, reply_tx));
                    let json = match reply_rx.recv() {
                        Ok(WifiResponse::Status(Some(s))) => {
                            format!(r#"{{"connected":true,"ip":"{}","ssid":"{}"}}"#, s.ip, escape_json(&s.ssid))
                        }
                        _ => r#"{"connected":false}"#.to_string(),
                    };
                    let mut response = _request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            let tx_connect = tx.clone();
            server.fn_handler(
                "/api/wifi/connect",
                esp_idf_svc::http::Method::Post,
                move |mut request| {
                    let mut body = [0u8; 512];
                    let mut size = 0usize;
                    loop {
                        let read = request.read(&mut body[size..])?;
                        if read == 0 {
                            break;
                        }
                        size += read;
                        if size >= body.len() {
                            break;
                        }
                    }
                    let payload = std::str::from_utf8(&body[..size]).unwrap_or("");
                    let (ssid, password, username, auth) = parse_connect_body(payload);
                    let cmd = WifiCommand::Connect {
                        ssid,
                        password,
                        username,
                        auth,
                    };
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = tx_connect.send((cmd, reply_tx));
                    let (code, json) = match reply_rx.recv() {
                        Ok(WifiResponse::Connect(Ok(sta))) => (
                            200,
                            format!(
                                r#"{{"ok":true,"ip":"{}","ssid":"{}"}}"#,
                                sta.ip,
                                escape_json(&sta.ssid)
                            ),
                        ),
                        Ok(WifiResponse::Connect(Err(e))) => {
                            (400, format!(r#"{{"ok":false,"error":"{}"}}"#, escape_json(&e)))
                        }
                        _ => (500, r#"{"ok":false,"error":"timeout"}"#.to_string()),
                    };
                    let mut response = request.into_response(code, Some(if code == 200 { "OK" } else { "Error" }), &[])?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            let tx_stop = tx.clone();
            server.fn_handler(
                "/api/wifi/stop_ap",
                esp_idf_svc::http::Method::Post,
                move |request| {
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = tx_stop.send((WifiCommand::StopAp, reply_tx));
                    let (code, json) = match reply_rx.recv() {
                        Ok(WifiResponse::StopAp(Ok(()))) => (200, r#"{"ok":true}"#.to_string()),
                        Ok(WifiResponse::StopAp(Err(e))) => {
                            (500, format!(r#"{{"ok":false,"error":"{}"}}"#, escape_json(&e)))
                        }
                        _ => (500, r#"{"ok":false,"error":"timeout"}"#.to_string()),
                    };
                    let mut response = request.into_response(code, Some("OK"), &[])?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            info!("WiFi API endpoints registered");
        }

        // API endpoints for hardware status
        if let Some(hw) = hw_status {
            let hw_clone = hw.clone();
            server.fn_handler(
                "/api/status",
                esp_idf_svc::http::Method::Get,
                move |request| {
                    let distance = (hw_clone.distance)();
                    let servo_angle = (hw_clone.servo_angle)();
                    let servo2_angle = (hw_clone.servo2_angle)();

                    let json = format!(
                        r#"{{"distance":{},"servo_angle":{},"servo2_angle":{}}}"#,
                        distance, servo_angle, servo2_angle
                    );

                    let mut response = request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            let hw_clone2 = hw.clone();
            server.fn_handler(
                "/api/distance",
                esp_idf_svc::http::Method::Get,
                move |request| {
                    let distance = (hw_clone2.distance)();
                    let json = format!(r#"{{"distance":{}}}"#, distance);

                    let mut response = request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            // Servo 1 status endpoint
            let hw_clone5 = hw.clone();
            server.fn_handler(
                "/api/servo",
                esp_idf_svc::http::Method::Get,
                move |request| {
                    let angle = (hw_clone5.servo_angle)();
                    let json = format!(r#"{{"angle":{},"status":"ok"}}"#, angle);
                    let mut response = request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            // Servo 1 control endpoint
            let hw_clone6 = hw.clone();
            server.fn_handler(
                "/api/servo",
                esp_idf_svc::http::Method::Post,
                move |mut request| {
                    let mut body = [0u8; 256];
                    let mut size = 0usize;
                    loop {
                        let read = request.read(&mut body[size..])?;
                        if read == 0 {
                            break;
                        }
                        size += read;
                        if size >= body.len() {
                            break;
                        }
                    }

                    let payload = std::str::from_utf8(&body[..size]).unwrap_or("");
                    let angle = parse_angle(payload).unwrap_or_else(|| (hw_clone6.servo_angle)());

                    match (hw_clone6.servo_set)(angle) {
                        Ok(()) => {
                            let json = format!(r#"{{"ok":true,"angle":{}}}"#, angle);
                            let mut response = request.into_ok_response()?;
                            response.write_all(json.as_bytes())?;
                        }
                        Err(err) => {
                            warn!("/api/servo set failed: {}", err);
                            let json = format!(r#"{{"ok":false,"error":"{}"}}"#, err);
                            let mut response =
                                request.into_response(500, Some("Internal Server Error"), &[])?;
                            response.write_all(json.as_bytes())?;
                        }
                    }

                    Ok::<(), EspIOError>(())
                },
            )?;

            // Servo 2 status endpoint
            let hw_clone7 = hw.clone();
            server.fn_handler(
                "/api/servo2",
                esp_idf_svc::http::Method::Get,
                move |request| {
                    let angle = (hw_clone7.servo2_angle)();
                    let json = format!(r#"{{"angle":{},"status":"ok"}}"#, angle);
                    let mut response = request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            // Servo 2 control endpoint
            let hw_clone8 = hw.clone();
            server.fn_handler(
                "/api/servo2",
                esp_idf_svc::http::Method::Post,
                move |mut request| {
                    let mut body = [0u8; 256];
                    let mut size = 0usize;
                    loop {
                        let read = request.read(&mut body[size..])?;
                        if read == 0 {
                            break;
                        }
                        size += read;
                        if size >= body.len() {
                            break;
                        }
                    }

                    let payload = std::str::from_utf8(&body[..size]).unwrap_or("");
                    let angle = parse_angle(payload).unwrap_or_else(|| (hw_clone8.servo2_angle)());

                    match (hw_clone8.servo2_set)(angle) {
                        Ok(()) => {
                            let json = format!(r#"{{"ok":true,"angle":{}}}"#, angle);
                            let mut response = request.into_ok_response()?;
                            response.write_all(json.as_bytes())?;
                        }
                        Err(err) => {
                            warn!("/api/servo2 set failed: {}", err);
                            let json = format!(r#"{{"ok":false,"error":"{}"}}"#, err);
                            let mut response =
                                request.into_response(500, Some("Internal Server Error"), &[])?;
                            response.write_all(json.as_bytes())?;
                        }
                    }

                    Ok::<(), EspIOError>(())
                },
            )?;

            info!("Hardware API endpoints registered");
        }

        // Test mode (distance-triggered servo) on/off
        if let Some(flag) = test_mode {
            let test_flag = flag.clone();
            server.fn_handler(
                "/api/test-mode",
                esp_idf_svc::http::Method::Post,
                move |mut request| {
                    let mut body = [0u8; 64];
                    let mut size = 0usize;
                    while size < body.len() {
                        let read = request.read(&mut body[size..])?;
                        if read == 0 {
                            break;
                        }
                        size += read;
                    }
                    let payload = std::str::from_utf8(&body[..size]).unwrap_or("");
                    let on = payload.contains("\"on\":true") || payload.contains("\"on\": true");
                    test_flag.store(on, Ordering::Relaxed);
                    let json = format!(r#"{{"ok":true,"on":{}}}"#, on);
                    let mut response = request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;
        }

        // Favicon handler to prevent 404 errors
        server.fn_handler("/favicon.ico", esp_idf_svc::http::Method::Get, |request| {
            // Return empty 204 No Content for favicon
            request.into_response(204, Some("No Content"), &[])?;
            Ok::<(), EspIOError>(())
        })?;

        // Attach captive portal detection handlers
        // This handles all common OS captive portal detection endpoints
        CaptivePortal::attach(&mut server, ap_ip)?;

        // Fallback handler for any other request - redirect to welcome
        // This catches all the WeChat, QQ, and other app requests
        let redirect_url = format!("http://{}/", ap_ip);
        server.fn_handler("/*", esp_idf_svc::http::Method::Get, move |request| {
            request.into_response(302, Some("Found"), &[("Location", redirect_url.as_str())])?;
            Ok::<(), EspIOError>(())
        })?;

        info!("HTTP server started on port 80");
        info!("Captive Portal handlers registered");
        info!("All unknown paths redirect to: http://{}/", ap_ip);

        Ok(Self { _server: server })
    }
}

fn parse_angle(payload: &str) -> Option<u16> {
    let key = "\"angle\"";
    let idx = payload.find(key)?;
    let rest = &payload[idx + key.len()..];
    let colon = rest.find(':')?;

    let mut digits = String::new();
    for c in rest[colon + 1..].chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if !digits.is_empty() {
            break;
        }
    }

    if digits.is_empty() {
        return None;
    }

    digits.parse::<u16>().ok()
}

fn wifi_scan_json(list: &[ApEntry]) -> String {
    let parts: Vec<String> = list
        .iter()
        .map(|ap| {
            format!(
                r#"{{"ssid":"{}","rssi":{},"auth":"{}"}}"#,
                escape_json(&ap.ssid),
                ap.rssi,
                escape_json(&ap.auth)
            )
        })
        .collect();
    format!(r#"{{"networks":[{}]}}"#, parts.join(","))
}

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn parse_connect_body(payload: &str) -> (String, Option<String>, Option<String>, String) {
    let compact: String = payload
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let ssid = extract_json_string(&compact, "ssid").unwrap_or_default();
    let password = extract_json_string(&compact, "password");
    let username = extract_json_string(&compact, "username");
    let auth = extract_json_string(&compact, "auth").unwrap_or_else(|| "wpa2".to_string());
    (ssid, password, username, auth)
}

fn extract_json_string(compact: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start = compact.find(&needle)?;
    let rest = &compact[start + needle.len()..];
    let mut end = 0;
    let mut escape = false;
    for (i, c) in rest.chars().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '"' {
            end = i;
            break;
        }
    }
    let s = &rest[..end];
    let s = s.replace("\\\"", "\"").replace("\\\\", "\\");
    Some(s)
}
