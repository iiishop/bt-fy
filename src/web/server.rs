use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::io::Write;
use log::info;
use anyhow::Error;

const WELCOME_HTML: &str = include_str!("welcome.html");

pub struct ButterflyWeb {
    _server: EspHttpServer<'static>,
}

impl ButterflyWeb {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        info!("Starting HTTP server...");

        let config = Configuration::default();
        let mut server = EspHttpServer::new(&config)?;

        // Root handler - serves the welcome page
        server.fn_handler("/", esp_idf_svc::http::Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write_all(WELCOME_HTML.as_bytes())?;
            Ok::<(), Error>(())
        })?;

        // Captive Portal: Apple's hotspot-detect
        server.fn_handler(
            "/hotspot-detect.html",
            esp_idf_svc::http::Method::Get,
            |request| {
                let mut response = request.into_ok_response()?;
                response.write_all(WELCOME_HTML.as_bytes())?;
                Ok::<(), Error>(())
            },
        )?;

        // Captive Portal: Android's connectivity check
        server.fn_handler("/gen_204", esp_idf_svc::http::Method::Get, |request| {
            request.into_response(302, Some("Found"), &[("Location", "http://192.168.71.1/")])?;
            Ok::<(), Error>(())
        })?;

        // Captive Portal: Android variant
        server.fn_handler("/generate_204", esp_idf_svc::http::Method::Get, |request| {
            request.into_response(302, Some("Found"), &[("Location", "http://192.168.71.1/")])?;
            Ok::<(), Error>(())
        })?;

        // Captive Portal: Windows connectivity check
        server.fn_handler("/ncsi.txt", esp_idf_svc::http::Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write_all(b"Microsoft NCSI")?;
            Ok::<(), Error>(())
        })?;

        // Fallback handler for any other request - redirect to welcome
        server.fn_handler("/*", esp_idf_svc::http::Method::Get, |request| {
            request.into_response(302, Some("Found"), &[("Location", "http://192.168.71.1/")])?;
            Ok::<(), Error>(())
        })?;

        info!("HTTP server started on port 80");
        info!("Captive Portal handlers registered");

        Ok(Self { _server: server })
    }
}
