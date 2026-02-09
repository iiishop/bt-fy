use log::{info, warn};
use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    time::Duration,
};

pub struct SimpleDns {
    footer: [u8; 16],
    socket: UdpSocket,
}

impl SimpleDns {
    /// Create a new DNS server bound to the specified IP address on port 53
    ///
    /// This DNS server acts as a wildcard DNS, responding to all queries
    /// with the same IP address (the captive portal IP).
    ///
    /// Key difference from previous implementation: we bind to a specific
    /// IP address (e.g., 192.168.71.1:53) instead of 0.0.0.0:53, which
    /// is not supported on ESP32.
    pub fn try_new(addr: Ipv4Addr) -> io::Result<Self> {
        info!("Starting DNS server on {}:53", addr);

        // CRITICAL: Bind to specific IP address, not 0.0.0.0
        // ESP32 doesn't support binding to wildcard address for DNS
        let socket = UdpSocket::bind(SocketAddrV4::new(addr, 53))?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;

        // Prepare DNS response footer with the IP address
        // This is the "Answer" section of the DNS response
        let mut footer = [
            0xc0, 0x0c, // Name pointer to query
            0x00, 0x01, // Type A (host address)
            0x00, 0x01, // Class IN (Internet)
            0x00, 0x00, 0x00, 0x0a, // TTL: 10 seconds
            0x00, 0x04, // Data length: 4 bytes (IPv4)
            0x00, 0x00, 0x00, 0x00, // IP address (will be filled below)
        ];
        footer[12..].copy_from_slice(&addr.octets());

        info!("DNS server started successfully");
        Ok(Self { footer, socket })
    }

    /// Poll for DNS queries and respond
    ///
    /// This should be called frequently (e.g., every 50ms) in a loop.
    /// All DNS queries will receive a response pointing to the captive portal IP.
    pub fn poll(&mut self) -> io::Result<()> {
        let mut scratch = [0; 128];
        match self.socket.recv_from(&mut scratch) {
            Ok((len, addr)) => {
                if len > 100 {
                    warn!("Received DNS request with invalid packet size: {}", len);
                } else {
                    // Modify the DNS query to be a response
                    scratch[2] |= 0x80; // Set QR bit (query/response): 1 = response
                    scratch[3] |= 0x80; // Set RA bit (recursion available)
                    scratch[7] = 0x01; // Set answer count to 1

                    // Append our answer section
                    let total = len + self.footer.len();
                    scratch[len..total].copy_from_slice(&self.footer);

                    // Send response back to client
                    self.socket.send_to(&scratch[0..total], addr)?;
                }
                Ok(())
            }
            Err(err) => match err.kind() {
                io::ErrorKind::TimedOut => Ok(()),
                io::ErrorKind::WouldBlock => Ok(()),
                _ => Err(err),
            },
        }
    }
}
