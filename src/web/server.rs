use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::io::{EspIOError, Write};
use log::{info, warn};
use std::net::Ipv4Addr;
use std::sync::Arc;

use crate::hardware::MotorDirection;

use super::captive::CaptivePortal;

const WELCOME_HTML: &str = include_str!("welcome.html");

pub struct ButterflyWeb {
    _server: EspHttpServer<'static>,
}

/// Hardware status data structure
#[derive(Clone)]
pub struct HardwareStatus {
    pub distance: Arc<dyn Fn() -> u16 + Send + Sync>,
    pub motor_speed: Arc<dyn Fn() -> u8 + Send + Sync>,
    pub motor_direction: Arc<dyn Fn() -> MotorDirection + Send + Sync>,
    pub motor_set: Arc<dyn Fn(u8, MotorDirection) -> Result<(), String> + Send + Sync>,
    pub servo_angle: Arc<dyn Fn() -> u16 + Send + Sync>,
    pub servo_set: Arc<dyn Fn(u16) -> Result<(), String> + Send + Sync>,
}

impl ButterflyWeb {
    pub fn new(
        ap_ip: Ipv4Addr,
        hw_status: Option<HardwareStatus>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Starting HTTP server...");

        // Configure HTTP server with larger buffer sizes for Captive Portal
        let config = Configuration {
            max_uri_handlers: 32,
            max_resp_headers: 8,
            max_sessions: 7,
            session_timeout: std::time::Duration::from_secs(20),
            stack_size: 8192,
            ..Default::default()
        };

        let mut server = EspHttpServer::new(&config)?;

        // Root handler - serves the welcome page
        server.fn_handler("/", esp_idf_svc::http::Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write_all(WELCOME_HTML.as_bytes())?;
            Ok::<(), EspIOError>(())
        })?;

        // API endpoints for hardware status
        if let Some(hw) = hw_status {
            let hw_clone = hw.clone();
            server.fn_handler(
                "/api/status",
                esp_idf_svc::http::Method::Get,
                move |request| {
                    let distance = (hw_clone.distance)();
                    let speed = (hw_clone.motor_speed)();
                    let direction = (hw_clone.motor_direction)().as_str();
                    let servo_angle = (hw_clone.servo_angle)();

                    let json = format!(
                        r#"{{"distance":{},"speed":{},"direction":"{}","servo_angle":{}}}"#,
                        distance, speed, direction, servo_angle
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

            // Motor status endpoint
            let hw_clone3 = hw.clone();
            server.fn_handler(
                "/api/motor",
                esp_idf_svc::http::Method::Get,
                move |request| {
                    let speed = (hw_clone3.motor_speed)();
                    let direction = (hw_clone3.motor_direction)().as_str();
                    let json = format!(
                        r#"{{"speed":{},"direction":"{}","status":"ok"}}"#,
                        speed, direction
                    );

                    let mut response = request.into_ok_response()?;
                    response.write_all(json.as_bytes())?;
                    Ok::<(), EspIOError>(())
                },
            )?;

            // Motor control endpoint
            let hw_clone4 = hw.clone();
            server.fn_handler(
                "/api/motor",
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
                    info!("/api/motor payload: {}", payload);

                    let speed = parse_speed(payload).unwrap_or_else(|| (hw_clone4.motor_speed)());
                    let direction =
                        parse_direction(payload).unwrap_or_else(|| (hw_clone4.motor_direction)());

                    info!(
                        "/api/motor parsed speed={}, direction={}",
                        speed,
                        direction.as_str()
                    );

                    match (hw_clone4.motor_set)(speed, direction) {
                        Ok(()) => {
                            let json = format!(
                                r#"{{"ok":true,"speed":{},"direction":"{}"}}"#,
                                speed,
                                direction.as_str()
                            );
                            let mut response = request.into_ok_response()?;
                            response.write_all(json.as_bytes())?;
                        }
                        Err(err) => {
                            warn!("/api/motor set failed: {}", err);
                            let json = format!(r#"{{"ok":false,"error":"{}"}}"#, err);
                            let mut response =
                                request.into_response(500, Some("Internal Server Error"), &[])?;
                            response.write_all(json.as_bytes())?;
                        }
                    }

                    Ok::<(), EspIOError>(())
                },
            )?;

            // Servo status endpoint
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

            // Servo control endpoint
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
                    info!("/api/servo payload: {}", payload);

                    let angle = parse_angle(payload).unwrap_or_else(|| (hw_clone6.servo_angle)());
                    info!("/api/servo parsed angle={}", angle);

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

            info!("Hardware API endpoints registered");
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

fn parse_speed(payload: &str) -> Option<u8> {
    let key = "\"speed\"";
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

    digits.parse::<u8>().ok().map(|n| n.min(100))
}

fn parse_direction(payload: &str) -> Option<MotorDirection> {
    let compact = payload
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();

    if compact.contains("\"direction\":\"forward\"") {
        Some(MotorDirection::Forward)
    } else if compact.contains("\"direction\":\"reverse\"") {
        Some(MotorDirection::Reverse)
    } else if compact.contains("\"direction\":\"brake\"") {
        Some(MotorDirection::Brake)
    } else if compact.contains("\"direction\":\"coast\"") {
        Some(MotorDirection::Coast)
    } else {
        None
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

    digits.parse::<u16>().ok().map(|n| n.min(300))
}
