use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::io::{EspIOError, Write};
use log::info;
use std::net::Ipv4Addr;

use super::captive::CaptivePortal;

const WELCOME_HTML: &str = include_str!("welcome.html");

pub struct ButterflyWeb {
    _server: EspHttpServer<'static>,
}

impl ButterflyWeb {
    pub fn new(ap_ip: Ipv4Addr) -> Result<Self, Box<dyn std::error::Error>> {
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
