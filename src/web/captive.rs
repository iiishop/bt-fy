use esp_idf_svc::http::{
    server::{EspHttpConnection, EspHttpServer, Request},
    Method,
};
use esp_idf_svc::io::EspIOError;
use std::net::Ipv4Addr;

/// Captive Portal detection handler
///
/// Different operating systems and devices use various endpoints to detect
/// captive portals (login pages for WiFi networks). This module handles all
/// common detection endpoints and redirects them to the main page.
///
/// Detection mechanisms:
/// - Apple (iOS/macOS): /hotspot-detect.html, /library/test/success.html
/// - Android: /gen_204, /generate_204
/// - Windows: /ncsi.txt, /check_network_status.txt, /connectivity-check.html, /fwlink
pub struct CaptivePortal;

impl CaptivePortal {
    /// Attach all captive portal detection handlers to the HTTP server
    pub fn attach<'a>(server: &mut EspHttpServer<'a>, addr: Ipv4Addr) -> Result<(), EspIOError> {
        let redirect_url = format!("http://{}", addr);

        // Apple iOS/macOS captive portal detection
        // iOS expects a 200 OK response with specific content, or a redirect
        Self::register_redirect(server, "/hotspot-detect.html", &redirect_url)?;
        Self::register_redirect(server, "/library/test/success.html", &redirect_url)?;

        // Android captive portal detection
        // Android expects a 204 No Content response for internet connectivity
        // We redirect instead to trigger captive portal detection
        Self::register_redirect(server, "/gen_204", &redirect_url)?;
        Self::register_redirect(server, "/generate_204", &redirect_url)?;

        // Windows captive portal detection (NCSI)
        // Windows Network Connectivity Status Indicator
        Self::register_redirect(server, "/ncsi.txt", &redirect_url)?;
        Self::register_redirect(server, "/check_network_status.txt", &redirect_url)?;
        Self::register_redirect(server, "/connectivity-check.html", &redirect_url)?;
        Self::register_redirect(server, "/fwlink", &redirect_url)?;

        Ok(())
    }

    /// Register a redirect handler for a specific path
    fn register_redirect<'a>(
        server: &mut EspHttpServer<'a>,
        path: &'static str,
        redirect_url: &str,
    ) -> Result<(), EspIOError> {
        let url = redirect_url.to_string();
        server.fn_handler(
            path,
            Method::Get,
            move |request: Request<&mut EspHttpConnection>| {
                request.into_response(302, Some("Found"), &[("Location", url.as_str())])?;
                Ok::<(), EspIOError>(())
            },
        )?;
        Ok(())
    }
}
